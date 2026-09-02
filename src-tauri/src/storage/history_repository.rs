#[cfg(test)]
use super::AppDatabase;

mod ai_settings_repository;
mod codex_report_template_repository;
mod devdock_repository;
mod git_history_repository;
mod quota_repository;

pub use ai_settings_repository::{load_ai_settings_from_db, save_ai_settings_to_db};
pub use codex_report_template_repository::{
    default_builtin_report_templates, delete_codex_report_template, list_codex_report_templates,
    reset_builtin_codex_report_templates, save_codex_report_template,
    CodexReportPromptTemplate, DEFAULT_STANDARD_REPORT_PROMPT, DEFAULT_STANDUP_PROMPT,
    DEFAULT_TECH_SUMMARY_PROMPT,
};

pub use devdock_repository::{
    clear_devdock_run_history_records, delete_expired_devdock_run_history,
    list_devdock_projects, list_devdock_run_history_records, remove_devdock_project_record,
    save_devdock_project_record, save_devdock_run_history_record, DevDockProjectRecord,
    DevDockRunHistoryRecord,
};
pub use git_history_repository::{
    clear_git_commit_history_entries, delete_expired_git_commit_history,
    list_git_commit_history_entries, save_git_commit_history_entry, GitCommitHistoryRecord,
};
pub use quota_repository::{load_quota_accounts_from_db, save_quota_accounts_to_db};

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
        let temp_dir = std::env::temp_dir().join(format!("halowake-history-test-{}", nanos));
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

