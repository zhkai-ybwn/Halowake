use rusqlite::{params, OptionalExtension};

use crate::storage::AppDatabase;

use super::models::{
    AiCallUsage, ReviewFileRecord, ReviewFinding, ReviewOverview, ReviewRule, ReviewSession,
    ReviewSessionSummary,
};

pub fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

pub fn create_session(
    database: &AppDatabase,
    id: &str,
    repo_root: &str,
    fingerprint: &str,
    budget_mode: &str,
    model_id: &str,
    selected_files: &[String],
    rule_snapshot: &[ReviewRule],
    retention_days: u32,
) -> Result<(), String> {
    let now = now_millis();
    let expires_at = now + i64::from(retention_days) * 86_400_000;
    let connection = database.connect()?;
    connection
        .execute(
            "INSERT INTO review_sessions (
              id, repo_root, diff_fingerprint, status, phase, budget_mode, model_id,
              rule_snapshot_json, selected_files_json, created_at, updated_at, expires_at
            ) VALUES (?1, ?2, ?3, 'running', 'planning', ?4, ?5, ?6, ?7, ?8, ?8, ?9)",
            params![
                id,
                repo_root,
                fingerprint,
                budget_mode,
                model_id,
                json(rule_snapshot)?,
                json(selected_files)?,
                now,
                expires_at,
            ],
        )
        .map_err(|error| format!("创建 Review session 失败: {error}"))?;
    Ok(())
}

pub fn replace_files(database: &AppDatabase, session_id: &str, files: &[ReviewFileRecord]) -> Result<(), String> {
    let mut connection = database.connect()?;
    let transaction = connection
        .transaction()
        .map_err(|error| format!("开启 Review 文件事务失败: {error}"))?;
    transaction
        .execute("DELETE FROM review_files WHERE session_id = ?1", [session_id])
        .map_err(|error| format!("清理 Review 文件失败: {error}"))?;
    for file in files {
        transaction
            .execute(
                "INSERT INTO review_files (
                   session_id, path, change_kind, attention_score, score_categories_json,
                   score_breakdown_json, selected, review_status, batch_id, limitation
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    session_id,
                    file.path,
                    file.change_kind,
                    file.attention_score,
                    json(&file.score_categories)?,
                    json(&file.score_breakdown)?,
                    file.selected,
                    file.review_status,
                    file.batch_id,
                    file.limitation,
                ],
            )
            .map_err(|error| format!("保存 Review 文件失败: {error}"))?;
    }
    transaction
        .commit()
        .map_err(|error| format!("提交 Review 文件失败: {error}"))
}

pub fn append_findings(database: &AppDatabase, session_id: &str, findings: &[ReviewFinding]) -> Result<(), String> {
    let mut connection = database.connect()?;
    let transaction = connection
        .transaction()
        .map_err(|error| format!("开启 finding 事务失败: {error}"))?;
    let now = now_millis();
    for finding in findings {
        let stored_id = format!("{session_id}:{}", finding.id);
        transaction
            .execute(
                "INSERT OR IGNORE INTO review_findings (
                  id, session_id, file_path, fingerprint, source, rule_id, category, severity,
                  confidence, start_line, end_line, title, problem, impact, trigger_scenario,
                  evidence, suggestion, verified, status, user_note, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                          ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?21)",
                params![
                    stored_id,
                    session_id,
                    finding.file_path,
                    finding.fingerprint,
                    finding.source,
                    finding.rule_id,
                    finding.category,
                    finding.severity,
                    finding.confidence,
                    finding.start_line,
                    finding.end_line,
                    finding.title,
                    finding.problem,
                    finding.impact,
                    finding.trigger_scenario,
                    finding.evidence,
                    finding.suggestion,
                    finding.verified,
                    finding.status,
                    finding.user_note,
                    now,
                ],
            )
            .map_err(|error| format!("保存 Review finding 失败: {error}"))?;
    }
    transaction
        .commit()
        .map_err(|error| format!("提交 Review findings 失败: {error}"))
}

pub fn append_ai_call(database: &AppDatabase, session_id: &str, model_id: &str, usage: &AiCallUsage) -> Result<(), String> {
    let connection = database.connect()?;
    connection
        .execute(
            "INSERT INTO review_ai_calls (
              session_id, batch_id, model_id, files_json, input_tokens, output_tokens,
              usage_estimated, duration_ms, status, error_message, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                session_id,
                usage.batch_id,
                model_id,
                json(&usage.files)?,
                usage.input_tokens,
                usage.output_tokens,
                usage.estimated,
                usage.duration_ms,
                usage.status,
                usage.error,
                now_millis(),
            ],
        )
        .map_err(|error| format!("保存 AI 调用明细失败: {error}"))?;
    Ok(())
}

