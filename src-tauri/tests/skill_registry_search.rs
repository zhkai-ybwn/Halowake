//! Deterministic Production Skill Registry Search Regression Suite (T07)
//!
//! Validates:
//! 1. Exact name matching & exact tier
//! 2. Chinese and mixed CJK/English domain terms parsing & ranking
//! 3. Multi-constraint query (stage + category)
//! 4. Generalization fallback notice when specific tech is not found
//! 5. Negative examples are rejected without noise
//! 6. Filter before truncation recalls category skills located past the 500th global item (F06)
//! 7. Stable transitive sorting (name, popular, relevance) with tiebreaker
//! 8. Pagination contracts: total_matched, has_more, is_truncated

use std::{
    fs,
    path::PathBuf,
    process,
    time::{SystemTime, UNIX_EPOCH},
};

use git_assistant_lib::storage::{
    migrations::run_migrations,
    skill_registry_repository::{
        get_skill_registry_revision, get_skill_registry_revision_from_conn,
        query_skill_registry_candidates_with_conn, save_skill_registry,
        search_skill_registry, SkillRegistryFilter, SkillSearchRequest,
    },
    AppDatabase,
};
use serde_json::Value;

struct IsolatedTestDb {
    path: PathBuf,
    database: AppDatabase,
}

impl IsolatedTestDb {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "lumina-test-reg-search-{}-{}-{}.db",
            label,
            process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let database = AppDatabase::from_path(path.clone());
        let mut conn = database.connect().expect("Failed to open test database");
        run_migrations(&mut conn).expect("Failed to run migrations");
        drop(conn);

        let fixture_str = include_str!("fixtures/skill_registry.json");
        let fixture_val: Value =
            serde_json::from_str(fixture_str).expect("Failed to parse fixture JSON");
        save_skill_registry(&database, &fixture_val).expect("Failed to seed fixture data");

        Self { path, database }
    }
}

impl Drop for IsolatedTestDb {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
        let _ = fs::remove_file(format!("{}-wal", self.path.display()));
        let _ = fs::remove_file(format!("{}-shm", self.path.display()));
    }
}

#[test]
fn test_exact_name_matching_yields_exact_tier() {
    let db = IsolatedTestDb::new("exact-name");
    let req = SkillSearchRequest {
        query: "antigravity-guide".to_string(),
        ..Default::default()
    };
    let res = search_skill_registry(&db.database, &req).expect("Search failed");

    assert!(!res.hits.is_empty(), "Should return hits for exact name");
    assert_eq!(res.hits[0].id, "antigravity-guide");
    assert_eq!(res.match_tier, "exact");
    assert!(res.fallback_notice.is_none());
    assert!(res.hits[0].match_reason.matched_fields.contains(&"name".to_string()));
}

#[test]
fn test_cjk_domain_terms_and_synonyms() {
    let db = IsolatedTestDb::new("cjk-domain");

    // 1. 国际化 (i18n) -> angular-i18n-master
    let req_i18n = SkillSearchRequest {
        query: "国际化".to_string(),
        ..Default::default()
    };
    let res_i18n = search_skill_registry(&db.database, &req_i18n).expect("Search failed");
    assert!(!res_i18n.hits.is_empty());
    assert_eq!(res_i18n.hits[0].id, "angular-i18n-master");

    // 2. 代码审查 -> code-review-assistant
    let req_review = SkillSearchRequest {
        query: "代码审查".to_string(),
        ..Default::default()
    };
    let res_review = search_skill_registry(&db.database, &req_review).expect("Search failed");
    assert!(!res_review.hits.is_empty());
    assert_eq!(res_review.hits[0].id, "code-review-assistant");

    // 3. 内存泄漏排查 -> memory-leak-hunter
    let req_mem = SkillSearchRequest {
        query: "内存泄漏".to_string(),
        ..Default::default()
    };
    let res_mem = search_skill_registry(&db.database, &req_mem).expect("Search failed");
    assert!(!res_mem.hits.is_empty());
    assert_eq!(res_mem.hits[0].id, "memory-leak-hunter");

    // 4. SQL 慢查询 -> slow-query-optimizer
    let req_sql = SkillSearchRequest {
        query: "慢查询".to_string(),
        ..Default::default()
    };
    let res_sql = search_skill_registry(&db.database, &req_sql).expect("Search failed");
    assert!(!res_sql.hits.is_empty());
    assert_eq!(res_sql.hits[0].id, "slow-query-optimizer");
}

