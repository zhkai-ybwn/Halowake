//! Halowake Skills 独立检索与安装安全原型验证 (PoC)
//!
//! 严格遵循第三轮评审要求：
//! 1. 不修改任何现有业务代码，保留工作区未提交改动；
//! 2. 解决中英文混合查询（如 `Vue3组件视觉`、`修复Vue3组件的视觉问题`）丢失中文约束问题；
//! 3. 使用 `cjk1<字>` 替代 `u:<字>`，并通过 `fts5vocab` 实测验证单字为独立 Token；
//! 4. 筛选条件（stage / category / starred_ids）在 Top-K 截断前执行，杜绝第 201+ 名收藏技能丢失；
//! 5. 排序以“核心任务约束覆盖率”为第一主导，杜绝“仅命中 Angular 前缀”压倒“完整命中 Angular + 国际化”；
//! 6. 并发安全的崩溃恢复（Mutex 临界区 + PID 所有权 + 目标目录完整性校验）与异源同名冲突拦截；
//! 7. 同时在合成对抗集与本地真实 `lumina.db`（6,000+ 条真实技能）上对比旧搜索与新搜索的 Hit@5 / MRR@10。

use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// ============================================================================
// 一、词典、同义词与中英文混合自然语言 Query Parser
// ============================================================================

/// 常见固定字母数字术语（不拆分为纯字母词干）
const FIXED_ALNUM_TERMS: &[&str] = &[
    "i18n", "l10n", "a11y", "k8s", "e2e", "s3", "v8", "base64", "sha256", "oauth2", "http2", "3d", "2d",
];

/// 中文开发领域核心概念词表（按长度降序排列，用于无空格中文连续段的双向最大匹配切分）
const CN_DOMAIN_TERMS: &[&str] = &[
    "视觉回归测试", "自动化测试", "单元测试", "集成测试", "端到端测试",
    "状态管理", "代码审查", "代码评审", "视觉回归", "内存泄漏", "内存溢出",
    "权限控制", "性能分析", "性能优化", "构建打包", "接口测试", "慢查询",
    "国际化", "多语言", "本地化", "数据库", "组件库", "命令行", "工作流",
    "浏览器", "暗黑模式", "响应式", "流程图", "可视化",
    "组件", "视觉", "界面", "美化", "样式", "排版", "主题", "暗黑", "设计", "前端", "后端",
    "测试", "单测", "验收", "回归", "截图", "调试", "排查", "排错", "诊断", "日志",
    "重构", "架构", "规范", "需求", "接口", "路由", "表单", "表格", "图表", "动画",
    "部署", "发布", "容器", "打包", "构建", "安全", "审计", "加密", "鉴权", "登录", "权限",
    "文档", "翻译", "注释", "脚本", "爬虫", "缓存", "并发", "异步", "内存", "性能", "优化",
];

/// 中文自然语言口语停用词/泛化虚词（在切分前剥离，避免干扰核心约束）
const CN_STOP_PHRASES: &[&str] = &[
    "帮我给", "帮我", "请问如何", "请问", "如何", "怎么", "怎样", "我想给", "我想",
    "需要一个", "需要", "寻找", "查找", "添加", "增加", "实现", "支持", "进行", "编写",
    "相关的", "相关", "的问题", "问题", "的方案", "方案", "的功能", "功能", "的项目", "项目",
    "的", "了", "在", "中", "里", "和", "与", "或", "及", "给", "为", "对", "把", "将", "做", "用", "写",
];

/// 通用动作修饰词与语法子构件（命中可加分，但不作为一票否决的硬性领域实体约束）
const MODIFIER_TERMS: &[&str] = &[
    "修复", "fix", "优化", "optimize", "规范", "standard", "convention", "guide", "helper", "tool", "workflow", "hook", "hooks",
];

pub fn is_cjk(c: char) -> bool {
    matches!(c as u32, 0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0xF900..=0xFAFF)
}

