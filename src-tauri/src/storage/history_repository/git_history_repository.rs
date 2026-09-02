use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::storage::AppDatabase;

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
            id: row
                .get(0)
                .map_err(|error| format!("解析 id 失败: {error}"))?,
            repo_path: row
                .get(1)
                .map_err(|error| format!("解析 repo_path 失败: {error}"))?,
            repo_name: row
                .get(2)
                .map_err(|error| format!("解析 repo_name 失败: {error}"))?,
            title: row
                .get(3)
                .map_err(|error| format!("解析 title 失败: {error}"))?,
            body: row.get(4).unwrap_or_default(),
            source: row.get(5).unwrap_or_else(|_| "ai".to_string()),
            selected_file_count: count_raw.max(0) as usize,
            created_at: row
                .get(7)
                .map_err(|error| format!("解析 created_at 失败: {error}"))?,
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