#[test]
fn test_mixed_cjk_and_english_queries() {
    let db = IsolatedTestDb::new("mixed-cjk-en");

    // Vue3组件视觉 -> vue-visual-diff
    let req_vue = SkillSearchRequest {
        query: "Vue3组件视觉".to_string(),
        ..Default::default()
    };
    let res_vue = search_skill_registry(&db.database, &req_vue).expect("Search failed");
    assert!(!res_vue.hits.is_empty());
    assert_eq!(res_vue.hits[0].id, "vue-visual-diff");

    // React单元测试 -> react-testing-library-helper
    let req_react = SkillSearchRequest {
        query: "React单元测试".to_string(),
        ..Default::default()
    };
    let res_react = search_skill_registry(&db.database, &req_react).expect("Search failed");
    assert!(!res_react.hits.is_empty());
    assert_eq!(res_react.hits[0].id, "react-testing-library-helper");
}

#[test]
fn test_multi_constraint_stage_and_category_filtering() {
    let db = IsolatedTestDb::new("multi-constraint");

    let req = SkillSearchRequest {
        query: "".to_string(),
        stage: Some("testing".to_string()),
        category: Some("frontend-ui".to_string()),
        ..Default::default()
    };
    let res = search_skill_registry(&db.database, &req).expect("Search failed");

    assert_eq!(res.total_matched, 1);
    assert_eq!(res.hits[0].id, "vue-visual-diff");
    assert_eq!(res.hits[0].skill["stage"], "testing");
    assert_eq!(res.hits[0].skill["category"], "frontend-ui");
}

#[test]
fn test_general_fallback_notice_when_specific_tech_stack_missing() {
    let db = IsolatedTestDb::new("fallback-notice");

    // Query 'Rust 国际化': catalog has 'Angular 国际化' but no 'Rust 国际化'
    let req = SkillSearchRequest {
        query: "Rust 国际化".to_string(),
        ..Default::default()
    };
    let res = search_skill_registry(&db.database, &req).expect("Search failed");

    assert!(!res.hits.is_empty(), "Should return fallback matches");
    assert_eq!(res.match_tier, "general_fallback");
    assert!(res.fallback_notice.is_some());
    let notice = res.fallback_notice.unwrap();
    assert!(
        notice.contains("通用可用") || notice.contains("最相关"),
        "Notice must inform user about fallback: {notice}"
    );
}

#[test]
fn test_negative_example_does_not_misrecall() {
    let db = IsolatedTestDb::new("negative-example");

    let req = SkillSearchRequest {
        query: "量子芯片光刻".to_string(),
        ..Default::default()
    };
    let res = search_skill_registry(&db.database, &req).expect("Search failed");

    assert_eq!(res.hits.len(), 0);
    assert_eq!(res.total_matched, 0);
    assert_eq!(res.match_tier, "none");
}

#[test]
fn test_filter_before_truncation_recalls_past_rank_500_item() {
    let db = IsolatedTestDb::new("filter-past-500");

    // 1. Global browse with limit 500: total_matched is 530, hits length is 500
    let browse_req = SkillSearchRequest {
        query: "".to_string(),
        limit: Some(500),
        ..Default::default()
    };
    let browse_res = search_skill_registry(&db.database, &browse_req).expect("Browse failed");
    assert_eq!(browse_res.total_matched, 530);
    assert_eq!(browse_res.hits.len(), 500);

    // Verify deep-arch-catalog-item is NOT in the first 500 hits of global browse
    let in_top_500 = browse_res.hits.iter().any(|h| h.id == "deep-arch-catalog-item");
    assert!(!in_top_500, "deep-arch-catalog-item must be past index 500");

    // 2. Now filter specifically by category 'deep-architecture'
    let cat_req = SkillSearchRequest {
        query: "".to_string(),
        category: Some("deep-architecture".to_string()),
        limit: Some(500),
        ..Default::default()
    };
    let cat_res = search_skill_registry(&db.database, &cat_req).expect("Category search failed");
    assert_eq!(cat_res.total_matched, 1, "Category filter must recall matching item");
    assert_eq!(cat_res.hits.len(), 1);
    assert_eq!(cat_res.hits[0].id, "deep-arch-catalog-item");
}

