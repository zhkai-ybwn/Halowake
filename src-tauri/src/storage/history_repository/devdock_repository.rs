use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::storage::AppDatabase;

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

pub fn list_devdock_projects(database: &AppDatabase) -> Result<Vec<DevDockProjectRecord>, String> {
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
            path: row
                .get(0)
                .map_err(|error| format!("解析 path 失败: {error}"))?,
            name: row
                .get(1)
                .map_err(|error| format!("解析 name 失败: {error}"))?,
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

pub fn remove_devdock_project_record(database: &AppDatabase, path: &str) -> Result<(), String> {
    let connection = database.connect()?;
    connection
        .execute(
            "DELETE FROM devdock_projects WHERE path = ?1 COLLATE NOCASE;",
            params![path],
        )
        .map_err(|error| format!("删除 DevDock 项目记录失败: {error}"))?;
    Ok(())
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