pub fn update_progress(
    database: &AppDatabase,
    session_id: &str,
    status: &str,
    phase: &str,
    done: usize,
    total: usize,
    current_file: Option<&str>,
) -> Result<(), String> {
    let connection = database.connect()?;
    connection
        .execute(
            "UPDATE review_sessions SET status=?2, phase=?3, progress_done=?4,
             progress_total=?5, current_file=?6, updated_at=?7 WHERE id=?1",
            params![session_id, status, phase, done, total, current_file, now_millis()],
        )
        .map_err(|error| format!("更新 Review 进度失败: {error}"))?;
    Ok(())
}

pub fn finish_session(
    database: &AppDatabase,
    session_id: &str,
    status: &str,
    overview: &ReviewOverview,
    limitations: &[String],
    input_tokens: usize,
    output_tokens: usize,
    estimated: bool,
    error: Option<&str>,
) -> Result<(), String> {
    let now = now_millis();
    let connection = database.connect()?;
    connection
        .execute(
            "UPDATE review_sessions SET status=?2, phase='complete', overview_json=?3,
             limitations_json=?4, input_tokens=?5, output_tokens=?6, usage_estimated=?7,
             error_message=?8, updated_at=?9, completed_at=?9 WHERE id=?1",
            params![
                session_id,
                status,
                json(overview)?,
                json(limitations)?,
                input_tokens,
                output_tokens,
                estimated,
                error,
                now,
            ],
        )
        .map_err(|db_error| format!("完成 Review session 失败: {db_error}"))?;
    Ok(())
}

pub fn get_session(database: &AppDatabase, session_id: &str) -> Result<ReviewSession, String> {
    let connection = database.connect()?;
    let mut session = connection
        .query_row(
            "SELECT id, repo_root, diff_fingerprint, status, phase, progress_done,
             progress_total, current_file, budget_mode, model_id, selected_files_json,
             overview_json, limitations_json, input_tokens, output_tokens, usage_estimated,
             error_message, created_at, updated_at, completed_at, is_pinned
             FROM review_sessions WHERE id=?1",
            [session_id],
            |row| {
                Ok(ReviewSession {
                    id: row.get(0)?, repo_root: row.get(1)?, diff_fingerprint: row.get(2)?,
                    status: row.get(3)?, phase: row.get(4)?, progress_done: row.get(5)?,
                    progress_total: row.get(6)?, current_file: row.get(7)?, budget_mode: row.get(8)?,
                    model_id: row.get(9)?, selected_files: from_json(row.get::<_, String>(10)?)?,
                    overview: from_json(row.get::<_, String>(11)?)?, limitations: from_json(row.get::<_, String>(12)?)?,
                    input_tokens: row.get(13)?, output_tokens: row.get(14)?, usage_estimated: row.get(15)?,
                    error_message: row.get(16)?, created_at: row.get(17)?, updated_at: row.get(18)?,
                    completed_at: row.get(19)?, is_pinned: row.get(20)?, files: Vec::new(),
                    findings: Vec::new(), ai_calls: Vec::new(),
                })
            },
        )
        .optional()
        .map_err(|error| format!("读取 Review session 失败: {error}"))?
        .ok_or_else(|| "Review session 不存在".to_string())?;
    session.files = load_files(&connection, session_id)?;
    session.findings = load_findings(&connection, session_id)?;
    session.ai_calls = load_ai_calls(&connection, session_id)?;
    Ok(session)
}

