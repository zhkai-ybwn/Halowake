use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::AppDatabase;

fn current_time_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitHistoryRecord {
    pub id: String,
    pub repo_path: String,
    pub repo_name: String,
    pub title: String,
    pub body: String,
    pub source: String,
    pub selected_file_count: usize,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevDockProjectRecord {
    pub path: String,
    pub name: String,
    pub is_pinned: bool,
    pub sort_order: i64,
    pub created_at: i64,
    pub opened_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevDockRunHistoryRecord {
    pub id: String,
    pub project_path: String,
    pub project_name: String,
    pub command_id: String,
    pub command_name: String,
    pub executor: String,
    pub command_preview: Option<String>,
    pub exit_code: Option<i32>,
    pub status: String,
    pub started_at: i64,
    pub duration_ms: i64,
    pub last_log_line: Option<String>,
    pub expires_at: Option<i64>,
}

pub fn save_git_commit_history_entry(
    database: &AppDatabase,
    entry: &GitCommitHistoryRecord,
) -> Result<(), String> {
    let connection = database.connect()?;
    connection
        .execute(
            "INSERT INTO git_commit_history (
                id, repo_path, repo_name, title, body, source, selected_file_count, created_at, expires_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                body = excluded.body,
                source = excluded.source,
                selected_file_count = excluded.selected_file_count,
                created_at = excluded.created_at,
                expires_at = excluded.expires_at;",
            params![
                entry.id,
                entry.repo_path,
                entry.repo_name,
                entry.title,
                entry.body,
                entry.source,
                entry.selected_file_count as i64,
                entry.created_at,
                entry.expires_at,
            ],
        )
        .map_err(|error| format!("保存 Git 提交历史失败: {error}"))?;
    Ok(())
}

pub fn list_git_commit_history_entries(
    database: &AppDatabase,
    repo_path: Option<&str>,
    limit: usize,
) -> Result<Vec<GitCommitHistoryRecord>, String> {
    let connection = database.connect()?;
    let limit = if limit == 0 { 50 } else { limit.min(200) };

    let mut sql = "SELECT id, repo_path, repo_name, title, body, source, selected_file_count, created_at, expires_at
                   FROM git_commit_history ".to_string();
    if repo_path.is_some() {
        sql.push_str("WHERE repo_path = ?1 COLLATE NOCASE ");
    }
    sql.push_str("ORDER BY created_at DESC LIMIT ?;");

    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| format!("查询 Git 提交历史失败: {error}"))?;

    let mut rows = if let Some(path) = repo_path {
        statement
            .query(params![path, limit as i64])
            .map_err(|error| format!("执行 Git 提交历史查询失败: {error}"))?
    } else {
        statement
            .query(params![limit as i64])
            .map_err(|error| format!("执行 Git 提交历史查询失败: {error}"))?
    };

    let mut entries = Vec::new();
    while let Some(row) = rows
        .next()
        .map_err(|error| format!("读取 Git 提交历史项失败: {error}"))?
    {
        let count_raw: i64 = row.get(6).unwrap_or(0);
        entries.push(GitCommitHistoryRecord {
            id: row.get(0).map_err(|e| format!("解析 id 失败: {e}"))?,
            repo_path: row.get(1).map_err(|e| format!("解析 repo_path 失败: {e}"))?,
            repo_name: row.get(2).map_err(|e| format!("解析 repo_name 失败: {e}"))?,
            title: row.get(3).map_err(|e| format!("解析 title 失败: {e}"))?,
            body: row.get(4).unwrap_or_default(),
            source: row.get(5).unwrap_or_else(|_| "ai".to_string()),
            selected_file_count: count_raw.max(0) as usize,
            created_at: row.get(7).map_err(|e| format!("解析 created_at 失败: {e}"))?,
            expires_at: row.get(8).ok(),
        });
    }
    Ok(entries)
}

pub fn delete_expired_git_commit_history(
    database: &AppDatabase,
    now: i64,
    cutoff: i64,
) -> Result<u64, String> {
    let connection = database.connect()?;
    let affected = connection
        .execute(
            "DELETE FROM git_commit_history
             WHERE (expires_at IS NOT NULL AND expires_at < ?1)
                OR (created_at < ?2);",
            params![now, cutoff],
        )
        .map_err(|error| format!("清理过期 Git 提交历史失败: {error}"))?;
    Ok(affected as u64)
}

pub fn clear_git_commit_history_entries(
    database: &AppDatabase,
    repo_path: Option<&str>,
) -> Result<(), String> {
    let connection = database.connect()?;
    if let Some(path) = repo_path {
        connection
            .execute(
                "DELETE FROM git_commit_history WHERE repo_path = ?1 COLLATE NOCASE;",
                params![path],
            )
            .map_err(|error| format!("清空 Git 提交历史失败: {error}"))?;
    } else {
        connection
            .execute("DELETE FROM git_commit_history;", [])
            .map_err(|error| format!("清空全部 Git 提交历史失败: {error}"))?;
    }
    Ok(())
}

pub fn list_devdock_projects(
    database: &AppDatabase,
) -> Result<Vec<DevDockProjectRecord>, String> {
    let connection = database.connect()?;
    let mut statement = connection
        .prepare(
            "SELECT path, name, is_pinned, sort_order, created_at, opened_at
             FROM devdock_projects
             ORDER BY is_pinned DESC, opened_at DESC;",
        )
        .map_err(|error| format!("查询 DevDock 项目失败: {error}"))?;

    let mut rows = statement
        .query([])
        .map_err(|error| format!("执行 DevDock 项目查询失败: {error}"))?;

    let mut projects = Vec::new();
    while let Some(row) = rows
        .next()
        .map_err(|error| format!("读取 DevDock 项目项失败: {error}"))?
    {
        let is_pinned_raw: i64 = row.get(2).unwrap_or(0);
        projects.push(DevDockProjectRecord {
            path: row.get(0).map_err(|e| format!("解析 path 失败: {e}"))?,
            name: row.get(1).map_err(|e| format!("解析 name 失败: {e}"))?,
            is_pinned: is_pinned_raw != 0,
            sort_order: row.get(3).unwrap_or(0),
            created_at: row.get(4).unwrap_or(0),
            opened_at: row.get(5).unwrap_or(0),
        });
    }
    Ok(projects)
}

pub fn save_devdock_project_record(
    database: &AppDatabase,
    project: &DevDockProjectRecord,
) -> Result<(), String> {
    let connection = database.connect()?;
    connection
        .execute(
            "INSERT INTO devdock_projects (
                path, name, is_pinned, sort_order, created_at, opened_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(path) DO UPDATE SET
                name = excluded.name,
                is_pinned = excluded.is_pinned,
                sort_order = excluded.sort_order,
                opened_at = excluded.opened_at;",
            params![
                project.path,
                project.name,
                if project.is_pinned { 1 } else { 0 },
                project.sort_order,
                project.created_at,
                project.opened_at,
            ],
        )
        .map_err(|error| format!("保存 DevDock 项目记录失败: {error}"))?;
    Ok(())
}

pub fn remove_devdock_project_record(
    database: &AppDatabase,
    path: &str,
) -> Result<(), String> {
    let connection = database.connect()?;
    connection
        .execute(
            "DELETE FROM devdock_projects WHERE path = ?1 COLLATE NOCASE;",
            params![path],
        )
        .map_err(|error| format!("删除 DevDock 项目记录失败: {error}"))?;
    Ok(())
}

pub fn save_devdock_run_history_record(
    database: &AppDatabase,
    record: &DevDockRunHistoryRecord,
) -> Result<(), String> {
    let connection = database.connect()?;
    connection
        .execute(
            "INSERT INTO devdock_run_history (
                id, project_path, project_name, command_id, command_name,
                executor, command_preview, exit_code, status,
                started_at, duration_ms, last_log_line, expires_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            ON CONFLICT(id) DO UPDATE SET
                exit_code = excluded.exit_code,
                status = excluded.status,
                duration_ms = excluded.duration_ms,
                last_log_line = excluded.last_log_line,
                expires_at = excluded.expires_at;",
            params![
                record.id,
                record.project_path,
                record.project_name,
                record.command_id,
                record.command_name,
                record.executor,
                record.command_preview,
                record.exit_code,
                record.status,
                record.started_at,
                record.duration_ms,
                record.last_log_line,
                record.expires_at,
            ],
        )
        .map_err(|error| format!("保存 DevDock 运行历史失败: {error}"))?;
    Ok(())
}

pub fn list_devdock_run_history_records(
    database: &AppDatabase,
    project_path: Option<&str>,
    limit: usize,
) -> Result<Vec<DevDockRunHistoryRecord>, String> {
    let connection = database.connect()?;
    let limit = if limit == 0 { 50 } else { limit.min(200) };

    let mut sql = "SELECT id, project_path, project_name, command_id, command_name,
                          executor, command_preview, exit_code, status,
                          started_at, duration_ms, last_log_line, expires_at
                   FROM devdock_run_history ".to_string();
    if project_path.is_some() {
        sql.push_str("WHERE project_path = ?1 COLLATE NOCASE ");
    }
    sql.push_str("ORDER BY started_at DESC LIMIT ?;");

    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| format!("查询 DevDock 运行历史失败: {error}"))?;

    let mut rows = if let Some(path) = project_path {
        statement
            .query(params![path, limit as i64])
            .map_err(|error| format!("执行 DevDock 运行历史查询失败: {error}"))?
    } else {
        statement
            .query(params![limit as i64])
            .map_err(|error| format!("执行 DevDock 运行历史查询失败: {error}"))?
    };

    let mut records = Vec::new();
    while let Some(row) = rows
        .next()
        .map_err(|error| format!("读取 DevDock 运行历史项失败: {error}"))?
    {
        records.push(DevDockRunHistoryRecord {
            id: row.get(0).map_err(|e| format!("解析 id 失败: {e}"))?,
            project_path: row.get(1).map_err(|e| format!("解析 project_path 失败: {e}"))?,
            project_name: row.get(2).map_err(|e| format!("解析 project_name 失败: {e}"))?,
            command_id: row.get(3).map_err(|e| format!("解析 command_id 失败: {e}"))?,
            command_name: row.get(4).map_err(|e| format!("解析 command_name 失败: {e}"))?,
            executor: row.get(5).unwrap_or_default(),
            command_preview: row.get(6).ok(),
            exit_code: row.get(7).ok(),
            status: row.get(8).unwrap_or_else(|_| "stopped".to_string()),
            started_at: row.get(9).map_err(|e| format!("解析 started_at 失败: {e}"))?,
            duration_ms: row.get(10).unwrap_or(0),
            last_log_line: row.get(11).ok(),
            expires_at: row.get(12).ok(),
        });
    }
    Ok(records)
}