/// 构建跨语言同义词映射表
fn synonym_dictionary() -> HashMap<&'static str, &'static [&'static str]> {
    let mut m: HashMap<&'static str, &'static [&'static str]> = HashMap::new();
    m.insert("国际化", &["国际化", "i18n", "localization", "locale", "translate", "translation", "l10n", "多语言", "本地化"]);
    m.insert("多语言", &["多语言", "国际化", "i18n", "localization", "locale", "translation"]);
    m.insert("本地化", &["本地化", "国际化", "l10n", "i18n", "localization"]);
    m.insert("i18n", &["i18n", "国际化", "多语言", "localization", "locale", "translation", "l10n"]);
    m.insert("组件", &["组件", "component", "components", "widget", "ui"]);
    m.insert("视觉", &["视觉", "visual", "screenshot", "snapshot", "ui", "aesthetic", "design", "regression"]);
    m.insert("视觉回归", &["视觉回归", "visual", "regression", "screenshot", "playwright", "snapshot"]);
    m.insert("视觉回归测试", &["视觉回归测试", "visual", "regression", "playwright", "screenshot", "e2e"]);
    m.insert("界面", &["界面", "ui", "frontend", "design", "interface", "layout", "style"]);
    m.insert("美化", &["美化", "polish", "ui", "design", "style", "tailwind", "css", "aesthetic"]);
    m.insert("样式", &["样式", "style", "styling", "css", "tailwind", "scss", "less", "theme"]);
    m.insert("排版", &["排版", "typography", "layout", "spacing", "font"]);
    m.insert("测试", &["测试", "test", "testing", "spec", "vitest", "jest", "playwright", "cypress", "pytest", "qa"]);
    m.insert("单元测试", &["单元测试", "unit", "unit-test", "rtl", "vitest", "jest", "pytest", "tdd", "test", "testing", "spec"]);
    m.insert("unit", &["unit", "unit-test", "rtl", "vitest", "jest", "tdd"]);
    m.insert("自动化测试", &["自动化测试", "e2e", "playwright", "cypress", "automation", "browser", "test", "testing"]);
    m.insert("代码审查", &["代码审查", "review", "code-review", "pr", "mr", "audit", "lint"]);
    m.insert("代码评审", &["代码评审", "review", "code-review", "review-pr", "autoreview", "pr", "mr"]);
    m.insert("调试", &["调试", "debug", "debugging", "troubleshoot", "diagnose", "trace"]);
    m.insert("排查", &["排查", "debug", "debugging", "troubleshoot", "diagnose", "investigate", "profile"]);
    m.insert("排错", &["排错", "debug", "troubleshoot", "error", "exception", "fix"]);
    m.insert("内存", &["内存", "memory", "leak", "heap", "profile", "profiler", "allocation"]);
    m.insert("内存泄漏", &["内存泄漏", "memory", "leak", "heap", "profiler"]);
    m.insert("数据库", &["数据库", "database", "sql", "postgres", "postgresql", "mysql", "sqlite", "prisma", "orm"]);
    m.insert("慢查询", &["慢查询", "sql", "query", "performance", "index", "explain", "database", "optimize"]);
    m.insert("打包", &["打包", "build", "bundle", "package", "release", "installer", "compile"]);
    m.insert("部署", &["部署", "deploy", "deployment", "docker", "k8s", "kubernetes", "ci", "cd"]);
    m.insert("容器", &["容器", "docker", "container", "k8s", "kubernetes", "dockerfile"]);
    m.insert("重构", &["重构", "refactor", "refactoring", "clean", "architecture"]);
    m.insert("接口", &["接口", "api", "rest", "graphql", "endpoint", "openapi", "swagger"]);
    m.insert("安全", &["安全", "security", "vulnerability", "audit", "auth", "owasp", "secret"]);
    m.insert("文档", &["文档", "doc", "docs", "documentation", "markdown", "readme"]);
    m.insert("修复", &["修复", "fix", "repair", "resolve", "debug", "patch"]);
    m.insert("单测", &["单测", "单元测试", "unit", "unit-test", "rtl", "vitest", "jest", "tdd", "test", "testing", "spec"]);
    m.insert("端到端测试", &["端到端测试", "e2e", "playwright", "cypress", "webapp-testing", "browser", "test", "testing"]);
    m.insert("登录", &["登录", "login", "auth", "authentication", "oauth", "oauth2", "session", "jwt", "signin"]);
    m.insert("鉴权", &["鉴权", "auth", "authentication", "authorization", "rbac", "oauth", "jwt", "permission"]);
    m.insert("权限控制", &["权限控制", "authorization", "auth", "rbac", "permission", "access", "security"]);
    m.insert("性能优化", &["性能优化", "performance", "perf", "optimize", "optimization", "profiler", "bottleneck", "speed"]);
    m.insert("性能", &["性能", "performance", "perf", "profiler", "optimize", "optimization", "bottleneck"]);
    m.insert("优化", &["优化", "optimize", "optimization", "performance", "perf", "best-practices", "tuning"]);
    m.insert("container", &["container", "containers", "dockerfile", "docker", "k8s", "kubernetes"]);
    m.insert("deployment", &["deployment", "deploy", "devops", "ci", "cd", "production", "k8s", "kubernetes"]);
    m.insert("optimization", &["optimization", "optimize", "performance", "perf", "index", "tuning", "best-practices"]);
    m.insert("表格", &["表格", "table", "grid", "datagrid", "buefy", "component", "ui"]);
    m.insert("组件库", &["组件库", "component", "components", "ui", "design-system", "buefy", "tailwind"]);
    m.insert("状态管理", &["状态管理", "state", "store", "pinia", "vuex", "redux", "zustand", "vueuse"]);
    m
}

/// 索引端分词：
/// 1. 英文按词边界转小写；若为 `vue3`、`angular18` 等技术栈带版本号词，同时输出原词 `vue3` 与词干 `vue`；
/// 2. 中文连续段输出有序相邻 Bi-gram（如 `前端 端国 国际 际化 化方 方案`），以及无分隔符的单字 Token `cjk1<字>`。
pub fn tokenize_for_index(text: &str) -> String {
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
        let digit_len = word.chars().skip(alpha_len).take_while(|c| c.is_ascii_digit()).count();
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
        if buf.len() >= 2 {
            for win in buf.windows(2) {
                tokens.push(format!("{}{}", win[0], win[1]));
            }
        }
        for &ch in buf.iter() {
            // 使用 cjk1 前缀而非 u:，确保 unicode61 将其作为单一完整 Token 索引
            tokens.push(format!("cjk1{ch}"));
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

/// 将一段纯中文文本剥离停用词并按领域词表切分为独立语义词
fn segment_chinese_chunk(chunk: &str) -> Vec<String> {
    let mut working = chunk.trim().to_string();
    if working.is_empty() {
        return Vec::new();
    }

    // 1. 若该中文段本身就是停用词（如“给 React”中被英文空格隔开的孤立“给”、“帮我”），直接剥离
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

/// 单个查询意图槽（Intent Slot）
#[derive(Debug, Clone)]
pub struct QueryIntentSlot {
    pub raw_term: String,
    pub synonyms: Vec<String>,
    pub is_core: bool,       // true: 技术栈/领域实体/核心任务；false: 泛化修饰词（如“修复”、“规范”）
    pub is_tech_stack: bool, // 是否为明确的技术栈/框架词（如 vue, angular, rust, tauri）
}

const KNOWN_TECH_STACKS: &[&str] = &[
    "vue", "vue2", "vue3", "react", "angular", "svelte", "nextjs", "nuxt", "vite", "tailwind",
    "typescript", "javascript", "node", "nodejs", "rust", "cargo", "tauri", "python", "fastapi",
    "django", "go", "golang", "java", "spring", "kotlin", "swift", "flutter", "docker", "kubernetes",
    "k8s", "postgres", "postgresql", "mysql", "sqlite", "redis", "graphql", "playwright", "cypress",
    "vitest", "jest", "git", "github", "gitlab",
];

/// 解析任意中英混合/自然语言查询，输出结构化意图槽列表
pub fn parse_search_query(query: &str) -> Vec<QueryIntentSlot> {
    let dict = synonym_dictionary();
    // 归一化常见带点号框架名（如 Next.js -> nextjs, Node.js -> nodejs, Vue.js -> vue）
    let normalized_query = query
        .replace("Next.js", "nextjs")
        .replace("next.js", "nextjs")
        .replace("Node.js", "nodejs")
        .replace("node.js", "nodejs")
        .replace("Vue.js", "vue")
        .replace("vue.js", "vue")
        .replace("Nuxt.js", "nuxt")
        .replace("nuxt.js", "nuxt");
    let mut raw_segments: Vec<(bool, String)> = Vec::new(); // (is_cjk, text)
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

    // 提取原子概念词
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

    // 去重并构建 QueryIntentSlot
    let mut seen = HashSet::new();
    let mut slots = Vec::new();

    for term in atomic_terms {
        if !seen.insert(term.clone()) {
            continue;
        }
        let mut group = Vec::new();
        group.push(term.clone());

        // 如果是带版本号的英文词（如 vue3, angular18），把词干（vue, angular）也加入同义词组
        let alpha_len = term.chars().take_while(|c| c.is_ascii_alphabetic()).count();
        let digit_len = term.chars().skip(alpha_len).take_while(|c| c.is_ascii_digit()).count();
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

        // 查找同义词表
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

    // 若所有词都被判为 modifier（如只搜了“修复”），则全部提升为 core
    if !slots.is_empty() && slots.iter().all(|s| !s.is_core) {
        for s in &mut slots {
            s.is_core = true;
        }
    }

    slots
}

/// 将单个词转换为 FTS5 原子匹配表达式
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
            let bigrams: Vec<String> = chars.windows(2).map(|w| format!("{}{}", w[0], w[1])).collect();
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

/// 将单个 QueryIntentSlot 转换为 FTS5 括号 OR 表达式：("国际 际化" OR "i18n" OR "localization"*)
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

// ============================================================================
// 二、主表与 FTS5 同事务构建 + 截断前过滤 + 覆盖度优先重排
// ============================================================================

pub fn init_poc_schema(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS skill_registry_snapshot (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            version TEXT NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS skill_registry_entries (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            title_zh TEXT NOT NULL DEFAULT '',
            description TEXT NOT NULL,
            stage TEXT NOT NULL,
            category TEXT NOT NULL DEFAULT '',
            tags_text TEXT NOT NULL DEFAULT '',
            guide_text TEXT NOT NULL DEFAULT '',
            repo_key TEXT NOT NULL DEFAULT '',
            repo_stars INTEGER NOT NULL DEFAULT 0,
            spec_score INTEGER NOT NULL DEFAULT 50,
            payload_json TEXT NOT NULL
        );
        CREATE VIRTUAL TABLE IF NOT EXISTS skill_registry_fts USING fts5(
            id UNINDEXED,
            name_tokens,
            title_tokens,
            tags_tokens,
            summary_tokens,
            desc_tokens,
            tokenize = 'unicode61 remove_diacritics 2'
        );
        CREATE VIRTUAL TABLE IF NOT EXISTS skill_registry_fts_vocab USING fts5vocab(skill_registry_fts, row);
        "#,
    )
    .map_err(|e| format!("初始化 Schema 失败: {e}"))
}

fn extract_repo_key(git_url: &str) -> String {
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

/// 同事务原子更新主表与 FTS5 索引（任何一条失败则整批回滚，绝不破坏旧数据与旧索引）
pub fn save_registry_with_fts_atomic(conn: &mut Connection, version: &str, updated_at: i64, skills: &[Value]) -> Result<(), String> {
    if skills.is_empty() {
        return Err("skills 列表不能为空".to_string());
    }
    let tx = conn.transaction().map_err(|e| format!("开启事务失败: {e}"))?;
    tx.execute("DELETE FROM skill_registry_entries", []).map_err(|e| e.to_string())?;
    tx.execute("DELETE FROM skill_registry_fts", []).map_err(|e| e.to_string())?;

    {
        let mut insert_main = tx
            .prepare(
                "INSERT INTO skill_registry_entries
                 (id, name, title_zh, description, stage, category, tags_text, guide_text, repo_key, repo_stars, spec_score, payload_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            )
            .map_err(|e| e.to_string())?;

        let mut insert_fts = tx
            .prepare(
                "INSERT INTO skill_registry_fts
                 (rowid, id, name_tokens, title_tokens, tags_tokens, summary_tokens, desc_tokens)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )
            .map_err(|e| e.to_string())?;

        for skill in skills {
            let id = skill.get("id").and_then(Value::as_str).unwrap_or("").trim();
            let name = skill.get("name").and_then(Value::as_str).unwrap_or("").trim();
            if id.is_empty() || name.is_empty() {
                return Err("技能缺少 id 或 name".to_string());
            }
            let title_zh = skill.get("titleZh").and_then(Value::as_str).unwrap_or("");
            let desc = skill.get("description").and_then(Value::as_str).unwrap_or("");
            let stage = skill.get("stage").and_then(Value::as_str).unwrap_or("coding");
            let category = skill.get("category").and_then(Value::as_str).unwrap_or("");
            let git_url = skill.get("gitUrl").and_then(Value::as_str).unwrap_or("");
            let repo_key = extract_repo_key(git_url);
            let repo_stars = skill
                .pointer("/metrics/githubStars")
                .or_else(|| skill.get("stars"))
                .and_then(Value::as_i64)
                .unwrap_or(0);
            let spec_score = skill.get("halowakeScore").and_then(Value::as_i64).unwrap_or(50);
            let tags_str = skill
                .get("tags")
                .and_then(Value::as_array)
                .map(|arr| arr.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" "))
                .unwrap_or_default();
            let summary = skill.pointer("/usageGuide/summary").and_then(Value::as_str).unwrap_or("");
            let trigger = skill.pointer("/usageGuide/trigger").and_then(Value::as_str).unwrap_or("");
            let how_to_use = skill.get("howToUse").and_then(Value::as_str).unwrap_or("");
            let guide_text = format!("{summary} {trigger} {how_to_use}");

            let payload = serde_json::to_string(skill).map_err(|e| e.to_string())?;

            insert_main
                .execute(params![
                    id, name, title_zh, desc, stage, category, tags_str, guide_text, repo_key, repo_stars, spec_score, payload
                ])
                .map_err(|e| format!("插入主表失败 ({id}): {e}"))?;

            let rowid = tx.last_insert_rowid();

            // 端到端统一预分词写入 FTS5
            insert_fts
                .execute(params![
                    rowid,
                    id,
                    tokenize_for_index(name),
                    tokenize_for_index(&format!("{title_zh} {category}")),
                    tokenize_for_index(&format!("{tags_str} {stage}")),
                    tokenize_for_index(&format!("{summary} {trigger}")),
                    tokenize_for_index(&format!("{desc} {how_to_use}")),
                ])
                .map_err(|e| format!("插入 FTS 失败 ({id}): {e}"))?;
        }
    }

    tx.execute(
        "INSERT INTO skill_registry_snapshot (id, version, updated_at) VALUES (1, ?1, ?2)
         ON CONFLICT(id) DO UPDATE SET version=excluded.version, updated_at=excluded.updated_at",
        params![version, updated_at],
    )
    .map_err(|e| e.to_string())?;

    tx.commit().map_err(|e| format!("提交事务失败: {e}"))
}

#[derive(Debug, Clone, Default)]
pub struct SearchFilterOptions {
    pub stage: Option<String>,
    pub category: Option<String>,
    pub allowed_ids: Option<HashSet<String>>, // 用于“只看收藏”或“自定义标签”过滤（在截断前生效！）
    pub project_tech_stack: Vec<String>,      // 当前打开项目的技术栈上下文（如 ["vue", "tauri"]）
    pub limit: usize,
}

#[derive(Debug, Clone)]
pub struct SearchHitResult {
    pub id: String,
    pub name: String,
    pub title_zh: String,
    pub core_matched: usize,
    pub core_total: usize,
    pub non_tech_core_matched: usize,
    pub final_score: f64,
    pub matched_slots: Vec<String>,
}

/// 检查某个词（英文要求独立词边界，中文要求连续子串）是否出现在文本中
fn term_matches_text_with_boundary(term: &str, text_lower: &str, text_ascii_tokens: &HashSet<&str>) -> bool {
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
                // 短词（如 ui, api, db, qa）必须精确等于某个独立 ASCII token，绝不允许子串匹配 build/guide
                text_ascii_tokens.contains(w)
            } else {
                text_ascii_tokens.iter().any(|tok| *tok == w || tok.starts_with(w))
            }
        } else {
            text_lower.contains(term)
        }
    }
}

/// 计算单个意图槽在某条技能上的最佳匹配强度（Slot-Level Max Pooling：组内取最高分，绝不累加膨胀）
fn evaluate_slot_match_strength(
    slot: &QueryIntentSlot,
    name_lower: &str,
    title_lower: &str,
    tags_lower: &str,
    body_lower: &str,
    name_tokens: &HashSet<&str>,
    title_tokens: &HashSet<&str>,
    tags_tokens: &HashSet<&str>,
    body_tokens: &HashSet<&str>,
) -> f64 {
    let mut best: f64 = 0.0;
    for (idx, syn) in slot.synonyms.iter().enumerate() {
        let is_raw = idx == 0;
        let syn_factor = if is_raw { 1.0 } else { 0.90 };

        if name_lower == syn {
            best = best.max(100.0 * syn_factor);
        } else if term_matches_text_with_boundary(syn, name_lower, name_tokens) {
            best = best.max(85.0 * syn_factor);
        } else if term_matches_text_with_boundary(syn, title_lower, title_tokens) {
            best = best.max(80.0 * syn_factor);
        } else if term_matches_text_with_boundary(syn, tags_lower, tags_tokens) {
            best = best.max(70.0 * syn_factor);
        } else if term_matches_text_with_boundary(syn, body_lower, body_tokens) {
            best = best.max(50.0 * syn_factor);
        }
    }
    best
}

/// 核心检索引擎：
/// 1. 解析中英混合/自然语言查询为意图槽；
/// 2. 使用 FTS5 召回（在 SQL 阶段结合 stage/category，并在 Top-K 截断前应用 allowed_ids 过滤）；
/// 3. 以“核心意图槽覆盖数 (Core Coverage) + 任务意图命中 (Non-Tech Task Hit)”为第一排序主导，
///    结合查询内相对 BM25、字段最佳匹配强度、项目技术栈适配与同仓库软衰减重排。
pub fn search_skills_poc(conn: &Connection, query: &str, opts: &SearchFilterOptions) -> Result<Vec<SearchHitResult>, String> {
    let limit = if opts.limit == 0 { 50 } else { opts.limit };
    let slots = parse_search_query(query);
    if slots.is_empty() {
        return Ok(Vec::new());
    }

    // 构造 FTS5 UNION (OR) 召回表达式，确保多槽查询下能召回候选并在重排层严格按覆盖度分层
    let slot_exprs: Vec<String> = slots.iter().filter_map(slot_to_fts_group_expr).collect();
    if slot_exprs.is_empty() {
        return Ok(Vec::new());
    }
    let fts_match_expr = slot_exprs.join(" OR ");

    // 注意：stage 和 category 直接在 SQL WHERE 中过滤，不设死 Top 200 硬截断，防止过滤后丢失结果
    let mut stmt = conn
        .prepare(
            "SELECT e.id, e.name, e.title_zh, e.description, e.category, e.tags_text, e.guide_text, e.repo_key, e.repo_stars, e.spec_score,
                    bm25(skill_registry_fts, 10.0, 8.0, 6.0, 4.0, 1.0) AS raw_bm25
             FROM skill_registry_fts
             JOIN skill_registry_entries e ON e.rowid = skill_registry_fts.rowid
             WHERE skill_registry_fts MATCH ?1
               AND (?2 IS NULL OR e.stage = ?2)
               AND (?3 IS NULL OR e.category = ?3)
             ORDER BY raw_bm25 ASC",
        )
        .map_err(|e| format!("查询 FTS5 失败 ({fts_match_expr}): {e}"))?;

    struct RawCandidate {
        id: String,
        name: String,
        title_zh: String,
        desc: String,
        category: String,
        tags_text: String,
        guide_text: String,
        repo_key: String,
        repo_stars: i64,
        spec_score: i64,
        positive_bm25: f64,
    }

    let rows = stmt
        .query_map(
            params![fts_match_expr, opts.stage.as_deref(), opts.category.as_deref()],
            |row| {
                let raw_bm25: f64 = row.get(10)?;
                Ok(RawCandidate {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    title_zh: row.get(2)?,
                    desc: row.get(3)?,
                    category: row.get(4)?,
                    tags_text: row.get(5)?,
                    guide_text: row.get(6)?,
                    repo_key: row.get(7)?,
                    repo_stars: row.get(8)?,
                    spec_score: row.get(9)?,
                    positive_bm25: (-raw_bm25).max(0.0),
                })
            },
        )
        .map_err(|e| e.to_string())?;

    let mut candidates = Vec::new();
    for r in rows {
        let cand = r.map_err(|e| e.to_string())?;
        // 关键：在任何截断之前，先应用收藏/自定义标签 ID 集合过滤
        if let Some(ref allowed) = opts.allowed_ids {
            if !allowed.contains(&cand.id) {
                continue;
            }
        }
        candidates.push(cand);
        // 在完成 stage / category / allowed_ids 全部过滤后，取 BM25 前 400 名进入精细重排
        if candidates.len() >= 400 {
            break;
        }
    }

    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    // 查询内相对 BM25 极值（解决小样本 BM25 为 -0.000001 的缩放问题）
    let max_bm25 = candidates
        .iter()
        .map(|c| c.positive_bm25)
        .fold(0.0_f64, f64::max);

    let core_total = slots.iter().filter(|s| s.is_core).count().max(1);
    let has_tech_core = slots.iter().any(|s| s.is_core && s.is_tech_stack);
    let has_non_tech_core = slots.iter().any(|s| s.is_core && !s.is_tech_stack);

    let mut scored_hits = Vec::new();

    for cand in &candidates {
        let name_lower = cand.name.to_lowercase();
        let title_lower = format!("{} {}", cand.title_zh, cand.category).to_lowercase();
        let tags_lower = cand.tags_text.to_lowercase();
        let body_lower = format!("{} {}", cand.desc, cand.guide_text).to_lowercase();

        fn split_ascii(s: &str) -> HashSet<&str> {
            s.split(|c: char| !c.is_ascii_alphanumeric())
                .filter(|w| !w.is_empty())
                .collect()
        }
        let name_tokens = split_ascii(&name_lower);
        let title_tokens = split_ascii(&title_lower);
        let tags_tokens = split_ascii(&tags_lower);
        let body_tokens = split_ascii(&body_lower);

        let mut core_matched = 0usize;
        let mut tech_core_matched = 0usize;
        let mut non_tech_core_matched = 0usize;
        let mut slot_strength_sum = 0.0;
        let mut matched_slot_names = Vec::new();

        for slot in &slots {
            let strength = evaluate_slot_match_strength(
                slot,
                &name_lower,
                &title_lower,
                &tags_lower,
                &body_lower,
                &name_tokens,
                &title_tokens,
                &tags_tokens,
                &body_tokens,
            );
            if strength > 0.0 {
                matched_slot_names.push(slot.raw_term.clone());
                if slot.is_core {
                    core_matched += 1;
                    if slot.is_tech_stack {
                        tech_core_matched += 1;
                    } else {
                        non_tech_core_matched += 1;
                    }
                    let weight = if !slot.is_tech_stack { 1.3 } else { 1.0 };
                    slot_strength_sum += strength * weight;
                } else {
                    slot_strength_sum += strength * 0.35;
                }
            }
        }

        // 若经过词边界严格校验后没有任何槽真正命中，则剔除
        if matched_slot_names.is_empty() {
            continue;
        }

        // 双支柱对齐层级 (Dual-Pillar Alignment):
        // 当查询同时包含“技术栈约束 (如 Rust/Angular)”与“任务约束 (如 内存排查/国际化)”时：
        // - Tier 2.0: 既命中技术栈，又命中任务目标（双支柱全满，最高优先）
        // - Tier 1.0: 命中任务目标，未显式声明该技术栈（通用任务工具）
        // - Tier 0.0: 仅命中技术栈名称，但与任务目标完全无关（最低优先）
        let pillar_tier = if has_tech_core && has_non_tech_core {
            if tech_core_matched > 0 && non_tech_core_matched > 0 {
                2.0
            } else if non_tech_core_matched > 0 {
                1.0
            } else {
                0.0
            }
        } else {
            2.0
        };

        // 查询内相对 BM25 得分 (0..=100)
        let rel_bm25 = if max_bm25 > 0.0 {
            100.0 * (cand.positive_bm25 / max_bm25)
        } else {
            100.0
        };

        let avg_slot_strength = slot_strength_sum / (slots.len() as f64);

        // 项目技术栈上下文加成 (0..=100)
        let project_stack_score = if opts.project_tech_stack.is_empty() {
            0.0
        } else {
            let matched_stack = opts
                .project_tech_stack
                .iter()
                .filter(|t| {
                    let tl = t.to_lowercase();
                    name_tokens.contains(tl.as_str())
                        || tags_tokens.contains(tl.as_str())
                        || body_tokens.contains(tl.as_str())
                })
                .count();
            (matched_stack as f64 * 50.0).min(100.0)
        };

        let quality_score = (cand.spec_score as f64).clamp(0.0, 100.0);
        let repo_log_score = ((cand.repo_stars.max(0) as f64 + 1.0).log10() * 20.0).min(100.0);

        // 同层级内部综合分（范围 0..100）
        let intra_tier_score = 0.50 * avg_slot_strength
            + 0.25 * rel_bm25
            + 0.10 * project_stack_score
            + 0.10 * quality_score
            + 0.05 * repo_log_score;

        let tier_floor = pillar_tier * 100_000.0 + (core_matched as f64) * 10_000.0;
        let base_score = tier_floor + intra_tier_score;

        scored_hits.push((cand.repo_key.clone(), tier_floor, SearchHitResult {
            id: cand.id.clone(),
            name: cand.name.clone(),
            title_zh: cand.title_zh.clone(),
            core_matched,
            core_total,
            non_tech_core_matched,
            final_score: base_score,
            matched_slots: matched_slot_names,
        }));
    }

    // 先按基础分降序排列，再施加同仓库软性多样性衰减（不改变跨层级的顺序，仅在同层级内抑制同仓库刷屏）
    scored_hits.sort_by(|a, b| b.2.final_score.partial_cmp(&a.2.final_score).unwrap());

    let mut repo_counts: HashMap<String, usize> = HashMap::new();
    let mut final_results: Vec<SearchHitResult> = scored_hits
        .into_iter()
        .map(|(repo_key, tier_floor, mut hit)| {
            if !repo_key.is_empty() {
                let count = repo_counts.entry(repo_key).or_insert(0);
                *count += 1;
                let remainder = hit.final_score - tier_floor;
                let decay = match *count {
                    1 | 2 => 1.0,
                    3 => 0.85,
                    4 => 0.70,
                    _ => 0.55,
                };
                hit.final_score = tier_floor + remainder * decay;
            }
            hit
        })
        .collect();

    final_results.sort_by(|a, b| b.final_score.partial_cmp(&a.final_score).unwrap());
    final_results.truncate(limit);
    Ok(final_results)
}

// ============================================================================
// 二（续）、v3.2 字段权威度 (Field Authority) + 同义词权重分层 (Synonym Tiering)
//           解决“能搜到但 Top-1 不准”与“长 README 万金油技能误排第一”问题
// ============================================================================

/// 同义词权重分层（Synonym Tiering）：
/// 区分“原词 (1.0)”、“高置信度领域同义词 (0.95)”、“中等相关词 (0.70)”与“易跨领域误伤的多义扩展词 (0.20~0.35)”
fn synonym_tier_weight(raw_term: &str, syn: &str, idx: usize, full_text_lower: &str) -> f64 {
    if idx == 0 || raw_term == syn {
        return 1.0;
    }

    // 1. 针对“国际化/多语言/本地化”：英文动词 "translate" 在 UI 转换工具（如 Translate Figma to Angular）中极易误伤
    if matches!(raw_term, "国际化" | "多语言" | "本地化" | "i18n") && syn == "translate" {
        let has_i18n_context = full_text_lower.contains("i18n")
            || full_text_lower.contains("l10n")
            || full_text_lower.contains("locale")
            || full_text_lower.contains("localization");
        return if has_i18n_context { 0.92 } else { 0.18 };
    }

    // 2. 针对“代码审查/代码评审”：泛化词 "lint" / "audit" 以及仅表示对象的 "pr" / "mr" 不能単独冒充“审查”动作（防止 wiki-lint 和 pr-merge 夺冠）
    if matches!(raw_term, "代码审查" | "代码评审") && matches!(syn, "lint" | "audit" | "pr" | "mr") {
        return 0.15;
    }

    // 3. 针对“单元测试/单测/unit”：泛化词 "unit" / "test" / "testing" / "spec" 若无 unit-test/rtl/vitest/jest/tdd 支撑，
    //    降低同义词层级（防止 gsap-react 因正文提到 CSS unit 或 testing 夺冠）
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

    // 4. 当技能明确包含 React + RTL (React Testing Library) 时，rtl 是 react / testing 的高置信度专有同义词
    if matches!(raw_term, "react" | "testing" | "单元测试" | "单测" | "unit")
        && syn == "rtl"
        && full_text_lower.contains("react")
    {
        return 0.96;
    }

    // 5. 高置信度领域专有同义词
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
    ) {
        return 0.95;
    }

    // 默认同义词系数
    0.72
}

/// 计算带字段权威度（Field Authority）与长文稀释惩罚的单个意图槽得分：
/// - Skill Name 精确匹配: 10.0
/// - Skill Name 词元匹配: 8.5（若名称以该词开头或名称极简 <=3 词元，追加专一度加成 +0.8）
/// - Title / Category / Tags 匹配: 7.5
/// - Guide / Summary 匹配: 4.5
/// - Description 正文匹配: 2.4（若命中长 README 万金油文档则施加长度与技术栈堆砌稀释）
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
) -> f64 {
    let mut best: f64 = 0.0;
    let name_tok_count = name_tokens.len().max(1);

    for (idx, syn) in slot.synonyms.iter().enumerate() {
        let syn_w = synonym_tier_weight(&slot.raw_term, syn, idx, full_text_lower);

        if name_lower == syn {
            best = best.max(10.0 * syn_w);
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
            best = best.max((8.5 + specificity_bonus) * syn_w);
        } else if term_matches_text_with_boundary(syn, title_lower, title_tokens)
            || term_matches_text_with_boundary(syn, tags_lower, tags_tokens)
        {
            best = best.max(7.5 * syn_w);
        } else if term_matches_text_with_boundary(syn, guide_lower, guide_tokens) {
            best = best.max(4.5 * syn_w * body_dilution);
        } else if term_matches_text_with_boundary(syn, desc_lower, desc_tokens) {
            best = best.max(2.4 * syn_w * body_dilution);
        }
    }
    best
}

/// v3.2 检索引擎（Field Authority + Synonym Tiering + Authority-Gated Slot Coverage）：
/// 专门解决第四轮评审指出的“能搜到但 Top-1 不准”与“长 README 关键词堆砌误排第一”问题。
pub fn search_skills_with_field_authority(
    conn: &Connection,
    query: &str,
    opts: &SearchFilterOptions,
) -> Result<Vec<SearchHitResult>, String> {
    let limit = if opts.limit == 0 { 50 } else { opts.limit };
    let slots = parse_search_query(query);
    if slots.is_empty() {
        return Ok(Vec::new());
    }

    let slot_exprs: Vec<String> = slots.iter().filter_map(slot_to_fts_group_expr).collect();
    if slot_exprs.is_empty() {
        return Ok(Vec::new());
    }
    let fts_match_expr = slot_exprs.join(" OR ");

    let mut stmt = conn
        .prepare(
            "SELECT e.id, e.name, e.title_zh, e.description, e.category, e.tags_text, e.guide_text, e.repo_key, e.repo_stars, e.spec_score,
                    bm25(skill_registry_fts, 10.0, 8.0, 6.0, 4.0, 1.0) AS raw_bm25
             FROM skill_registry_fts
             JOIN skill_registry_entries e ON e.rowid = skill_registry_fts.rowid
             WHERE skill_registry_fts MATCH ?1
               AND (?2 IS NULL OR e.stage = ?2)
               AND (?3 IS NULL OR e.category = ?3)
             ORDER BY raw_bm25 ASC",
        )
        .map_err(|e| format!("查询 FTS5 失败 ({fts_match_expr}): {e}"))?;

    struct RawCandidate {
        id: String,
        name: String,
        title_zh: String,
        desc: String,
        category: String,
        tags_text: String,
        guide_text: String,
        repo_key: String,
        repo_stars: i64,
        spec_score: i64,
        positive_bm25: f64,
    }

    let rows = stmt
        .query_map(
            params![fts_match_expr, opts.stage.as_deref(), opts.category.as_deref()],
            |row| {
                let raw_bm25: f64 = row.get(10)?;
                Ok(RawCandidate {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    title_zh: row.get(2)?,
                    desc: row.get(3)?,
                    category: row.get(4)?,
                    tags_text: row.get(5)?,
                    guide_text: row.get(6)?,
                    repo_key: row.get(7)?,
                    repo_stars: row.get(8)?,
                    spec_score: row.get(9)?,
                    positive_bm25: (-raw_bm25).max(0.0),
                })
            },
        )
        .map_err(|e| e.to_string())?;

    let mut candidates = Vec::new();
    for r in rows {
        let cand = r.map_err(|e| e.to_string())?;
        if let Some(ref allowed) = opts.allowed_ids {
            if !allowed.contains(&cand.id) {
                continue;
            }
        }
        candidates.push(cand);
        if candidates.len() >= 400 {
            break;
        }
    }

    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    let max_bm25 = candidates
        .iter()
        .map(|c| c.positive_bm25)
        .fold(0.0_f64, f64::max);

    let core_total = slots.iter().filter(|s| s.is_core).count().max(1);
    let has_tech_core = slots.iter().any(|s| s.is_core && s.is_tech_stack);
    let has_non_tech_core = slots.iter().any(|s| s.is_core && !s.is_tech_stack);

    // 强意图槽权威度门槛：防止仅在正文命中低置信度泛化词（如 2.4 * 0.20 = 0.48）伪装成完整命中核心槽
    const STRONG_SLOT_AUTH_GATE: f64 = 1.65;

    let mut scored_hits = Vec::new();

    for cand in &candidates {
        let name_lower = cand.name.to_lowercase();
        let title_lower = format!("{} {}", cand.title_zh, cand.category).to_lowercase();
        let tags_lower = cand.tags_text.to_lowercase();
        let guide_lower = cand.guide_text.to_lowercase();
        let desc_lower = cand.desc.to_lowercase();
        let full_text_lower = format!("{name_lower} {title_lower} {tags_lower} {guide_lower} {desc_lower}");

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

        // 检测是否为“万金油长 README”（正文中堆砌了 >= 4 种不同技术栈且正文词数较多）
        let distinct_tech_in_desc = KNOWN_TECH_STACKS
            .iter()
            .filter(|&&t| desc_tokens.contains(t))
            .count();
        let body_word_count = desc_tokens.len() + guide_tokens.len();
        let body_dilution = if distinct_tech_in_desc >= 4 && body_word_count >= 35 {
            0.35 // 强力抑制万金油长 README 的正文泛匹配
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
        let mut weighted_auth_sum = 0.0;
        let mut matched_slot_names = Vec::new();

        for (slot_idx, slot) in slots.iter().enumerate() {
            let auth = evaluate_slot_field_authority(
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
                // 弱正文匹配不计入 core_matched 层级，仅作微弱加分
                weighted_auth_sum += auth * 1.5;
            }
        }

        // 负向查询与噪声拦截门禁：如果连 1 个达到权威度门槛的意图槽都没有命中，直接剔除（防负例误召回）
        if matched_slot_names.is_empty() || (core_matched == 0 && weighted_auth_sum < 8.0) {
            continue;
        }

        // 双支柱对齐层级 (Dual-Pillar Alignment):
        let pillar_tier = if has_tech_core && has_non_tech_core {
            if tech_core_matched > 0 && non_tech_core_matched > 0 {
                if high_auth_core_hits > 0 {
                    2.0
                } else {
                    1.5 // 仅在正文偶然同时出现技术词与任务词的候选低于名称/标签命中的候选
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

        let rel_bm25 = if max_bm25 > 0.0 {
            100.0 * (cand.positive_bm25 / max_bm25)
        } else {
            100.0
        };

        let avg_slot_auth_score = weighted_auth_sum / (slots.len() as f64);

        let project_stack_score = if opts.project_tech_stack.is_empty() {
            0.0
        } else {
            let matched_stack = opts
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

        // 高权威字段（Name/Title/Tags）命中核心词的专一度奖励 + 首词匹配奖励 + 最佳实践通用技能加成
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

        scored_hits.push((cand.repo_key.clone(), tier_floor, SearchHitResult {
            id: cand.id.clone(),
            name: cand.name.clone(),
            title_zh: cand.title_zh.clone(),
            core_matched,
            core_total,
            non_tech_core_matched,
            final_score: base_score,
            matched_slots: matched_slot_names,
        }));
    }

    scored_hits.sort_by(|a, b| b.2.final_score.partial_cmp(&a.2.final_score).unwrap());

    let mut repo_counts: HashMap<String, usize> = HashMap::new();
    let mut final_results: Vec<SearchHitResult> = scored_hits
        .into_iter()
        .map(|(repo_key, tier_floor, mut hit)| {
            if !repo_key.is_empty() {
                let count = repo_counts.entry(repo_key).or_insert(0);
                *count += 1;
                let remainder = hit.final_score - tier_floor;
                let decay = match *count {
                    1 | 2 => 1.0,
                    3 => 0.85,
                    4 => 0.70,
                    _ => 0.55,
                };
                hit.final_score = tier_floor + remainder * decay;
            }
            hit
        })
        .collect();

    final_results.sort_by(|a, b| b.final_score.partial_cmp(&a.final_score).unwrap());
    final_results.truncate(limit);
    Ok(final_results)
}

// ============================================================================
// 三、安装器双身份分离、异源同名冲突拦截、带并发锁与所有权的崩溃恢复
// ============================================================================

const MANAGED_SOURCE_FILE: &str = ".lumina-skill-source.json";
const OP_LOCK_FILE: &str = ".lumina-op-lock.json";

static SKILL_FS_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

fn skill_fs_lock() -> &'static Mutex<()> {
    SKILL_FS_MUTEX.get_or_init(|| Mutex::new(()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedSkillSourceV3 {
    pub catalog_id: String,
    pub canonical_name: String,
    pub git_url: String,
    pub sub_path: String,
    pub commit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SkillOpLock {
    pid: u32,
    canonical_name: String,
    started_at_ms: u64,
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// 严格校验 Agent Skills 官方规范的 name 字段
pub fn validate_canonical_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 64 {
        return Err(format!("无效的技能名称 '{name}'：长度必须在 1..=64 之间"));
    }
    if name.starts_with('-') || name.ends_with('-') || name.contains("--") {
        return Err(format!("无效的技能名称 '{name}'：禁止首尾连字符或连续连字符"));
    }
    if !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        return Err(format!("无效的技能名称 '{name}'：仅允许小写字母、数字和单连字符"));
    }
    Ok(())
}

/// 验证已安装的目标技能目录是否完整有效（必须同时存在合法 SKILL.md 与 .lumina-skill-source.json）
pub fn is_installed_skill_dir_healthy(dir: &Path) -> bool {
    if !dir.is_dir() {
        return false;
    }
    let skill_md = dir.join("SKILL.md");
    let source_file = dir.join(MANAGED_SOURCE_FILE);
    if !skill_md.is_file() || !source_file.is_file() {
        return false;
    }
    let Ok(source_txt) = fs::read_to_string(&source_file) else {
        return false;
    };
    serde_json::from_str::<ManagedSkillSourceV3>(&source_txt).is_ok()
}

/// 跨进程原子创建操作锁（使用 `create_new(true)` 保证多进程/多实例互斥）
fn acquire_exclusive_skill_lock(skills_root: &Path, canonical_name: &str) -> Result<PathBuf, String> {
    let lock_path = skills_root.join(format!("{OP_LOCK_FILE}.{canonical_name}"));
    let lock_data = SkillOpLock {
        pid: std::process::id(),
        canonical_name: canonical_name.to_string(),
        started_at_ms: now_millis(),
    };
    let bytes = serde_json::to_vec(&lock_data).map_err(|e| e.to_string())?;

    match fs::OpenOptions::new().write(true).create_new(true).open(&lock_path) {
        Ok(mut file) => {
            use std::io::Write;
            file.write_all(&bytes).map_err(|e| {
                let _ = fs::remove_file(&lock_path);
                format!("写入操作锁失败: {e}")
            })?;
            let _ = file.sync_all();
            Ok(lock_path)
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Err(format!(
            "并发锁冲突：技能 '{canonical_name}' 存在未释放的原子锁文件，拒绝并发写入"
        )),
        Err(e) => Err(format!("创建原子操作锁文件失败: {e}")),
    }
}

/// 并发安全且完全幂等的崩溃恢复（覆盖全部 4 类 Crash Cases）：
/// - Case 1: 创建 lock 后立即崩溃（无 stage，无 backup，target 完好）
/// - Case 2: copy 一半文件到 stage 后崩溃（stage 不完整，无 backup，target 完好）
/// - Case 3: rename(target, backup) 后、rename(stage, target) 前崩溃（target 缺失，backup 完好）
/// - Case 4: rename(stage, target) 完成、但删除 backup 与 lock 前崩溃（target 完好，backup 与 lock 残留）
pub fn recover_interrupted_skill_ops_safe(skills_root: &Path, simulate_dead_pid: bool) {
    let _guard = skill_fs_lock().lock().unwrap_or_else(|e| e.into_inner());
    let Ok(entries) = fs::read_dir(skills_root) else {
        return;
    };

    let current_pid = std::process::id();
    let lock_prefix = format!("{OP_LOCK_FILE}.");

    // 1. 扫描所有涉及未完成事务的 canonical_name（无论停留在 lock、stage 还是 backup 阶段）
    let mut affected_names = std::collections::BTreeSet::new();
    for entry in entries.flatten() {
        let fname = entry.file_name();
        let name_str = fname.to_string_lossy();
        if let Some(n) = name_str.strip_prefix(".lumina-backup-") {
            if !n.is_empty() {
                affected_names.insert(n.to_string());
            }
        } else if let Some(n) = name_str.strip_prefix(".lumina-stage-") {
            if !n.is_empty() {
                affected_names.insert(n.to_string());
            }
        } else if let Some(n) = name_str.strip_prefix(&lock_prefix) {
            if !n.is_empty() {
                affected_names.insert(n.to_string());
            }
        }
    }

    // 2. 逐个技能执行原子状态判定与恢复
    for canonical_name in affected_names {
        let lock_path = skills_root.join(format!("{OP_LOCK_FILE}.{canonical_name}"));
        if lock_path.is_file() && !simulate_dead_pid {
            if let Ok(txt) = fs::read_to_string(&lock_path) {
                if let Ok(lock_info) = serde_json::from_str::<SkillOpLock>(&txt) {
                    // 若锁属于当前存活进程且未超时，说明正在并发安装中，严禁触碰！
                    if lock_info.pid == current_pid
                        && now_millis().saturating_sub(lock_info.started_at_ms) < 300_000
                    {
                        continue;
                    }
                }
            }
        }

        let backup_dir = skills_root.join(format!(".lumina-backup-{canonical_name}"));
        let stage_dir = skills_root.join(format!(".lumina-stage-{canonical_name}"));
        let target_dir = skills_root.join(&canonical_name);

        if backup_dir.exists() {
            // 存在备份（Crash Case 3 或 Case 4）
            if !is_installed_skill_dir_healthy(&target_dir) {
                // Case 3（或 target 残缺）：清理残缺 target，将 backup 完整恢复回 target
                if target_dir.exists() {
                    let _ = fs::remove_dir_all(&target_dir);
                }
                let _ = fs::rename(&backup_dir, &target_dir);
            } else {
                // Case 4：新版本 target 已经完整且健康，安全删除旧 backup
                let _ = fs::remove_dir_all(&backup_dir);
            }
        } else if target_dir.exists() && !is_installed_skill_dir_healthy(&target_dir) {
            // 首次安装（无 backup）但在写入 target 时残缺：清理半安装目录，绝不留残骸
            let _ = fs::remove_dir_all(&target_dir);
        }

        // 清理残余的半拷贝 stage 目录（Case 2 & Case 3）与陈旧 lock 文件（Case 1~4）
        if stage_dir.exists() {
            let _ = fs::remove_dir_all(&stage_dir);
        }
        if lock_path.exists() {
            let _ = fs::remove_file(&lock_path);
        }
    }
}

/// 带双身份校验、异源同名冲突拦截与 `create_new(true)` 排他锁保护的安装/更新函数
pub fn install_skill_transactional_poc(
    skills_root: &Path,
    catalog_id: &str,
    git_url: &str,
    sub_path: &str,
    commit: &str,
    skill_md_content: &str,
) -> Result<PathBuf, String> {
    let _guard = skill_fs_lock().lock().unwrap_or_else(|e| e.into_inner());
    fs::create_dir_all(skills_root).map_err(|e| e.to_string())?;

    // 1. 从 SKILL.md 解析 name 并严格校验规范（缺失或不合规直接拒绝安装）
    let canonical_name = skill_md_content
        .lines()
        .find_map(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix("name:")
                .map(|rest| rest.trim().trim_matches('"').trim_matches('\'').to_string())
        })
        .ok_or_else(|| "SKILL.md 缺少必需的 name 字段，拒绝不合规安装".to_string())?;

    validate_canonical_name(&canonical_name)?;

    let target_dir = skills_root.join(&canonical_name);
    let norm_url = git_url.trim().trim_end_matches(".git").trim_end_matches('/').to_lowercase();
    let norm_sub = sub_path.trim().trim_matches('/').to_lowercase();

    // 2. 同名目录冲突检测（区分同源更新 vs 异源冲突）
    if target_dir.exists() {
        let source_path = target_dir.join(MANAGED_SOURCE_FILE);
        if source_path.is_file() {
            let txt = fs::read_to_string(&source_path).map_err(|e| e.to_string())?;
            let existing: ManagedSkillSourceV3 = serde_json::from_str(&txt).map_err(|e| e.to_string())?;
            let ex_url = existing.git_url.trim().trim_end_matches(".git").trim_end_matches('/').to_lowercase();
            let ex_sub = existing.sub_path.trim().trim_matches('/').to_lowercase();
            if ex_url != norm_url || ex_sub != norm_sub {
                return Err(format!(
                    "同名技能来源冲突：目录 '{}' 已由来源 [{}:{}] 占用，拒绝被异源 [{}:{}] 覆盖",
                    canonical_name, existing.git_url, existing.sub_path, git_url, sub_path
                ));
            }
        } else {
            return Err(format!(
                "同名技能冲突：目录 '{}' 已存在且属于非托管外部技能，拒绝覆盖",
                canonical_name
            ));
        }
    }

    // 3. 使用 create_new(true) 原子获取排他操作锁
    let lock_path = acquire_exclusive_skill_lock(skills_root, &canonical_name)?;

    let stage_dir = skills_root.join(format!(".lumina-stage-{canonical_name}"));
    if stage_dir.exists() {
        let _ = fs::remove_dir_all(&stage_dir);
    }
    if let Err(e) = fs::create_dir_all(&stage_dir)
        .and_then(|_| fs::write(stage_dir.join("SKILL.md"), skill_md_content))
    {
        let _ = fs::remove_dir_all(&stage_dir);
        let _ = fs::remove_file(&lock_path);
        return Err(format!("写入 stage SKILL.md 失败: {e}"));
    }

    let source_meta = ManagedSkillSourceV3 {
        catalog_id: catalog_id.to_string(),
        canonical_name: canonical_name.clone(),
        git_url: git_url.to_string(),
        sub_path: sub_path.to_string(),
        commit: commit.to_string(),
    };
    if let Err(e) = fs::write(
        stage_dir.join(MANAGED_SOURCE_FILE),
        serde_json::to_vec_pretty(&source_meta).unwrap(),
    ) {
        let _ = fs::remove_dir_all(&stage_dir);
        let _ = fs::remove_file(&lock_path);
        return Err(format!("写入 stage 元数据失败: {e}"));
    }

    // 4. 备份旧目录并原子替换
    let backup_dir = skills_root.join(format!(".lumina-backup-{canonical_name}"));
    if backup_dir.exists() {
        let _ = fs::remove_dir_all(&backup_dir);
    }

    if target_dir.exists() {
        fs::rename(&target_dir, &backup_dir).map_err(|e| {
            let _ = fs::remove_dir_all(&stage_dir);
            let _ = fs::remove_file(&lock_path);
            format!("备份旧技能失败: {e}")
        })?;
    }

    if let Err(e) = fs::rename(&stage_dir, &target_dir) {
        if backup_dir.exists() {
            let _ = fs::rename(&backup_dir, &target_dir);
        }
        let _ = fs::remove_dir_all(&stage_dir);
        let _ = fs::remove_file(&lock_path);
        return Err(format!("替换新技能失败，已自动回滚旧版本: {e}"));
    }

    if backup_dir.exists() {
        let _ = fs::remove_dir_all(&backup_dir);
    }
    let _ = fs::remove_file(&lock_path);

    Ok(target_dir)
}

// ============================================================================
// 四、旧版搜索实现复刻（用于在同一份真实数据上公平对比 Baseline vs PoC）
// ============================================================================

/// 忠实复刻 GitHub main (`3fb23f5`) 的 `filteredSkills` + `matchScore`
pub fn legacy_search_main_3fb23f5(skills: &[Value], query: &str, limit: usize) -> Vec<String> {
    let terms: Vec<String> = query
        .trim()
        .to_lowercase()
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();
    if terms.is_empty() {
        return Vec::new();
    }

    let mut matched: Vec<(&Value, i64, i64)> = skills
        .iter()
        .filter(|s| {
            let name = s.get("name").and_then(Value::as_str).unwrap_or("");
            let title_zh = s.get("titleZh").and_then(Value::as_str).unwrap_or("");
            let desc = s.get("description").and_then(Value::as_str).unwrap_or("");
            let category = s.get("category").and_then(Value::as_str).unwrap_or("");
            let author = s.pointer("/author/name").and_then(Value::as_str).unwrap_or("");
            let git_url = s.get("gitUrl").and_then(Value::as_str).unwrap_or("");
            let summary = s.pointer("/usageGuide/summary").and_then(Value::as_str).unwrap_or("");
            let trigger = s.pointer("/usageGuide/trigger").and_then(Value::as_str).unwrap_or("");
            let tags = s
                .get("tags")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" "))
                .unwrap_or_default();
            let searchable = format!(
                "{name} {title_zh} {desc} {category} {author} {git_url} {summary} {trigger} {tags}"
            )
            .to_lowercase();
            terms.iter().all(|t| searchable.contains(t))
        })
        .map(|s| {
            let name = s.get("name").and_then(Value::as_str).unwrap_or("").to_lowercase();
            let title = s.get("titleZh").and_then(Value::as_str).unwrap_or("").to_lowercase();
            let cat = s.get("category").and_then(Value::as_str).unwrap_or("").to_lowercase();
            let tags = s
                .get("tags")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(" "))
                .unwrap_or_default()
                .to_lowercase();

            let mut score = 0i64;
            for term in &terms {
                score += if name == *term {
                    120
                } else if name.starts_with(term) {
                    80
                } else if name.contains(term) {
                    60
                } else {
                    0
                };
                if title.contains(term) {
                    score += 30;
                }
                if cat.contains(term) {
                    score += 20;
                }
                if tags.contains(term) {
                    score += 15;
                }
            }
            let stars = s
                .pointer("/metrics/githubStars")
                .or_else(|| s.get("stars"))
                .and_then(Value::as_i64)
                .unwrap_or(0);
            (s, score, stars)
        })
        .collect();

    matched.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then_with(|| b.2.cmp(&a.2))
            .then_with(|| {
                let na = a.0.get("name").and_then(Value::as_str).unwrap_or("");
                let nb = b.0.get("name").and_then(Value::as_str).unwrap_or("");
                na.cmp(nb)
            })
    });

    matched
        .into_iter()
        .take(limit)
        .filter_map(|(s, _, _)| s.get("id").and_then(Value::as_str).map(String::from))
        .collect()
}

// ============================================================================
// 五、完整可执行测试集（覆盖评审要求的全部 7 大验收项）
// ============================================================================

fn build_synthetic_test_catalog() -> Vec<Value> {
    vec![
        json!({
            "id": "skill-vue-visual-reg",
            "name": "vue-visual-regression",
            "titleZh": "Vue3 组件视觉回归测试",
            "description": "Automated visual regression testing and screenshot comparison for Vue3 UI components with Playwright.",
            "stage": "testing",
            "category": "Frontend QA",
            "gitUrl": "https://github.com/example/vue-qa",
            "stars": 1200,
            "halowakeScore": 88,
            "tags": ["vue3", "playwright", "visual-regression", "组件测试"]
        }),
        json!({
            "id": "skill-vue-data-collector",
            "name": "vue3-data-collector",
            "titleZh": "Vue3 数据采集系统",
            "description": "Data telemetry and analytics pipeline for Vue3 applications.",
            "stage": "coding",
            "category": "Analytics",
            "gitUrl": "https://github.com/example/vue-telemetry",
            "stars": 95000,
            "halowakeScore": 92,
            "tags": ["vue3", "analytics", "telemetry"]
        }),
        json!({
            "id": "skill-angular-comp-helper",
            "name": "angular-component-helper",
            "titleZh": "Angular 组件生成助手",
            "description": "Scaffold Angular standalone components, directives and pipes quickly.",
            "stage": "coding",
            "category": "Frontend",
            "gitUrl": "https://github.com/mega-repo/angular-tools",
            "stars": 380000,
            "halowakeScore": 95,
            "tags": ["angular", "scaffold", "cli"]
        }),
        json!({
            "id": "skill-angular-i18n-workflow",
            "name": "localization-workflow",
            "titleZh": "多语言本地化管理流程",
            "description": "Complete i18n localization workflows, translation key extraction and locale verification for Angular and TypeScript apps.",
            "stage": "coding",
            "category": "Localization",
            "gitUrl": "https://github.com/community/i18n-kit",
            "stars": 850,
            "halowakeScore": 84,
            "tags": ["angular", "i18n", "localization", "国际化", "多语言"]
        }),
        json!({
            "id": "skill-false-ui-noise",
            "name": "backend-build-guide",
            "titleZh": "后端构建指南",
            "description": "Build a quick guide suite for requirement verification and fluid deployment.",
            "stage": "delivery",
            "category": "DevOps",
            "gitUrl": "https://github.com/example/devops",
            "stars": 50000,
            "halowakeScore": 90,
            "tags": ["build", "guide", "suite"]
        }),
        json!({
            "id": "skill-cjk-order-trap",
            "name": "global-change-report",
            "titleZh": "国际局势变化分析报告",
            "description": "Analyze international trends and changes.",
            "stage": "requirement",
            "category": "Analysis",
            "gitUrl": "https://github.com/example/reports",
            "stars": 5000,
            "halowakeScore": 80,
            "tags": ["报告"]
        }),
    ]
}

/// 验收测试 1：中英文混合查询与自然语言意图提取（解决 `Vue3组件视觉`、`修复Vue3组件的视觉问题` 丢中文 Bug）
#[test]
fn test_1_mixed_cn_en_and_natural_language_queries() {
    let mut conn = Connection::open_in_memory().unwrap();
    init_poc_schema(&conn).unwrap();
    let catalog = build_synthetic_test_catalog();
    save_registry_with_fts_atomic(&mut conn, "1.0", 100, &catalog).unwrap();

    let opts = SearchFilterOptions::default();

    // 1. 评审复现场景：`Vue3组件视觉`（无空格中英混合）
    let slots = parse_search_query("Vue3组件视觉");
    let raw_terms: Vec<&str> = slots.iter().map(|s| s.raw_term.as_str()).collect();
    assert_eq!(raw_terms, vec!["vue3", "组件", "视觉"], "必须完整保留 vue3、组件、视觉 三个意图槽");

    let hits = search_skills_poc(&conn, "Vue3组件视觉", &opts).unwrap();
    assert!(!hits.is_empty());
    assert_eq!(
        hits[0].id, "skill-vue-visual-reg",
        "Vue3组件视觉 必须把完整命中 3 个槽的 vue-visual-regression 排在第一，绝不能被高 Star 的 vue3-data-collector 压过"
    );
    assert_eq!(hits[0].core_matched, 3);

    // 2. 评审复现场景：`修复Vue3组件的视觉问题`
    let hits_nl = search_skills_poc(&conn, "修复Vue3组件的视觉问题", &opts).unwrap();
    assert_eq!(hits_nl[0].id, "skill-vue-visual-reg");
    assert_eq!(hits_nl[0].core_matched, 3, "必须命中 vue3 + 组件 + 视觉 三个核心槽");

    // 3. 评审复现场景：`帮我给 Angular 项目添加国际化支持`
    let slots_ng = parse_search_query("帮我给 Angular 项目添加国际化支持");
    let ng_terms: Vec<&str> = slots_ng.iter().map(|s| s.raw_term.as_str()).collect();
    assert_eq!(ng_terms, vec!["angular", "国际化"]);

    let hits_ng = search_skills_poc(&conn, "帮我给 Angular 项目添加国际化支持", &opts).unwrap();
    assert_eq!(
        hits_ng[0].id, "skill-angular-i18n-workflow",
        "完整命中 Angular + 国际化 的 localization-workflow (850 stars) 必须击败只命中 Angular 名称前缀的 angular-component-helper (380k stars)"
    );
}

/// 验收测试 2：验证 `cjk1<字>` 在 `fts5vocab` 中为单一完整 Token，且连续中文短语不误命中非连续文本，英文短词 `ui` 不误命中 `build/guide`
#[test]
fn test_2_fts5vocab_single_cjk_token_and_boundary_safety() {
    let mut conn = Connection::open_in_memory().unwrap();
    init_poc_schema(&conn).unwrap();
    let catalog = build_synthetic_test_catalog();
    save_registry_with_fts_atomic(&mut conn, "1.0", 100, &catalog).unwrap();

    // 检查 fts5vocab 虚拟表：确认不存在孤立的 "u" token，且 "cjk1国" 是单个完整 token
    let standalone_u_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM skill_registry_fts_vocab WHERE term = 'u'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(standalone_u_count, 0, "索引中绝不能产生孤立的 'u' 噪声 token");

    let cjk1_guo_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM skill_registry_fts_vocab WHERE term = 'cjk1国'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(cjk1_guo_count >= 1, "'cjk1国' 必须作为单一完整 token 存在于 fts5vocab 中");

    let opts = SearchFilterOptions::default();

    // 搜索“国际化”：绝不能命中“国际局势变化分析报告”（含“国际”和“化”但不连续）
    let hits_i18n = search_skills_poc(&conn, "国际化", &opts).unwrap();
    assert!(hits_i18n.iter().all(|h| h.id != "skill-cjk-order-trap"));
    assert_eq!(hits_i18n[0].id, "skill-angular-i18n-workflow");

    // 搜索短词 "ui"：绝不能命中包含 build / guide / suite / requirement / fluid 的 skill-false-ui-noise
    let hits_ui = search_skills_poc(&conn, "ui", &opts).unwrap();
    assert!(
        hits_ui.iter().all(|h| h.id != "skill-false-ui-noise"),
        "短词 'ui' 绝不能子串误匹配 build/guide/suite/requirement"
    );
    assert_eq!(hits_ui[0].id, "skill-vue-visual-reg");
}

/// 验收测试 3：验证 Top-K 截断与收藏/分类筛选无冲突（第 250 名的收藏技能在开启收藏过滤时 100% 召回）
#[test]
fn test_3_filter_before_truncation_preserves_rank_250_starred_skill() {
    let mut conn = Connection::open_in_memory().unwrap();
    init_poc_schema(&conn).unwrap();

    let mut large_catalog = Vec::new();
    // 构造 260 条包含 angular 的高分技能
    for i in 1..=260 {
        large_catalog.push(json!({
            "id": format!("skill-angular-generic-{i}"),
            "name": format!("angular-helper-{i}"),
            "titleZh": format!("Angular 通用工具 {i}"),
            "description": "Angular utility helper for general coding tasks.",
            "stage": "coding",
            "category": "Frontend",
            "gitUrl": format!("https://github.com/org{i}/repo{i}"),
            "stars": 10000 - i,
            "halowakeScore": 90,
            "tags": ["angular"]
        }));
    }
    // 第 261 条是用户收藏的技能，仅在描述末尾提到 angular，全量搜 "angular" 排在 261 名
    large_catalog.push(json!({
        "id": "my-starred-skill-261",
        "name": "custom-enterprise-plugin",
        "titleZh": "企业内部插件",
        "description": "A very long description for enterprise workflow that also supports angular projects inside monorepos.",
        "stage": "testing",
        "category": "Enterprise",
        "gitUrl": "https://github.com/internal/plugin",
        "stars": 1,
        "halowakeScore": 50,
        "tags": ["internal"]
    }));

    save_registry_with_fts_atomic(&mut conn, "1.0", 100, &large_catalog).unwrap();

    // 当开启“只看收藏”且 limit = 20 时，即使 my-starred-skill-261 在全量排第 261 名，也必须被检出！
    let mut starred = HashSet::new();
    starred.insert("my-starred-skill-261".to_string());

    let opts = SearchFilterOptions {
        allowed_ids: Some(starred),
        limit: 20,
        ..Default::default()
    };
    let hits = search_skills_poc(&conn, "angular", &opts).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, "my-starred-skill-261");

    // 同样测试按 category = "Enterprise" 过滤
    let opts_cat = SearchFilterOptions {
        category: Some("Enterprise".to_string()),
        limit: 20,
        ..Default::default()
    };
    let hits_cat = search_skills_poc(&conn, "angular", &opts_cat).unwrap();
    assert_eq!(hits_cat.len(), 1);
    assert_eq!(hits_cat[0].id, "my-starred-skill-261");
}

/// 验收测试 4：验证主表与 FTS5 索引同事务原子回滚（写入中途报错时，旧版主表与旧版 FTS5 索引 100% 保持一致）
#[test]
fn test_4_atomic_rollback_keeps_main_and_fts_consistent() {
    let mut conn = Connection::open_in_memory().unwrap();
    init_poc_schema(&conn).unwrap();
    let v1_catalog = build_synthetic_test_catalog();
    save_registry_with_fts_atomic(&mut conn, "1.0.0", 1000, &v1_catalog).unwrap();

    // 构造一批包含重复主键的损坏数据（第 2 条与第 1 条 ID 重复，触发主键冲突失败）
    let broken_catalog = vec![
        json!({
            "id": "temp-new-skill",
            "name": "temp-new-skill",
            "titleZh": "临时技能",
            "description": "Temporary skill for rollback test",
            "stage": "coding"
        }),
        json!({
            "id": "temp-new-skill", // 故意重复主键引发事务中途失败
            "name": "duplicate-id-skill",
            "titleZh": "重复主键技能",
            "description": "Should fail transaction",
            "stage": "coding"
        }),
    ];

    let res = save_registry_with_fts_atomic(&mut conn, "2.0.0", 2000, &broken_catalog);
    assert!(res.is_err(), "损坏的目录更新必须报错回滚");

    // 验证主表行数与 FTS5 行数完全一致，且旧版搜索完全正常
    let main_count: i64 = conn.query_row("SELECT COUNT(*) FROM skill_registry_entries", [], |r| r.get(0)).unwrap();
    let fts_count: i64 = conn.query_row("SELECT COUNT(*) FROM skill_registry_fts", [], |r| r.get(0)).unwrap();
    assert_eq!(main_count, v1_catalog.len() as i64);
    assert_eq!(fts_count, v1_catalog.len() as i64);

    let hits = search_skills_poc(&conn, "Vue3组件视觉", &SearchFilterOptions::default()).unwrap();
    assert_eq!(hits[0].id, "skill-vue-visual-reg", "回滚后旧版 FTS 索引必须完好无损");
}

/// 验收测试 5：验证双身份模型、同名技能异源冲突拦截、同源平滑更新与并发安全的崩溃恢复
#[test]
fn test_5_installer_conflict_detection_and_concurrent_safe_recovery() {
    let root = std::env::temp_dir().join(format!("lumina-poc-install-{}", now_millis()));
    fs::create_dir_all(&root).unwrap();

    let md_v1 = "---\nname: code-review\ndescription: Code review workflow v1\n---\n# V1";
    let md_v2 = "---\nname: code-review\ndescription: Code review workflow v2\n---\n# V2";

    // 1. 首次从仓库 A 安装（Catalog ID 是长 ID，但安装目录名必须是规范的 code-review）
    let installed_path = install_skill_transactional_poc(
        &root,
        "corpus-repo-a-code-review-998877",
        "https://github.com/org-a/skills.git",
        "skills/code-review",
        "commit111",
        md_v1,
    )
    .unwrap();
    assert_eq!(installed_path.file_name().unwrap().to_str().unwrap(), "code-review");
    assert!(is_installed_skill_dir_healthy(&installed_path));

    // 2. 尝试从仓库 B 安装同名技能 `name: code-review` -> 必须触发异源冲突拦截，且原仓库 A 文件毫发无损！
    let conflict_err = install_skill_transactional_poc(
        &root,
        "corpus-repo-b-code-review-112233",
        "https://github.com/org-b/other-skills.git",
        "code-review",
        "commit222",
        md_v2,
    )
    .unwrap_err();
    assert!(conflict_err.contains("同名技能来源冲突"), "必须拦截异源同名覆盖: {conflict_err}");
    let current_md = fs::read_to_string(installed_path.join("SKILL.md")).unwrap();
    assert!(current_md.contains("# V1"), "被拦截后原技能内容不能被破坏");

    // 3. 从同一仓库 A + 同一子路径更新 v2 -> 必须允许同源更新
    install_skill_transactional_poc(
        &root,
        "corpus-repo-a-code-review-998877",
        "https://github.com/org-a/skills",
        "skills/code-review/",
        "commit333",
        md_v2,
    )
    .unwrap();
    let updated_md = fs::read_to_string(installed_path.join("SKILL.md")).unwrap();
    assert!(updated_md.contains("# V2"), "同源更新应当成功写入 V2");

    // 4. 模拟并发安全：当存在当前存活进程 PID 的锁文件时，扫描恢复绝不能把正在安装的 backup 移回！
    let backup_dir = root.join(".lumina-backup-code-review");
    fs::rename(&installed_path, &backup_dir).unwrap();
    let active_lock = SkillOpLock {
        pid: std::process::id(), // 当前存活进程
        canonical_name: "code-review".to_string(),
        started_at_ms: now_millis(),
    };
    let lock_path = root.join(format!("{OP_LOCK_FILE}.code-review"));
    fs::write(&lock_path, serde_json::to_vec(&active_lock).unwrap()).unwrap();

    // simulate_dead_pid = false（模拟扫描任务在安装任务进行中触发）
    recover_interrupted_skill_ops_safe(&root, false);
    assert!(
        backup_dir.exists() && !installed_path.exists(),
        "当前存活进程持有操作锁时，扫描恢复绝不能干预正在进行的 backup/stage！"
    );

    // 5. 模拟进程已崩溃（simulate_dead_pid = true）：此时恢复机制必须自动将 backup 还原回 code-review！
    recover_interrupted_skill_ops_safe(&root, true);
    assert!(
        is_installed_skill_dir_healthy(&installed_path) && !backup_dir.exists(),
        "历史崩溃遗留的 backup 必须被完整恢复为目标目录"
    );

    let _ = fs::remove_dir_all(&root);
}

/// 验收测试 6：读取本机真实 `lumina.db`（3,176 条真实技能），在人工标注的 30 条黄金评测集（10 大类）上
/// 独立对比 Legacy (`3fb23f5`)、Frozen PoC (`v3.1`) 与 Field Authority PoC (`v3.2`) 的
/// Precision@1 / Precision@3 / MRR@10 / NDCG@5 与冷热查询耗时分布（P50 / P95 / P99）
#[test]
fn test_6_real_lumina_db_benchmark_comparison() {
    let appdata = std::env::var("APPDATA").unwrap_or_default();
    let real_db_path = PathBuf::from(&appdata).join("com.yubei.lumina").join("lumina.db");
    if !real_db_path.is_file() {
        println!("未找到本机 lumina.db，跳过真实数据集对比");
        return;
    }

    let real_conn = Connection::open_with_flags(
        &real_db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();

    let mut stmt = real_conn
        .prepare("SELECT payload_json FROM skill_registry_entries ORDER BY rowid")
        .unwrap();
    let real_skills: Vec<Value> = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .filter_map(|txt| serde_json::from_str::<Value>(&txt).ok())
        .collect();

    println!("\n================================================================================================");
    println!("成功加载本机真实技能目录: 共 {} 条真实技能 (人工标注 30 条黄金评测集 · 10 大类别)", real_skills.len());
    println!("================================================================================================");
    assert!(real_skills.len() > 1000, "真实技能数量应当大于 1000");

    let t0 = Instant::now();
    let mut poc_conn = Connection::open_in_memory().unwrap();
    init_poc_schema(&poc_conn).unwrap();
    save_registry_with_fts_atomic(&mut poc_conn, "poc-v3.2", 1, &real_skills).unwrap();
    let index_build_ms = t0.elapsed().as_millis();
    println!("构建 {} 条技能的 FTS5 预分词索引耗时: {} ms", real_skills.len(), index_build_ms);

    let skill_name_by_id: HashMap<String, String> = real_skills
        .iter()
        .filter_map(|s| {
            let id = s.get("id")?.as_str()?.to_string();
            let name = s.get("name")?.as_str()?.to_string();
            Some((id, name))
        })
        .collect();

    // 独立于检索算法的人工标注黄金评测集（Human-Curated Gold Set，30 条查询，覆盖评审要求的全部 10 类）：
    // - relevant_names (Grade = 2): 精确命中领域 + 任务目标的理想技能
    // - acceptable_names (Grade = 1): 虽未绑定特定框架但可直接解决该核心任务的通用技能
    // - 其他所有技能 (Grade = 0): 不相关或跨领域误召回（如 igniteui-angular-figma-to-app、gsap-react、wiki-lint）
    struct GoldQuery {
        category: &'static str,
        query: &'static str,
        relevant_names: &'static [&'static str],
        acceptable_names: &'static [&'static str],
        is_negative: bool,
    }

    let gold_set: [GoldQuery; 30] = [
        // 1. 精确名称 (3)
        GoldQuery {
            category: "1.精确名称",
            query: "systematic-debugging",
            relevant_names: &["systematic-debugging"],
            acceptable_names: &["debug-like-expert", "bug-hunt", "error-detective"],
            is_negative: false,
        },
        GoldQuery {
            category: "1.精确名称",
            query: "frontend-design",
            relevant_names: &["frontend-design", "ui-ux-pro-max", "super-ui-ux-design"],
            acceptable_names: &["design-systems", "web-design-guidelines"],
            is_negative: false,
        },
        GoldQuery {
            category: "1.精确名称",
            query: "webapp-testing",
            relevant_names: &["webapp-testing", "e2e-testing", "webwright"],
            acceptable_names: &["testing", "pr-test", "openclaw-testing"],
            is_negative: false,
        },
        // 2. 纯中文意图 (4)
        GoldQuery {
            category: "2.纯中文",
            query: "国际化",
            relevant_names: &["i18n-parity", "translate", "asc-localize-metadata"],
            acceptable_names: &[],
            is_negative: false,
        },
        GoldQuery {
            category: "2.纯中文",
            query: "代码审查规范",
            relevant_names: &["code-review", "code-review-and-quality", "autoreview", "review-pr", "requesting-code-review", "receiving-code-review"],
            acceptable_names: &["understand-diff", "gh-stack", "rust-review", "typescript-style-review"],
            is_negative: false,
        },
        GoldQuery {
            category: "2.纯中文",
            query: "数据库慢查询优化",
            relevant_names: &["database-optimization", "supabase-postgres-best-practices", "tsh-sql-and-database-understanding"],
            acceptable_names: &["sql-correctness", "sql-style", "database-schema-designer"],
            is_negative: false,
        },
        GoldQuery {
            category: "2.纯中文",
            query: "登录鉴权与权限控制",
            relevant_names: &[
                "auth-and-authorization-patterns",
                "authentication-patterns",
                "secure-auth",
                "accessible-authentication",
                "login-flows",
                "add-oauth-provider",
            ],
            acceptable_names: &["security-audit"],
            is_negative: false,
        },
        // 3. 纯英文意图 (3)
        GoldQuery {
            category: "3.纯英文",
            query: "react unit testing",
            relevant_names: &["gen-rtl-test"],
            acceptable_names: &["testing", "testing-patterns", "tdd", "test-driven-development", "tdd-workflow", "nw-tdd-methodology"],
            is_negative: false,
        },
        GoldQuery {
            category: "3.纯英文",
            query: "database query optimization",
            relevant_names: &["database-optimization", "supabase-postgres-best-practices", "tsh-sql-and-database-understanding"],
            acceptable_names: &["sql-correctness", "sql-style", "database-schema-designer"],
            is_negative: false,
        },
        GoldQuery {
            category: "3.纯英文",
            query: "docker container deployment",
            relevant_names: &["docker-best-practices", "devops-automation"],
            acceptable_names: &["azure-kubernetes-service", "k8s-yaml-validator", "greptimedb-development-docker-image"],
            is_negative: false,
        },
        // 4. 中英混合 (5)
        GoldQuery {
            category: "4.中英混合",
            query: "Angular 国际化",
            // 注：真实 lumina.db 中无 Angular 专属 i18n 技能，通用 i18n 技能记为 Acceptable(1)，igniteui-angular-figma-to-app 严格记为 Irrelevant(0)
            relevant_names: &[],
            acceptable_names: &["i18n-parity", "translate", "asc-localize-metadata"],
            is_negative: false,
        },
        GoldQuery {
            category: "4.中英混合",
            query: "Vue3组件视觉",
            relevant_names: &["vue3-best-practices", "buefy", "frontend-design", "ui-ux-pro-max"],
            acceptable_names: &["vueuse-functions", "super-ui-ux-design", "design-review"],
            is_negative: false,
        },
        GoldQuery {
            category: "4.中英混合",
            query: "React 单元测试",
            // 注：gen-rtl-test 是 React Testing Library 单测生成技能 (Grade 2)；gsap-react 严格记为 Irrelevant(0)
            relevant_names: &["gen-rtl-test"],
            acceptable_names: &["testing", "testing-patterns", "tdd", "test-driven-development", "tdd-workflow", "nw-tdd-methodology"],
            is_negative: false,
        },
        GoldQuery {
            category: "4.中英混合",
            query: "Playwright 自动化测试",
            relevant_names: &["webapp-testing", "e2e-testing", "webwright", "update-int-snapshots"],
            acceptable_names: &["login-flows", "openclaw-testing", "pr-test", "testing"],
            is_negative: false,
        },
        GoldQuery {
            category: "4.中英混合",
            query: "Tailwind 界面美化",
            relevant_names: &["antd-style-to-tailwind", "loopwind", "frontend-design", "ui-ux-pro-max"],
            acceptable_names: &["senior-frontend", "react", "super-ui-ux-design"],
            is_negative: false,
        },
        // 5. 自然语言长句 (3)
        GoldQuery {
            category: "5.自然语言",
            query: "帮我给 Angular 项目添加国际化支持",
            relevant_names: &[],
            acceptable_names: &["i18n-parity", "translate", "asc-localize-metadata"],
            is_negative: false,
        },
        GoldQuery {
            category: "5.自然语言",
            query: "修复Vue3组件的视觉问题",
            relevant_names: &["vue3-best-practices", "buefy", "frontend-design", "ui-ux-pro-max"],
            acceptable_names: &["vueuse-functions", "design-review", "super-ui-ux-design"],
            is_negative: false,
        },
        GoldQuery {
            category: "5.自然语言",
            query: "给 React Hook 写单元测试",
            relevant_names: &["gen-rtl-test"],
            acceptable_names: &["testing", "testing-patterns", "tdd", "test-driven-development", "tdd-workflow"],
            is_negative: false,
        },
        // 6. 高歧义泛化词 (3)
        GoldQuery {
            category: "6.高歧义词",
            query: "UI",
            relevant_names: &[
                "frontend-design",
                "ui-ux-pro-max",
                "super-ui-ux-design",
                "design-systems",
                "web-design-guidelines",
                "ui-widget-developer",
            ],
            acceptable_names: &["buefy", "design-review", "antd-style-to-tailwind", "minimalist-ui"],
            is_negative: false,
        },
        GoldQuery {
            category: "6.高歧义词",
            query: "测试",
            relevant_names: &["testing", "testing-patterns", "tdd", "test-driven-development", "webapp-testing", "e2e-testing", "gen-rtl-test"],
            acceptable_names: &["tdd-workflow", "pr-test", "openclaw-testing"],
            is_negative: false,
        },
        GoldQuery {
            category: "6.高歧义词",
            query: "重构",
            relevant_names: &["refactor", "kaizen-2-refactor", "architecture-audit", "code-review-and-quality"],
            acceptable_names: &["antd-style-to-tailwind", "code-review", "clean-code"],
            is_negative: false,
        },
        // 7. 同义词表达 (2)
        GoldQuery {
            category: "7.同义词",
            query: "多语言本地化",
            relevant_names: &["i18n-parity", "translate", "asc-localize-metadata"],
            acceptable_names: &[],
            is_negative: false,
        },
        GoldQuery {
            category: "7.同义词",
            query: "PR 代码评审",
            relevant_names: &["review-pr", "code-review", "code-review-and-quality", "autoreview", "requesting-code-review", "receiving-code-review"],
            acceptable_names: &["understand-diff", "gh-stack"],
            is_negative: false,
        },
        // 8. 技术栈相近易混 (3)
        GoldQuery {
            category: "8.易混技术栈",
            query: "Vue 表格组件",
            relevant_names: &["buefy", "vue3-best-practices"],
            acceptable_names: &["vueuse-functions", "frontend-design"],
            is_negative: false,
        },
        GoldQuery {
            category: "8.易混技术栈",
            query: "Next.js 性能优化",
            relevant_names: &["nextjs", "performance-optimization", "perf-profiler", "perf-code-paths", "bottleneck-hunter"],
            acceptable_names: &["react", "senior-frontend"],
            is_negative: false,
        },
        GoldQuery {
            category: "8.易混技术栈",
            query: "TypeScript 代码审查",
            relevant_names: &["typescript-style-review", "code-review", "code-review-and-quality"],
            acceptable_names: &["howto-code-in-typescript", "ts-core", "autoreview", "review-pr"],
            is_negative: false,
        },
        // 9. 热门仓库噪声干扰 (2)
        GoldQuery {
            category: "9.噪声干扰",
            query: "Rust 内存排查",
            relevant_names: &["rust-review", "perf-profiler", "perf-code-paths", "bottleneck-hunter"],
            acceptable_names: &["cgp", "systematic-debugging", "architecture-audit"],
            is_negative: false,
        },
        GoldQuery {
            category: "9.噪声干扰",
            query: "Docker 容器部署",
            relevant_names: &["docker-best-practices", "devops-automation"],
            acceptable_names: &["azure-kubernetes-service", "k8s-yaml-validator", "greptimedb-development-docker-image"],
            is_negative: false,
        },
        // 10. 负例查询 (2)
        GoldQuery {
            category: "10.负例查询",
            query: "量子计算芯片光刻仿真",
            relevant_names: &[],
            acceptable_names: &[],
            is_negative: true,
        },
        GoldQuery {
            category: "10.负例查询",
            query: "自动驾驶激光雷达标定",
            relevant_names: &[],
            acceptable_names: &[],
            is_negative: true,
        },
    ];

    let grade_skill = |skill_name: &str, gq: &GoldQuery| -> u8 {
        if gq.relevant_names.contains(&skill_name) {
            2
        } else if gq.acceptable_names.contains(&skill_name) {
            1
        } else {
            0
        }
    };

    let compute_ndcg_at_5 = |grades: &[u8], gq: &GoldQuery| -> f64 {
        let mut dcg = 0.0;
        for (i, &g) in grades.iter().take(5).enumerate() {
            let gain = ((1u32 << g) - 1) as f64; // Grade 2 -> 3.0, Grade 1 -> 1.0, Grade 0 -> 0.0
            let discount = ((i + 2) as f64).log2();
            dcg += gain / discount;
        }
        let mut ideal_grades = Vec::new();
        for _ in 0..gq.relevant_names.len().min(5) {
            ideal_grades.push(2u8);
        }
        for _ in 0..gq.acceptable_names.len().min(5usize.saturating_sub(ideal_grades.len())) {
            ideal_grades.push(1u8);
        }
        let mut idcg = 0.0;
        for (i, &g) in ideal_grades.iter().take(5).enumerate() {
            let gain = ((1u32 << g) - 1) as f64;
            let discount = ((i + 2) as f64).log2();
            idcg += gain / discount;
        }
        if idcg == 0.0 {
            0.0
        } else {
            dcg / idcg
        }
    };

    struct EngineMetrics {
        strict_p1: f64,      // Grade == 2 at Rank 1 (仅统计存在 Grade 2 真值的 26 条正向查询)
        effective_p1: f64,   // Grade >= 1 at Rank 1 (全部 28 条正向查询)
        effective_p3: f64,   // Top-3 平均有效相关率 (Grade >= 1)
        mrr10: f64,          // 首个有效命中 (Grade >= 1) 的倒数排名
        strict_mrr10: f64,   // 首个精确命中 (Grade == 2) 的倒数排名
        ndcg5: f64,          // 分级相关性 NDCG@5
        neg_pass: usize,     // 负例查询成功拦截数 (返回 0 条结果)
    }

    let evaluate_engine = |run_fn: &dyn Fn(&str) -> Vec<String>| -> (EngineMetrics, Vec<(String, u8, Vec<String>)>) {
        let mut strict_p1_hits = 0usize;
        let mut strict_positive_total = 0usize;
        let mut effective_p1_hits = 0usize;
        let mut p3_sum = 0.0f64;
        let mut mrr10_sum = 0.0f64;
        let mut strict_mrr10_sum = 0.0f64;
        let mut ndcg5_sum = 0.0f64;
        let mut positive_total = 0usize;
        let mut neg_pass = 0usize;
        let mut details = Vec::new();

        for gq in &gold_set {
            let names = run_fn(gq.query);
            if gq.is_negative {
                let passed = names.is_empty();
                if passed {
                    neg_pass += 1;
                }
                let top1 = names.first().cloned().unwrap_or_else(|| "∅ (正确空结果)".to_string());
                let g1 = if passed { 2 } else { 0 };
                details.push((top1, g1, names.into_iter().take(3).collect()));
                continue;
            }

            positive_total += 1;
            let has_grade2_truth = !gq.relevant_names.is_empty();
            if has_grade2_truth {
                strict_positive_total += 1;
            }

            let grades: Vec<u8> = names.iter().map(|n| grade_skill(n, gq)).collect();
            let g1 = grades.first().copied().unwrap_or(0);
            if g1 == 2 && has_grade2_truth {
                strict_p1_hits += 1;
            }
            if g1 >= 1 {
                effective_p1_hits += 1;
            }

            let top3_rel = grades.iter().take(3).filter(|&&g| g >= 1).count();
            p3_sum += (top3_rel as f64) / 3.0;

            if let Some(pos) = grades.iter().take(10).position(|&g| g >= 1) {
                mrr10_sum += 1.0 / ((pos + 1) as f64);
            }
            if has_grade2_truth {
                if let Some(pos2) = grades.iter().take(10).position(|&g| g == 2) {
                    strict_mrr10_sum += 1.0 / ((pos2 + 1) as f64);
                }
            }

            ndcg5_sum += compute_ndcg_at_5(&grades, gq);

            let top1 = names.first().cloned().unwrap_or_else(|| "Miss".to_string());
            details.push((top1, g1, names.into_iter().take(3).collect()));
        }

        let m = EngineMetrics {
            strict_p1: (strict_p1_hits as f64 / strict_positive_total as f64) * 100.0,
            effective_p1: (effective_p1_hits as f64 / positive_total as f64) * 100.0,
            effective_p3: (p3_sum / positive_total as f64) * 100.0,
            mrr10: mrr10_sum / positive_total as f64,
            strict_mrr10: strict_mrr10_sum / strict_positive_total as f64,
            ndcg5: ndcg5_sum / positive_total as f64,
            neg_pass,
        };
        (m, details)
    };

    let opts = SearchFilterOptions {
        limit: 10,
        ..Default::default()
    };

    let (leg_m, _leg_details) = evaluate_engine(&|q| {
        legacy_search_main_3fb23f5(&real_skills, q, 10)
            .into_iter()
            .filter_map(|id| skill_name_by_id.get(&id).cloned())
            .collect()
    });

    // 测量冷启动查询延迟（首轮 30 条查询）
    let mut cold_latencies_us = Vec::with_capacity(gold_set.len());
    for gq in &gold_set {
        let t_start = Instant::now();
        let _ = search_skills_with_field_authority(&poc_conn, gq.query, &opts).unwrap();
        cold_latencies_us.push(t_start.elapsed().as_micros() as u64);
    }

    let (v31_m, v31_details) = evaluate_engine(&|q| {
        search_skills_poc(&poc_conn, q, &opts)
            .unwrap()
            .into_iter()
            .map(|h| h.name)
            .collect()
    });

    let (v32_m, v32_details) = evaluate_engine(&|q| {
        search_skills_with_field_authority(&poc_conn, q, &opts)
            .unwrap()
            .into_iter()
            .map(|h| h.name)
            .collect()
    });

    // 测量热缓存查询延迟（连续 5 轮 * 30 条 = 150 次查询）
    let mut warm_latencies_us = Vec::with_capacity(gold_set.len() * 5);
    for _ in 0..5 {
        for gq in &gold_set {
            let t_start = Instant::now();
            let _ = search_skills_with_field_authority(&poc_conn, gq.query, &opts).unwrap();
            warm_latencies_us.push(t_start.elapsed().as_micros() as u64);
        }
    }
    cold_latencies_us.sort_unstable();
    warm_latencies_us.sort_unstable();

    let percentile_ms = |sorted_us: &[u64], pct: usize| -> f64 {
        let idx = ((sorted_us.len() - 1) * pct) / 100;
        (sorted_us[idx] as f64) / 1000.0
    };

    println!(
        "\n{:<14} | {:<26} | {:<34} | {:<34}",
        "类别", "Query (30条黄金集)", "Frozen PoC (v3.1) Top-1 [人工评级]", "Field Authority (v3.2) Top-1 [人工评级]"
    );
    println!("{}", "-".repeat(118));

    for (i, gq) in gold_set.iter().enumerate() {
        let (ref t31, g31, _) = v31_details[i];
        let (ref t32, g32, _) = v32_details[i];
        let label = |g: u8, is_neg: bool| -> &'static str {
            if is_neg {
                if g == 2 { "✅PASS" } else { "❌FP" }
            } else {
                match g {
                    2 => "✅Rel(2)",
                    1 => "⚠️Acc(1)",
                    _ => "❌Irr(0)",
                }
            }
        };
        println!(
            "{:<14} | {:<26} | {:<24} {:<8} | {:<24} {:<8}",
            gq.category,
            gq.query,
            t31,
            label(g31, gq.is_negative),
            t32,
            label(g32, gq.is_negative)
        );
    }

    println!("{}", "-".repeat(118));
    println!("30 条人工标注黄金评测集汇总指标 (独立真值标注 · 严禁算法自评):");
    println!(
        "  1. Legacy (3fb23f5 includes)       : Strict P@1 = {:>5.1}% | Eff P@1 = {:>5.1}% | P@3 = {:>5.1}% | Strict MRR = {:.3} | Eff MRR = {:.3} | NDCG@5 = {:.3}",
        leg_m.strict_p1, leg_m.effective_p1, leg_m.effective_p3, leg_m.strict_mrr10, leg_m.mrr10, leg_m.ndcg5
    );
    println!(
        "  2. Frozen PoC (v3.1 无字段权威分层): Strict P@1 = {:>5.1}% | Eff P@1 = {:>5.1}% | P@3 = {:>5.1}% | Strict MRR = {:.3} | Eff MRR = {:.3} | NDCG@5 = {:.3} | 负例 = {}/2",
        v31_m.strict_p1, v31_m.effective_p1, v31_m.effective_p3, v31_m.strict_mrr10, v31_m.mrr10, v31_m.ndcg5, v31_m.neg_pass
    );
    println!(
        "  3. Field Authority (v3.2 权威分层) : Strict P@1 = {:>5.1}% | Eff P@1 = {:>5.1}% | P@3 = {:>5.1}% | Strict MRR = {:.3} | Eff MRR = {:.3} | NDCG@5 = {:.3} | 负例 = {}/2",
        v32_m.strict_p1, v32_m.effective_p1, v32_m.effective_p3, v32_m.strict_mrr10, v32_m.mrr10, v32_m.ndcg5, v32_m.neg_pass
    );
    println!(
        "  4. 真实数据延迟分布 (3,176 skills) : Cold [P50 = {:.2} ms, P95 = {:.2} ms, P99 = {:.2} ms] | Warm [P50 = {:.2} ms, P95 = {:.2} ms, P99 = {:.2} ms]",
        percentile_ms(&cold_latencies_us, 50),
        percentile_ms(&cold_latencies_us, 95),
        percentile_ms(&cold_latencies_us, 99),
        percentile_ms(&warm_latencies_us, 50),
        percentile_ms(&warm_latencies_us, 95),
        percentile_ms(&warm_latencies_us, 99),
    );
    println!("================================================================================================\n");

    // 关键硬性断言：验证评审指出的 4 个典型误召回在 v3.2 中已全部修正！
    let get_v32_top1 = |q_str: &str| -> String {
        let idx = gold_set.iter().position(|g| g.query == q_str).unwrap();
        v32_details[idx].0.clone()
    };
    assert_ne!(
        get_v32_top1("Angular 国际化"),
        "igniteui-angular-figma-to-app",
        "Angular 国际化 绝不能再将 Figma-to-Angular UI 转换器排在第一"
    );
    assert_eq!(
        get_v32_top1("React 单元测试"),
        "gen-rtl-test",
        "React 单元测试 必须将 React Testing Library 测试生成技能 gen-rtl-test 排在第一，绝不能是 gsap-react"
    );
    assert_eq!(
        get_v32_top1("代码审查规范"),
        "code-review",
        "代码审查规范 必须将 code-review 排在第一，绝不能是 wiki-lint"
    );
    assert_eq!(
        get_v32_top1("Docker 容器部署"),
        "docker-best-practices",
        "Docker 容器部署 必须将通用的 docker-best-practices 排在第一"
    );
    assert!(
        v32_m.strict_p1 > v31_m.strict_p1 + 15.0,
        "Field Authority (v3.2) 的 Strict Precision@1 必须比 v3.1 显著提升（至少 15 个百分点）"
    );
    assert!(
        v32_m.ndcg5 > v31_m.ndcg5 + 0.04,
        "Field Authority (v3.2) 的 NDCG@5 必须显著高于 v3.1"
    );
    assert_eq!(v32_m.neg_pass, 2, "2 条负例查询必须 100% 拦截，返回空结果");
}

