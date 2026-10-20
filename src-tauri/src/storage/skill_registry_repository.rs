use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{Mutex, OnceLock},
    time::Instant,
};

use ring::digest::{Context, SHA256};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::AppDatabase;

const FTS_FINGERPRINT_META_KEY: &str = "skill_registry_fts_fingerprint_v2";

/// Process-level cache of database paths whose FTS5 content consistency has been verified.
/// Prevents running a full-table SHA-256 consistency scan on every search keystroke.
static VERIFIED_FTS_DBS: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();

fn verified_fts_dbs() -> &'static Mutex<HashSet<PathBuf>> {
    VERIFIED_FTS_DBS.get_or_init(|| Mutex::new(HashSet::new()))
}

fn mark_db_fts_verified(db_path: & std::path::Path) {
    if let Ok(mut guard) = verified_fts_dbs().lock() {
        guard.insert(db_path.to_path_buf());
    }
}

fn unmark_db_fts_verified(db_path: &std::path::Path) {
    if let Ok(mut guard) = verified_fts_dbs().lock() {
        guard.remove(db_path);
    }
}

#[derive(Debug, Clone)]
pub struct SkillIndexFields {
    pub title_zh: String,
    pub category: String,
    pub tags_text: String,
    pub guide_text: String,
    pub repo_key: String,
    pub repo_stars: i64,
    pub spec_score: i64,
    pub name_tokens: String,
    pub title_tokens: String,
    pub task_tokens: String,
    pub guide_tokens: String,
    pub desc_tokens: String,
}

#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct SkillRegistryFilter<'a> {
    pub stage: Option<&'a str>,
    pub category: Option<&'a str>,
    pub allowed_ids: Option<&'a HashSet<String>>,
    pub limit: usize,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SkillRegistryCandidateRow {
    pub id: String,
    pub name: String,
    pub title_zh: String,
    pub description: String,
    pub stage: String,
    pub category: String,
    pub tags_text: String,
    pub guide_text: String,
    pub repo_key: String,
    pub repo_stars: i64,
    pub spec_score: i64,
    pub payload_json: String,
    pub bm25_raw: f64,
}