pub fn delete_expired_devdock_run_history(
    database: &AppDatabase,
    now: i64,
    cutoff: i64,
) -> Result<u64, String> {
    let connection = database.connect()?;
    let affected = connection
        .execute(
            "DELETE FROM devdock_run_history
             WHERE (expires_at IS NOT NULL AND expires_at < ?1)
                OR (started_at < ?2);",
            params![now, cutoff],
        )
        .map_err(|error| format!("清理过期 DevDock 运行历史失败: {error}"))?;
    Ok(affected as u64)
}

pub fn clear_devdock_run_history_records(
    database: &AppDatabase,
    project_path: Option<&str>,
) -> Result<(), String> {
    let connection = database.connect()?;
    if let Some(path) = project_path {
        connection
            .execute(
                "DELETE FROM devdock_run_history WHERE project_path = ?1 COLLATE NOCASE;",
                params![path],
            )
            .map_err(|error| format!("清空 DevDock 运行历史失败: {error}"))?;
    } else {
        connection
            .execute("DELETE FROM devdock_run_history;", [])
            .map_err(|error| format!("清空全部 DevDock 运行历史失败: {error}"))?;
    }
    Ok(())
}

pub fn load_ai_settings_from_db(
    database: &AppDatabase,
) -> Result<Option<crate::commands::ai_settings::AiSettings>, String> {
    let connection = database.connect()?;
    let mut statement = connection
        .prepare("SELECT settings_json FROM app_ai_settings WHERE key = 'main' LIMIT 1;")
        .map_err(|error| format!("准备查询 AI 设置失败: {error}"))?;

    let mut rows = statement
        .query([])
        .map_err(|error| format!("查询 AI 设置失败: {error}"))?;

    if let Some(row) = rows.next().map_err(|error| format!("读取 AI 设置行失败: {error}"))? {
        let json_str: String = row.get(0).map_err(|e| format!("解析 settings_json 失败: {e}"))?;
        let settings = serde_json::from_str(&json_str)
            .map_err(|error| format!("反序列化 AI 设置失败: {error}"))?;
        Ok(Some(settings))
    } else {
        Ok(None)
    }
}