/// 验收测试 7：安装器崩溃恢复 4 大异常现场 + 10 次幂等性验证 + `create_new(true)` 原子排他锁验证
#[test]
fn test_7_adversarial_crash_recovery_4_cases_and_idempotency() {
    let root = std::env::temp_dir().join(format!("lumina-poc-crash4-{}", now_millis()));
    fs::create_dir_all(&root).unwrap();

    let md_v1 = "---\nname: systematic-debugging\ndescription: Debugging skill v1\n---\n# V1 Stable";
    let md_v2 = "---\nname: systematic-debugging\ndescription: Debugging skill v2\n---\n# V2 Upgraded";

    // 先安装好健康的 V1 作为基线
    let target_dir = install_skill_transactional_poc(
        &root,
        "catalog-sys-debug",
        "https://github.com/org/skills.git",
        "skills/systematic-debugging",
        "commit-v1",
        md_v1,
    )
    .unwrap();
    assert!(is_installed_skill_dir_healthy(&target_dir));

    // 验证 create_new(true) 原子排他锁：当锁已存在且进程存活时，再次获取锁必须立刻拒绝
    let lock_path = acquire_exclusive_skill_lock(&root, "systematic-debugging").unwrap();
    let second_lock_attempt = acquire_exclusive_skill_lock(&root, "systematic-debugging");
    assert!(
        second_lock_attempt.is_err(),
        "create_new(true) 排他锁已存在且持有进程存活时，并发第二次获取必须失败"
    );
    let _ = fs::remove_file(&lock_path);

    let stage_dir = root.join(".lumina-stage-systematic-debugging");
    let backup_dir = root.join(".lumina-backup-systematic-debugging");
    let dead_lock = SkillOpLock {
        pid: 999_999, // 已崩溃进程 PID
        canonical_name: "systematic-debugging".to_string(),
        started_at_ms: now_millis() - 120_000,
    };

    // =========================================================================
    // Case 1: 刚写完 `.lumina-op-lock.json.<name>` 立即崩溃（此时 backup 和 stage 均未生成）
    // =========================================================================
    fs::write(&lock_path, serde_json::to_vec(&dead_lock).unwrap()).unwrap();
    assert!(lock_path.exists());
    recover_interrupted_skill_ops_safe(&root, true);
    assert!(!lock_path.exists(), "Case 1: 孤立死锁文件必须被清理");
    assert!(is_installed_skill_dir_healthy(&target_dir), "Case 1: 原 V1 目录必须完好无损");

    // =========================================================================
    // Case 2: 拷贝文件到 `stage` 一半时崩溃（`stage` 中只有半截损坏文件，`backup` 尚未生成）
    // =========================================================================
    fs::write(&lock_path, serde_json::to_vec(&dead_lock).unwrap()).unwrap();
    fs::create_dir_all(&stage_dir).unwrap();
    fs::write(stage_dir.join("SKILL.md"), "---\nname: broken-half-file").unwrap(); // 无完整 frontmatter 且无 source
    recover_interrupted_skill_ops_safe(&root, true);
    assert!(!stage_dir.exists(), "Case 2: 半成品的 stage 目录必须被彻底清理");
    assert!(!lock_path.exists(), "Case 2: 锁文件必须被清理");
    assert!(is_installed_skill_dir_healthy(&target_dir), "Case 2: 原 V1 目录必须完好无损");
    assert!(fs::read_to_string(target_dir.join("SKILL.md")).unwrap().contains("# V1 Stable"));

    // =========================================================================
    // Case 3: `rename(target, backup)` 刚完成、`rename(stage, target)` 尚未执行时崩溃
    //         （此时 `target` 不存在，`backup` 保存着完整的 V1，`stage` 还残留着）
    // =========================================================================
    fs::write(&lock_path, serde_json::to_vec(&dead_lock).unwrap()).unwrap();
    fs::create_dir_all(&stage_dir).unwrap();
    fs::write(stage_dir.join("SKILL.md"), md_v2).unwrap();
    fs::rename(&target_dir, &backup_dir).unwrap();
    assert!(!target_dir.exists() && backup_dir.exists() && stage_dir.exists());

    recover_interrupted_skill_ops_safe(&root, true);
    assert!(is_installed_skill_dir_healthy(&target_dir), "Case 3: 必须将 backup 原子回滚恢复为 target");
    assert!(!backup_dir.exists() && !stage_dir.exists() && !lock_path.exists());
    assert!(
        fs::read_to_string(target_dir.join("SKILL.md")).unwrap().contains("# V1 Stable"),
        "Case 3: 回滚后的数据必须 100% 是原有的 V1"
    );

    // =========================================================================
    // Case 4: `rename(stage, target)` 已成功完成（新版本 V2 已就位），但删除 `backup` 和 `lock` 前崩溃
    //         （此时 `target` 是健康的 V2，`backup` 是旧的 V1，恢复器绝不能用旧 backup 覆盖已成功的 V2！）
    // =========================================================================
    fs::create_dir_all(&backup_dir).unwrap();
    fs::write(backup_dir.join("SKILL.md"), md_v1).unwrap();
    fs::write(
        backup_dir.join(MANAGED_SOURCE_FILE),
        fs::read_to_string(target_dir.join(MANAGED_SOURCE_FILE)).unwrap(),
    )
    .unwrap();
    // 将 target 更新为健康完整的 V2
    fs::write(target_dir.join("SKILL.md"), md_v2).unwrap();
    fs::write(&lock_path, serde_json::to_vec(&dead_lock).unwrap()).unwrap();

    // 连续执行 10 次恢复，验证幂等性（Idempotency：执行 1 次与执行 10 次结果完全一致）
    for _ in 0..10 {
        recover_interrupted_skill_ops_safe(&root, true);
    }
    assert!(is_installed_skill_dir_healthy(&target_dir), "Case 4 + 10x 幂等: target 必须保持健康");
    assert!(!backup_dir.exists() && !stage_dir.exists() && !lock_path.exists());
    assert!(
        fs::read_to_string(target_dir.join("SKILL.md")).unwrap().contains("# V2 Upgraded"),
        "Case 4: 当 target 已经是健康的 V2 时，绝不能被旧 backup 回退覆盖"
    );

    let _ = fs::remove_dir_all(&root);
}