pub fn list_sessions(database: &AppDatabase, repo_root: &str, limit: usize) -> Result<Vec<ReviewSessionSummary>, String> {
    let connection = database.connect()?;
    let mut statement = connection
        .prepare(
            "SELECT id, repo_root, status, phase, selected_files_json, overview_json,
             input_tokens, output_tokens, usage_estimated, created_at, updated_at, is_pinned
             FROM review_sessions WHERE repo_root=?1 ORDER BY created_at DESC LIMIT ?2",
        )
        .map_err(|error| format!("准备 Review 历史查询失败: {error}"))?;
    let rows = statement
        .query_map(params![repo_root, limit], |row| {
            let selected: Vec<String> = from_json(row.get::<_, String>(4)?)?;
            Ok(ReviewSessionSummary {
                id: row.get(0)?, repo_root: row.get(1)?, status: row.get(2)?, phase: row.get(3)?,
                selected_file_count: selected.len(), overview: from_json(row.get::<_, String>(5)?)?,
                input_tokens: row.get(6)?, output_tokens: row.get(7)?, usage_estimated: row.get(8)?,
                created_at: row.get(9)?, updated_at: row.get(10)?, is_pinned: row.get(11)?,
            })
        })
        .map_err(|error| format!("查询 Review 历史失败: {error}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("读取 Review 历史失败: {error}"))
}

pub fn update_finding(database: &AppDatabase, finding_id: &str, status: &str, note: Option<&str>) -> Result<(), String> {
    if !matches!(status, "open" | "confirmed" | "ignored" | "fixed") {
        return Err("不支持的 finding 状态".to_string());
    }
    let connection = database.connect()?;
    connection
        .execute(
            "UPDATE review_findings SET status=?2, user_note=?3, updated_at=?4 WHERE id=?1",
            params![finding_id, status, note, now_millis()],
        )
        .map_err(|error| format!("更新 finding 失败: {error}"))?;
    Ok(())
}

pub fn upsert_rule(database: &AppDatabase, rule: &ReviewRule) -> Result<(), String> {
    validate_rule(rule)?;
    let connection = database.connect()?;
    let now = now_millis();
    connection.execute(
        "INSERT INTO review_rules (id,name,description,kind,enabled,severity,category,include_globs_json,
         exclude_globs_json,languages_json,definition_json,version,created_at,updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?13)
         ON CONFLICT(id) DO UPDATE SET name=excluded.name,description=excluded.description,
         kind=excluded.kind,enabled=excluded.enabled,severity=excluded.severity,category=excluded.category,
         include_globs_json=excluded.include_globs_json,exclude_globs_json=excluded.exclude_globs_json,
         languages_json=excluded.languages_json,definition_json=excluded.definition_json,
         version=review_rules.version+1,updated_at=excluded.updated_at",
        params![rule.id,rule.name,rule.description,rule.kind,rule.enabled,rule.severity,rule.category,
            json(&rule.include_globs)?,json(&rule.exclude_globs)?,json(&rule.languages)?,rule.definition.to_string(),rule.version,now]
    ).map_err(|error| format!("保存 Review 规则失败: {error}"))?;
    Ok(())
}

pub fn list_rules(database: &AppDatabase) -> Result<Vec<ReviewRule>, String> {
    let connection = database.connect()?;
    let mut statement = connection.prepare(
        "SELECT id,name,description,kind,enabled,severity,category,include_globs_json,
         exclude_globs_json,languages_json,definition_json,version FROM review_rules ORDER BY name"
    ).map_err(|error| format!("准备 Review 规则查询失败: {error}"))?;
    let rows = statement.query_map([], |row| Ok(ReviewRule {
        id: row.get(0)?, name: row.get(1)?, description: row.get(2)?, kind: row.get(3)?, enabled: row.get(4)?,
        severity: row.get(5)?, category: row.get(6)?, include_globs: from_json(row.get::<_,String>(7)?)?,
        exclude_globs: from_json(row.get::<_,String>(8)?)?, languages: from_json(row.get::<_,String>(9)?)?,
        definition: from_json(row.get::<_,String>(10)?)?, source: "global".to_string(), version: row.get(11)?,
    })).map_err(|error| format!("查询 Review 规则失败: {error}"))?;
    rows.collect::<Result<Vec<_>,_>>().map_err(|error| format!("读取 Review 规则失败: {error}"))
}

pub fn delete_rule(database: &AppDatabase, id: &str) -> Result<bool, String> {
    let connection = database.connect()?;
    connection.execute("DELETE FROM review_rules WHERE id=?1", [id])
        .map(|count| count > 0).map_err(|error| format!("删除 Review 规则失败: {error}"))
}

pub fn mark_running_sessions_interrupted(database: &AppDatabase) -> Result<(), String> {
    let connection = database.connect()?;
    connection.execute(
        "UPDATE review_sessions SET status='interrupted', phase='interrupted', updated_at=?1
         WHERE status='running'", [now_millis()]
    ).map_err(|error| format!("恢复未完成 Review 状态失败: {error}"))?;
    Ok(())
}

pub fn delete_expired_sessions(database: &AppDatabase, cutoff: i64) -> Result<u64, String> {
    let connection = database.connect()?;
    connection.execute(
        "DELETE FROM review_sessions WHERE is_pinned=0 AND expires_at IS NOT NULL AND expires_at < ?1",
        [cutoff]
    ).map(|count| count as u64).map_err(|error| format!("清理过期 Review 失败: {error}"))
}

fn load_files(connection: &rusqlite::Connection, session_id: &str) -> Result<Vec<ReviewFileRecord>, String> {
    let mut statement = connection.prepare(
        "SELECT path,change_kind,attention_score,score_categories_json,score_breakdown_json,
         selected,review_status,batch_id,limitation FROM review_files WHERE session_id=?1 ORDER BY attention_score DESC,path"
    ).map_err(|error| format!("准备 Review 文件查询失败: {error}"))?;
    let rows = statement.query_map([session_id], |row| Ok(ReviewFileRecord {
        path: row.get(0)?, change_kind: row.get(1)?, attention_score: row.get(2)?,
        score_categories: from_json(row.get::<_,String>(3)?)?, score_breakdown: from_json(row.get::<_,String>(4)?)?,
        selected: row.get(5)?, review_status: row.get(6)?, batch_id: row.get(7)?, limitation: row.get(8)?,
    })).map_err(|error| format!("查询 Review 文件失败: {error}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("读取 Review 文件失败: {error}"))
}

fn load_findings(connection: &rusqlite::Connection, session_id: &str) -> Result<Vec<ReviewFinding>, String> {
    let mut statement = connection.prepare(
        "SELECT id,fingerprint,source,rule_id,category,severity,confidence,file_path,start_line,end_line,
         title,problem,impact,trigger_scenario,evidence,suggestion,verified,status,user_note
         FROM review_findings WHERE session_id=?1
         ORDER BY CASE severity WHEN 'critical' THEN 0 WHEN 'major' THEN 1 WHEN 'minor' THEN 2 ELSE 3 END, confidence DESC"
    ).map_err(|error| format!("准备 finding 查询失败: {error}"))?;
    let rows = statement.query_map([session_id], |row| Ok(ReviewFinding {
        id: row.get(0)?, fingerprint: row.get(1)?, source: row.get(2)?, rule_id: row.get(3)?,
        category: row.get(4)?, severity: row.get(5)?, confidence: row.get(6)?, file_path: row.get(7)?,
        start_line: row.get(8)?, end_line: row.get(9)?, title: row.get(10)?, problem: row.get(11)?,
        impact: row.get(12)?, trigger_scenario: row.get(13)?, evidence: row.get(14)?, suggestion: row.get(15)?,
        verified: row.get(16)?, status: row.get(17)?, user_note: row.get(18)?,
    })).map_err(|error| format!("查询 findings 失败: {error}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("读取 Review findings 失败: {error}"))
}

fn load_ai_calls(connection: &rusqlite::Connection, session_id: &str) -> Result<Vec<AiCallUsage>, String> {
    let mut statement = connection.prepare(
        "SELECT batch_id,files_json,input_tokens,output_tokens,usage_estimated,duration_ms,status,error_message
         FROM review_ai_calls WHERE session_id=?1 ORDER BY id"
    ).map_err(|error| format!("准备 AI 明细查询失败: {error}"))?;
    let rows = statement.query_map([session_id], |row| Ok(AiCallUsage {
        batch_id: row.get(0)?, files: from_json(row.get::<_,String>(1)?)?, input_tokens: row.get(2)?,
        output_tokens: row.get(3)?, estimated: row.get(4)?, duration_ms: row.get(5)?, status: row.get(6)?, error: row.get(7)?,
    })).map_err(|error| format!("查询 AI 明细失败: {error}"))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("读取 Review AI 明细失败: {error}"))
}

fn validate_rule(rule: &ReviewRule) -> Result<(), String> {
    if rule.id.trim().is_empty() || rule.name.trim().is_empty() { return Err("规则 id 和名称不能为空".to_string()); }
    if !matches!(rule.kind.as_str(), "deterministic" | "semantic") { return Err("规则类型必须是 deterministic 或 semantic".to_string()); }
    if rule.kind == "semantic" && rule.definition.to_string().chars().count() > 600 { return Err("语义规则过长，请收窄到 300 字符左右".to_string()); }
    Ok(())
}

fn json<T: serde::Serialize + ?Sized>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|error| format!("序列化 Review 数据失败: {error}"))
}

fn from_json<T: serde::de::DeserializeOwned>(value: String) -> rusqlite::Result<T> {
    serde_json::from_str(&value).map_err(|error| rusqlite::Error::FromSqlConversionFailure(
        value.len(), rusqlite::types::Type::Text, Box::new(error)
    ))
}