pub fn save_ai_settings_to_db(
    database: &AppDatabase,
    settings: &crate::commands::ai_settings::AiSettings,
) -> Result<(), String> {
    let connection = database.connect()?;
    let json_str = serde_json::to_string(settings)
        .map_err(|error| format!("序列化 AI 设置失败: {error}"))?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    connection
        .execute(
            "INSERT INTO app_ai_settings (key, settings_json, updated_at)
             VALUES ('main', ?1, ?2)
             ON CONFLICT(key) DO UPDATE SET
                settings_json = excluded.settings_json,
                updated_at = excluded.updated_at;",
            params![json_str, now],
        )
        .map_err(|error| format!("保存 AI 设置失败: {error}"))?;
    Ok(())
}

pub fn load_quota_accounts_from_db(
    database: &AppDatabase,
) -> Result<Vec<crate::quota::models::AccountConfig>, String> {
    let connection = database.connect()?;
    let mut statement = connection
        .prepare("SELECT account_json FROM app_quota_accounts ORDER BY sort_order ASC, updated_at DESC;")
        .map_err(|error| format!("准备查询 Quota 账号失败: {error}"))?;

    let mut rows = statement
        .query([])
        .map_err(|error| format!("查询 Quota 账号失败: {error}"))?;

    let mut accounts = Vec::new();
    while let Some(row) = rows.next().map_err(|error| format!("读取 Quota 账号行失败: {error}"))? {
        let json_str: String = row.get(0).map_err(|e| format!("解析 account_json 失败: {e}"))?;
        if let Ok(account) = serde_json::from_str(&json_str) {
            accounts.push(account);
        }
    }
    Ok(accounts)
}