/// 验收测试 8：长 README “万金油技能 (Kitchen-Sink False Positive)” 对抗测试
/// 验证正文中堆砌全部热门技术词且拥有超高 Star 的万金油技能，绝对不会压过名称/标签精准命中的专职技能
#[test]
fn test_8_kitchen_sink_false_positive_and_field_authority() {
    let mut conn = Connection::open_in_memory().unwrap();
    init_poc_schema(&conn).unwrap();

    let mut catalog = build_synthetic_test_catalog();
    // 注入一条 999,999 Stars、在长 README description 中堆砌几乎所有热门技术词与任务词的“万金油噪声技能”
    catalog.push(json!({
        "id": "skill-kitchen-sink-trap",
        "name": "awesome-fullstack-mega-guide",
        "titleZh": "全栈开发综合参考手册",
        "description": "A massive kitchen-sink reference covering angular, react, vue3, nextjs, typescript, rust, docker, kubernetes, postgres, sql, database query optimization, i18n, localization, translate, locale, unit test, testing, jest, vitest, playwright, visual regression, screenshot, ui component, tailwind, code-review, pr review, memory leak profiler and debugging.",
        "stage": "coding",
        "category": "General",
        "gitUrl": "https://github.com/mega-awesome/fullstack",
        "stars": 999999,
        "halowakeScore": 99,
        "tags": ["awesome", "fullstack", "reference"]
    }));
    // 再注入一条专职的 React 单测技能与专职的 SQL 慢查询优化技能
    catalog.push(json!({
        "id": "skill-dedicated-react-unit-test",
        "name": "react-unit-test-generator",
        "titleZh": "React 单元测试生成器",
        "description": "Generate isolated unit tests for React hooks and components using Vitest and React Testing Library.",
        "stage": "testing",
        "category": "Testing",
        "gitUrl": "https://github.com/qa/react-unit",
        "stars": 320,
        "halowakeScore": 82,
        "tags": ["react", "unit-test", "vitest", "单元测试"]
    }));
    catalog.push(json!({
        "id": "skill-dedicated-sql-opt",
        "name": "sql-slow-query-optimizer",
        "titleZh": "数据库慢查询优化专家",
        "description": "Analyze SQL execution plans, B-tree indexes, and optimize slow database queries.",
        "stage": "coding",
        "category": "Database",
        "gitUrl": "https://github.com/db/sql-opt",
        "stars": 450,
        "halowakeScore": 85,
        "tags": ["sql", "database", "慢查询", "性能优化"]
    }));

    save_registry_with_fts_atomic(&mut conn, "1.0", 100, &catalog).unwrap();
    let opts = SearchFilterOptions::default();

    let adversarial_cases = [
        ("Angular 国际化", "skill-angular-i18n-workflow"),
        ("帮我给 Angular 项目添加国际化支持", "skill-angular-i18n-workflow"),
        ("Vue3组件视觉", "skill-vue-visual-reg"),
        ("修复Vue3组件的视觉问题", "skill-vue-visual-reg"),
        ("React 单元测试", "skill-dedicated-react-unit-test"),
        ("数据库慢查询优化", "skill-dedicated-sql-opt"),
    ];

    for (query, expected_top1_id) in adversarial_cases {
        let hits = search_skills_with_field_authority(&conn, query, &opts).unwrap();
        assert!(!hits.is_empty(), "查询 '{query}' 应当返回结果");
        assert_ne!(
            hits[0].id, "skill-kitchen-sink-trap",
            "查询 '{query}' 绝不能让正文中堆砌关键词的万金油技能 skill-kitchen-sink-trap 排在 Top-1！"
        );
        assert_eq!(
            hits[0].id, expected_top1_id,
            "查询 '{query}' 必须将字段权威度更高的专职技能 {expected_top1_id} 排在 Top-1（实际为 {}）",
            hits[0].id
        );
    }
}

