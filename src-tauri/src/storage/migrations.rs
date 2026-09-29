use rusqlite::{Connection, Transaction};

const MIGRATIONS: &[&str] = &[
    r#"
    CREATE TABLE IF NOT EXISTS review_sessions (
      id TEXT PRIMARY KEY,
      repo_root TEXT NOT NULL,
      diff_fingerprint TEXT NOT NULL,
      status TEXT NOT NULL,
      phase TEXT NOT NULL,
      progress_done INTEGER NOT NULL DEFAULT 0,
      progress_total INTEGER NOT NULL DEFAULT 0,
      current_file TEXT,
      budget_mode TEXT NOT NULL,
      model_id TEXT NOT NULL,
      rule_snapshot_json TEXT NOT NULL DEFAULT '[]',
      selected_files_json TEXT NOT NULL,
      overview_json TEXT NOT NULL DEFAULT '{}',
      limitations_json TEXT NOT NULL DEFAULT '[]',
      input_tokens INTEGER NOT NULL DEFAULT 0,
      output_tokens INTEGER NOT NULL DEFAULT 0,
      usage_estimated INTEGER NOT NULL DEFAULT 1,
      error_message TEXT,
      created_at INTEGER NOT NULL,
      updated_at INTEGER NOT NULL,
      completed_at INTEGER,
      expires_at INTEGER,
      is_pinned INTEGER NOT NULL DEFAULT 0
    );

    CREATE TABLE IF NOT EXISTS review_files (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      session_id TEXT NOT NULL REFERENCES review_sessions(id) ON DELETE CASCADE,
      path TEXT NOT NULL,
      change_kind TEXT NOT NULL DEFAULT 'modified',
      attention_score INTEGER NOT NULL DEFAULT 0,
      score_categories_json TEXT NOT NULL DEFAULT '[]',
      score_breakdown_json TEXT NOT NULL DEFAULT '[]',
      selected INTEGER NOT NULL DEFAULT 1,
      review_status TEXT NOT NULL DEFAULT 'pending',
      batch_id TEXT,
      limitation TEXT,
      UNIQUE(session_id, path)
    );

    CREATE TABLE IF NOT EXISTS review_findings (
      id TEXT PRIMARY KEY,
      session_id TEXT NOT NULL REFERENCES review_sessions(id) ON DELETE CASCADE,
      file_path TEXT NOT NULL,
      fingerprint TEXT NOT NULL,
      source TEXT NOT NULL,
      rule_id TEXT,
      category TEXT NOT NULL,
      severity TEXT NOT NULL,
      confidence REAL NOT NULL,
      start_line INTEGER NOT NULL,
      end_line INTEGER NOT NULL,
      title TEXT NOT NULL,
      problem TEXT NOT NULL,
      impact TEXT NOT NULL,
      trigger_scenario TEXT NOT NULL,
      evidence TEXT NOT NULL,
      suggestion TEXT,
      verified INTEGER NOT NULL DEFAULT 0,
      status TEXT NOT NULL DEFAULT 'open',
      user_note TEXT,
      created_at INTEGER NOT NULL,
      updated_at INTEGER NOT NULL,
      UNIQUE(session_id, fingerprint)
    );

    CREATE TABLE IF NOT EXISTS review_rules (
      id TEXT PRIMARY KEY,
      name TEXT NOT NULL,
      description TEXT,
      kind TEXT NOT NULL,
      enabled INTEGER NOT NULL DEFAULT 1,
      severity TEXT NOT NULL,
      category TEXT NOT NULL,
      include_globs_json TEXT NOT NULL DEFAULT '[]',
      exclude_globs_json TEXT NOT NULL DEFAULT '[]',
      languages_json TEXT NOT NULL DEFAULT '[]',
      definition_json TEXT NOT NULL,
      version INTEGER NOT NULL DEFAULT 1,
      created_at INTEGER NOT NULL,
      updated_at INTEGER NOT NULL
    );

    CREATE TABLE IF NOT EXISTS review_ai_calls (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      session_id TEXT NOT NULL REFERENCES review_sessions(id) ON DELETE CASCADE,
      batch_id TEXT NOT NULL,
      model_id TEXT NOT NULL,
      files_json TEXT NOT NULL,
      input_tokens INTEGER NOT NULL DEFAULT 0,
      output_tokens INTEGER NOT NULL DEFAULT 0,
      usage_estimated INTEGER NOT NULL DEFAULT 1,
      duration_ms INTEGER NOT NULL DEFAULT 0,
      status TEXT NOT NULL,
      error_message TEXT,
      created_at INTEGER NOT NULL
    );

    CREATE INDEX IF NOT EXISTS idx_review_sessions_repo_created
      ON review_sessions(repo_root, created_at DESC);
    CREATE INDEX IF NOT EXISTS idx_review_sessions_status_updated
      ON review_sessions(status, updated_at DESC);
    CREATE INDEX IF NOT EXISTS idx_review_sessions_expiry
      ON review_sessions(expires_at, is_pinned);
    CREATE INDEX IF NOT EXISTS idx_review_findings_session_severity
      ON review_findings(session_id, severity, confidence DESC);
    CREATE INDEX IF NOT EXISTS idx_review_findings_session_status
      ON review_findings(session_id, status);
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS git_commit_history (
      id TEXT PRIMARY KEY,
      repo_path TEXT NOT NULL,
      repo_name TEXT NOT NULL,
      title TEXT NOT NULL,
      body TEXT NOT NULL DEFAULT '',
      source TEXT NOT NULL DEFAULT 'ai',
      selected_file_count INTEGER NOT NULL DEFAULT 0,
      created_at INTEGER NOT NULL,
      expires_at INTEGER
    );

    CREATE TABLE IF NOT EXISTS devdock_projects (
      path TEXT PRIMARY KEY,
      name TEXT NOT NULL,
      is_pinned INTEGER NOT NULL DEFAULT 0,
      sort_order INTEGER NOT NULL DEFAULT 0,
      created_at INTEGER NOT NULL,
      opened_at INTEGER NOT NULL
    );

    CREATE INDEX IF NOT EXISTS idx_git_commit_history_repo
      ON git_commit_history(repo_path, created_at DESC);
    CREATE INDEX IF NOT EXISTS idx_git_commit_history_expiry
      ON git_commit_history(expires_at);
    CREATE INDEX IF NOT EXISTS idx_devdock_projects_opened
      ON devdock_projects(opened_at DESC);
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS devdock_run_history (
      id TEXT PRIMARY KEY,
      project_path TEXT NOT NULL,
      project_name TEXT NOT NULL,
      command_id TEXT NOT NULL,
      command_name TEXT NOT NULL,
      executor TEXT NOT NULL DEFAULT '',
      command_preview TEXT,
      exit_code INTEGER,
      status TEXT NOT NULL,
      started_at INTEGER NOT NULL,
      duration_ms INTEGER NOT NULL DEFAULT 0,
      last_log_line TEXT,
      expires_at INTEGER
    );

    CREATE INDEX IF NOT EXISTS idx_devdock_run_history_proj_started
      ON devdock_run_history(project_path, started_at DESC);
    CREATE INDEX IF NOT EXISTS idx_devdock_run_history_started
      ON devdock_run_history(started_at DESC);
    CREATE INDEX IF NOT EXISTS idx_devdock_run_history_expiry
      ON devdock_run_history(expires_at);
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS app_ai_settings (
      key TEXT PRIMARY KEY,
      settings_json TEXT NOT NULL,
      updated_at INTEGER NOT NULL
    );

    CREATE TABLE IF NOT EXISTS app_quota_accounts (
      id TEXT PRIMARY KEY,
      account_json TEXT NOT NULL,
      enabled INTEGER NOT NULL DEFAULT 1,
      sort_order INTEGER NOT NULL DEFAULT 0,
      updated_at INTEGER NOT NULL
    );
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS codex_report_templates (
      id TEXT PRIMARY KEY,
      name TEXT NOT NULL,
      content TEXT NOT NULL,
      is_builtin INTEGER NOT NULL DEFAULT 0,
      sort_order INTEGER NOT NULL DEFAULT 0,
      created_at INTEGER NOT NULL,
      updated_at INTEGER NOT NULL
    );

    CREATE INDEX IF NOT EXISTS idx_codex_report_templates_sort
      ON codex_report_templates(sort_order ASC, updated_at DESC);
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS app_metadata (
      key TEXT PRIMARY KEY,
      value TEXT NOT NULL,
      updated_at INTEGER NOT NULL
    );
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS skill_registry_snapshot (
      id INTEGER PRIMARY KEY CHECK (id = 1),
      version TEXT NOT NULL,
      updated_at INTEGER NOT NULL,
      description TEXT,
      pipelines_json TEXT NOT NULL DEFAULT '[]'
    );
    CREATE TABLE IF NOT EXISTS skill_registry_entries (
      id TEXT PRIMARY KEY,
      name TEXT NOT NULL,
      description TEXT NOT NULL,
      stage TEXT NOT NULL,
      payload_json TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_skill_registry_stage_name
      ON skill_registry_entries(stage, name);
    "#,
    r#"
    CREATE TABLE IF NOT EXISTS skill_install_history (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      skill_id TEXT NOT NULL,
      display_name TEXT NOT NULL,
      sub_skill_id TEXT,
      git_url TEXT NOT NULL,
      sub_path TEXT,
      target_mode TEXT NOT NULL,
      target_agents_json TEXT NOT NULL DEFAULT '[]',
      project_cwd TEXT,
      status TEXT NOT NULL,
      message TEXT NOT NULL,
      installed_paths_json TEXT NOT NULL DEFAULT '[]',
      created_at INTEGER NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_skill_install_history_created
      ON skill_install_history(created_at DESC, id DESC);
    "#,
    r#"
    UPDATE skill_install_history
      SET display_name = COALESCE(NULLIF(sub_skill_id, ''), skill_id)
      WHERE display_name = '';
    "#,
];

pub fn run_migrations(connection: &mut Connection) -> Result<(), String> {
    let current = connection
        .query_row("PRAGMA user_version", [], |row| row.get::<_, usize>(0))
        .map_err(|error| format!("读取 SQLite schema 版本失败: {error}"))?;

    for (index, sql) in MIGRATIONS.iter().enumerate().skip(current) {
        let transaction = connection
            .transaction()
            .map_err(|error| format!("开启 SQLite migration 事务失败: {error}"))?;
        apply_migration(&transaction, index + 1, sql)?;
        transaction
            .commit()
            .map_err(|error| format!("提交 SQLite migration 失败: {error}"))?;
    }
    Ok(())
}

fn apply_migration(transaction: &Transaction<'_>, version: usize, sql: &str) -> Result<(), String> {
    if version == 9 {
        let mut statement = transaction
            .prepare("PRAGMA table_info(skill_install_history)")
            .map_err(|error| format!("检查技能安装记录字段失败: {error}"))?;
        let columns = statement
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|error| format!("读取技能安装记录字段失败: {error}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("读取技能安装记录字段失败: {error}"))?;
        if !columns.iter().any(|column| column == "display_name") {
            transaction
                .execute_batch("ALTER TABLE skill_install_history ADD COLUMN display_name TEXT NOT NULL DEFAULT '';")
                .map_err(|error| format!("补齐技能安装记录名称字段失败: {error}"))?;
        }
    }
    transaction
        .execute_batch(sql)
        .map_err(|error| format!("执行 SQLite migration v{version} 失败: {error}"))?;
    transaction
        .pragma_update(None, "user_version", version)
        .map_err(|error| format!("更新 SQLite schema 版本失败: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_creates_review_and_history_schema() {
        let mut connection = Connection::open_in_memory().expect("memory database");
        run_migrations(&mut connection).expect("migration");
        let review_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name LIKE 'review_%'",
                [],
                |row| row.get(0),
            )
            .expect("review table count");
        assert_eq!(review_count, 5);

        let history_exists: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('git_commit_history', 'devdock_projects', 'devdock_run_history', 'app_ai_settings', 'app_quota_accounts', 'codex_report_templates')",
                [],
                |row| row.get(0),
            )
            .expect("history table count");
        assert_eq!(history_exists, 6);

        let metadata_exists: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'app_metadata'",
                [],
                |row| row.get(0),
            )
            .expect("metadata table count");
        assert_eq!(metadata_exists, 1);

        let display_name_exists = connection
            .prepare("PRAGMA table_info(skill_install_history)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .any(|column| column.unwrap() == "display_name");
        assert!(display_name_exists);
    }

    #[test]
    fn migration_repairs_existing_install_history_without_losing_rows() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(r#"
            CREATE TABLE skill_install_history (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              skill_id TEXT NOT NULL,
              sub_skill_id TEXT,
              git_url TEXT NOT NULL,
              sub_path TEXT,
              target_mode TEXT NOT NULL,
              target_agents_json TEXT NOT NULL DEFAULT '[]',
              project_cwd TEXT,
              status TEXT NOT NULL,
              message TEXT NOT NULL,
              installed_paths_json TEXT NOT NULL DEFAULT '[]',
              created_at INTEGER NOT NULL
            );
            INSERT INTO skill_install_history (skill_id, sub_skill_id, git_url, target_mode, status, message, created_at)
              VALUES ('catalog-native', 'native-ui', 'https://example.com/native', 'specific_agent', 'success', '已安装', 123);
            PRAGMA user_version = 8;
        "#).unwrap();

        run_migrations(&mut connection).unwrap();
        let (name, source): (String, String) = connection.query_row(
            "SELECT display_name, git_url FROM skill_install_history WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).unwrap();
        assert_eq!(name, "native-ui");
        assert_eq!(source, "https://example.com/native");
        run_migrations(&mut connection).unwrap();
        assert_eq!(connection.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0)).unwrap(), 9);
    }
}