pub fn save_quota_accounts_to_db(
    database: &AppDatabase,
    accounts: &[crate::quota::models::AccountConfig],
) -> Result<(), String> {
    let mut connection = database.connect()?;
    let transaction = connection
        .transaction()
        .map_err(|error| format!("开启 Quota 账号事务失败: {error}"))?;

    transaction
        .execute("DELETE FROM app_quota_accounts;", [])
        .map_err(|error| format!("清理旧 Quota 账号失败: {error}"))?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    for (index, account) in accounts.iter().enumerate() {
        let json_str = serde_json::to_string(account)
            .map_err(|error| format!("序列化 Quota 账号失败: {error}"))?;
        transaction
            .execute(
                "INSERT INTO app_quota_accounts (id, account_json, enabled, sort_order, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5);",
                params![
                    account.id,
                    json_str,
                    if account.enabled { 1 } else { 0 },
                    index as i64,
                    now,
                ],
            )
            .map_err(|error| format!("插入 Quota 账号失败: {error}"))?;
    }

    transaction
        .commit()
        .map_err(|error| format!("提交 Quota 账号事务失败: {error}"))?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CodexReportPromptTemplate {
    pub id: String,
    pub name: String,
    pub content: String,
    pub is_builtin: bool,
    pub sort_order: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

pub const DEFAULT_STANDARD_REPORT_PROMPT: &str = r#"请将下方 AI 工具（Codex / Claude Code / Antigravity / OpenCode）工作记录整理为一份**简洁、专业、适合日报及绩效评估的中文工作总结**。

要求：

1. **严格基于事实**
   * 不编造未完成事项、业务收益、效率数据、工作时长或个人贡献。
   * 不根据 AI 会话时间推断实际工作量。

2. **只写“今日完成”**
   * 不生成“待跟进”“明日计划”等无事实依据的内容。
   * 只整理已有明确结果的工作事项。

3. **突出成果，不写过程**
   * 优先表达：**完成了什么、解决了什么、覆盖哪些范围、最终结果如何**。
   * 不罗列文件路径、代码行号、命令执行过程等无关细节。

4. **体现实际工作价值**
   * 在事实支持的前提下，优先体现：
     * 前后端、接口、数据库等跨层修改；
     * 查询、列表、详情、搜索等覆盖范围；
     * 多语言、多状态、多场景适配；
     * 兼容性处理；
     * 测试、校验等明确结果。
   * 不夸大，不使用“显著提升”“重大突破”“极大优化”等无事实依据的表述。

5. **合并跨工具重复事项**
   * 按项目归类。
   * 多个工具相同目标下的修改合并成一条成果。
   * 一个会话包含多个独立成果时可以拆分。

6. **尽可能精简**
   * 每个项目优先控制在 **1～3 条**。
   * 每条尽量控制在 **1～2 句话**。
   * 删除重复描述和实现细节。
   * 保留能体现工作范围、技术处理和交付结果的信息。

## 输出格式

# 工作日报｜YYYY-MM-DD

## 今日完成

### 项目名称

1. **成果标题**：简洁描述完成事项、覆盖范围和结果。
2. **成果标题**：……

## 最终要求

* 内容可直接提交日报；
* 专业、客观、有成果感；
* 在不改变事实的前提下，尽可能准确体现实际工作贡献；
* **宁可少写，不要长篇大论。**"#;

pub const DEFAULT_STANDUP_PROMPT: &str = r#"请根据下方 AI 辅助编程工作记录，整理为一份**敏捷开发每日站会 (Daily Standup)** 发言：

## 格式要求：
- **【昨日/今日完成】**：按项目列出核心产出与解决的问题（1-3条，精简有成果感）。
- **【今日计划】**：根据已完成上下文简要列出合理的下一步跟进项。
- **【风险与阻塞】**：无（或如有未解决报错则简要提炼）。"#;

pub const DEFAULT_TECH_SUMMARY_PROMPT: &str = r#"请将下方工作记录整理为一份**技术攻坚与重构小结**，分模块提炼核心技术点、修改范围、架构/逻辑变动及验证结果。语言风格严谨、技术向、要点清晰。"#;

pub fn default_builtin_report_templates() -> Vec<CodexReportPromptTemplate> {
    let now = current_time_ms();
    vec![
        CodexReportPromptTemplate {
            id: "builtin-standard".to_string(),
            name: "标准日报".to_string(),
            content: DEFAULT_STANDARD_REPORT_PROMPT.to_string(),
            is_builtin: true,
            sort_order: 1,
            created_at: now,
            updated_at: now,
        },
        CodexReportPromptTemplate {
            id: "builtin-standup".to_string(),
            name: "敏捷站会".to_string(),
            content: DEFAULT_STANDUP_PROMPT.to_string(),
            is_builtin: true,
            sort_order: 2,
            created_at: now,
            updated_at: now,
        },
        CodexReportPromptTemplate {
            id: "builtin-tech".to_string(),
            name: "技术攻坚".to_string(),
            content: DEFAULT_TECH_SUMMARY_PROMPT.to_string(),
            is_builtin: true,
            sort_order: 3,
            created_at: now,
            updated_at: now,
        },
    ]
}

pub fn list_codex_report_templates(
    database: &AppDatabase,
) -> Result<Vec<CodexReportPromptTemplate>, String> {
    let connection = database.connect()?;
    let mut stmt = connection
        .prepare(
            "SELECT id, name, content, is_builtin, sort_order, created_at, updated_at
             FROM codex_report_templates
             ORDER BY sort_order ASC, created_at ASC",
        )
        .map_err(|error| format!("查询 Prompt 模板列表失败: {error}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(CodexReportPromptTemplate {
                id: row.get(0)?,
                name: row.get(1)?,
                content: row.get(2)?,
                is_builtin: row.get::<_, i64>(3)? != 0,
                sort_order: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })
        .map_err(|error| format!("遍历 Prompt 模板失败: {error}"))?;

    let mut list = Vec::new();
    for item in rows {
        list.push(item.map_err(|error| format!("解析 Prompt 模板行失败: {error}"))?);
    }

    if list.is_empty() {
        drop(stmt);
        drop(connection);
        let defaults = default_builtin_report_templates();
        for t in &defaults {
            save_codex_report_template(database, t)?;
        }
        return Ok(defaults);
    }

    Ok(list)
}

pub fn save_codex_report_template(
    database: &AppDatabase,
    template: &CodexReportPromptTemplate,
) -> Result<(), String> {
    let connection = database.connect()?;
    let now = current_time_ms();
    let updated_at = if template.updated_at > 0 { template.updated_at } else { now };
    let created_at = if template.created_at > 0 { template.created_at } else { now };

    connection
        .execute(
            "INSERT INTO codex_report_templates (id, name, content, is_builtin, sort_order, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
               name = excluded.name,
               content = excluded.content,
               is_builtin = excluded.is_builtin,
               sort_order = excluded.sort_order,
               updated_at = excluded.updated_at",
            params![
                template.id,
                template.name,
                template.content,
                if template.is_builtin { 1 } else { 0 },
                template.sort_order,
                created_at,
                updated_at,
            ],
        )
        .map_err(|error| format!("保存 Prompt 模板失败: {error}"))?;

    Ok(())
}

pub fn delete_codex_report_template(
    database: &AppDatabase,
    id: &str,
) -> Result<(), String> {
    let connection = database.connect()?;

    // 检查是否为内置模板
    let is_builtin: Option<i64> = connection
        .query_row(
            "SELECT is_builtin FROM codex_report_templates WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("查询 Prompt 模板失败: {error}"))?;

    match is_builtin {
        Some(1) => Err("系统内置模板不允许删除".to_string()),
        Some(_) => {
            connection
                .execute("DELETE FROM codex_report_templates WHERE id = ?1", params![id])
                .map_err(|error| format!("删除 Prompt 模板失败: {error}"))?;
            Ok(())
        }
        None => Ok(()),
    }
}

pub fn reset_builtin_codex_report_templates(
    database: &AppDatabase,
) -> Result<Vec<CodexReportPromptTemplate>, String> {
    let defaults = default_builtin_report_templates();
    for t in &defaults {
        save_codex_report_template(database, t)?;
    }
    list_codex_report_templates(database)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::migrations::run_migrations;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn setup_test_db() -> AppDatabase {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let temp_dir = std::env::temp_dir().join(format!("lumina-history-test-{}", nanos));
        let _ = std::fs::create_dir_all(&temp_dir);
        let db_path = temp_dir.join("test.db");
        let db = AppDatabase::from_path(db_path);
        let mut conn = db.connect().expect("connect");
        run_migrations(&mut conn).expect("migrations");
        db
    }

    #[test]
    fn saves_and_queries_commit_history_and_cleans_expired() {
        let db = setup_test_db();
        let entry = GitCommitHistoryRecord {
            id: "msg-1".to_string(),
            repo_path: "C:/test/repo".to_string(),
            repo_name: "repo".to_string(),
            title: "feat: initial commit".to_string(),
            body: "test description".to_string(),
            source: "ai".to_string(),
            selected_file_count: 2,
            created_at: 1000,
            expires_at: Some(2000),
        };
        save_git_commit_history_entry(&db, &entry).expect("save entry");

        let items = list_git_commit_history_entries(&db, Some("C:/test/repo"), 10).expect("list");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "feat: initial commit");

        let cleaned = delete_expired_git_commit_history(&db, 2500, 2500).expect("clean");
        assert_eq!(cleaned, 1);

        let items_after = list_git_commit_history_entries(&db, None, 10).expect("list after");
        assert_eq!(items_after.len(), 0);

        // 测试即使 expires_at 为 None，但 created_at 早于 cutoff 时也能被清理
        let entry_no_expiry = GitCommitHistoryRecord {
            id: "msg-2".to_string(),
            repo_path: "C:/test/repo".to_string(),
            repo_name: "repo".to_string(),
            title: "fix: test cutoff".to_string(),
            body: "description".to_string(),
            source: "manual".to_string(),
            selected_file_count: 1,
            created_at: 1000,
            expires_at: None,
        };
        save_git_commit_history_entry(&db, &entry_no_expiry).expect("save entry");
        let cleaned_by_cutoff = delete_expired_git_commit_history(&db, 5000, 2000).expect("clean by cutoff");
        assert_eq!(cleaned_by_cutoff, 1);
    }

    #[test]
    fn saves_lists_and_removes_devdock_projects() {
        let db = setup_test_db();
        let project = DevDockProjectRecord {
            path: "C:/projects/app".to_string(),
            name: "App".to_string(),
            is_pinned: true,
            sort_order: 1,
            created_at: 100,
            opened_at: 200,
        };
        save_devdock_project_record(&db, &project).expect("save project");

        let projects = list_devdock_projects(&db).expect("list projects");
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "App");
        assert!(projects[0].is_pinned);

        remove_devdock_project_record(&db, "C:/projects/app").expect("remove project");
        let projects_empty = list_devdock_projects(&db).expect("list empty");
        assert_eq!(projects_empty.len(), 0);
    }

    #[test]
    fn saves_queries_and_cleans_devdock_run_history() {
        let db = setup_test_db();
        let record = DevDockRunHistoryRecord {
            id: "run-1".to_string(),
            project_path: "C:/projects/app".to_string(),
            project_name: "App".to_string(),
            command_id: "build".to_string(),
            command_name: "Build App".to_string(),
            executor: "npm".to_string(),
            command_preview: Some("npm run build".to_string()),
            exit_code: Some(0),
            status: "succeeded".to_string(),
            started_at: 1000,
            duration_ms: 5200,
            last_log_line: Some("Build succeeded".to_string()),
            expires_at: Some(3000),
        };
        save_devdock_run_history_record(&db, &record).expect("save run history");

        let runs = list_devdock_run_history_records(&db, Some("C:/projects/app"), 10).expect("list");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].command_name, "Build App");
        assert_eq!(runs[0].status, "succeeded");

        let cleaned = delete_expired_devdock_run_history(&db, 4000, 4000).expect("clean");
        assert_eq!(cleaned, 1);

        let runs_after = list_devdock_run_history_records(&db, None, 10).expect("list after clean");
        assert_eq!(runs_after.len(), 0);

        // 测试即使 expires_at 为 None，但 started_at 早于 cutoff 时也能被清理
        let record_no_expiry = DevDockRunHistoryRecord {
            id: "run-2".to_string(),
            project_path: "C:/projects/app".to_string(),
            project_name: "App".to_string(),
            command_id: "test".to_string(),
            command_name: "Test App".to_string(),
            executor: "cargo".to_string(),
            command_preview: Some("cargo test".to_string()),
            exit_code: Some(0),
            status: "succeeded".to_string(),
            started_at: 1000,
            duration_ms: 200,
            last_log_line: None,
            expires_at: None,
        };
        save_devdock_run_history_record(&db, &record_no_expiry).expect("save run");
        let cleaned_run_by_cutoff = delete_expired_devdock_run_history(&db, 5000, 2000).expect("clean run by cutoff");
        assert_eq!(cleaned_run_by_cutoff, 1);
    }

    #[test]
    fn saves_and_loads_ai_settings() {
        use crate::commands::ai_settings::{AiModelConfig, AiProviderType, AiSettings};
        use std::collections::HashMap;

        let db = setup_test_db();
        let settings = AiSettings {
            default_model_id: "m-1".to_string(),
            models: vec![AiModelConfig {
                id: "m-1".to_string(),
                name: "GPT-4o".to_string(),
                provider: AiProviderType::OpenaiCompatible,
                base_url: "https://api.openai.com/v1".to_string(),
                api_key: Some("sk-test".to_string()),
                model: "gpt-4o".to_string(),
                enabled: true,
            }],
            task_model_map: HashMap::new(),
        };

        save_ai_settings_to_db(&db, &settings).expect("save ai settings");
        let loaded = load_ai_settings_from_db(&db).expect("load ai settings");
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.default_model_id, "m-1");
        assert_eq!(loaded.models.len(), 1);
        assert_eq!(loaded.models[0].name, "GPT-4o");
    }

    #[test]
    fn saves_and_loads_quota_accounts() {
        use crate::quota::models::{AccountConfig, ProviderType};

        let db = setup_test_db();
        let accounts = vec![
            AccountConfig {
                id: "acc-1".to_string(),
                provider_type: ProviderType::Deepseek,
                name: "DeepSeek Primary".to_string(),
                api_key: Some("sk-ds".to_string()),
                base_url: None,
                enabled: true,
                auto_discovered: false,
            }
        ];

        save_quota_accounts_to_db(&db, &accounts).expect("save quota accounts");
        let loaded = load_quota_accounts_from_db(&db).expect("load quota accounts");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "DeepSeek Primary");
    }

    #[test]
    fn manages_codex_report_templates() {
        let db = setup_test_db();
        let templates = list_codex_report_templates(&db).expect("list templates");
        assert_eq!(templates.len(), 3);
        assert_eq!(templates[0].id, "builtin-standard");

        let custom = CodexReportPromptTemplate {
            id: "custom-1".to_string(),
            name: "周报总结".to_string(),
            content: "周报 Prompt".to_string(),
            is_builtin: false,
            sort_order: 10,
            created_at: 1000,
            updated_at: 1000,
        };
        save_codex_report_template(&db, &custom).expect("save custom");

        let after_save = list_codex_report_templates(&db).expect("list after save");
        assert_eq!(after_save.len(), 4);

        // 内置模板不允许删除
        let del_builtin = delete_codex_report_template(&db, "builtin-standard");
        assert!(del_builtin.is_err());

        // 自定义模板可以删除
        delete_codex_report_template(&db, "custom-1").expect("delete custom");
        let after_del = list_codex_report_templates(&db).expect("list after del");
        assert_eq!(after_del.len(), 3);

        // 重置内置模板
        let reset = reset_builtin_codex_report_templates(&db).expect("reset");
        assert_eq!(reset.len(), 3);
    }
}