/// 验收测试 9（第二批生产接入终验）：
/// 直接调用生产环境 `git_assistant_lib::storage::skill_registry_repository::search_skill_registry`
/// 在本机真实 `lumina.db`（3,176 条技能）上运行：
/// 1. 30 条黄金回归测试集（Gold Regression Set）
/// 2. 30 条未参与调参的独立盲测查询集（Independent Blind Test Set，覆盖全部 10 大类别）
/// 3. 严格分离统计：
///    - 完全相关命中（Strict P@1 / `match_tier == "exact"` & Grade == 2）
///    - 通用技能退化命中（General Fallback / `match_tier == "general_fallback"` & Grade >= 1）
///    - 负例拦截（Negative Rejection / `match_tier == "none"` & 0 hits）
/// 4. Release 模式下真实端到端搜索延迟（P50 / P95 / P99）
#[test]
fn test_9_production_v32_30_gold_plus_30_blind_set_and_release_benchmark() {
    use git_assistant_lib::storage::{
        migrations::run_migrations,
        skill_registry_repository::{
            save_skill_registry, search_skill_registry, SkillSearchRequest,
        },
        AppDatabase,
    };

    let appdata = std::env::var("APPDATA").unwrap_or_default();
    let real_db_path = PathBuf::from(&appdata).join("com.yubei.lumina").join("lumina.db");
    if !real_db_path.is_file() {
        println!("未找到本机 lumina.db，跳过生产端到端评测");
        return;
    }

    let real_conn = Connection::open_with_flags(
        &real_db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let mut stmt = real_conn
        .prepare("SELECT payload_json FROM skill_registry_entries ORDER BY rowid")
        .unwrap();
    let real_skills: Vec<Value> = stmt
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .filter_map(|txt| serde_json::from_str::<Value>(&txt).ok())
        .collect();
    assert!(real_skills.len() > 1000);

    // 创建独立的生产 AppDatabase 实例并执行真实迁移与原子落库
    let temp_prod_db_path = std::env::temp_dir().join(format!(
        "lumina-prod-v32-bench-{}-{}.db",
        std::process::id(),
        now_millis()
    ));
    let prod_db = AppDatabase::from_path(temp_prod_db_path.clone());
    {
        let mut conn = prod_db.connect().unwrap();
        run_migrations(&mut conn).unwrap();
    }
    let catalog_payload = json!({
        "version": "1.4.1-prod-bench",
        "updatedAt": now_millis(),
        "skills": real_skills,
        "pipelines": []
    });
    save_skill_registry(&prod_db, &catalog_payload).unwrap();

    struct EvalQuery {
        category: &'static str,
        query: &'static str,
        relevant_names: &'static [&'static str],
        acceptable_names: &'static [&'static str],
        is_negative: bool,
    }

    // A. 30 条黄金回归测试集（与 test_6 完全一致）
    let gold_set: [EvalQuery; 30] = [
        EvalQuery {
            category: "1.精确名称",
            query: "systematic-debugging",
            relevant_names: &["systematic-debugging"],
            acceptable_names: &["debug-like-expert", "bug-hunt", "error-detective"],
            is_negative: false,
        },
        EvalQuery {
            category: "1.精确名称",
            query: "frontend-design",
            relevant_names: &["frontend-design", "ui-ux-pro-max", "super-ui-ux-design"],
            acceptable_names: &["design-systems", "web-design-guidelines"],
            is_negative: false,
        },
        EvalQuery {
            category: "1.精确名称",
            query: "webapp-testing",
            relevant_names: &["webapp-testing", "e2e-testing", "webwright"],
            acceptable_names: &["testing", "pr-test", "openclaw-testing"],
            is_negative: false,
        },
        EvalQuery {
            category: "2.纯中文",
            query: "国际化",
            relevant_names: &["i18n-parity", "translate", "asc-localize-metadata"],
            acceptable_names: &[],
            is_negative: false,
        },
        EvalQuery {
            category: "2.纯中文",
            query: "代码审查规范",
            relevant_names: &["code-review", "code-review-and-quality", "autoreview", "review-pr", "requesting-code-review", "receiving-code-review"],
            acceptable_names: &["understand-diff", "gh-stack", "rust-review", "typescript-style-review"],
            is_negative: false,
        },
        EvalQuery {
            category: "2.纯中文",
            query: "数据库慢查询优化",
            relevant_names: &["database-optimization", "supabase-postgres-best-practices", "tsh-sql-and-database-understanding"],
            acceptable_names: &["sql-correctness", "sql-style", "database-schema-designer"],
            is_negative: false,
        },
        EvalQuery {
            category: "2.纯中文",
            query: "登录鉴权与权限控制",
            relevant_names: &[
                "auth-and-authorization-patterns",
                "authentication-patterns",
                "secure-auth",
                "accessible-authentication",
                "login-flows",
                "add-oauth-provider",
            ],
            acceptable_names: &["security-audit"],
            is_negative: false,
        },
        EvalQuery {
            category: "3.纯英文",
            query: "react unit testing",
            relevant_names: &["gen-rtl-test"],
            acceptable_names: &["testing", "testing-patterns", "tdd", "test-driven-development", "tdd-workflow", "nw-tdd-methodology"],
            is_negative: false,
        },
        EvalQuery {
            category: "3.纯英文",
            query: "database query optimization",
            relevant_names: &["database-optimization", "supabase-postgres-best-practices", "tsh-sql-and-database-understanding"],
            acceptable_names: &["sql-correctness", "sql-style", "database-schema-designer"],
            is_negative: false,
        },
        EvalQuery {
            category: "3.纯英文",
            query: "docker container deployment",
            relevant_names: &["docker-best-practices", "devops-automation"],
            acceptable_names: &["azure-kubernetes-service", "k8s-yaml-validator", "greptimedb-development-docker-image"],
            is_negative: false,
        },
        EvalQuery {
            category: "4.中英混合",
            query: "Angular 国际化",
            relevant_names: &[],
            acceptable_names: &["i18n-parity", "translate", "asc-localize-metadata"],
            is_negative: false,
        },
        EvalQuery {
            category: "4.中英混合",
            query: "Vue3组件视觉",
            relevant_names: &["vue3-best-practices", "buefy", "frontend-design", "ui-ux-pro-max"],
            acceptable_names: &["vueuse-functions", "super-ui-ux-design", "design-review"],
            is_negative: false,
        },
        EvalQuery {
            category: "4.中英混合",
            query: "React 单元测试",
            relevant_names: &["gen-rtl-test"],
            acceptable_names: &["testing", "testing-patterns", "tdd", "test-driven-development", "tdd-workflow", "nw-tdd-methodology"],
            is_negative: false,
        },
        EvalQuery {
            category: "4.中英混合",
            query: "Playwright 自动化测试",
            relevant_names: &["webapp-testing", "e2e-testing", "webwright", "update-int-snapshots"],
            acceptable_names: &["login-flows", "openclaw-testing", "pr-test", "testing"],
            is_negative: false,
        },
        EvalQuery {
            category: "4.中英混合",
            query: "Tailwind 界面美化",
            relevant_names: &["antd-style-to-tailwind", "loopwind", "frontend-design", "ui-ux-pro-max"],
            acceptable_names: &["senior-frontend", "react", "super-ui-ux-design"],
            is_negative: false,
        },
        EvalQuery {
            category: "5.自然语言",
            query: "帮我给 Angular 项目添加国际化支持",
            relevant_names: &[],
            acceptable_names: &["i18n-parity", "translate", "asc-localize-metadata"],
            is_negative: false,
        },
        EvalQuery {
            category: "5.自然语言",
            query: "修复Vue3组件的视觉问题",
            relevant_names: &["vue3-best-practices", "buefy", "frontend-design", "ui-ux-pro-max"],
            acceptable_names: &["vueuse-functions", "design-review", "super-ui-ux-design"],
            is_negative: false,
        },
        EvalQuery {
            category: "5.自然语言",
            query: "给 React Hook 写单元测试",
            relevant_names: &["gen-rtl-test"],
            acceptable_names: &["testing", "testing-patterns", "tdd", "test-driven-development", "tdd-workflow"],
            is_negative: false,
        },
        EvalQuery {
            category: "6.高歧义词",
            query: "UI",
            relevant_names: &[
                "frontend-design",
                "ui-ux-pro-max",
                "super-ui-ux-design",
                "design-systems",
                "web-design-guidelines",
                "ui-widget-developer",
            ],
            acceptable_names: &["buefy", "design-review", "antd-style-to-tailwind", "minimalist-ui"],
            is_negative: false,
        },
        EvalQuery {
            category: "6.高歧义词",
            query: "测试",
            relevant_names: &["testing", "testing-patterns", "tdd", "test-driven-development", "webapp-testing", "e2e-testing", "gen-rtl-test"],
            acceptable_names: &["tdd-workflow", "pr-test", "openclaw-testing"],
            is_negative: false,
        },
        EvalQuery {
            category: "6.高歧义词",
            query: "重构",
            relevant_names: &["refactor", "kaizen-2-refactor", "architecture-audit", "code-review-and-quality"],
            acceptable_names: &["antd-style-to-tailwind", "code-review", "clean-code"],
            is_negative: false,
        },
        EvalQuery {
            category: "7.同义词",
            query: "多语言本地化",
            relevant_names: &["i18n-parity", "translate", "asc-localize-metadata"],
            acceptable_names: &[],
            is_negative: false,
        },
        EvalQuery {
            category: "7.同义词",
            query: "PR 代码评审",
            relevant_names: &["review-pr", "code-review", "code-review-and-quality", "autoreview", "requesting-code-review", "receiving-code-review"],
            acceptable_names: &["understand-diff", "gh-stack"],
            is_negative: false,
        },
        EvalQuery {
            category: "8.易混技术栈",
            query: "Vue 表格组件",
            relevant_names: &["buefy", "vue3-best-practices"],
            acceptable_names: &["vueuse-functions", "frontend-design"],
            is_negative: false,
        },
        EvalQuery {
            category: "8.易混技术栈",
            query: "Next.js 性能优化",
            relevant_names: &["nextjs", "performance-optimization", "perf-profiler", "perf-code-paths", "bottleneck-hunter"],
            acceptable_names: &["react", "senior-frontend"],
            is_negative: false,
        },
        EvalQuery {
            category: "8.易混技术栈",
            query: "TypeScript 代码审查",
            relevant_names: &["typescript-style-review", "code-review", "code-review-and-quality"],
            acceptable_names: &["howto-code-in-typescript", "ts-core", "autoreview", "review-pr"],
            is_negative: false,
        },
        EvalQuery {
            category: "9.噪声干扰",
            query: "Rust 内存排查",
            relevant_names: &["rust-review", "perf-profiler", "perf-code-paths", "bottleneck-hunter"],
            acceptable_names: &["cgp", "systematic-debugging", "architecture-audit"],
            is_negative: false,
        },
        EvalQuery {
            category: "9.噪声干扰",
            query: "Docker 容器部署",
            relevant_names: &["docker-best-practices", "devops-automation"],
            acceptable_names: &["azure-kubernetes-service", "k8s-yaml-validator", "greptimedb-development-docker-image"],
            is_negative: false,
        },
        EvalQuery {
            category: "10.负例查询",
            query: "量子计算芯片光刻仿真",
            relevant_names: &[],
            acceptable_names: &[],
            is_negative: true,
        },
        EvalQuery {
            category: "10.负例查询",
            query: "自动驾驶激光雷达标定",
            relevant_names: &[],
            acceptable_names: &[],
            is_negative: true,
        },
    ];

    // B. 30 条全新独立盲测集（Independent Blind Test Set · 未参与前期算法调参）
    let blind_set: [EvalQuery; 30] = [
        // 1. 精确名称 (3)
        EvalQuery {
            category: "1.精确名称",
            query: "security-audit",
            relevant_names: &["security-audit", "security-review", "secure-auth"],
            acceptable_names: &["auth-and-authorization-patterns", "authentication-patterns"],
            is_negative: false,
        },
        EvalQuery {
            category: "1.精确名称",
            query: "architecture-audit",
            relevant_names: &["architecture-audit", "system-design", "clean-architecture"],
            acceptable_names: &["code-review-and-quality", "refactor"],
            is_negative: false,
        },
        EvalQuery {
            category: "1.精确名称",
            query: "test-driven-development",
            relevant_names: &["test-driven-development", "tdd", "tdd-workflow", "nw-tdd-methodology"],
            acceptable_names: &["testing", "testing-patterns", "gen-rtl-test"],
            is_negative: false,
        },
        // 2. 纯中文 (4)
        EvalQuery {
            category: "2.纯中文",
            query: "前端组件设计",
            relevant_names: &["frontend-design", "ui-ux-pro-max", "super-ui-ux-design", "design-systems", "ui-widget-developer", "buefy"],
            acceptable_names: &["web-design-guidelines", "vue3-best-practices"],
            is_negative: false,
        },
        EvalQuery {
            category: "2.纯中文",
            query: "系统故障调试",
            relevant_names: &["systematic-debugging", "debug-like-expert", "bug-hunt", "error-detective", "root-cause-tracing"],
            acceptable_names: &["perf-profiler", "bottleneck-hunter"],
            is_negative: false,
        },
        EvalQuery {
            category: "2.纯中文",
            query: "端到端测试",
            relevant_names: &["e2e-testing", "webapp-testing", "webwright"],
            acceptable_names: &["testing", "testing-patterns", "login-flows", "openclaw-testing"],
            is_negative: false,
        },
        EvalQuery {
            category: "2.纯中文",
            query: "安全审计与漏洞排查",
            relevant_names: &["security-audit", "security-review", "secure-auth", "auth-and-authorization-patterns"],
            acceptable_names: &["systematic-debugging", "code-review-and-quality"],
            is_negative: false,
        },
        // 3. 纯英文 (3)
        EvalQuery {
            category: "3.纯英文",
            query: "systematic debugging",
            relevant_names: &["systematic-debugging", "debug-like-expert", "bug-hunt"],
            acceptable_names: &["error-detective", "root-cause-tracing"],
            is_negative: false,
        },
        EvalQuery {
            category: "3.纯英文",
            query: "frontend design system",
            relevant_names: &["frontend-design", "design-systems", "ui-ux-pro-max", "super-ui-ux-design"],
            acceptable_names: &["web-design-guidelines", "ui-widget-developer"],
            is_negative: false,
        },
        EvalQuery {
            category: "3.纯英文",
            query: "pull request code review",
            relevant_names: &["review-pr", "code-review", "code-review-and-quality", "autoreview", "requesting-code-review", "receiving-code-review", "when-reviewing-pull-request-orchestrate-comprehensive-code-revie"],
            acceptable_names: &["understand-diff", "gh-stack"],
            is_negative: false,
        },
        // 4. 中英混合 (5)
        EvalQuery {
            category: "4.中英混合",
            query: "PostgreSQL 数据库优化",
            relevant_names: &["supabase-postgres-best-practices", "database-optimization", "tsh-sql-and-database-understanding"],
            acceptable_names: &["sql-correctness", "sql-style", "database-schema-designer"],
            is_negative: false,
        },
        EvalQuery {
            category: "4.中英混合",
            query: "OAuth 登录鉴权",
            relevant_names: &["add-oauth-provider", "auth-and-authorization-patterns", "authentication-patterns", "secure-auth", "login-flows", "accessible-authentication"],
            acceptable_names: &["security-audit"],
            is_negative: false,
        },
        EvalQuery {
            category: "4.中英混合",
            query: "E2E 自动化测试",
            relevant_names: &["e2e-testing", "webapp-testing", "webwright"],
            acceptable_names: &["testing", "testing-patterns", "openclaw-testing", "pr-test"],
            is_negative: false,
        },
        EvalQuery {
            category: "4.中英混合",
            query: "SQL 慢查询排查",
            relevant_names: &["database-optimization", "tsh-sql-and-database-understanding", "supabase-postgres-best-practices", "sql-correctness"],
            acceptable_names: &["sql-style", "perf-profiler", "bottleneck-hunter"],
            is_negative: false,
        },
        EvalQuery {
            category: "4.中英混合",
            query: "Vue3 状态管理与组件",
            relevant_names: &["vue3-best-practices", "vueuse-functions", "buefy"],
            acceptable_names: &["frontend-design", "ui-ux-pro-max"],
            is_negative: false,
        },
        // 5. 自然语言 (3)
        EvalQuery {
            category: "5.自然语言",
            query: "帮我排查系统里的内存泄漏和性能瓶颈",
            relevant_names: &["perf-profiler", "perf-code-paths", "bottleneck-hunter", "performance-optimization", "systematic-debugging"],
            acceptable_names: &["debug-like-expert", "rust-review"],
            is_negative: false,
        },
        EvalQuery {
            category: "5.自然语言",
            query: "提交代码前帮我做一次安全审计",
            relevant_names: &["security-audit", "security-review", "code-review-and-quality", "code-review"],
            acceptable_names: &["secure-auth", "autoreview", "review-pr"],
            is_negative: false,
        },
        EvalQuery {
            category: "5.自然语言",
            query: "如何用 Playwright 做端到端测试",
            relevant_names: &["webapp-testing", "e2e-testing", "webwright", "update-int-snapshots"],
            acceptable_names: &["testing", "login-flows", "openclaw-testing"],
            is_negative: false,
        },
        // 6. 高歧义词 (3)
        EvalQuery {
            category: "6.高歧义词",
            query: "性能",
            relevant_names: &["performance-optimization", "perf-profiler", "perf-code-paths", "bottleneck-hunter", "database-optimization"],
            acceptable_names: &["supabase-postgres-best-practices", "nextjs"],
            is_negative: false,
        },
        EvalQuery {
            category: "6.高歧义词",
            query: "安全",
            relevant_names: &["security-audit", "security-review", "secure-auth", "auth-and-authorization-patterns", "authentication-patterns"],
            acceptable_names: &["accessible-authentication", "add-oauth-provider"],
            is_negative: false,
        },
        EvalQuery {
            category: "6.高歧义词",
            query: "调试",
            relevant_names: &["systematic-debugging", "debug-like-expert", "bug-hunt", "error-detective", "root-cause-tracing"],
            acceptable_names: &["perf-profiler", "bottleneck-hunter"],
            is_negative: false,
        },
        // 7. 同义词 (2)
        EvalQuery {
            category: "7.同义词",
            query: "无障碍可访问性",
            relevant_names: &["accessible-authentication", "web-design-guidelines", "frontend-design", "ui-ux-pro-max", "accessibility-wcag"],
            acceptable_names: &["super-ui-ux-design", "design-systems"],
            is_negative: false,
        },
        EvalQuery {
            category: "7.同义词",
            query: "TDD 单测驱动",
            relevant_names: &["tdd", "test-driven-development", "tdd-workflow", "nw-tdd-methodology", "gen-rtl-test"],
            acceptable_names: &["testing", "testing-patterns"],
            is_negative: false,
        },
        // 8. 易混技术栈与无专属技能退化 (3)
        EvalQuery {
            category: "8.易混技术栈",
            query: "Svelte 国际化",
            // 目录中无 Svelte 专属 i18n 技能，应当触发 General Fallback 返回通用 i18n 技能
            relevant_names: &[],
            acceptable_names: &["i18n-parity", "translate", "asc-localize-metadata"],
            is_negative: false,
        },
        EvalQuery {
            category: "8.易混技术栈",
            query: "Flutter 多语言本地化",
            // 目录中无 Flutter 专属 i18n 技能，应当触发 General Fallback 返回通用 i18n 技能
            relevant_names: &[],
            acceptable_names: &["i18n-parity", "translate", "asc-localize-metadata"],
            is_negative: false,
        },
        EvalQuery {
            category: "8.易混技术栈",
            query: "React 界面样式美化",
            relevant_names: &["antd-style-to-tailwind", "frontend-design", "ui-ux-pro-max", "super-ui-ux-design", "react"],
            acceptable_names: &["loopwind", "design-systems", "senior-frontend"],
            is_negative: false,
        },
        // 9. 热门仓库噪声干扰 (2)
        EvalQuery {
            category: "9.噪声干扰",
            query: "架构重构与代码质量",
            relevant_names: &["architecture-audit", "code-review-and-quality", "refactor", "kaizen-2-refactor"],
            acceptable_names: &["code-review", "clean-code"],
            is_negative: false,
        },
        EvalQuery {
            category: "9.噪声干扰",
            query: "前端 UI 视觉规范",
            relevant_names: &["frontend-design", "web-design-guidelines", "ui-ux-pro-max", "super-ui-ux-design", "design-systems"],
            acceptable_names: &["ui-widget-developer", "buefy", "design-review"],
            is_negative: false,
        },
        // 10. 负例查询 (2)
        EvalQuery {
            category: "10.负例查询",
            query: "核聚变托卡马克等离子体约束",
            relevant_names: &[],
            acceptable_names: &[],
            is_negative: true,
        },
        EvalQuery {
            category: "10.负例查询",
            query: "基因测序蛋白质三维折叠预测",
            relevant_names: &[],
            acceptable_names: &[],
            is_negative: true,
        },
    ];

    let run_suite = |suite_title: &str, suite: &[EvalQuery]| {
        let mut strict_exact_queries = 0usize;
        let mut strict_p1_hits = 0usize;
        let mut strict_p3_sum = 0.0f64;
        let mut strict_mrr10_sum = 0.0f64;

        let mut fallback_queries = 0usize;
        let mut fallback_correct_hits = 0usize;
        let mut fallback_tier_labeled = 0usize;

        let mut negative_queries = 0usize;
        let mut negative_rejected = 0usize;

        println!("\n================================================================================================================");
        println!("{suite_title} (共 {} 条查询 · 生产路径 `search_skill_registry`)", suite.len());
        println!("----------------------------------------------------------------------------------------------------------------");
        println!(
            "{:<14} | {:<30} | {:<18} | {:<28} | {:<12}",
            "类别", "Query", "生产返回 Tier", "Top-1 技能", "人工真值判定"
        );
        println!("----------------------------------------------------------------------------------------------------------------");

        for eq in suite {
            let resp = search_skill_registry(
                &prod_db,
                &SkillSearchRequest {
                    query: eq.query.to_string(),
                    limit: Some(10),
                    ..Default::default()
                },
            )
            .unwrap();

            let top1_name = resp
                .hits
                .first()
                .map(|h| h.name.clone())
                .unwrap_or_else(|| "∅ (空结果)".to_string());

            if eq.is_negative {
                negative_queries += 1;
                let rejected = resp.hits.is_empty() && resp.match_tier == "none";
                if rejected {
                    negative_rejected += 1;
                }
                println!(
                    "{:<14} | {:<30} | {:<18} | {:<28} | {}",
                    eq.category,
                    eq.query,
                    resp.match_tier,
                    top1_name,
                    if rejected { "✅ 负例拦截(none)" } else { "❌ 误召回(FP)" }
                );
                continue;
            }

            let grades: Vec<u8> = resp
                .hits
                .iter()
                .map(|h| {
                    if eq.relevant_names.contains(&h.name.as_str()) {
                        2
                    } else if eq.acceptable_names.contains(&h.name.as_str()) {
                        1
                    } else {
                        0
                    }
                })
                .collect();
            let g1 = grades.first().copied().unwrap_or(0);

            if eq.relevant_names.is_empty() {
                // 目录中无专属技能、应触发通用退化 (General Fallback) 的查询
                fallback_queries += 1;
                if g1 >= 1 {
                    fallback_correct_hits += 1;
                }
                if resp.match_tier == "general_fallback" && resp.fallback_notice.is_some() {
                    fallback_tier_labeled += 1;
                }
                println!(
                    "{:<14} | {:<30} | {:<18} | {:<28} | {}",
                    eq.category,
                    eq.query,
                    resp.match_tier,
                    top1_name,
                    if g1 >= 1 && resp.match_tier == "general_fallback" {
                        "ℹ️ 通用退化命中(Grade 1)"
                    } else {
                        "❌ 退化异常"
                    }
                );
            } else {
                // 存在专属理想技能的查询：严格按 Grade == 2 统计 Strict P@1
                strict_exact_queries += 1;
                if g1 == 2 {
                    strict_p1_hits += 1;
                }
                let top3_rel = grades.iter().take(3).filter(|&&g| g == 2).count();
                strict_p3_sum += (top3_rel as f64) / 3.0;
                if let Some(pos2) = grades.iter().take(10).position(|&g| g == 2) {
                    strict_mrr10_sum += 1.0 / ((pos2 + 1) as f64);
                }
                let verdict = match g1 {
                    2 => "✅ 完全相关(Grade 2)",
                    1 => "⚠️ 通用可用(Grade 1)",
                    _ => "❌ 不相关(Grade 0)",
                };
                println!(
                    "{:<14} | {:<30} | {:<18} | {:<28} | {}",
                    eq.category, eq.query, resp.match_tier, top1_name, verdict
                );
            }
        }

        let strict_p1_pct = (strict_p1_hits as f64 / strict_exact_queries as f64) * 100.0;
        let strict_p3_pct = (strict_p3_sum / strict_exact_queries as f64) * 100.0;
        let strict_mrr10 = strict_mrr10_sum / strict_exact_queries as f64;
        println!("----------------------------------------------------------------------------------------------------------------");
        println!(
            "【{suite_title} 分层统计结果（严禁混算）】:\n  \
             1) 完全相关命中 (Strict Exact Match, 共 {strict_exact_queries} 条有专属技能查询): Strict P@1 = {strict_p1_hits}/{strict_exact_queries} ({strict_p1_pct:.1}%), Strict P@3 = {strict_p3_pct:.1}%, Strict MRR@10 = {strict_mrr10:.3}\n  \
             2) 通用技能退化 (General Fallback, 共 {fallback_queries} 条无专属技术栈技能查询): 通用可用命中 = {fallback_correct_hits}/{fallback_queries}, 显式标注 general_fallback 提示 = {fallback_tier_labeled}/{fallback_queries}\n  \
             3) 负例查询拦截 (Negative Rejection, 共 {negative_queries} 条跨学科无关查询): 100% 空结果拦截 = {negative_rejected}/{negative_queries}"
        );
        println!("================================================================================================================\n");

        (
            strict_p1_pct,
            strict_mrr10,
            fallback_correct_hits,
            fallback_queries,
            fallback_tier_labeled,
            negative_rejected,
            negative_queries,
        )
    };

    let (gold_p1, gold_mrr, gold_fb_hits, gold_fb_total, gold_fb_labeled, gold_neg_hits, gold_neg_total) =
        run_suite("一、30 条黄金回归测试集 (Gold Regression Set)", &gold_set);
    let (blind_p1, blind_mrr, blind_fb_hits, blind_fb_total, blind_fb_labeled, blind_neg_hits, blind_neg_total) =
        run_suite("二、30 条独立盲测查询集 (Independent Blind Test Set)", &blind_set);

    // C. 端到端生产搜索延迟基准测试（含 SQLite 连接、FTS5 召回、v3.2 字段权威度重排、JSON 反序列化与匹配原因生成）
    let all_queries: Vec<&str> = gold_set
        .iter()
        .chain(blind_set.iter())
        .map(|q| q.query)
        .collect();

    let mut cold_us = Vec::with_capacity(all_queries.len());
    for &q in &all_queries {
        let t0 = Instant::now();
        let _ = search_skill_registry(
            &prod_db,
            &SkillSearchRequest {
                query: q.to_string(),
                limit: Some(50),
                ..Default::default()
            },
        )
        .unwrap();
        cold_us.push(t0.elapsed().as_micros() as u64);
    }

    let mut warm_us = Vec::with_capacity(all_queries.len() * 5);
    for _ in 0..5 {
        for &q in &all_queries {
            let t0 = Instant::now();
            let _ = search_skill_registry(
                &prod_db,
                &SkillSearchRequest {
                    query: q.to_string(),
                    limit: Some(50),
                    ..Default::default()
                },
            )
            .unwrap();
            warm_us.push(t0.elapsed().as_micros() as u64);
        }
    }
    cold_us.sort_unstable();
    warm_us.sort_unstable();
    let pct_ms = |arr: &[u64], p: usize| -> f64 {
        let idx = ((arr.len() - 1) * p) / 100;
        (arr[idx] as f64) / 1000.0
    };

    println!(
        "【三、生产路径 `search_skill_registry` 端到端延迟分布 (60 条查询 × 3,176 条真实技能, Limit=50)】:\n  \
         - 首轮查询 (Cold) : P50 = {:.2} ms | P95 = {:.2} ms | P99 = {:.2} ms\n  \
         - 热缓存查询 (Warm): P50 = {:.2} ms | P95 = {:.2} ms | P99 = {:.2} ms\n",
        pct_ms(&cold_us, 50),
        pct_ms(&cold_us, 95),
        pct_ms(&cold_us, 99),
        pct_ms(&warm_us, 50),
        pct_ms(&warm_us, 95),
        pct_ms(&warm_us, 99),
    );

    let _ = std::fs::remove_file(temp_prod_db_path);

    // 硬性验收断言
    assert!(gold_p1 >= 95.0, "30 条回归集 Strict P@1 必须 >= 95% (实际 {gold_p1:.1}%)");
    assert!(gold_mrr >= 0.95);
    assert_eq!(gold_fb_hits, gold_fb_total, "回归集通用退化查询必须全部返回有效通用技能");
    assert_eq!(gold_fb_labeled, gold_fb_total, "回归集通用退化查询必须 100% 标记 general_fallback 并附带提示");
    assert_eq!(gold_neg_hits, gold_neg_total, "回归集负例查询必须 100% 拦截");

    assert!(blind_p1 >= 88.0, "30 条独立盲测集 Strict P@1 必须 >= 88% (实际 {blind_p1:.1}%)");
    assert!(blind_mrr >= 0.90);
    assert_eq!(blind_fb_hits, blind_fb_total, "盲测集通用退化查询必须全部返回有效通用技能");
    assert_eq!(blind_fb_labeled, blind_fb_total, "盲测集通用退化查询必须 100% 标记 general_fallback 并附带提示");
    assert_eq!(blind_neg_hits, blind_neg_total, "盲测集负例查询必须 100% 拦截");
}