#[test]
fn test_stable_transitive_sorting_and_pagination() {
    let db = IsolatedTestDb::new("sorting-pagination");

    // 1. Sort by name: strictly alphabetical A-Z
    let name_req = SkillSearchRequest {
        query: "".to_string(),
        sort: Some("name".to_string()),
        limit: Some(50),
        ..Default::default()
    };
    let name_res = search_skill_registry(&db.database, &name_req).expect("Search failed");
    for window in name_res.hits.windows(2) {
        assert!(
            window[0].name <= window[1].name,
            "Name ordering must be monotonic: {} vs {}",
            window[0].name,
            window[1].name
        );
    }

    // 2. Sort by popular: descending stars
    let pop_req = SkillSearchRequest {
        query: "".to_string(),
        sort: Some("popular".to_string()),
        limit: Some(50),
        ..Default::default()
    };
    let pop_res = search_skill_registry(&db.database, &pop_req).expect("Search failed");
    for window in pop_res.hits.windows(2) {
        let stars_a = window[0]
            .skill
            .get("githubStars")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let stars_b = window[1]
            .skill
            .get("githubStars")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        assert!(
            stars_a >= stars_b,
            "Popular ordering must be descending: {} vs {}",
            stars_a,
            stars_b
        );
    }

    // 3. Pagination: page 1 vs page 2 disjoint and consistent
    let page1_req = SkillSearchRequest {
        query: "".to_string(),
        category: Some("general-utilities".to_string()),
        offset: Some(0),
        limit: Some(20),
        sort: Some("name".to_string()),
        ..Default::default()
    };
    let page1_res = search_skill_registry(&db.database, &page1_req).expect("Search failed");
    assert_eq!(page1_res.hits.len(), 20);
    assert_eq!(page1_res.total_matched, 520);
    assert!(page1_res.has_more);

    let page2_req = SkillSearchRequest {
        query: "".to_string(),
        category: Some("general-utilities".to_string()),
        offset: Some(20),
        limit: Some(20),
        sort: Some("name".to_string()),
        ..Default::default()
    };
    let page2_res = search_skill_registry(&db.database, &page2_req).expect("Search failed");
    assert_eq!(page2_res.hits.len(), 20);
    assert_eq!(page2_res.total_matched, 520);
    assert!(page2_res.has_more);

    // Verify page 1 and page 2 are disjoint
    let page1_ids: std::collections::HashSet<_> =
        page1_res.hits.iter().map(|h| h.id.clone()).collect();
    for hit in &page2_res.hits {
        assert!(
            !page1_ids.contains(&hit.id),
            "Page 2 must not duplicate Page 1 hit: {}",
            hit.id
        );
    }

    // 4. Last page has_more is false
    let last_page_req = SkillSearchRequest {
        query: "".to_string(),
        category: Some("general-utilities".to_string()),
        offset: Some(515),
        limit: Some(20),
        ..Default::default()
    };
    let last_res = search_skill_registry(&db.database, &last_page_req).expect("Search failed");
    assert_eq!(last_res.hits.len(), 5);
    assert_eq!(last_res.total_matched, 520);
    assert!(!last_res.has_more);
}

