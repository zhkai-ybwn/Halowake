use super::*;

pub(super) fn data_roots(home: &Path) -> Vec<PathBuf> {
    let mut roots = vec![home.join(".local/share/opencode"), home.join(".opencode")];
    if let Some(xdg) = env::var_os("XDG_DATA_HOME").filter(|p| !p.is_empty()) {
        roots.push(PathBuf::from(xdg).join("opencode"));
    }
    for dir in [get_appdata_dir(), get_local_appdata_dir()]
        .into_iter()
        .flatten()
    {
        roots.push(dir.join("opencode"));
    }
    roots.sort();
    roots.dedup();
    roots
}

fn open_database(path: &Path) -> Result<Connection, String> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("读取 OpenCode 数据库 {} 失败: {e}", path.display()))?;
    conn.busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|e| e.to_string())?;
    Ok(conn)
}

fn json_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_dir() {
            json_files(&path, files)?;
        } else if path.extension().is_some_and(|e| e == "json") {
            files.push(path);
        }
    }
    Ok(())
}

fn session_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let storage = root.join("storage");
    let mut files = Vec::new();
    json_files(&storage.join("session"), &mut files)?;
    if storage.exists() {
        for entry in fs::read_dir(&storage).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.extension().is_some_and(|e| e == "json") {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

fn read_json(path: &Path) -> Option<Value> {
    serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
}

fn millis_timestamp(ms: i64) -> Option<String> {
    DateTime::<Utc>::from_timestamp_millis(ms)
        .map(|ts| ts.to_rfc3339_opts(SecondsFormat::Millis, true))
}

pub(super) fn message_timestamp(value: &Value) -> Option<String> {
    value
        .get("timestamp")
        .and_then(Value::as_str)
        .and_then(canonical_timestamp)
        .or_else(|| {
            value
                .pointer("/time/created")
                .or_else(|| value.get("time_created"))
                .or_else(|| value.get("timestamp"))
                .and_then(Value::as_i64)
                .and_then(millis_timestamp)
        })
}

fn cwd(value: &Value) -> Option<String> {
    value
        .get("directory")
        .or_else(|| value.get("cwd"))
        .or_else(|| value.get("projectPath"))
        .and_then(Value::as_str)
        .map(normalize_project_path)
}

// Session index avoids reading message bodies when populating the tool/project pickers.
pub(super) fn session_index(
    roots: &[PathBuf],
) -> Result<Vec<(String, String, Option<String>)>, String> {
    let mut sessions = HashMap::new();
    for root in roots {
        for path in session_files(root)? {
            if let Some(value) = read_json(&path) {
                if let (Some(id), Some(cwd)) =
                    (value.get("id").and_then(Value::as_str), cwd(&value))
                {
                    let ts = value
                        .pointer("/time/updated")
                        .and_then(Value::as_i64)
                        .and_then(millis_timestamp);
                    sessions.insert(id.to_string(), (cwd, ts));
                }
            }
        }
        let db = root.join("opencode.db");
        if !db.exists() {
            continue;
        }
        let conn = open_database(&db)?;
        let mut stmt = conn
            .prepare("SELECT id, directory, time_updated FROM session")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            let (id, cwd, ts) = row.map_err(|e| e.to_string())?;
            sessions.insert(id, (normalize_project_path(&cwd), millis_timestamp(ts)));
        }
    }
    Ok(sessions
        .into_iter()
        .map(|(id, (cwd, ts))| (id, cwd, ts))
        .collect())
}

fn add_text(
    draft: &mut SessionDraft,
    role: &str,
    text: &str,
    timestamp: &str,
    from: &str,
    to: &str,
) {
    if !is_in_range(timestamp, from, to) {
        return;
    }
    let cleaned = match role {
        "user" => clean_user_message(text),
        "assistant" => clean_result_message(text),
        _ => None,
    };
    if let Some(text) = cleaned {
        update_bounds(draft, timestamp);
        if role == "user" {
            draft.user_messages.push(text);
        } else {
            draft.assistant_messages.push(text);
        }
    }
}

fn load_database(path: &Path, from: &str, to: &str) -> Result<Vec<CodexReportSession>, String> {
    let conn = open_database(path)?;
    let from_ms = DateTime::parse_from_rfc3339(from)
        .map_err(|e| e.to_string())?
        .timestamp_millis();
    let to_ms = DateTime::parse_from_rfc3339(to)
        .map_err(|e| e.to_string())?
        .timestamp_millis();
    // Filter messages by their own date: session creation/update dates do not describe continued work.
    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.directory, m.id, m.time_created, m.data, p.data
         FROM session s JOIN message m ON m.session_id = s.id
         JOIN part p ON p.message_id = m.id
         WHERE m.time_created >= ?1 AND m.time_created <= ?2
         ORDER BY m.time_created, m.id, p.time_created, p.id",
        )
        .map_err(|e| format!("查询 OpenCode 历史记录失败: {e}"))?;
    let rows = stmt
        .query_map(rusqlite::params![from_ms, to_ms], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let mut drafts = HashMap::new();
    // Assemble all text parts of a message before applying the existing report truncation.
    let mut messages: Vec<(String, String, String, String)> = Vec::new();
    let mut previous_id = String::new();
    for row in rows {
        let (id, cwd, message_id, ms, message, part) = row.map_err(|e| e.to_string())?;
        let (Ok(message), Ok(part)) = (
            serde_json::from_str::<Value>(&message),
            serde_json::from_str::<Value>(&part),
        ) else {
            continue;
        };
        if part.get("type").and_then(Value::as_str) != Some("text") {
            continue;
        }
        let Some(text) = part
            .get("text")
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
        else {
            continue;
        };
        let Some(timestamp) = millis_timestamp(ms) else {
            continue;
        };
        let role = message
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or_default();
        drafts.entry(id.clone()).or_insert_with(|| SessionDraft {
            id: id.clone(),
            provider: "opencode".into(),
            cwd: Some(normalize_project_path(&cwd)),
            ..Default::default()
        });
        if previous_id == message_id {
            if let Some(last) = messages.last_mut() {
                last.2.push('\n');
                last.2.push_str(text);
            }
        } else {
            messages.push((id, role.to_string(), text.to_string(), timestamp));
            previous_id = message_id;
        }
    }
    for (id, role, text, timestamp) in messages {
        if let Some(draft) = drafts.get_mut(&id) {
            add_text(draft, &role, &text, &timestamp, from, to);
        }
    }
    Ok(drafts
        .into_values()
        .filter_map(|d| finalize_draft(d, from, to))
        .collect())
}

fn load_legacy(root: &Path, path: &Path, from: &str, to: &str) -> Option<CodexReportSession> {
    let value = read_json(path)?;
    // Retain the original flat session export format.
    if value.get("messages").is_some() {
        return parse_opencode_session(path, from, to);
    }
    let id = value.get("id")?.as_str()?;
    let mut draft = SessionDraft {
        id: id.to_string(),
        provider: "opencode".into(),
        cwd: cwd(&value),
        ..Default::default()
    };
    let storage = root.join("storage");
    let mut paths = Vec::new();
    json_files(&storage.join("message").join(id), &mut paths).ok()?;
    let mut messages: Vec<_> = paths.iter().filter_map(|p| read_json(p)).collect();
    messages.sort_by_key(message_timestamp);
    for message in messages {
        let Some(timestamp) = message_timestamp(&message) else {
            continue;
        };
        let role = message
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let mut parts = Vec::new();
        let message_id = message
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default();
        json_files(&storage.join("part").join(message_id), &mut parts).ok()?;
        let text = parts
            .iter()
            .filter_map(|p| read_json(p))
            .filter(|p| p.get("type").and_then(Value::as_str) == Some("text"))
            .filter_map(|p| p.get("text").and_then(Value::as_str).map(str::to_string))
            .collect::<Vec<_>>()
            .join("\n");
        add_text(&mut draft, role, &text, &timestamp, from, to);
    }
    finalize_draft(draft, from, to)
}

pub(super) fn load_sessions(
    roots: &[PathBuf],
    from: &str,
    to: &str,
) -> Result<Vec<CodexReportSession>, String> {
    let mut sessions = Vec::new();
    for root in roots {
        for path in session_files(root)? {
            if let Some(session) = load_legacy(root, &path, from, to) {
                sessions.push(session);
            }
        }
        let db = root.join("opencode.db");
        if db.exists() {
            sessions.extend(load_database(&db, from, to)?);
        }
    }
    Ok(merge_sessions(sessions))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_reads_message_dates_and_text_parts_without_writing() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("opencode.db");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE session(id TEXT, directory TEXT, time_updated INTEGER);
            CREATE TABLE message(id TEXT, session_id TEXT, time_created INTEGER, data TEXT);
            CREATE TABLE part(id TEXT, message_id TEXT, time_created INTEGER, data TEXT);
            INSERT INTO session VALUES('s', 'D:/actual-project', 0);
            INSERT INTO message VALUES('u','s',1782864000000,'{\"role\":\"user\"}');
            INSERT INTO message VALUES('a','s',1782864001000,'{\"role\":\"assistant\"}');
            INSERT INTO part VALUES('p1','u',0,'{\"type\":\"text\",\"text\":\"question\"}');
            INSERT INTO part VALUES('p2','a',0,'{\"type\":\"reasoning\",\"text\":\"hidden\"}');
            INSERT INTO part VALUES('p3','a',1,'{\"type\":\"text\",\"text\":\"answer\"}');
            INSERT INTO part VALUES('p4','a',2,'{\"type\":\"text\",\"text\":\"continued\"}');",
        )
        .unwrap();
        drop(conn);
        let before = fs::read(&path).unwrap();
        let sessions = load_sessions(
            &[temp.path().to_path_buf()],
            "2026-07-01T00:00:00Z",
            "2026-09-30T23:59:59Z",
        )
        .unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].project_name, "actual-project");
        assert_eq!(sessions[0].user_messages, ["question"]);
        assert_eq!(sessions[0].assistant_messages, ["answer\ncontinued"]);
        assert_eq!(fs::read(path).unwrap(), before);
        assert_eq!(
            session_index(&[temp.path().to_path_buf()]).unwrap().len(),
            1
        );
    }

    #[test]
    fn legacy_split_storage_reads_parts_and_filters_message_dates() {
        let temp = tempfile::tempdir().unwrap();
        let storage = temp.path().join("storage");
        for dir in ["session/project", "message/s", "part/u", "part/a"] {
            fs::create_dir_all(storage.join(dir)).unwrap();
        }
        fs::write(
            storage.join("session/project/s.json"),
            r#"{"id":"s","directory":"D:/legacy"}"#,
        )
        .unwrap();
        fs::write(
            storage.join("message/s/u.json"),
            r#"{"id":"u","role":"user","time":{"created":1782864000000}}"#,
        )
        .unwrap();
        fs::write(
            storage.join("message/s/a.json"),
            r#"{"id":"a","role":"assistant","time":{"created":1782864001000}}"#,
        )
        .unwrap();
        fs::write(
            storage.join("part/u/p.json"),
            r#"{"type":"text","text":"question"}"#,
        )
        .unwrap();
        fs::write(
            storage.join("part/a/p.json"),
            r#"{"type":"text","text":"answer"}"#,
        )
        .unwrap();
        let sessions = load_sessions(
            &[temp.path().to_path_buf()],
            "2026-07-01T00:00:00Z",
            "2026-07-02T00:00:00Z",
        )
        .unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].assistant_messages, ["answer"]);
        assert!(load_sessions(
            &[temp.path().to_path_buf()],
            "2026-08-01T00:00:00Z",
            "2026-08-02T00:00:00Z"
        )
        .unwrap()
        .is_empty());
    }
}