// ============================================================================
// v3.2 Search Request / Response & Match Reason Data Structures
// ============================================================================

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillSearchRequest {
    pub query: String,
    pub stage: Option<String>,
    pub category: Option<String>,
    pub allowed_ids: Option<Vec<String>>,
    #[serde(default)]
    pub project_tech_stack: Vec<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub sort: Option<String>,
    pub catalog_revision: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillSearchMatchReason {
    /// `"exact"` (完全相关) | `"general"` (通用可用) | `"browse"` (目录浏览)
    pub tier: String,
    /// Concise human-readable explanation of why this skill matched
    pub summary: String,
    /// Fields hit with high confidence, e.g. `["name", "titleZh", "tags"]`
    pub matched_fields: Vec<String>,
    /// Query intent slots matched by this skill
    pub matched_terms: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillSearchHit {
    pub id: String,
    pub name: String,
    pub title_zh: String,
    pub score: f64,
    pub core_matched: usize,
    pub core_total: usize,
    pub tech_core_matched: usize,
    pub non_tech_core_matched: usize,
    pub match_reason: SkillSearchMatchReason,
    pub skill: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillSearchResponse {
    pub hits: Vec<SkillSearchHit>,
    pub total_matched: usize,
    /// `"browse"` | `"exact"` | `"general_fallback"` | `"none"`
    pub match_tier: String,
    /// Banner notice when `match_tier == "general_fallback"` or when fallback search was used
    pub fallback_notice: Option<String>,
    pub parsed_terms: Vec<String>,
    pub used_fallback_engine: bool,
    pub elapsed_ms: f64,
    #[serde(default)]
    pub has_more: bool,
    #[serde(default)]
    pub is_truncated: bool,
    pub catalog_revision: Option<String>,
}

// ============================================================================
// Tokenizer, Synonym Tiering, and Mixed CJK/EN Query Parser (v3.2)
// ============================================================================

const FIXED_ALNUM_TERMS: &[&str] = &[
    "i18n", "l10n", "a11y", "k8s", "e2e", "s3", "v8", "base64", "sha256", "oauth2", "http2", "3d", "2d",
];

const CN_DOMAIN_TERMS: &[&str] = &[
    "视觉回归测试", "自动化测试", "单元测试", "集成测试", "端到端测试",
    "状态管理", "代码审查", "代码评审", "代码质量", "视觉回归", "内存泄漏", "内存溢出",
    "权限控制", "性能分析", "性能优化", "性能瓶颈", "构建打包", "接口测试", "慢查询",
    "安全审计", "架构设计", "无障碍", "可访问性", "微前端",
    "国际化", "多语言", "本地化", "数据库", "组件库", "命令行", "工作流",
    "浏览器", "暗黑模式", "响应式", "流程图", "可视化",
    "组件", "视觉", "界面", "样式", "排版", "主题", "暗黑", "设计", "前端", "后端",
    "测试", "单测", "验收", "回归", "截图", "调试", "排查", "排错", "诊断", "故障", "日志",
    "重构", "架构", "规范", "需求", "接口", "路由", "表单", "表格", "图表", "动画",
    "部署", "发布", "容器", "打包", "构建", "安全", "审计", "加密", "鉴权", "登录", "权限",
    "文档", "翻译", "注释", "脚本", "爬虫", "缓存", "并发", "异步", "内存", "性能", "瓶颈", "优化",
    "迁移", "漏洞", "提交", "分支", "质量", "代码", "美化",
];

const CN_STOP_PHRASES: &[&str] = &[
    "帮我给", "帮我", "请问如何", "请问", "如何", "怎么", "怎样", "我想给", "我想",
    "需要一个", "需要", "寻找", "查找", "添加", "增加", "实现", "支持", "进行", "编写",
    "相关的", "相关", "的问题", "问题", "的方案", "方案", "的功能", "功能", "的项目", "项目",
    "线上的", "线上", "做一次", "一次", "一套", "好看的", "好看",
    "提交代码前", "提交代码后", "代码提交前", "代码提交后", "代码前", "代码后",
    "提交前", "提交后", "上线前", "上线后", "发布前", "发布后", "构建前", "构建后",
    "部署前", "部署后", "发版前", "发版后", "测试前", "测试后",
    "之前帮我", "之后帮我", "前帮我", "后帮我", "之前", "之后",
    "的", "了", "在", "中", "里", "和", "与", "或", "及", "给", "为", "对", "把", "将", "做", "用", "写",
];

const MODIFIER_TERMS: &[&str] = &[
    "修复", "fix", "优化", "optimize", "规范", "standard", "convention", "guide", "helper", "tool", "workflow", "hook", "hooks",
    "美化", "beautify", "polish",
];

const KNOWN_TECH_STACKS: &[&str] = &[
    "vue", "vue2", "vue3", "react", "angular", "svelte", "nextjs", "nuxt", "vite", "tailwind",
    "typescript", "javascript", "node", "nodejs", "rust", "cargo", "tauri", "python", "fastapi",
    "django", "go", "golang", "java", "spring", "kotlin", "swift", "flutter", "docker", "kubernetes",
    "k8s", "postgres", "postgresql", "mysql", "sqlite", "redis", "graphql", "playwright", "cypress",
    "vitest", "jest", "git", "github", "gitlab",
];

pub fn is_cjk(c: char) -> bool {
    matches!(c as u32,
        0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0xF900..=0xFAFF | 0x20000..=0x2A6DF
    )
}

/// Tokenizes mixed CJK/ASCII text for `skill_registry_fts`:
/// - ASCII runs remain lowercase tokens; versioned framework tokens like `vue3` or `angular18`
///   also emit their alphabetic stem (`vue`, `angular`) unless listed in `FIXED_ALNUM_TERMS`.
/// - CJK runs emit overlapping CJK bigrams for phrase matching plus explicit `cjk1<字>` unigram tokens.
pub fn tokenize_for_skill_fts_index(text: &str) -> String {
    let mut out = Vec::new();
    let mut ascii = String::new();
    let mut cjk: Vec<char> = Vec::new();

    let flush_ascii = |buf: &mut String, tokens: &mut Vec<String>| {
        if buf.is_empty() {
            return;
        }
        let word = std::mem::take(buf);
        tokens.push(word.clone());
        let alpha_len = word.chars().take_while(|c| c.is_ascii_alphabetic()).count();
        let digit_len = word
            .chars()
            .skip(alpha_len)
            .take_while(|c| c.is_ascii_digit())
            .count();
        if alpha_len >= 3
            && digit_len > 0
            && alpha_len + digit_len == word.len()
            && !FIXED_ALNUM_TERMS.contains(&word.as_str())
        {
            tokens.push(word[..alpha_len].to_string());
        }
    };

    let flush_cjk = |buf: &mut Vec<char>, tokens: &mut Vec<String>| {
        if buf.is_empty() {
            return;
        }
        for &ch in buf.iter() {
            tokens.push(format!("cjk1{ch}"));
        }
        if buf.len() >= 2 {
            for win in buf.windows(2) {
                tokens.push(format!("{}{}", win[0], win[1]));
            }
        }
        buf.clear();
    };

    for ch in text.chars() {
        if is_cjk(ch) {
            flush_ascii(&mut ascii, &mut out);
            cjk.push(ch);
        } else if ch.is_ascii_alphanumeric() {
            flush_cjk(&mut cjk, &mut out);
            ascii.push(ch.to_ascii_lowercase());
        } else {
            flush_ascii(&mut ascii, &mut out);
            flush_cjk(&mut cjk, &mut out);
        }
    }
    flush_ascii(&mut ascii, &mut out);
    flush_cjk(&mut cjk, &mut out);
    out.join(" ")
}

fn extract_repo_key_from_git_url(git_url: &str) -> String {
    let trimmed = git_url.trim().trim_end_matches(".git").trim_end_matches('/');
    if let Some(idx) = trimmed.find("github.com/") {
        let rest = &trimmed[idx + "github.com/".len()..];
        let parts: Vec<&str> = rest.split('/').take(2).collect();
        if parts.len() == 2 {
            return format!("{}/{}", parts[0].to_lowercase(), parts[1].to_lowercase());
        }
    }
    trimmed.to_lowercase()
}

pub fn extract_skill_index_fields(
    skill: &Value,
    fallback_name: &str,
    fallback_desc: &str,
    fallback_stage: &str,
) -> SkillIndexFields {
    let name = skill
        .get("name")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or(fallback_name);
    let description = skill
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or(fallback_desc);
    let stage = skill
        .get("stage")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or(fallback_stage);
    let title_zh = skill
        .get("titleZh")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let category = skill
        .get("category")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or("general")
        .to_string();
    let tags_text = skill
        .get("tags")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();

    let mut guide_parts = Vec::new();
    if let Some(g) = skill.get("guide") {
        if let Some(s) = g.get("summary").and_then(Value::as_str).filter(|s| !s.is_empty()) {
            guide_parts.push(s.to_string());
        }
        for field in ["whenToUse", "inputs", "outputs"] {
            if let Some(arr) = g.get(field).and_then(Value::as_array) {
                for item in arr.iter().filter_map(Value::as_str).filter(|s| !s.is_empty()) {
                    guide_parts.push(item.to_string());
                }
            }
        }
    }
    if let Some(ug) = skill.get("usageGuide") {
        for field in ["summary", "trigger"] {
            if let Some(s) = ug.get(field).and_then(Value::as_str).filter(|s| !s.is_empty()) {
                guide_parts.push(s.to_string());
            }
        }
    }
    if let Some(how_to_use) = skill.get("howToUse").and_then(Value::as_str).filter(|s| !s.is_empty()) {
        guide_parts.push(how_to_use.to_string());
    }
    let guide_text = guide_parts.join(" ");

    let repo_key = skill
        .get("source")
        .and_then(|s| s.get("repoKey"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| {
            let git_url = skill.get("gitUrl").and_then(Value::as_str).unwrap_or("");
            extract_repo_key_from_git_url(git_url)
        });

    let repo_stars = skill
        .pointer("/metrics/github/stars")
        .or_else(|| skill.pointer("/metrics/githubStars"))
        .or_else(|| skill.get("stars"))
        .and_then(Value::as_i64)
        .unwrap_or(0);

    let spec_score = skill
        .pointer("/metrics/quality/specCompleteness")
        .or_else(|| skill.get("halowakeScore"))
        .and_then(Value::as_i64)
        .unwrap_or(75);

    let name_tokens = tokenize_for_skill_fts_index(&format!("{} {}", name, repo_key));
    let title_tokens = tokenize_for_skill_fts_index(&format!("{} {}", title_zh, category));
    let task_tokens = tokenize_for_skill_fts_index(&format!("{} {} {}", tags_text, category, stage));
    let guide_tokens = tokenize_for_skill_fts_index(&guide_text);
    let desc_tokens = tokenize_for_skill_fts_index(description);

    SkillIndexFields {
        title_zh,
        category,
        tags_text,
        guide_text,
        repo_key,
        repo_stars,
        spec_score,
        name_tokens,
        title_tokens,
        task_tokens,
        guide_tokens,
        desc_tokens,
    }
}

// ============================================================================
// Part 1.2: Verifiable FTS5 Content Consistency & Explicit Rebuild
// ============================================================================

/// Computes a deterministic SHA-256 content fingerprint of:
/// 1. `expected_digest`: Every row in `skill_registry_entries` (`rowid`, `id`, `name`, `description`, `stage`, `payload_json`)
///    and the expected tokenized FTS fields derived from `extract_skill_index_fields`.
/// 2. `actual_digest`: Every row's stored extended columns on `skill_registry_entries` plus every row's
///    actual indexed tokens in `skill_registry_fts`.
fn digest_to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

/// Computes a deterministic SHA-256 content fingerprint of:
/// 1. `expected_digest`: Every row in `skill_registry_entries` (`rowid`, `id`, `name`, `description`, `stage`, `payload_json`)
///    and the expected tokenized FTS fields derived from `extract_skill_index_fields`.
/// 2. `actual_digest`: Every row's stored extended columns on `skill_registry_entries` plus every row's
///    actual indexed tokens in `skill_registry_fts`.
fn compute_fts_content_digests(connection: &Connection) -> Result<(i64, i64, String, String), String> {
    let mut expected_main_ctx = Context::new(&SHA256);
    let mut expected_fts_ctx = Context::new(&SHA256);
    let mut actual_main_ctx = Context::new(&SHA256);
    let mut actual_fts_ctx = Context::new(&SHA256);
    let mut main_count: i64 = 0;

    {
        let mut stmt = connection
            .prepare(
                "SELECT rowid, id, name, title_zh, description, stage, category,
                        tags_text, guide_text, repo_key, repo_stars, spec_score, payload_json
                 FROM skill_registry_entries
                 ORDER BY rowid ASC",
            )
            .map_err(|e| format!("准备校验技能主表内容失败: {e}"))?;

        let mut rows = stmt
            .query([])
            .map_err(|e| format!("读取技能主表内容失败: {e}"))?;

        while let Some(row) = rows.next().map_err(|e| format!("遍历技能主表行失败: {e}"))? {
            main_count += 1;
            let rowid: i64 = row.get(0).map_err(|e| e.to_string())?;
            let id: String = row.get(1).map_err(|e| e.to_string())?;
            let name: String = row.get(2).map_err(|e| e.to_string())?;
            let stored_title_zh: String = row.get(3).map_err(|e| e.to_string())?;
            let description: String = row.get(4).map_err(|e| e.to_string())?;
            let stage: String = row.get(5).map_err(|e| e.to_string())?;
            let stored_category: String = row.get(6).map_err(|e| e.to_string())?;
            let stored_tags: String = row.get(7).map_err(|e| e.to_string())?;
            let stored_guide: String = row.get(8).map_err(|e| e.to_string())?;
            let stored_repo: String = row.get(9).map_err(|e| e.to_string())?;
            let stored_stars: i64 = row.get(10).map_err(|e| e.to_string())?;
            let stored_spec: i64 = row.get(11).map_err(|e| e.to_string())?;
            let payload_json: String = row.get(12).map_err(|e| e.to_string())?;

            let parsed = serde_json::from_str::<Value>(&payload_json).unwrap_or_else(|_| json!({}));
            let mut expected = extract_skill_index_fields(&parsed, &name, &description, &stage);
            if parsed.get("titleZh").and_then(Value::as_str).unwrap_or("") != stored_title_zh
                && !stored_title_zh.is_empty()
            {
                expected.title_zh = stored_title_zh.clone();
                expected.title_tokens = tokenize_for_skill_fts_index(&format!(
                    "{} {}",
                    expected.title_zh, expected.category
                ));
            }

            let rowid_s = rowid.to_string();
            let exp_stars_s = expected.repo_stars.to_string();
            let exp_spec_s = expected.spec_score.to_string();
            for piece in [
                &rowid_s,
                &id,
                &name,
                &expected.title_zh,
                &description,
                &stage,
                &expected.category,
                &expected.tags_text,
                &expected.guide_text,
                &expected.repo_key,
                &exp_stars_s,
                &exp_spec_s,
            ] {
                expected_main_ctx.update(piece.as_bytes());
                expected_main_ctx.update(&[0x1f]);
            }

            for piece in [
                &rowid_s,
                &id,
                &expected.name_tokens,
                &expected.title_tokens,
                &expected.task_tokens,
                &expected.guide_tokens,
                &expected.desc_tokens,
            ] {
                expected_fts_ctx.update(piece.as_bytes());
                expected_fts_ctx.update(&[0x1e]);
            }

            let act_stars_s = stored_stars.to_string();
            let act_spec_s = stored_spec.to_string();
            for piece in [
                &rowid_s,
                &id,
                &name,
                &stored_title_zh,
                &description,
                &stage,
                &stored_category,
                &stored_tags,
                &stored_guide,
                &stored_repo,
                &act_stars_s,
                &act_spec_s,
            ] {
                actual_main_ctx.update(piece.as_bytes());
                actual_main_ctx.update(&[0x1f]);
            }
        }
    }

    let mut fts_count: i64 = 0;
    {
        let mut fts_stmt = connection
            .prepare(
                "SELECT rowid, id, name_tokens, title_tokens, task_tokens, guide_tokens, desc_tokens
                 FROM skill_registry_fts
                 ORDER BY rowid ASC",
            )
            .map_err(|e| format!("准备校验技能 FTS 表内容失败: {e}"))?;

        let mut fts_rows = fts_stmt
            .query([])
            .map_err(|e| format!("读取技能 FTS 表内容失败: {e}"))?;

        while let Some(row) = fts_rows.next().map_err(|e| format!("遍历技能 FTS 表行失败: {e}"))? {
            fts_count += 1;
            let rowid: i64 = row.get(0).map_err(|e| e.to_string())?;
            let id: String = row.get(1).map_err(|e| e.to_string())?;
            let name_tokens: String = row.get(2).map_err(|e| e.to_string())?;
            let title_tokens: String = row.get(3).map_err(|e| e.to_string())?;
            let task_tokens: String = row.get(4).map_err(|e| e.to_string())?;
            let guide_tokens: String = row.get(5).map_err(|e| e.to_string())?;
            let desc_tokens: String = row.get(6).map_err(|e| e.to_string())?;

            let rowid_s = rowid.to_string();
            for piece in [
                &rowid_s,
                &id,
                &name_tokens,
                &title_tokens,
                &task_tokens,
                &guide_tokens,
                &desc_tokens,
            ] {
                actual_fts_ctx.update(piece.as_bytes());
                actual_fts_ctx.update(&[0x1e]);
            }
        }
    }

    let expected_hex = format!(
        "{}:{}",
        digest_to_hex(expected_main_ctx.finish().as_ref()),
        digest_to_hex(expected_fts_ctx.finish().as_ref())
    );
    let actual_hex = format!(
        "{}:{}",
        digest_to_hex(actual_main_ctx.finish().as_ref()),
        digest_to_hex(actual_fts_ctx.finish().as_ref())
    );
    Ok((main_count, fts_count, expected_hex, actual_hex))
}

fn save_fts_fingerprint_tx(transaction: &Transaction<'_>, fingerprint: &str) -> Result<(), String> {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    transaction
        .execute(
            "INSERT INTO app_metadata (key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![FTS_FINGERPRINT_META_KEY, fingerprint, now_ms],
        )
        .map_err(|e| format!("写入技能 FTS 一致性指纹失败: {e}"))?;
    Ok(())
}

fn read_stored_fts_fingerprint(connection: &Connection) -> Option<String> {
    connection
        .query_row(
            "SELECT value FROM app_metadata WHERE key = ?1",
            params![FTS_FINGERPRINT_META_KEY],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .ok()
        .flatten()
}

/// Rebuilds `skill_registry_fts` and backfills extended columns on `skill_registry_entries`
/// inside an active SQLite transaction. Used both by Migration v10 and by `rebuild_skill_registry_fts`.
pub(crate) fn backfill_skill_registry_fts_tx(transaction: &Transaction<'_>) -> Result<usize, String> {
    let existing_rows: Vec<(i64, String, String, String, String, String, String)> = {
        let mut stmt = transaction
            .prepare(
                "SELECT rowid, id, name, title_zh, description, stage, payload_json
                 FROM skill_registry_entries
                 ORDER BY rowid",
            )
            .map_err(|error| format!("准备读取存量技能条目失败: {error}"))?;
        let mapped = stmt
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            })
            .map_err(|error| format!("读取存量技能条目失败: {error}"))?;
        mapped
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("解析存量技能条目失败: {error}"))?
    };

    transaction
        .execute("DELETE FROM skill_registry_fts", [])
        .map_err(|error| format!("清理旧技能 FTS 索引失败: {error}"))?;

    if existing_rows.is_empty() {
        let (_, _, expected_hex, _) = compute_fts_content_digests(transaction)?;
        let _ = save_fts_fingerprint_tx(transaction, &expected_hex);
        return Ok(0);
    }

    let mut update_entry = transaction
        .prepare(
            "UPDATE skill_registry_entries
             SET title_zh = ?1,
                 category = ?2,
                 tags_text = ?3,
                 guide_text = ?4,
                 repo_key = ?5,
                 repo_stars = ?6,
                 spec_score = ?7
             WHERE rowid = ?8",
        )
        .map_err(|error| format!("准备回填技能扩展列失败: {error}"))?;

    let mut insert_fts = transaction
        .prepare(
            "INSERT INTO skill_registry_fts (
                rowid, id, name_tokens, title_tokens, task_tokens, guide_tokens, desc_tokens
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .map_err(|error| format!("准备写入技能 FTS 索引失败: {error}"))?;

    for (rowid, id, name, stored_title_zh, description, stage, payload_json) in &existing_rows {
        let parsed = serde_json::from_str::<Value>(payload_json).unwrap_or_else(|_| json!({}));
        let mut fields = extract_skill_index_fields(&parsed, name, description, stage);
        if parsed.get("titleZh").and_then(Value::as_str).unwrap_or("") != stored_title_zh
            && !stored_title_zh.is_empty()
        {
            fields.title_zh = stored_title_zh.clone();
            fields.title_tokens =
                tokenize_for_skill_fts_index(&format!("{} {}", fields.title_zh, fields.category));
        }

        update_entry
            .execute(params![
                fields.title_zh,
                fields.category,
                fields.tags_text,
                fields.guide_text,
                fields.repo_key,
                fields.repo_stars,
                fields.spec_score,
                rowid,
            ])
            .map_err(|error| format!("回填技能 {} 扩展列失败: {error}", id))?;

        insert_fts
            .execute(params![
                rowid,
                id,
                fields.name_tokens,
                fields.title_tokens,
                fields.task_tokens,
                fields.guide_tokens,
                fields.desc_tokens,
            ])
            .map_err(|error| format!("回填技能 {} FTS 索引失败: {error}", id))?;
    }

    drop(update_entry);
    drop(insert_fts);

    let (_, _, expected_hex, _) = compute_fts_content_digests(transaction)?;
    let _ = save_fts_fingerprint_tx(transaction, &expected_hex);

    Ok(existing_rows.len())
}

/// Verifies FTS5 index content consistency (row counts + SHA-256 content digest of main table vs FTS table
/// + stored metadata fingerprint), or unconditionally rebuilds when `force == true`.
pub fn rebuild_skill_registry_fts(database: &AppDatabase, force: bool) -> Result<bool, String> {
    let mut connection = database.connect()?;
    if !force {
        let (main_count, fts_count, expected_hex, actual_hex) =
            compute_fts_content_digests(&connection)?;
        let stored_fp = read_stored_fts_fingerprint(&connection);
        if main_count == fts_count
            && expected_hex == actual_hex
            && stored_fp.as_deref() == Some(actual_hex.as_str())
        {
            mark_db_fts_verified(database.path());
            return Ok(false);
        }
    }

    let transaction = connection
        .transaction()
        .map_err(|error| format!("开启技能 FTS 重建事务失败: {error}"))?;
    backfill_skill_registry_fts_tx(&transaction)?;
    transaction
        .commit()
        .map_err(|error| format!("提交技能 FTS 重建事务失败: {error}"))?;
    mark_db_fts_verified(database.path());
    Ok(true)
}

/// Checks if `skill_registry_fts` is out of sync with `skill_registry_entries` (by row count OR
/// SHA-256 content digest) and atomically rebuilds the FTS index if needed.
pub fn rebuild_skill_registry_fts_if_needed(database: &AppDatabase) -> Result<bool, String> {
    rebuild_skill_registry_fts(database, false)
}

/// Runs `rebuild_skill_registry_fts_if_needed` at most once per database path per process lifecycle
/// so search queries never pay per-keystroke full-table verification overhead.
pub fn ensure_skill_registry_fts_healthy_once(database: &AppDatabase) -> Result<bool, String> {
    let already_verified = verified_fts_dbs()
        .lock()
        .map(|set| set.contains(database.path()))
        .unwrap_or(false);
    if already_verified {
        return Ok(false);
    }
    rebuild_skill_registry_fts(database, false)
}

pub fn load_skill_registry(database: &AppDatabase) -> Result<Option<Value>, String> {
    let _ = ensure_skill_registry_fts_healthy_once(database);
    let connection = database.connect()?;
    let snapshot: Option<(String, i64, Option<String>, String)> = connection
        .query_row(
            "SELECT version, updated_at, description, pipelines_json FROM skill_registry_snapshot WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(|error| format!("读取技能目录快照失败: {error}"))?;
    let Some((version, updated_at, description, pipelines_json)) = snapshot else {
        return Ok(None);
    };
    let mut statement = connection
        .prepare("SELECT payload_json FROM skill_registry_entries ORDER BY rowid")
        .map_err(|error| format!("读取技能目录失败: {error}"))?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|error| format!("查询技能目录失败: {error}"))?;
    let mut skills = Vec::new();
    for row in rows {
        let text = row.map_err(|error| format!("读取技能条目失败: {error}"))?;
        skills.push(
            serde_json::from_str::<Value>(&text)
                .map_err(|error| format!("解析技能条目失败: {error}"))?,
        );
    }
    let pipelines: Value = serde_json::from_str(&pipelines_json)
        .map_err(|error| format!("解析技能流水线失败: {error}"))?;
    let catalog_revision = if updated_at > 0 {
        format!("{version}:{updated_at}")
    } else {
        version.clone()
    };
    Ok(Some(json!({
        "version": version,
        "updatedAt": updated_at,
        "catalogRevision": catalog_revision,
        "description": description,
        "skills": skills,
        "pipelines": pipelines,
    })))
}

pub fn get_skill_registry_revision_from_conn(conn: &Connection) -> Result<Option<String>, String> {
    if conn.is_autocommit() {
        let _ = conn.execute_batch("BEGIN DEFERRED;");
    }
    let row: Option<(String, i64)> = conn
        .query_row(
            "SELECT version, updated_at FROM skill_registry_snapshot WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| format!("读取技能注册表版本失败: {error}"))?;
    Ok(row.map(|(ver, updated_at)| {
        if updated_at > 0 {
            format!("{ver}:{updated_at}")
        } else {
            ver
        }
    }))
}

pub fn get_skill_registry_revision(database: &AppDatabase) -> Result<Option<String>, String> {
    let connection = database.connect_read()?;
    get_skill_registry_revision_from_conn(&connection)
}

pub fn save_skill_registry(database: &AppDatabase, registry: &Value) -> Result<(), String> {
    let skills = registry
        .get("skills")
        .and_then(Value::as_array)
        .ok_or_else(|| "技能目录缺少 skills 数组".to_string())?;
    if skills.is_empty() || skills.len() > 100_000 {
        return Err("技能目录数量无效".to_string());
    }
    let version = registry
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or("1.0.0");
    let updated_at = registry
        .get("updatedAt")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let description = registry.get("description").and_then(Value::as_str);
    let pipelines = registry
        .get("pipelines")
        .cloned()
        .unwrap_or_else(|| json!([]));
    let mut connection = database.connect()?;
    let transaction = connection
        .transaction()
        .map_err(|error| format!("开启技能目录事务失败: {error}"))?;
    transaction
        .execute("DELETE FROM skill_registry_entries", [])
        .map_err(|error| format!("清理旧技能目录失败: {error}"))?;
    transaction
        .execute("DELETE FROM skill_registry_fts", [])
        .map_err(|error| format!("清理旧技能 FTS 索引失败: {error}"))?;
    {
        let mut insert_entry = transaction
            .prepare(
                "INSERT INTO skill_registry_entries (
                    id, name, title_zh, description, stage, category,
                    tags_text, guide_text, repo_key, repo_stars, spec_score, payload_json
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            )
            .map_err(|error| format!("准备技能目录写入失败: {error}"))?;
        let mut insert_fts = transaction
            .prepare(
                "INSERT INTO skill_registry_fts (
                    rowid, id, name_tokens, title_tokens, task_tokens, guide_tokens, desc_tokens
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )
            .map_err(|error| format!("准备技能 FTS 索引写入失败: {error}"))?;

        let mut seen_ids = HashSet::new();
        for skill in skills {
            let id = skill.get("id").and_then(Value::as_str).unwrap_or("").trim();
            let name = skill.get("name").and_then(Value::as_str).unwrap_or("").trim();
            if id.is_empty() || name.is_empty() {
                return Err("技能条目缺少 id 或 name".to_string());
            }
            if !seen_ids.insert(id.to_ascii_lowercase()) {
                return Err(format!("技能目录包含重复的技能 ID: {id}"));
            }
            if let Some(git_url) = skill.get("gitUrl").and_then(Value::as_str) {
                let trimmed_url = git_url.trim();
                if trimmed_url.is_empty()
                    || trimmed_url.chars().any(char::is_whitespace)
                    || (!trimmed_url.starts_with("https://")
                        && !trimmed_url.starts_with("http://")
                        && !trimmed_url.starts_with("git@"))
                {
                    return Err(format!("技能 {id} 的 gitUrl 非法"));
                }
            }
            if let Some(sub_path) = skill.get("subPath").and_then(Value::as_str) {
                let trimmed_sub = sub_path.trim();
                if !trimmed_sub.is_empty() {
                    if trimmed_sub.starts_with('/')
                        || trimmed_sub.starts_with('\\')
                        || trimmed_sub.contains(':')
                        || trimmed_sub
                            .replace('\\', "/")
                            .split('/')
                            .any(|seg| seg.is_empty() || seg == "." || seg == "..")
                    {
                        return Err(format!("技能 {id} 的 subPath 包含非法路径段"));
                    }
                }
            }
            let desc = skill
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or("");
            let stage = skill
                .get("stage")
                .and_then(Value::as_str)
                .unwrap_or("coding");
            let text = serde_json::to_string(skill)
                .map_err(|error| format!("序列化技能条目失败: {error}"))?;
            let fields = extract_skill_index_fields(skill, name, desc, stage);

            insert_entry
                .execute(params![
                    id,
                    name,
                    fields.title_zh,
                    desc,
                    stage,
                    fields.category,
                    fields.tags_text,
                    fields.guide_text,
                    fields.repo_key,
                    fields.repo_stars,
                    fields.spec_score,
                    text,
                ])
                .map_err(|error| format!("写入技能 {} 失败: {error}", id))?;

            let rowid = transaction.last_insert_rowid();
            insert_fts
                .execute(params![
                    rowid,
                    id,
                    fields.name_tokens,
                    fields.title_tokens,
                    fields.task_tokens,
                    fields.guide_tokens,
                    fields.desc_tokens,
                ])
                .map_err(|error| format!("写入技能 {} FTS 索引失败: {error}", id))?;
        }
    }
    transaction
        .execute(
            "INSERT INTO skill_registry_snapshot (id, version, updated_at, description, pipelines_json)
             VALUES (1, ?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET version=excluded.version, updated_at=excluded.updated_at,
               description=excluded.description, pipelines_json=excluded.pipelines_json",
            params![version, updated_at, description, pipelines.to_string()],
        )
        .map_err(|error| format!("写入技能目录快照失败: {error}"))?;

    let (_, _, expected_hex, _) = compute_fts_content_digests(&transaction)?;
    save_fts_fingerprint_tx(&transaction, &expected_hex)?;

    transaction
        .commit()
        .map_err(|error| format!("提交技能目录失败: {error}"))?;
    mark_db_fts_verified(database.path());
    Ok(())
}

pub const MAX_SEARCH_CANDIDATES: usize = 10_000;

/// Queries candidate rows from `skill_registry_entries` (and `skill_registry_fts` when `fts_match_expr` is provided),
/// strictly applying `stage`, `category`, and `allowed_ids` filters **before** truncating to `filter.limit`.
pub fn query_skill_registry_candidates(
    database: &AppDatabase,
    fts_match_expr: Option<&str>,
    filter: &SkillRegistryFilter<'_>,
) -> Result<Vec<SkillRegistryCandidateRow>, String> {
    let connection = database.connect_read()?;
    query_skill_registry_candidates_with_conn(&connection, fts_match_expr, filter)
}

pub fn query_skill_registry_candidates_with_conn(
    connection: &Connection,
    fts_match_expr: Option<&str>,
    filter: &SkillRegistryFilter<'_>,
) -> Result<Vec<SkillRegistryCandidateRow>, String> {
    let limit = if filter.limit == 0 { 200 } else { filter.limit };
    let match_expr = fts_match_expr.map(str::trim).filter(|s| !s.is_empty());

    let mut out = Vec::new();
    if let Some(expr) = match_expr {
        let mut statement = connection
            .prepare_cached(
                "SELECT
                    e.id, e.name, e.title_zh, e.description, e.stage, e.category,
                    e.tags_text, e.guide_text, e.repo_key, e.repo_stars, e.spec_score,
                    e.payload_json,
                    bm25(skill_registry_fts, 10.0, 8.0, 6.0, 4.0, 1.0) AS raw_bm25
                 FROM skill_registry_fts
                 JOIN skill_registry_entries e ON e.rowid = skill_registry_fts.rowid
                 WHERE skill_registry_fts MATCH ?1
                   AND (?2 IS NULL OR e.stage = ?2)
                   AND (?3 IS NULL OR e.category = ?3)
                 ORDER BY raw_bm25 ASC, e.rowid ASC",
            )
            .map_err(|error| format!("准备 FTS 技能候选查询失败: {error}"))?;

        let rows = statement
            .query_map(params![expr, filter.stage, filter.category], |row| {
                Ok(SkillRegistryCandidateRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    title_zh: row.get(2)?,
                    description: row.get(3)?,
                    stage: row.get(4)?,
                    category: row.get(5)?,
                    tags_text: row.get(6)?,
                    guide_text: row.get(7)?,
                    repo_key: row.get(8)?,
                    repo_stars: row.get(9)?,
                    spec_score: row.get(10)?,
                    payload_json: row.get(11)?,
                    bm25_raw: row.get(12)?,
                })
            })
            .map_err(|error| format!("执行 FTS 技能候选查询失败: {error}"))?;

        for row in rows {
            let candidate = row.map_err(|error| format!("读取 FTS 技能候选失败: {error}"))?;
            if let Some(allowed) = filter.allowed_ids {
                if !allowed.contains(&candidate.id) {
                    continue;
                }
            }
            out.push(candidate);
            if out.len() >= limit {
                break;
            }
        }
    } else {
        let mut statement = connection
            .prepare_cached(
                "SELECT
                    e.id, e.name, e.title_zh, e.description, e.stage, e.category,
                    e.tags_text, e.guide_text, e.repo_key, e.repo_stars, e.spec_score,
                    e.payload_json
                 FROM skill_registry_entries e
                 WHERE (?1 IS NULL OR e.stage = ?1)
                   AND (?2 IS NULL OR e.category = ?2)
                 ORDER BY e.rowid ASC",
            )
            .map_err(|error| format!("准备技能过滤查询失败: {error}"))?;

        let rows = statement
            .query_map(params![filter.stage, filter.category], |row| {
                Ok(SkillRegistryCandidateRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    title_zh: row.get(2)?,
                    description: row.get(3)?,
                    stage: row.get(4)?,
                    category: row.get(5)?,
                    tags_text: row.get(6)?,
                    guide_text: row.get(7)?,
                    repo_key: row.get(8)?,
                    repo_stars: row.get(9)?,
                    spec_score: row.get(10)?,
                    payload_json: row.get(11)?,
                    bm25_raw: 0.0,
                })
            })
            .map_err(|error| format!("执行技能过滤查询失败: {error}"))?;

        for row in rows {
            let candidate = row.map_err(|error| format!("读取技能过滤结果失败: {error}"))?;
            if let Some(allowed) = filter.allowed_ids {
                if !allowed.contains(&candidate.id) {
                    continue;
                }
            }
            out.push(candidate);
            if out.len() >= limit {
                break;
            }
        }
    }

    Ok(out)
}

// ============================================================================
// Part 2: v3.2 Query Parser, Synonym Tiering, and Field-Authority Ranker
// ============================================================================

#[derive(Debug, Clone)]
pub struct QueryIntentSlot {
    pub raw_term: String,
    pub synonyms: Vec<String>,
    pub is_core: bool,
    pub is_tech_stack: bool,
}

fn synonym_dictionary() -> HashMap<&'static str, &'static [&'static str]> {
    let mut m: HashMap<&'static str, &'static [&'static str]> = HashMap::new();
    m.insert("国际化", &["国际化", "i18n", "localization", "locale", "translate", "translation", "l10n", "多语言", "本地化"]);
    m.insert("多语言", &["多语言", "国际化", "i18n", "localization", "locale", "translation"]);
    m.insert("本地化", &["本地化", "国际化", "l10n", "i18n", "localization"]);
    m.insert("i18n", &["i18n", "国际化", "多语言", "localization", "locale", "translation", "l10n"]);
    m.insert("前端", &["前端", "frontend", "ui", "web", "ux"]);
    m.insert("后端", &["后端", "backend", "server", "api", "service"]);
    m.insert("设计", &["设计", "design", "ui", "ux", "designer", "guidelines"]);
    m.insert("规范", &["规范", "guidelines", "standard", "best-practices", "convention", "style"]);
    m.insert("组件", &["组件", "component", "components", "widget", "ui"]);
    m.insert("视觉", &["视觉", "visual", "screenshot", "snapshot", "ui", "aesthetic", "design", "regression"]);
    m.insert("视觉回归", &["视觉回归", "visual", "regression", "screenshot", "playwright", "snapshot"]);
    m.insert("视觉回归测试", &["视觉回归测试", "visual", "regression", "playwright", "screenshot", "e2e"]);
    m.insert("界面", &["界面", "ui", "frontend", "design", "interface", "layout"]);
    m.insert("美化", &["美化", "polish", "ui", "design", "tailwind", "css", "aesthetic"]);
    m.insert("样式", &["样式", "style", "styling", "css", "tailwind", "scss", "less", "theme"]);
    m.insert("排版", &["排版", "typography", "layout", "spacing", "font"]);
    m.insert("主题", &["主题", "theme", "theming", "dark-mode", "css", "tailwind"]);
    m.insert("暗黑", &["暗黑", "dark", "dark-mode", "theme"]);
    m.insert("暗黑模式", &["暗黑模式", "dark-mode", "dark", "theme"]);
    m.insert("测试", &["测试", "test", "testing", "spec", "vitest", "jest", "playwright", "cypress", "pytest", "qa"]);
    m.insert("单元测试", &["单元测试", "unit", "unit-test", "rtl", "vitest", "jest", "pytest", "tdd", "test", "testing", "spec"]);
    m.insert("unit", &["unit", "unit-test", "rtl", "vitest", "jest", "tdd"]);
    m.insert("自动化测试", &["自动化测试", "e2e", "playwright", "cypress", "automation", "browser", "test", "testing"]);
    m.insert("代码审查", &["代码审查", "review", "code-review", "pr", "mr", "audit", "lint"]);
    m.insert("代码评审", &["代码评审", "review", "code-review", "review-pr", "autoreview", "pr", "mr"]);
    m.insert("调试", &["调试", "debug", "debugging", "troubleshoot", "diagnose", "trace"]);
    m.insert("故障", &["故障", "debug", "debugging", "bug", "error", "incident", "troubleshoot"]);
    m.insert("排查", &["排查", "debug", "debugging", "troubleshoot", "diagnose", "investigate"]);
    m.insert("排错", &["排错", "debug", "troubleshoot", "error", "exception", "fix"]);
    m.insert("日志", &["日志", "log", "logging", "logger", "trace", "telemetry"]);
    m.insert("内存", &["内存", "memory", "leak", "heap", "profile", "profiler", "allocation"]);
    m.insert("内存泄漏", &["内存泄漏", "memory", "leak", "heap", "profiler"]);
    m.insert("数据库", &["数据库", "database", "sql", "postgres", "postgresql", "mysql", "sqlite", "prisma", "orm"]);
    m.insert("慢查询", &["慢查询", "sql", "query", "performance", "index", "explain", "database", "optimize"]);
    m.insert("打包", &["打包", "build", "bundle", "package", "release", "installer", "compile"]);
    m.insert("构建", &["构建", "build", "bundle", "vite", "webpack", "compile"]);
    m.insert("部署", &["部署", "deploy", "deployment", "docker", "k8s", "kubernetes", "ci", "cd"]);
    m.insert("容器", &["容器", "docker", "container", "k8s", "kubernetes", "dockerfile"]);
    m.insert("重构", &["重构", "refactor", "refactoring", "clean", "architecture", "kaizen-2-refactor"]);
    m.insert("质量", &["质量", "quality", "clean", "review", "audit"]);
    m.insert("代码质量", &["code-review-and-quality", "code-quality", "quality", "clean-code", "review", "code-review", "architecture-audit"]);
    m.insert("代码", &["code", "source", "review", "clean"]);
    m.insert("接口", &["接口", "api", "rest", "graphql", "endpoint", "openapi", "swagger"]);
    m.insert("安全", &["安全", "security", "vulnerability", "audit", "auth", "owasp", "secret"]);
    m.insert("安全审计", &["安全审计", "security-audit", "security-review", "security", "vulnerability", "audit", "owasp"]);
    m.insert("审计", &["审计", "audit", "security-audit", "architecture-audit", "review"]);
    m.insert("漏洞", &["漏洞", "vulnerability", "security-audit", "cve", "owasp", "security", "audit"]);
    m.insert("文档", &["文档", "doc", "docs", "documentation", "markdown", "readme"]);
    m.insert("修复", &["修复", "fix", "repair", "resolve", "debug", "patch"]);
    m.insert("单测", &["单测", "单元测试", "unit", "unit-test", "rtl", "vitest", "jest", "tdd", "test", "testing", "spec"]);
    m.insert("端到端测试", &["端到端测试", "e2e", "playwright", "cypress", "webapp-testing", "browser", "test", "testing"]);
    m.insert("登录", &["登录", "login", "authentication", "auth", "oauth", "oauth2", "session", "jwt", "signin"]);
    m.insert("鉴权", &["鉴权", "authorization", "authentication", "auth", "rbac", "jwt", "permission"]);
    m.insert("权限控制", &["权限控制", "authorization", "auth", "rbac", "permission", "access", "security"]);
    m.insert("性能优化", &["性能优化", "performance", "perf", "optimize", "optimization", "profiler", "bottleneck", "speed"]);
    m.insert("性能瓶颈", &["性能瓶颈", "bottleneck", "profiler", "perf", "performance", "optimization"]);
    m.insert("瓶颈", &["瓶颈", "bottleneck", "profiler", "perf", "performance"]);
    m.insert("性能", &["性能", "performance", "perf", "profiler", "optimize", "optimization", "bottleneck"]);
    m.insert("优化", &["优化", "optimize", "optimization", "performance", "perf", "best-practices", "tuning"]);
    m.insert("缓存", &["缓存", "cache", "caching", "redis", "memoize"]);
    m.insert("并发", &["并发", "concurrency", "async", "parallel", "thread", "tokio"]);
    m.insert("异步", &["异步", "async", "await", "promise", "concurrency", "tokio"]);
    m.insert("提交", &["提交", "commit", "git", "pr"]);
    m.insert("分支", &["分支", "branch", "git", "merge", "worktree"]);
    m.insert("container", &["container", "containers", "dockerfile", "docker", "k8s", "kubernetes"]);
    m.insert("deployment", &["deployment", "deploy", "devops", "ci", "cd", "production", "k8s", "kubernetes"]);
    m.insert("optimization", &["optimization", "optimize", "performance", "perf", "index", "tuning", "best-practices"]);
    m.insert("表格", &["表格", "table", "grid", "datagrid", "buefy", "component", "ui"]);
    m.insert("组件库", &["组件库", "component", "components", "ui", "design-system", "buefy", "tailwind"]);
    m.insert("状态管理", &["状态管理", "state", "store", "pinia", "vuex", "redux", "zustand", "vueuse"]);
    m.insert("无障碍", &["无障碍", "a11y", "accessibility", "accessible", "aria", "wcag"]);
    m.insert("可访问性", &["可访问性", "a11y", "accessibility", "accessible", "aria", "wcag"]);
    m.insert("accessibility", &["accessibility", "accessible", "a11y", "wcag", "无障碍", "可访问性"]);
    m.insert("wcag", &["wcag", "accessibility", "accessible", "a11y", "无障碍"]);
    m.insert("动画", &["动画", "animation", "motion", "gsap", "transition", "keyframes"]);
    m.insert("图表", &["图表", "chart", "charts", "visualization", "d3", "echarts", "mermaid"]);
    m.insert("流程图", &["流程图", "mermaid", "diagram", "flowchart", "architecture"]);
    m.insert("表单", &["表单", "form", "forms", "validation", "input", "schema"]);
    m.insert("爬虫", &["爬虫", "scraper", "scraping", "crawl", "crawler", "playwright", "browser"]);
    m.insert("迁移", &["迁移", "migration", "migrate", "upgrade", "codemod"]);
    m.insert("架构", &["架构", "architecture", "system-design", "clean", "ddd", "modular"]);
    m.insert("架构设计", &["架构设计", "architecture", "system-design", "architecture-audit"]);
    m
}

fn segment_chinese_chunk(chunk: &str) -> Vec<String> {
    let mut working = chunk.trim().to_string();
    if working.is_empty() {
        return Vec::new();
    }

    if CN_STOP_PHRASES.contains(&working.as_str()) {
        return Vec::new();
    }
    if working.chars().count() > 1 {
        for stop in CN_STOP_PHRASES {
            if working.contains(stop) {
                working = working.replace(stop, " ");
            }
        }
    }

    let mut result = Vec::new();
    for part in working.split_whitespace() {
        let chars: Vec<char> = part.chars().collect();
        let mut idx = 0;
        let mut unknown_buf = String::new();

        while idx < chars.len() {
            let remaining: String = chars[idx..].iter().collect();
            let mut matched_domain = None;
            for &domain_word in CN_DOMAIN_TERMS {
                if remaining.starts_with(domain_word) {
                    matched_domain = Some(domain_word);
                    break;
                }
            }
            if let Some(dw) = matched_domain {
                if !unknown_buf.is_empty() {
                    result.push(std::mem::take(&mut unknown_buf));
                }
                result.push(dw.to_string());
                idx += dw.chars().count();
            } else {
                unknown_buf.push(chars[idx]);
                idx += 1;
            }
        }
        if !unknown_buf.is_empty() {
            result.push(unknown_buf);
        }
    }
    result
}

pub fn parse_search_query(query: &str) -> Vec<QueryIntentSlot> {
    let dict = synonym_dictionary();
    let normalized_query = query
        .replace("Next.js", "nextjs")
        .replace("next.js", "nextjs")
        .replace("Node.js", "nodejs")
        .replace("node.js", "nodejs")
        .replace("Vue.js", "vue")
        .replace("vue.js", "vue")
        .replace("Nuxt.js", "nuxt")
        .replace("nuxt.js", "nuxt");
    let mut raw_segments: Vec<(bool, String)> = Vec::new();
    let mut cur_cjk = String::new();
    let mut cur_ascii = String::new();

    for ch in normalized_query.chars() {
        if is_cjk(ch) {
            if !cur_ascii.is_empty() {
                raw_segments.push((false, std::mem::take(&mut cur_ascii)));
            }
            cur_cjk.push(ch);
        } else if ch.is_ascii_alphanumeric() {
            if !cur_cjk.is_empty() {
                raw_segments.push((true, std::mem::take(&mut cur_cjk)));
            }
            cur_ascii.push(ch.to_ascii_lowercase());
        } else {
            if !cur_cjk.is_empty() {
                raw_segments.push((true, std::mem::take(&mut cur_cjk)));
            }
            if !cur_ascii.is_empty() {
                raw_segments.push((false, std::mem::take(&mut cur_ascii)));
            }
        }
    }
    if !cur_cjk.is_empty() {
        raw_segments.push((true, cur_cjk));
    }
    if !cur_ascii.is_empty() {
        raw_segments.push((false, cur_ascii));
    }

    let mut atomic_terms = Vec::new();
    for (is_chinese, seg) in raw_segments {
        if is_chinese {
            for cn_word in segment_chinese_chunk(&seg) {
                if !cn_word.is_empty() {
                    atomic_terms.push(cn_word);
                }
            }
        } else if !seg.is_empty() {
            atomic_terms.push(seg);
        }
    }

    let mut seen = HashSet::new();
    let mut slots = Vec::new();

    for term in atomic_terms {
        if !seen.insert(term.clone()) {
            continue;
        }
        let mut group = Vec::new();
        group.push(term.clone());

        let alpha_len = term.chars().take_while(|c| c.is_ascii_alphabetic()).count();
        let digit_len = term
            .chars()
            .skip(alpha_len)
            .take_while(|c| c.is_ascii_digit())
            .count();
        if alpha_len >= 3
            && digit_len > 0
            && alpha_len + digit_len == term.len()
            && !FIXED_ALNUM_TERMS.contains(&term.as_str())
        {
            let stem = term[..alpha_len].to_string();
            if !group.contains(&stem) {
                group.push(stem);
            }
        }

        if let Some(syns) = dict.get(term.as_str()) {
            for &s in *syns {
                let s_str = s.to_lowercase();
                if !group.contains(&s_str) {
                    group.push(s_str);
                }
            }
        }

        let is_tech = group.iter().any(|g| KNOWN_TECH_STACKS.contains(&g.as_str()));
        let is_modifier = MODIFIER_TERMS.contains(&term.as_str());

        slots.push(QueryIntentSlot {
            raw_term: term,
            synonyms: group,
            is_core: !is_modifier,
            is_tech_stack: is_tech,
        });
    }

    if !slots.is_empty() && slots.iter().all(|s| !s.is_core) {
        for s in &mut slots {
            s.is_core = true;
        }
    }

    slots
}

pub fn single_word_to_fts(word: &str) -> Option<String> {
    let clean = word.trim().to_lowercase();
    if clean.is_empty() {
        return None;
    }
    let chars: Vec<char> = clean.chars().collect();
    if chars.iter().all(|&c| is_cjk(c)) {
        if chars.len() == 1 {
            Some(format!("\"cjk1{}\"", chars[0]))
        } else {
            let bigrams: Vec<String> = chars
                .windows(2)
                .map(|w| format!("{}{}", w[0], w[1]))
                .collect();
            Some(format!("\"{}\"", bigrams.join(" ")))
        }
    } else {
        let parts: Vec<&str> = clean
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|s| !s.is_empty())
            .collect();
        if parts.is_empty() {
            return None;
        }
        if parts.len() == 1 {
            let p = parts[0];
            if p.len() <= 3 {
                Some(format!("\"{p}\""))
            } else {
                Some(format!("\"{p}\"*"))
            }
        } else {
            Some(format!("\"{}\"", parts.join(" ")))
        }
    }
}

pub fn slot_to_fts_group_expr(slot: &QueryIntentSlot) -> Option<String> {
    let mut exprs = Vec::new();
    for syn in &slot.synonyms {
        if let Some(e) = single_word_to_fts(syn) {
            if !exprs.contains(&e) {
                exprs.push(e);
            }
        }
    }
    if exprs.is_empty() {
        None
    } else {
        Some(format!("({})", exprs.join(" OR ")))
    }
}

fn term_matches_text_with_boundary(
    term: &str,
    text_lower: &str,
    text_ascii_tokens: &HashSet<&str>,
) -> bool {
    let chars: Vec<char> = term.chars().collect();
    if chars.iter().all(|&c| is_cjk(c)) {
        text_lower.contains(term)
    } else {
        let sub: Vec<&str> = term
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|s| !s.is_empty())
            .collect();
        if sub.is_empty() {
            return false;
        }
        if sub.len() == 1 {
            let w = sub[0];
            if w.len() <= 3 {
                text_ascii_tokens.contains(w)
            } else {
                text_ascii_tokens
                    .iter()
                    .any(|tok| *tok == w || tok.starts_with(w))
            }
        } else {
            text_lower.contains(term)
        }
    }
}

fn synonym_tier_weight(raw_term: &str, syn: &str, idx: usize, full_text_lower: &str) -> f64 {
    if idx == 0 || raw_term == syn {
        return 1.0;
    }

    if matches!(raw_term, "国际化" | "多语言" | "本地化" | "i18n") && syn == "translate" {
        let has_i18n_context = full_text_lower.contains("i18n")
            || full_text_lower.contains("l10n")
            || full_text_lower.contains("locale")
            || full_text_lower.contains("localization");
        return if has_i18n_context { 0.92 } else { 0.18 };
    }

    if matches!(raw_term, "代码审查" | "代码评审")
        && matches!(syn, "lint" | "audit" | "pr" | "mr")
    {
        return 0.15;
    }

    if matches!(raw_term, "单元测试" | "单测" | "unit")
        && matches!(syn, "unit" | "test" | "testing" | "spec")
    {
        let has_unit_context = full_text_lower.contains("unit test")
            || full_text_lower.contains("unit-test")
            || full_text_lower.contains("rtl")
            || full_text_lower.contains("testing library")
            || full_text_lower.contains("vitest")
            || full_text_lower.contains("jest")
            || full_text_lower.contains("pytest")
            || full_text_lower.contains("tdd")
            || full_text_lower.contains("test.ts");
        return if has_unit_context { 0.88 } else { 0.15 };
    }

    if matches!(raw_term, "react" | "testing" | "单元测试" | "单测" | "unit")
        && syn == "rtl"
        && full_text_lower.contains("react")
    {
        return 0.96;
    }

    if matches!(
        syn,
        "i18n"
            | "localization"
            | "l10n"
            | "locale"
            | "unit-test"
            | "rtl"
            | "vitest"
            | "jest"
            | "pytest"
            | "tdd"
            | "code-review"
            | "autoreview"
            | "review-pr"
            | "dockerfile"
            | "postgres"
            | "postgresql"
            | "sql"
            | "database"
            | "oauth"
            | "oauth2"
            | "authentication"
            | "authorization"
            | "rbac"
            | "playwright"
            | "e2e"
            | "webapp-testing"
            | "profiler"
            | "perf"
            | "tailwind"
            | "tailwindcss"
            | "a11y"
            | "accessibility"
            | "security-audit"
    ) {
        return 0.95;
    }

    0.72
}

fn evaluate_slot_field_authority(
    slot: &QueryIntentSlot,
    name_lower: &str,
    title_lower: &str,
    tags_lower: &str,
    guide_lower: &str,
    desc_lower: &str,
    full_text_lower: &str,
    name_tokens: &HashSet<&str>,
    title_tokens: &HashSet<&str>,
    tags_tokens: &HashSet<&str>,
    guide_tokens: &HashSet<&str>,
    desc_tokens: &HashSet<&str>,
    body_dilution: f64,
) -> (f64, &'static str) {
    let mut best: f64 = 0.0;
    let mut best_field: &'static str = "";
    let name_tok_count = name_tokens.len().max(1);

    for (idx, syn) in slot.synonyms.iter().enumerate() {
        let syn_w = synonym_tier_weight(&slot.raw_term, syn, idx, full_text_lower);

        if name_lower == syn {
            let score = 10.0 * syn_w;
            if score > best {
                best = score;
                best_field = "name_exact";
            }
        } else if term_matches_text_with_boundary(syn, name_lower, name_tokens) {
            let specificity_bonus = if name_lower.starts_with(syn.as_str()) && name_tok_count <= 3 {
                0.8
            } else if name_tok_count <= 2 {
                0.5
            } else if name_tok_count >= 5 {
                -0.8
            } else {
                0.0
            };
            let score = (8.5 + specificity_bonus) * syn_w;
            if score > best {
                best = score;
                best_field = "name";
            }
        } else if term_matches_text_with_boundary(syn, title_lower, title_tokens) {
            let score = 7.5 * syn_w;
            if score > best {
                best = score;
                best_field = "titleZh";
            }
        } else if term_matches_text_with_boundary(syn, tags_lower, tags_tokens) {
            let score = 7.5 * syn_w;
            if score > best {
                best = score;
                best_field = "tags";
            }
        } else if term_matches_text_with_boundary(syn, guide_lower, guide_tokens) {
            let score = 4.5 * syn_w * body_dilution;
            if score > best {
                best = score;
                best_field = "guide";
            }
        } else if term_matches_text_with_boundary(syn, desc_lower, desc_tokens) {
            let score = 2.4 * syn_w * body_dilution;
            if score > best {
                best = score;
                best_field = "description";
            }
        }
    }
    (best, best_field)
}

fn format_field_label_zh(field: &str) -> &'static str {
    match field {
        "name_exact" | "name" => "名称",
        "titleZh" => "标题",
        "tags" => "标签",
        "guide" => "使用指南",
        "description" => "描述",
        _ => "内容",
    }
}

/// Primary production search entry point (v3.2):
/// - Uses Phase 1A FTS5 tables (`skill_registry_entries`, `skill_registry_fts`)
/// - Applies `stage`, `category`, and `allowed_ids` filters **before** Top-K truncation
/// - Ranks candidates with v3.2 Synonym Tiering + Field-Authority Ranker + Dual-Pillar Alignment
/// - Explicitly classifies results into `"browse"`, `"exact"`, `"general_fallback"`, or `"none"`
/// - Automatically falls back to `fallback_substring_search_skill_registry` if FTS5 encounters an error
pub fn search_skill_registry(
    database: &AppDatabase,
    request: &SkillSearchRequest,
) -> Result<SkillSearchResponse, String> {
    let t0 = Instant::now();
    let _ = ensure_skill_registry_fts_healthy_once(database);

    let response = match search_skill_registry_fts_v32(database, request, t0) {
        Ok(resp) => resp,
        Err(fts_err) => {
            log::warn!("FTS5 搜索异常 ({fts_err})，尝试自愈索引并启用安全回退搜索");
            unmark_db_fts_verified(database.path());
            if rebuild_skill_registry_fts(database, true).is_ok() {
                if let Ok(repaired_resp) = search_skill_registry_fts_v32(database, request, t0) {
                    repaired_resp
                } else {
                    fallback_substring_search_skill_registry(database, request, t0)?
                }
            } else {
                fallback_substring_search_skill_registry(database, request, t0)?
            }
        }
    };
    Ok(response)
}

fn search_skill_registry_fts_v32(
    database: &AppDatabase,
    request: &SkillSearchRequest,
    t0: Instant,
) -> Result<SkillSearchResponse, String> {
    let connection = database.connect_read()?;
    connection
        .execute_batch("BEGIN DEFERRED;")
        .map_err(|e| format!("开启技能只读快照事务失败: {e}"))?;
    let catalog_revision = get_skill_registry_revision_from_conn(&connection)?;
    let offset = request.offset.unwrap_or(0);
    let limit = request.limit.unwrap_or(500).max(1);
    let trimmed_query = request.query.trim();
    let stage_filter = request
        .stage
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "all");
    let category_filter = request
        .category
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "all");
    let allowed_set: Option<HashSet<String>> = request
        .allowed_ids
        .as_ref()
        .map(|v| v.iter().cloned().collect());

    // 1. Browse mode (empty search query)
    if trimmed_query.is_empty() {
        let rows = query_skill_registry_candidates_with_conn(
            &connection,
            None,
            &SkillRegistryFilter {
                stage: stage_filter,
                category: category_filter,
                allowed_ids: allowed_set.as_ref(),
                limit: MAX_SEARCH_CANDIDATES,
            },
        )?;
        let is_truncated = rows.len() >= MAX_SEARCH_CANDIDATES;
        let mut hits = Vec::with_capacity(rows.len());
        for row in rows {
            let skill_val = serde_json::from_str::<Value>(&row.payload_json).unwrap_or_else(|_| json!({}));
            let score = skill_val
                .get("halowakeScore")
                .and_then(Value::as_f64)
                .unwrap_or(row.spec_score as f64);
            hits.push(SkillSearchHit {
                id: row.id,
                name: row.name,
                title_zh: row.title_zh,
                score,
                core_matched: 0,
                core_total: 0,
                tech_core_matched: 0,
                non_tech_core_matched: 0,
                match_reason: SkillSearchMatchReason {
                    tier: "browse".to_string(),
                    summary: "目录浏览".to_string(),
                    matched_fields: Vec::new(),
                    matched_terms: Vec::new(),
                },
                skill: skill_val,
            });
        }
        match request.sort.as_deref() {
            Some("name") => {
                hits.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
            }
            Some("popular") => {
                hits.sort_by(|a, b| {
                    let stars_a = a.skill.get("githubStars").or_else(|| a.skill.get("stars")).and_then(Value::as_i64).unwrap_or(0);
                    let stars_b = b.skill.get("githubStars").or_else(|| b.skill.get("stars")).and_then(Value::as_i64).unwrap_or(0);
                    stars_b.cmp(&stars_a)
                        .then_with(|| a.name.cmp(&b.name))
                        .then_with(|| a.id.cmp(&b.id))
                });
            }
            _ => {
                hits.sort_by(|a, b| {
                    b.score
                        .partial_cmp(&a.score)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| a.name.cmp(&b.name))
                        .then_with(|| a.id.cmp(&b.id))
                });
            }
        }
        let total_matched = hits.len();
        let (paged_hits, has_more) = if offset < hits.len() {
            let end = (offset + limit).min(hits.len());
            let has_more = end < hits.len();
            (hits[offset..end].to_vec(), has_more)
        } else {
            (Vec::new(), false)
        };
        return Ok(SkillSearchResponse {
            hits: paged_hits,
            total_matched,
            match_tier: "browse".to_string(),
            fallback_notice: None,
            parsed_terms: Vec::new(),
            used_fallback_engine: false,
            elapsed_ms: t0.elapsed().as_secs_f64() * 1000.0,
            has_more,
            is_truncated,
            catalog_revision: catalog_revision.clone(),
        });
    }

    // 2. Parse query into structured Intent Slots
    let slots = parse_search_query(trimmed_query);
    let parsed_terms: Vec<String> = slots.iter().map(|s| s.raw_term.clone()).collect();
    if slots.is_empty() {
        return Ok(SkillSearchResponse {
            hits: Vec::new(),
            total_matched: 0,
            match_tier: "none".to_string(),
            fallback_notice: None,
            parsed_terms,
            used_fallback_engine: false,
            elapsed_ms: t0.elapsed().as_secs_f64() * 1000.0,
            has_more: false,
            is_truncated: false,
            catalog_revision: catalog_revision.clone(),
        });
    }

    let slot_exprs: Vec<String> = slots.iter().filter_map(slot_to_fts_group_expr).collect();
    if slot_exprs.is_empty() {
        return Ok(SkillSearchResponse {
            hits: Vec::new(),
            total_matched: 0,
            match_tier: "none".to_string(),
            fallback_notice: None,
            parsed_terms,
            used_fallback_engine: false,
            elapsed_ms: t0.elapsed().as_secs_f64() * 1000.0,
            has_more: false,
            is_truncated: false,
            catalog_revision: catalog_revision.clone(),
        });
    }
    let fts_match_expr = slot_exprs.join(" OR ");

    let candidates = query_skill_registry_candidates_with_conn(
        &connection,
        Some(&fts_match_expr),
        &SkillRegistryFilter {
            stage: stage_filter,
            category: category_filter,
            allowed_ids: allowed_set.as_ref(),
            limit: MAX_SEARCH_CANDIDATES,
        },
    )?;

    if candidates.is_empty() {
        return Ok(SkillSearchResponse {
            hits: Vec::new(),
            total_matched: 0,
            match_tier: "none".to_string(),
            fallback_notice: None,
            parsed_terms,
            used_fallback_engine: false,
            elapsed_ms: t0.elapsed().as_secs_f64() * 1000.0,
            has_more: false,
            is_truncated: false,
            catalog_revision: catalog_revision.clone(),
        });
    }

    let max_bm25 = candidates
        .iter()
        .map(|c| (-c.bm25_raw).max(0.0))
        .fold(0.0_f64, f64::max);

    let core_total = slots.iter().filter(|s| s.is_core).count().max(1);
    let has_tech_core = slots.iter().any(|s| s.is_core && s.is_tech_stack);
    let has_non_tech_core = slots.iter().any(|s| s.is_core && !s.is_tech_stack);
    let tech_slot_names: Vec<String> = slots
        .iter()
        .filter(|s| s.is_core && s.is_tech_stack)
        .map(|s| s.raw_term.clone())
        .collect();
    let task_slot_names: Vec<String> = slots
        .iter()
        .filter(|s| s.is_core && !s.is_tech_stack)
        .map(|s| s.raw_term.clone())
        .collect();

    const STRONG_SLOT_AUTH_GATE: f64 = 1.65;

    struct ScoredCandidate<'a> {
        repo_key: &'a str,
        repo_stars: i64,
        payload_json: &'a str,
        tier_floor: f64,
        hit: SkillSearchHit,
    }

    let mut scored_hits: Vec<ScoredCandidate<'_>> = Vec::new();

    for cand in &candidates {
        let name_lower = cand.name.to_lowercase();
        let title_lower = format!("{} {}", cand.title_zh, cand.category).to_lowercase();
        let tags_lower = cand.tags_text.to_lowercase();
        let guide_lower = cand.guide_text.to_lowercase();
        let desc_lower = cand.description.to_lowercase();
        let full_text_lower =
            format!("{name_lower} {title_lower} {tags_lower} {guide_lower} {desc_lower}");

        fn split_ascii(s: &str) -> HashSet<&str> {
            s.split(|c: char| !c.is_ascii_alphanumeric())
                .filter(|w| !w.is_empty())
                .collect()
        }
        let name_tokens = split_ascii(&name_lower);
        let title_tokens = split_ascii(&title_lower);
        let tags_tokens = split_ascii(&tags_lower);
        let guide_tokens = split_ascii(&guide_lower);
        let desc_tokens = split_ascii(&desc_lower);

        let distinct_tech_in_desc = KNOWN_TECH_STACKS
            .iter()
            .filter(|&&t| desc_tokens.contains(t))
            .count();
        let body_word_count = desc_tokens.len() + guide_tokens.len();
        let body_dilution = if distinct_tech_in_desc >= 4 && body_word_count >= 35 {
            0.35
        } else if body_word_count > 60 {
            0.75
        } else {
            1.0
        };

        let mut core_matched = 0usize;
        let mut tech_core_matched = 0usize;
        let mut non_tech_core_matched = 0usize;
        let mut high_auth_core_hits = 0usize;
        let mut head_slot_in_name = false;
        let mut exact_name_hit = false;
        let mut weighted_auth_sum = 0.0;
        let mut matched_slot_names = Vec::new();
        let mut matched_fields_vec: Vec<String> = Vec::new();

        for (slot_idx, slot) in slots.iter().enumerate() {
            let (auth, field_key) = evaluate_slot_field_authority(
                slot,
                &name_lower,
                &title_lower,
                &tags_lower,
                &guide_lower,
                &desc_lower,
                &full_text_lower,
                &name_tokens,
                &title_tokens,
                &tags_tokens,
                &guide_tokens,
                &desc_tokens,
                body_dilution,
            );

            if auth >= STRONG_SLOT_AUTH_GATE {
                matched_slot_names.push(slot.raw_term.clone());
                if !field_key.is_empty() {
                    let normalized_field = if field_key == "name_exact" {
                        exact_name_hit = true;
                        "name"
                    } else {
                        field_key
                    };
                    if !matched_fields_vec.iter().any(|f| f == normalized_field) {
                        matched_fields_vec.push(normalized_field.to_string());
                    }
                }

                if slot.is_core {
                    core_matched += 1;
                    if auth >= 5.5 {
                        high_auth_core_hits += 1;
                        if slot_idx == 0 {
                            head_slot_in_name = true;
                        }
                    }
                    if slot.is_tech_stack {
                        tech_core_matched += 1;
                        weighted_auth_sum += auth * 11.5;
                    } else {
                        non_tech_core_matched += 1;
                        weighted_auth_sum += auth * 14.0;
                    }
                } else {
                    weighted_auth_sum += auth * 4.0;
                }
            } else if auth > 0.0 {
                weighted_auth_sum += auth * 1.5;
            }
        }

        if matched_slot_names.is_empty() || (core_matched == 0 && weighted_auth_sum < 8.0) {
            continue;
        }

        let pillar_tier = if has_tech_core && has_non_tech_core {
            if tech_core_matched > 0 && non_tech_core_matched > 0 {
                if high_auth_core_hits > 0 {
                    2.0
                } else {
                    1.5
                }
            } else if non_tech_core_matched > 0 {
                1.0
            } else {
                0.0
            }
        } else if high_auth_core_hits > 0 {
            2.0
        } else {
            1.5
        };

        let positive_bm25 = (-cand.bm25_raw).max(0.0);
        let rel_bm25 = if max_bm25 > 0.0 {
            100.0 * (positive_bm25 / max_bm25)
        } else {
            100.0
        };

        let avg_slot_auth_score = weighted_auth_sum / (slots.len() as f64);

        let project_stack_score = if request.project_tech_stack.is_empty() {
            0.0
        } else {
            let matched_stack = request
                .project_tech_stack
                .iter()
                .filter(|t| {
                    let tl = t.to_lowercase();
                    name_tokens.contains(tl.as_str())
                        || tags_tokens.contains(tl.as_str())
                        || desc_tokens.contains(tl.as_str())
                })
                .count();
            (matched_stack as f64 * 50.0).min(100.0)
        };

        let quality_score = (cand.spec_score as f64).clamp(0.0, 100.0);
        let repo_log_score = ((cand.repo_stars.max(0) as f64 + 1.0).log10() * 20.0).min(100.0);

        let dedicated_field_bonus = (high_auth_core_hits as f64) * 22.0
            + if head_slot_in_name { 18.0 } else { 0.0 }
            + if name_lower.contains("best-practices") { 10.0 } else { 0.0 };

        let intra_tier_score = 0.62 * avg_slot_auth_score
            + dedicated_field_bonus
            + 0.18 * rel_bm25
            + 0.08 * project_stack_score
            + 0.08 * quality_score
            + 0.04 * repo_log_score;

        let tier_floor = pillar_tier * 100_000.0 + (core_matched as f64) * 10_000.0;
        let base_score = tier_floor + intra_tier_score;

        // Determine per-skill match tier & human-readable match reason
        let is_exact_hit = (!has_tech_core || tech_core_matched > 0)
            && (!has_non_tech_core || non_tech_core_matched > 0)
            && core_matched >= core_total
            && high_auth_core_hits > 0;

        let field_labels: Vec<&str> = matched_fields_vec
            .iter()
            .map(|f| format_field_label_zh(f))
            .collect();
        let fields_joined = if field_labels.is_empty() {
            "内容".to_string()
        } else {
            field_labels.join("/")
        };
        let terms_joined = matched_slot_names.join(" + ");

        let (reason_tier, reason_summary) = if exact_name_hit && is_exact_hit {
            (
                "exact".to_string(),
                format!("名称精确匹配 · {terms_joined}"),
            )
        } else if is_exact_hit {
            (
                "exact".to_string(),
                format!("完全相关 · 命中{fields_joined}（{terms_joined}）"),
            )
        } else if has_tech_core && has_non_tech_core && tech_core_matched == 0 && non_tech_core_matched > 0 {
            let missing_tech = tech_slot_names.join("/");
            (
                "general".to_string(),
                format!("通用可用 · 命中{fields_joined}「{terms_joined}」（未限定 {missing_tech}）"),
            )
        } else {
            (
                "general".to_string(),
                format!("通用可用 · 命中{fields_joined}（{terms_joined}）"),
            )
        };

        scored_hits.push(ScoredCandidate {
            repo_key: cand.repo_key.as_str(),
            repo_stars: cand.repo_stars,
            payload_json: cand.payload_json.as_str(),
            tier_floor,
            hit: SkillSearchHit {
                id: cand.id.clone(),
                name: cand.name.clone(),
                title_zh: cand.title_zh.clone(),
                score: base_score,
                core_matched,
                core_total,
                tech_core_matched,
                non_tech_core_matched,
                match_reason: SkillSearchMatchReason {
                    tier: reason_tier,
                    summary: reason_summary,
                    matched_fields: matched_fields_vec,
                    matched_terms: matched_slot_names,
                },
                skill: Value::Null,
            },
        });
    }

    // If the query has a concrete non-tech task intent (e.g. `Angular 国际化` or `Angular 量子芯片光刻`),
    // never let pure tech-stack-only hits (`non_tech_core_matched == 0`, e.g. `igniteui-angular-figma-to-app`)
    // masquerade as task matches!
    if has_non_tech_core {
        scored_hits.retain(|item| item.hit.non_tech_core_matched > 0);
    }

    if scored_hits.is_empty() {
        return Ok(SkillSearchResponse {
            hits: Vec::new(),
            total_matched: 0,
            match_tier: "none".to_string(),
            fallback_notice: None,
            parsed_terms,
            used_fallback_engine: false,
            elapsed_ms: t0.elapsed().as_secs_f64() * 1000.0,
            has_more: false,
            is_truncated: false,
            catalog_revision: catalog_revision.clone(),
        });
    }

    scored_hits.sort_by(|a, b| {
        b.hit
            .score
            .partial_cmp(&a.hit.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.hit.name.cmp(&b.hit.name))
            .then_with(|| a.hit.id.cmp(&b.hit.id))
    });

    let mut repo_counts: HashMap<&str, usize> = HashMap::new();
    for item in &mut scored_hits {
        if !item.repo_key.is_empty() {
            let count = repo_counts.entry(item.repo_key).or_insert(0);
            *count += 1;
            let remainder = item.hit.score - item.tier_floor;
            let decay = match *count {
                1 | 2 => 1.0,
                3 => 0.85,
                4 => 0.70,
                _ => 0.55,
            };
            item.hit.score = item.tier_floor + remainder * decay;
        }
    }

    // Determine overall response `match_tier` and `fallback_notice` from Top relevance hit
    let top_relevance_hit = scored_hits
        .iter()
        .max_by(|a, b| {
            a.hit
                .score
                .partial_cmp(&b.hit.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.hit.name.cmp(&a.hit.name))
        })
        .map(|item| item.hit.clone());

    let (match_tier, fallback_notice) = if let Some(top_hit) = top_relevance_hit.as_ref() {
        if top_hit.match_reason.tier == "exact" {
            ("exact".to_string(), None)
        } else if has_tech_core && has_non_tech_core && top_hit.tech_core_matched == 0 {
            let missing_tech = tech_slot_names.join(" / ");
            let matched_task = if task_slot_names.is_empty() {
                top_hit.match_reason.matched_terms.join(" / ")
            } else {
                task_slot_names.join(" / ")
            };
            (
                "general_fallback".to_string(),
                Some(format!(
                    "当前目录暂无「{missing_tech}」专属技能，已为您展示匹配「{matched_task}」的通用可用技能"
                )),
            )
        } else {
            (
                "general_fallback".to_string(),
                Some("未找到完全覆盖所有限定词的专属技能，已为您展示最相关的通用可用技能".to_string()),
            )
        }
    } else {
        ("none".to_string(), None)
    };

    // Sort according to request.sort
    match request.sort.as_deref() {
        Some("name") => {
            scored_hits.sort_by(|a, b| a.hit.name.cmp(&b.hit.name).then_with(|| a.hit.id.cmp(&b.hit.id)));
        }
        Some("popular") => {
            scored_hits.sort_by(|a, b| {
                b.repo_stars
                    .cmp(&a.repo_stars)
                    .then_with(|| {
                        b.hit
                            .score
                            .partial_cmp(&a.hit.score)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .then_with(|| a.hit.name.cmp(&b.hit.name))
                    .then_with(|| a.hit.id.cmp(&b.hit.id))
            });
        }
        _ => {
            // relevance / default
            scored_hits.sort_by(|a, b| {
                b.hit
                    .score
                    .partial_cmp(&a.hit.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.hit.name.cmp(&b.hit.name))
                    .then_with(|| a.hit.id.cmp(&b.hit.id))
            });
        }
    }

    let total_matched = scored_hits.len();
    let is_truncated = candidates.len() >= MAX_SEARCH_CANDIDATES;
    let offset = request.offset.unwrap_or(0);
    let (final_hits, has_more) = if offset < scored_hits.len() {
        let end = (offset + limit).min(scored_hits.len());
        let has_more = end < scored_hits.len();
        let hits = scored_hits[offset..end]
            .iter()
            .map(|item| {
                let mut hit = item.hit.clone();
                hit.skill =
                    serde_json::from_str::<Value>(item.payload_json).unwrap_or_else(|_| json!({}));
                hit
            })
            .collect();
        (hits, has_more)
    } else {
        (Vec::new(), false)
    };

    Ok(SkillSearchResponse {
        hits: final_hits,
        total_matched,
        match_tier,
        fallback_notice,
        parsed_terms,
        used_fallback_engine: false,
        elapsed_ms: t0.elapsed().as_secs_f64() * 1000.0,
        has_more,
        is_truncated,
        catalog_revision: catalog_revision.clone(),
    })
}

/// Legacy substring fallback search in Rust: invoked automatically if FTS5 ever errors.
fn fallback_substring_search_skill_registry(
    database: &AppDatabase,
    request: &SkillSearchRequest,
    t0: Instant,
) -> Result<SkillSearchResponse, String> {
    let limit = request.limit.unwrap_or(500).max(1);
    let q_lower = request.query.trim().to_lowercase();
    let stage_filter = request
        .stage
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "all");
    let category_filter = request
        .category
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "all");
    let allowed_set: Option<HashSet<String>> = request
        .allowed_ids
        .as_ref()
        .map(|v| v.iter().cloned().collect());

    let connection = database.connect_read()?;
    connection
        .execute_batch("BEGIN DEFERRED;")
        .map_err(|e| format!("开启技能回退只读快照事务失败: {e}"))?;
    let catalog_revision = get_skill_registry_revision_from_conn(&connection)?;

    let rows = query_skill_registry_candidates_with_conn(
        &connection,
        None,
        &SkillRegistryFilter {
            stage: stage_filter,
            category: category_filter,
            allowed_ids: allowed_set.as_ref(),
            limit: MAX_SEARCH_CANDIDATES,
        },
    )?;
    let is_truncated = rows.len() >= MAX_SEARCH_CANDIDATES;

    let mut hits = Vec::new();
    for row in rows {
        let name_hit = row.name.to_lowercase().contains(&q_lower);
        let title_hit = row.title_zh.to_lowercase().contains(&q_lower);
        let tags_hit = row.tags_text.to_lowercase().contains(&q_lower);
        let desc_hit = row.description.to_lowercase().contains(&q_lower);

        if q_lower.is_empty() || name_hit || title_hit || tags_hit || desc_hit {
            let skill_val =
                serde_json::from_str::<Value>(&row.payload_json).unwrap_or_else(|_| json!({}));
            let mut matched_fields = Vec::new();
            if name_hit {
                matched_fields.push("name".to_string());
            }
            if title_hit {
                matched_fields.push("titleZh".to_string());
            }
            if tags_hit {
                matched_fields.push("tags".to_string());
            }
            if desc_hit {
                matched_fields.push("description".to_string());
            }
            let score = if name_hit {
                100.0
            } else if title_hit || tags_hit {
                80.0
            } else {
                50.0
            };
            hits.push(SkillSearchHit {
                id: row.id,
                name: row.name,
                title_zh: row.title_zh,
                score,
                core_matched: 1,
                core_total: 1,
                tech_core_matched: 0,
                non_tech_core_matched: 1,
                match_reason: SkillSearchMatchReason {
                    tier: "general".to_string(),
                    summary: "回退检索匹配".to_string(),
                    matched_fields,
                    matched_terms: vec![request.query.trim().to_string()],
                },
                skill: skill_val,
            });
        }
    }

    match request.sort.as_deref() {
        Some("name") => {
            hits.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
        }
        Some("popular") => {
            hits.sort_by(|a, b| {
                let stars_a = a.skill.get("githubStars").or_else(|| a.skill.get("stars")).and_then(Value::as_i64).unwrap_or(0);
                let stars_b = b.skill.get("githubStars").or_else(|| b.skill.get("stars")).and_then(Value::as_i64).unwrap_or(0);
                stars_b.cmp(&stars_a)
                    .then_with(|| a.name.cmp(&b.name))
                    .then_with(|| a.id.cmp(&b.id))
            });
        }
        _ => {
            hits.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.name.cmp(&b.name))
                    .then_with(|| a.id.cmp(&b.id))
            });
        }
    }
    let total_matched = hits.len();
    let offset = request.offset.unwrap_or(0);
    let (paged_hits, has_more) = if offset < hits.len() {
        let end = (offset + limit).min(hits.len());
        let has_more = end < hits.len();
        (hits[offset..end].to_vec(), has_more)
    } else {
        (Vec::new(), false)
    };

    let match_tier = if paged_hits.is_empty() {
        "none".to_string()
    } else {
        "general_fallback".to_string()
    };

    Ok(SkillSearchResponse {
        hits: paged_hits,
        total_matched,
        match_tier,
        fallback_notice: Some("已启用基础回退检索模式".to_string()),
        parsed_terms: vec![request.query.trim().to_string()],
        used_fallback_engine: true,
        elapsed_ms: t0.elapsed().as_secs_f64() * 1000.0,
        has_more,
        is_truncated,
        catalog_revision,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::migrations::run_migrations;

    fn temp_db_path(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "lumina-skill-registry-{}-{}-{}.db",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn failed_refresh_keeps_last_complete_catalog_and_fts_in_sync() {
        let path = temp_db_path("atomic-rollback");
        let database = AppDatabase::from_path(path.clone());
        let mut connection = database.connect().unwrap();
        run_migrations(&mut connection).unwrap();
        drop(connection);

        let skill = json!({
            "id": "one",
            "name": "Example",
            "titleZh": "示例国际化技能",
            "description": "Example skill for i18n",
            "stage": "coding",
            "category": "frontend",
            "tags": ["i18n", "vue3"]
        });
        let good = json!({"version":"1","updatedAt":123,"skills":[skill],"pipelines":[]});
        save_skill_registry(&database, &good).unwrap();

        let skill_two = json!({
            "id": "two",
            "name": "Transient",
            "titleZh": "不应残留的技能",
            "description": "Should be rolled back",
            "stage": "testing"
        });
        let bad = json!({"version":"2","updatedAt":456,"skills":[skill_two, skill, skill],"pipelines":[]});
        assert!(save_skill_registry(&database, &bad).is_err());

        let loaded = load_skill_registry(&database).unwrap().unwrap();
        assert_eq!(loaded["updatedAt"], 123);
        assert_eq!(loaded["skills"].as_array().unwrap().len(), 1);

        let conn = database.connect().unwrap();
        let main_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM skill_registry_entries", [], |r| r.get(0))
            .unwrap();
        let fts_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM skill_registry_fts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(main_count, 1);
        assert_eq!(fts_count, 1);

        let transient_hits: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM skill_registry_fts WHERE skill_registry_fts MATCH '\"transient\"'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(transient_hits, 0, "Failed batch must not leave orphan FTS entries");

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn filter_before_truncation_recalls_starred_skill_beyond_rank_200() {
        let path = temp_db_path("filter-before-truncation");
        let database = AppDatabase::from_path(path.clone());
        let mut connection = database.connect().unwrap();
        run_migrations(&mut connection).unwrap();
        drop(connection);

        let mut skills = Vec::new();
        for idx in 1..=260 {
            let is_target = idx == 250;
            skills.push(json!({
                "id": format!("skill-{idx}"),
                "name": format!("react-helper-{idx}"),
                "titleZh": if is_target { "目標收藏技能" } else { "普通技能" },
                "description": "React component testing utility",
                "stage": if is_target { "testing" } else { "coding" },
                "category": if is_target { "qa-testing" } else { "frontend" },
                "tags": ["react"]
            }));
        }
        let catalog = json!({
            "version": "1.0.0",
            "updatedAt": 789,
            "skills": skills,
            "pipelines": []
        });
        save_skill_registry(&database, &catalog).unwrap();

        let mut starred = HashSet::new();
        starred.insert("skill-250".to_string());

        let results = query_skill_registry_candidates(
            &database,
            Some("\"react\""),
            &SkillRegistryFilter {
                stage: Some("testing"),
                category: Some("qa-testing"),
                allowed_ids: Some(&starred),
                limit: 50,
            },
        )
        .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "skill-250");

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn rebuild_fts_recovers_desynced_index_and_detects_same_row_count_content_mutation() {
        let path = temp_db_path("rebuild-fts-content-consistency");
        let database = AppDatabase::from_path(path.clone());
        let mut connection = database.connect().unwrap();
        run_migrations(&mut connection).unwrap();
        drop(connection);

        let skill = json!({
            "id": "sync-1",
            "name": "vue-i18n-helper",
            "titleZh": "Vue 国际化助手",
            "description": "Internationalization support",
            "stage": "coding"
        });
        save_skill_registry(
            &database,
            &json!({"version":"1","updatedAt":100,"skills":[skill],"pipelines":[]}),
        )
        .unwrap();

        // 1. Healthy index does not rebuild
        assert!(!rebuild_skill_registry_fts_if_needed(&database).unwrap());

        // 2. Mutate main table content WITHOUT changing row count (1 row in main, 1 row in FTS!)
        let updated_payload = json!({
            "id": "sync-1",
            "name": "playwright-visual-regression",
            "titleZh": "Playwright 视觉回归测试",
            "description": "Visual regression snapshot testing with Playwright",
            "stage": "testing"
        });
        let conn = database.connect().unwrap();
        conn.execute(
            "UPDATE skill_registry_entries
             SET name = 'playwright-visual-regression',
                 title_zh = 'Playwright 视觉回归测试',
                 description = 'Visual regression snapshot testing with Playwright',
                 stage = 'testing',
                 payload_json = ?1
             WHERE id = 'sync-1'",
            params![updated_payload.to_string()],
        )
        .unwrap();
        let main_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM skill_registry_entries", [], |r| r.get(0))
            .unwrap();
        let fts_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM skill_registry_fts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(main_count, 1);
        assert_eq!(fts_count, 1, "Row counts are identical (1 == 1), only content changed");
        drop(conn);

        // `rebuild_skill_registry_fts_if_needed` MUST detect the content digest mismatch and repair FTS!
        let repaired_after_main_mutation = rebuild_skill_registry_fts_if_needed(&database).unwrap();
        assert!(
            repaired_after_main_mutation,
            "Must detect main table content change even when main_count == fts_count"
        );
        assert!(!rebuild_skill_registry_fts_if_needed(&database).unwrap());

        // Verify the updated content is now searchable via FTS5
        let search_resp = search_skill_registry(
            &database,
            &SkillSearchRequest {
                query: "视觉回归测试".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(search_resp.hits.len(), 1);
        assert_eq!(search_resp.hits[0].id, "sync-1");
        assert_eq!(search_resp.hits[0].name, "playwright-visual-regression");

        // 3. Corrupt `skill_registry_fts` token content directly WITHOUT changing row count (1 == 1)
        let conn = database.connect().unwrap();
        conn.execute(
            "UPDATE skill_registry_fts SET name_tokens = 'corrupted_token', title_tokens = '' WHERE id = 'sync-1'",
            [],
        )
        .unwrap();
        drop(conn);

        let repaired_after_fts_corruption = rebuild_skill_registry_fts_if_needed(&database).unwrap();
        assert!(
            repaired_after_fts_corruption,
            "Must detect FTS token corruption even when main_count == fts_count"
        );
        assert!(!rebuild_skill_registry_fts_if_needed(&database).unwrap());

        // 4. Explicit forced rebuild (`force = true`) unconditionally rebuilds even when already consistent
        let forced = rebuild_skill_registry_fts(&database, true).unwrap();
        assert!(forced, "Explicit force=true must unconditionally rebuild FTS index");

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn v32_search_distinguishes_exact_general_fallback_and_none() {
        let path = temp_db_path("v32-search-tiers");
        let database = AppDatabase::from_path(path.clone());
        let mut connection = database.connect().unwrap();
        run_migrations(&mut connection).unwrap();
        drop(connection);

        let catalog = json!({
            "version": "1.0.0",
            "updatedAt": 1000,
            "skills": [
                {
                    "id": "s-i18n-parity",
                    "name": "i18n-parity",
                    "titleZh": "多语言国际化校验",
                    "description": "Check locale and translation files for missing keys.",
                    "stage": "coding",
                    "category": "Frontend",
                    "tags": ["i18n", "localization"]
                },
                {
                    "id": "s-angular-figma",
                    "name": "igniteui-angular-figma-to-app",
                    "titleZh": "Angular Figma 转代码",
                    "description": "Translate Figma designs into Angular components.",
                    "stage": "coding",
                    "category": "Frontend",
                    "tags": ["angular", "figma"]
                },
                {
                    "id": "s-rtl",
                    "name": "gen-rtl-test",
                    "titleZh": "React RTL 单元测试生成",
                    "description": "Generate React Testing Library unit tests for React components and hooks.",
                    "stage": "testing",
                    "category": "QA",
                    "tags": ["react", "rtl", "unit-test"]
                }
            ],
            "pipelines": []
        });
        save_skill_registry(&database, &catalog).unwrap();

        // 1. Exact Match: "React 单元测试" -> gen-rtl-test (matches both React and 单元测试)
        let exact_resp = search_skill_registry(
            &database,
            &SkillSearchRequest {
                query: "React 单元测试".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(exact_resp.match_tier, "exact");
        assert!(exact_resp.fallback_notice.is_none());
        assert_eq!(exact_resp.hits[0].name, "gen-rtl-test");
        assert_eq!(exact_resp.hits[0].match_reason.tier, "exact");

        // 2. General Fallback: "Angular 国际化" -> i18n-parity (general i18n skill; igniteui-angular-figma-to-app is excluded!)
        let fallback_resp = search_skill_registry(
            &database,
            &SkillSearchRequest {
                query: "Angular 国际化".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(fallback_resp.match_tier, "general_fallback");
        assert!(fallback_resp.fallback_notice.is_some());
        assert_eq!(fallback_resp.hits.len(), 1);
        assert_eq!(fallback_resp.hits[0].name, "i18n-parity");
        assert_eq!(fallback_resp.hits[0].match_reason.tier, "general");

        // 3. No Suitable Result: "量子计算芯片光刻仿真" -> none
        let none_resp = search_skill_registry(
            &database,
            &SkillSearchRequest {
                query: "量子计算芯片光刻仿真".to_string(),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(none_resp.match_tier, "none");
        assert!(none_resp.hits.is_empty());

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn t05_save_skill_registry_rejects_malformed_schema_and_preserves_existing_rows() {
        let path = temp_db_path("t05-schema-rollback");
        let database = AppDatabase::from_path(path.clone());
        let mut connection = database.connect().unwrap();
        run_migrations(&mut connection).unwrap();
        drop(connection);

        let valid_catalog = json!({
            "version": "1.0.0",
            "updatedAt": 1700000000000i64,
            "skills": [
                {
                    "id": "valid-skill-1",
                    "name": "valid-skill-1",
                    "titleZh": "有效技能",
                    "description": "Valid skill description",
                    "stage": "coding",
                    "gitUrl": "https://github.com/acme/skills",
                    "subPath": "skills/valid-skill-1"
                }
            ],
            "pipelines": []
        });
        save_skill_registry(&database, &valid_catalog).unwrap();

        // Attempt to overwrite with a catalog containing path traversal in subPath
        let bad_subpath_catalog = json!({
            "version": "1.0.1",
            "updatedAt": 1700000001000i64,
            "skills": [
                {
                    "id": "malicious-skill",
                    "name": "malicious-skill",
                    "description": "Traversal",
                    "stage": "coding",
                    "gitUrl": "https://github.com/acme/skills",
                    "subPath": "../outside"
                }
            ],
            "pipelines": []
        });
        let err = save_skill_registry(&database, &bad_subpath_catalog).unwrap_err();
        assert!(err.contains("subPath"), "Unexpected error: {err}");

        // Existing valid catalog in SQLite must remain intact after rollback
        let loaded = load_skill_registry(&database).unwrap().unwrap();
        let loaded_skills = loaded.get("skills").and_then(Value::as_array).unwrap();
        assert_eq!(loaded_skills.len(), 1);
        assert_eq!(
            loaded_skills[0].get("id").and_then(Value::as_str),
            Some("valid-skill-1")
        );

        let _ = std::fs::remove_file(path);
    }
}