#[test]
fn test_batch_b_review_candidate_window_probe() {
    let db = IsolatedTestDb::new("probe-window");
    for (offset, limit) in [(0, 20), (480, 20), (500, 20), (0, 1000)] {
        let res = search_skill_registry(
            &db.database,
            &SkillSearchRequest {
                query: "utility".into(),
                sort: Some("popular".into()),
                offset: Some(offset),
                limit: Some(limit),
                catalog_revision: Some("invented-version".into()),
                ..Default::default()
            },
        )
        .expect("Search failed");

        assert_eq!(
            res.total_matched, 520,
            "Total matched must be stable at 520 across offsets (offset={offset})"
        );
        assert!(
            !res.is_truncated,
            "520 items is within 10,000 candidate window, so is_truncated must be false"
        );
        assert_eq!(
            res.catalog_revision,
            Some("1.4.1:1729000000000".to_string()),
            "Catalog revision must return actual database snapshot revision (version:updatedAt), not client's invented-version"
        );
        if offset == 0 {
            assert_eq!(
                res.hits.first().map(|h| h.id.as_str()),
                Some("dummy-skill-520"),
                "First item by popular sort at offset 0 must be dummy-skill-520"
            );
        }
        if offset == 0 && limit == 20 {
            assert!(res.has_more);
        } else if offset == 500 && limit == 20 {
            assert_eq!(res.hits.len(), 20);
            assert!(!res.has_more);
        } else if offset == 0 && limit == 1000 {
            assert_eq!(res.hits.len(), 520);
            assert!(!res.has_more);
        }
    }
}

#[test]
fn test_catalog_revision_tracks_updated_at_change() {
    let db = IsolatedTestDb::new("rev-track");
    let initial_rev = get_skill_registry_revision(&db.database).expect("get revision").unwrap();
    assert_eq!(initial_rev, "1.4.1:1729000000000");

    // Update snapshot with same version but newer updatedAt timestamp
    let conn = db.database.connect().expect("connect");
    conn.execute(
        "UPDATE skill_registry_snapshot SET updated_at = 1729000999999 WHERE id = 1",
        [],
    ).expect("update snapshot");
    drop(conn);

    let updated_rev = get_skill_registry_revision(&db.database).expect("get revision").unwrap();
    assert_eq!(updated_rev, "1.4.1:1729000999999");
    assert_ne!(initial_rev, updated_rev, "Revision must change when updatedAt changes even if version is identical");

    let res = search_skill_registry(
        &db.database,
        &SkillSearchRequest {
            query: "antigravity".into(),
            ..Default::default()
        },
    ).expect("search");
    assert_eq!(res.catalog_revision, Some("1.4.1:1729000999999".into()));
}

#[test]
fn test_reader_snapshot_isolation_against_concurrent_write() {
    let db = IsolatedTestDb::new("snapshot-iso");
    let reader = db.database.connect_read().expect("open reader");

    // 1. Reader starts reading snapshot: starts read transaction and reads initial revision
    let old_rev = get_skill_registry_revision_from_conn(&reader).expect("read old rev").unwrap();
    assert_eq!(old_rev, "1.4.1:1729000000000");

    // 2. Concurrently, another connection writes a new generation with new updatedAt and a new skill
    let fixture_str = include_str!("fixtures/skill_registry.json");
    let mut fixture: Value = serde_json::from_str(fixture_str).unwrap();
    fixture["updatedAt"] = serde_json::json!(1729000999999_i64);
    fixture["skills"][0]["name"] = serde_json::json!("new-generation-name");
    save_skill_registry(&db.database, &fixture).expect("save new generation");

    // 3. The original reader continues querying candidates within its read transaction
    let rows = query_skill_registry_candidates_with_conn(
        &reader,
        None,
        &SkillRegistryFilter {
            stage: None,
            category: None,
            allowed_ids: None,
            limit: 10000,
        },
    ).expect("query candidates on existing reader");

    // The reader's snapshot must NOT see the newly committed generation!
    let has_new = rows.iter().any(|r| r.name == "new-generation-name");
    assert!(!has_new, "Active read transaction must remain on its pinned snapshot without leaking newer generation data");

    // Reader explicitly commits/releases its read transaction
    let _ = reader.execute_batch("COMMIT;");
    drop(reader);

    // 4. A fresh search on a new connection immediately sees the new generation
    let new_res = search_skill_registry(
        &db.database,
        &SkillSearchRequest {
            query: "new-generation-name".into(),
            ..Default::default()
        },
    ).expect("fresh search");
    assert_eq!(new_res.catalog_revision, Some("1.4.1:1729000999999".into()));
    assert!(new_res.hits.iter().any(|h| h.name == "new-generation-name"));
}

