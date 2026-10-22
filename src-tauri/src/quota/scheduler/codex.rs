use std::{env, fs, path::PathBuf, process::Stdio, time::Duration};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tempfile::TempDir;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    process::{Child, ChildStdin, ChildStdout, Command},
    time::timeout,
};

#[derive(Debug, Clone, Copy)]
pub struct Failure(pub &'static str, pub bool);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Window {
    pub used_percent: f64,
    pub window_duration_mins: i64,
    pub resets_at: i64,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub session: Window,
    pub weekly: Option<Window>,
    pub allowed: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WarmupEvidence {
    pub stage: String,
    pub model: Option<String>,
    pub thread_id: Option<String>,
    pub turn_id: Option<String>,
    pub reply_received: bool,
    pub output_tokens: Option<u64>,
    pub service_error: Option<String>,
}

pub struct PreparedWarmup {
    thread_id: String,
    effort: Option<String>,
}

pub fn local_login() -> Result<(Value, String), Failure> {
    let home = env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("USERPROFILE")
                .or_else(|| env::var_os("HOME"))
                .map(|p| PathBuf::from(p).join(".codex"))
        })
        .ok_or(Failure("loginRequired", false))?;
    let mut auth: Value = serde_json::from_slice(
        &fs::read(home.join("auth.json")).map_err(|_| Failure("loginRequired", false))?,
    )
    .map_err(|_| Failure("loginRequired", false))?;
    let account = auth
        .pointer("/tokens/account_id")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or(Failure("loginRequired", false))?
        .to_string();
    if auth
        .pointer("/tokens/access_token")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .is_none()
    {
        return Err(Failure("loginRequired", false));
    }
    // Never rotate the original CLI's refresh token from a temporary credential copy.
    auth["tokens"]["refresh_token"] = json!("");
    auth["OPENAI_API_KEY"] = Value::Null;
    auth["last_refresh"] = json!(chrono::Utc::now().to_rfc3339());
    Ok((auth, account))
}

pub struct Client {
    child: Child,
    input: ChildStdin,
    output: Lines<BufReader<ChildStdout>>,
    next_id: u64,
    account_id: String,
    pending_completion: Option<Value>,
    pending_events: Vec<Value>,
    pub workspace: PathBuf,
    // Declared last: credentials are removed after the child has been terminated.
    _directory: TempDir,
}

impl Drop for Client {
    fn drop(&mut self) {
        #[cfg(windows)]
        if self.child.try_wait().ok().flatten().is_none() {
            if let Some(id) = self.child.id() {
                use std::os::windows::process::CommandExt;
                let _ = std::process::Command::new("taskkill")
                    .args(["/PID", &id.to_string(), "/T", "/F"])
                    .creation_flags(0x08000000)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
            }
        }
        #[cfg(unix)]
        if let Some(id) = self.child.id() {
            unsafe {
                libc::kill(-(id as i32), libc::SIGKILL);
            }
        }
        let _ = self.child.start_kill();
    }
}

impl Client {
    pub async fn open(expected_account: &str) -> Result<Self, Failure> {
        let (auth, account) = local_login()?;
        if account != expected_account {
            return Err(Failure("accountChanged", false));
        }
        let directory = tempfile::Builder::new()
            .prefix("halowake-warmup-")
            .tempdir()
            .map_err(|_| Failure("storageError", false))?;
        let home = directory.path().join("home");
        let workspace = directory.path().join("workspace");
        fs::create_dir(&home)
            .and_then(|_| fs::create_dir(&workspace))
            .map_err(|_| Failure("storageError", false))?;
        let credentials = home.join("auth.json");
        fs::write(
            &credentials,
            serde_json::to_vec(&auth).map_err(|_| Failure("storageError", false))?,
        )
        .map_err(|_| Failure("storageError", false))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&credentials, fs::Permissions::from_mode(0o600))
                .map_err(|_| Failure("storageError", false))?;
        }
        fs::write(home.join("config.toml"), "cli_auth_credentials_store = \"file\"\nmodel_provider = \"openai\"\nsandbox_mode = \"read-only\"\napproval_policy = \"never\"\nweb_search = \"disabled\"\nproject_doc_max_bytes = 0\n[features]\nshell_tool = false\napps = false\nplugins = false\nskills = false\n")
            .map_err(|_| Failure("storageError", false))?;
        let mut command = codex_command()?;
        command
            .args(["app-server", "--listen", "stdio://"])
            .env("CODEX_HOME", &home)
            .current_dir(&workspace)
            .env_remove("OPENAI_API_KEY")
            .env_remove("CODEX_API_KEY")
            .env_remove("OPENAI_BASE_URL")
            .env_remove("CODEX_CONFIG")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        #[cfg(windows)]
        command.creation_flags(0x08000000);
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.as_std_mut().process_group(0);
        }
        let mut child = command
            .spawn()
            .map_err(|_| Failure("cliUnavailable", false))?;
        let input = child.stdin.take().ok_or(Failure("protocolError", false))?;
        let output =
            BufReader::new(child.stdout.take().ok_or(Failure("protocolError", false))?).lines();
        let mut client = Self {
            child,
            input,
            output,
            next_id: 0,
            account_id: expected_account.to_string(),
            pending_completion: None,
            pending_events: Vec::new(),
            workspace,
            _directory: directory,
        };
        client.request("initialize", json!({"clientInfo":{"name":"halowake_quota_scheduler","title":"Halowake","version":env!("CARGO_PKG_VERSION")}})).await?;
        client
            .send(json!({"method":"initialized","params":{}}))
            .await?;
        let identity = client
            .request("account/read", json!({"refreshToken":false}))
            .await?;
        if identity.pointer("/account/type").and_then(Value::as_str) != Some("chatgpt") {
            return Err(Failure("loginRequired", false));
        }
        if let Some(id) = identity.pointer("/account/id").and_then(Value::as_str) {
            if id != expected_account {
                return Err(Failure("accountChanged", false));
            }
        }
        Ok(client)
    }

    async fn send(&mut self, value: Value) -> Result<(), Failure> {
        let mut data = serde_json::to_vec(&value).map_err(|_| Failure("protocolError", false))?;
        data.push(b'\n');
        self.input
            .write_all(&data)
            .await
            .map_err(|_| Failure("connectionError", true))?;
        self.input
            .flush()
            .await
            .map_err(|_| Failure("connectionError", true))
    }

    async fn receive(&mut self) -> Result<Value, Failure> {
        let line = self
            .output
            .next_line()
            .await
            .map_err(|_| Failure("connectionError", true))?
            .ok_or(Failure("connectionError", true))?;
        if line.len() > 4 * 1024 * 1024 {
            return Err(Failure("protocolError", false));
        }
        let value: Value =
            serde_json::from_str(&line).map_err(|_| Failure("protocolError", false))?;
        // Never approve a tool or credential request from a background warmup.
        if value.get("method").is_some() && value.get("id").is_some() {
            self.send(json!({"id":value["id"],"error":{"code":-32601,"message":"Background warmup does not support this request"}})).await?;
            return Err(Failure("unsupportedRequest", false));
        }
        Ok(value)
    }

    async fn request(&mut self, method: &str, params: Value) -> Result<Value, Failure> {
        self.next_id += 1;
        let id = self.next_id;
        self.send(json!({"id":id,"method":method,"params":params}))
            .await?;
        timeout(Duration::from_secs(20), async {
            loop {
                let message = self.receive().await?;
                if message.get("id").and_then(Value::as_u64) == Some(id) {
                    if let Some(error) = message.get("error") {
                        return Err(classify_error(error));
                    }
                    return message
                        .get("result")
                        .cloned()
                        .ok_or(Failure("protocolError", false));
                }
                if message["method"] == "turn/completed" {
                    self.pending_completion = Some(message);
                } else if matches!(
                    message["method"].as_str(),
                    Some(
                        "item/completed"
                            | "thread/tokenUsage/updated"
                            | "item/agentMessage/delta"
                            | "error"
                    )
                ) {
                    self.pending_events.push(message);
                }
            }
        })
        .await
        .map_err(|_| Failure("requestTimeout", true))?
    }

    pub async fn snapshot(&mut self) -> Result<Snapshot, Failure> {
        let value = self.request("account/rateLimits/read", json!({})).await?;
        if value
            .get("accountId")
            .and_then(Value::as_str)
            .is_some_and(|id| id != self.account_id)
        {
            return Err(Failure("accountChanged", false));
        }
        let bucket =
            if let Some(buckets) = value.get("rateLimitsByLimitId").filter(|v| !v.is_null()) {
                buckets
                    .get("codex")
                    .ok_or(Failure("unsupportedQuota", false))?
            } else {
                value
                    .get("rateLimits")
                    .ok_or(Failure("unsupportedQuota", false))?
            };
        let windows: Vec<Window> = ["primary", "secondary"]
            .iter()
            .filter_map(|key| bucket.get(*key))
            .filter(|v| !v.is_null())
            .map(|v| {
                serde_json::from_value(v.clone()).map_err(|_| Failure("unsupportedQuota", false))
            })
            .collect::<Result<_, _>>()?;
        if windows.iter().any(|w| {
            !w.used_percent.is_finite()
                || !(0.0..=100.0).contains(&w.used_percent)
                || w.resets_at <= 0
        }) {
            return Err(Failure("unsupportedQuota", false));
        }
        let session = windows
            .iter()
            .find(|w| w.window_duration_mins == 300)
            .cloned()
            .ok_or(Failure("unsupportedQuota", false))?;
        let weekly = windows
            .iter()
            .find(|w| w.window_duration_mins == 10080)
            .cloned();
        Ok(Snapshot {
            session,
            weekly,
            allowed: value.get("ordinaryUsageAllowed").and_then(Value::as_bool),
        })
    }

    pub async fn prepare_warmup(
        &mut self,
        evidence: &mut WarmupEvidence,
    ) -> Result<PreparedWarmup, Failure> {
        evidence.stage = "listingModels".into();
        let models = self.request("model/list", json!({"limit":100})).await?;
        // Use the account's normal Codex model, rather than guessing that a cheaper model shares its quota bucket.
        let model = models["data"]
            .as_array()
            .and_then(|list| list.iter().find(|m| m["isDefault"] == true))
            .ok_or(Failure("modelUnavailable", false))?;
        let model_id = model["model"]
            .as_str()
            .ok_or(Failure("modelUnavailable", false))?
            .to_string();
        let effort = model["supportedReasoningEfforts"]
            .as_array()
            .and_then(|items| {
                ["minimal", "low", "medium"].iter().find_map(|effort| {
                    items
                        .iter()
                        .find(|v| v["reasoningEffort"] == *effort)
                        .map(|_| *effort)
                })
            })
            .map(String::from);
        evidence.model = Some(model_id.clone());
        evidence.stage = "startingThread".into();
        let thread = self.request("thread/start", json!({
            "model":model_id,"modelProvider":"openai","cwd":self.workspace,"ephemeral":true,
            "approvalPolicy":"never","sandbox":"read-only",
            "baseInstructions":"Reply exactly OK. Do not use tools or access files.","developerInstructions":"",
            "config":{"web_search":"disabled","features.shell_tool":false,"features.apps":false,"features.multi_agent":false}
        })).await?;
        let thread_id = thread
            .pointer("/thread/id")
            .and_then(Value::as_str)
            .ok_or(Failure("protocolError", false))?
            .to_string();
        evidence.thread_id = Some(thread_id.clone());
        Ok(PreparedWarmup { thread_id, effort })
    }

    pub async fn warm(
        &mut self,
        prepared: PreparedWarmup,
        evidence: &mut WarmupEvidence,
    ) -> Result<(), Failure> {
        evidence.stage = "startingTurn".into();
        let thread_id = prepared.thread_id;
        let response = self.request("turn/start", json!({"threadId":thread_id,"effort":prepared.effort,"input":[{"type":"text","text":"Reply exactly OK.","text_elements":[]}]})).await?;
        let turn_id = response
            .pointer("/turn/id")
            .and_then(Value::as_str)
            .ok_or(Failure("protocolError", false))?
            .to_string();
        evidence.turn_id = Some(turn_id.clone());
        evidence.stage = "waitingCompletion".into();
        for event in std::mem::take(&mut self.pending_events) {
            observe_turn_event(&event, &thread_id, &turn_id, evidence);
        }
        // A turn can finish before its start RPC response is consumed.
        if response.pointer("/turn/status").and_then(Value::as_str) == Some("completed") {
            observe_completed_turn(&response["turn"], evidence);
            evidence.stage = "completed".into();
            return Ok(());
        }
        timeout(Duration::from_secs(120), async {
            loop {
                let message = match self.pending_completion.take() {
                    Some(value) => value,
                    None => self.receive().await?,
                };
                observe_turn_event(&message, &thread_id, &turn_id, evidence);
                if message["method"] == "error"
                    && message.pointer("/params/threadId").and_then(Value::as_str)
                        == Some(&thread_id)
                    && message.pointer("/params/turnId").and_then(Value::as_str) == Some(&turn_id)
                    && message
                        .pointer("/params/willRetry")
                        .and_then(Value::as_bool)
                        == Some(false)
                {
                    return Err(message
                        .pointer("/params/error")
                        .map(classify_error)
                        .unwrap_or(Failure("warmupFailed", false)));
                }
                if is_matching_completion(&message, &thread_id, &turn_id) {
                    return if message
                        .pointer("/params/turn/status")
                        .and_then(Value::as_str)
                        == Some("completed")
                    {
                        observe_completed_turn(&message["params"]["turn"], evidence);
                        evidence.stage = "completed".into();
                        Ok(())
                    } else {
                        let error = message
                            .pointer("/params/turn/error")
                            .map(classify_error)
                            .unwrap_or(Failure("warmupFailed", false));
                        Err(error)
                    };
                }
            }
        })
        .await
        .map_err(|_| Failure("requestTimeout", true))?
    }
}

fn is_matching_completion(message: &Value, thread: &str, turn: &str) -> bool {
    message["method"] == "turn/completed"
        && message.pointer("/params/threadId").and_then(Value::as_str) == Some(thread)
        && message.pointer("/params/turn/id").and_then(Value::as_str) == Some(turn)
}

fn observe_completed_turn(turn: &Value, evidence: &mut WarmupEvidence) {
    if let Some(items) = turn["items"].as_array() {
        evidence.reply_received |= items.iter().any(|item| {
            item["type"] == "agentMessage"
                && item["text"].as_str().is_some_and(|s| !s.trim().is_empty())
        });
    }
}

fn observe_turn_event(message: &Value, thread: &str, turn: &str, evidence: &mut WarmupEvidence) {
    if message.pointer("/params/threadId").and_then(Value::as_str) != Some(thread) {
        return;
    }
    let event_turn = message.pointer("/params/turnId").and_then(Value::as_str);
    if event_turn != Some(turn) {
        return;
    }
    match message["method"].as_str() {
        Some("error") => {
            evidence.service_error = Some(classify_error(&message["params"]["error"]).0.into());
        }
        Some("item/agentMessage/delta") => {
            evidence.reply_received |= message
                .pointer("/params/delta")
                .and_then(Value::as_str)
                .is_some_and(|s| !s.trim().is_empty());
        }
        Some("item/completed") => {
            let item = &message["params"]["item"];
            evidence.reply_received |= item["type"] == "agentMessage"
                && item["text"].as_str().is_some_and(|s| !s.trim().is_empty());
        }
        Some("thread/tokenUsage/updated") => {
            if let Some(tokens) = message
                .pointer("/params/tokenUsage/last/outputTokens")
                .and_then(Value::as_u64)
            {
                evidence.output_tokens = Some(tokens);
            }
        }
        _ => {}
    }
}

fn classify_error(error: &Value) -> Failure {
    let text = error.to_string().to_lowercase();
    if text.contains("401")
        || text.contains("unauthorized")
        || text.contains("auth")
        || text.contains("token")
    {
        Failure("loginRequired", false)
    } else if text.contains("429") || text.contains("rate limit") {
        Failure("rateLimited", true)
    } else if text.contains("timeout")
        || text.contains("connection")
        || text.contains("500")
        || text.contains("503")
    {
        Failure("connectionError", true)
    } else {
        Failure("protocolError", false)
    }
}

fn codex_command() -> Result<Command, Failure> {
    let paths = env::var_os("PATH").unwrap_or_default();
    for directory in env::split_paths(&paths) {
        let native = directory.join(if cfg!(windows) { "codex.exe" } else { "codex" });
        if native.is_file() {
            return Ok(Command::new(native));
        }
        // npm's Windows shim is a batch file; invoke its JS entry directly without a shell.
        let script = directory.join("node_modules/@openai/codex/bin/codex.js");
        if script.is_file() {
            let mut command = Command::new("node");
            command.arg(script);
            return Ok(command);
        }
    }
    #[cfg(windows)]
    if let Some(local) = env::var_os("LOCALAPPDATA") {
        if let Ok(entries) = fs::read_dir(PathBuf::from(local).join("OpenAI/Codex/bin")) {
            let mut candidates: Vec<_> = entries
                .flatten()
                .map(|e| e.path().join("codex.exe"))
                .filter(|p| p.is_file())
                .collect();
            candidates.sort();
            if let Some(path) = candidates.last() {
                return Ok(Command::new(path));
            }
        }
    }
    Err(Failure("cliUnavailable", false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_accepts_completion_for_the_dispatched_turn() {
        let event = json!({"method":"turn/completed","params":{"threadId":"thread-1","turn":{"id":"turn-1","status":"completed"}}});
        assert!(is_matching_completion(&event, "thread-1", "turn-1"));
        assert!(!is_matching_completion(&event, "thread-1", "other-turn"));
        assert!(!is_matching_completion(&event, "other-thread", "turn-1"));
    }

    #[test]
    fn reply_and_usage_must_belong_to_the_expected_turn() {
        let mut evidence = WarmupEvidence::default();
        let reply = json!({"method":"item/completed","params":{"threadId":"thread-1","turnId":"turn-1","item":{"type":"agentMessage","text":"OK"}}});
        observe_turn_event(&reply, "thread-1", "other-turn", &mut evidence);
        assert!(!evidence.reply_received);
        observe_turn_event(&reply, "thread-1", "turn-1", &mut evidence);
        assert!(evidence.reply_received);
        let usage = json!({"method":"thread/tokenUsage/updated","params":{"threadId":"thread-1","turnId":"turn-1","tokenUsage":{"last":{"outputTokens":2}}}});
        observe_turn_event(&usage, "thread-1", "turn-1", &mut evidence);
        assert_eq!(evidence.output_tokens, Some(2));
    }

    #[test]
    fn completed_response_can_supply_reply_evidence_without_delta_events() {
        let mut evidence = WarmupEvidence::default();
        observe_completed_turn(
            &json!({"items":[{"type":"agentMessage","text":"OK"}]}),
            &mut evidence,
        );
        assert!(evidence.reply_received);
    }

    // Never runs in normal test suites: this consumes real subscription capacity.
    #[tokio::test]
    #[ignore = "requires explicit user permission for one real Codex inference"]
    async fn live_single_authorized_warmup() {
        let (_, account) = local_login().expect("local file login");
        let mut evidence = WarmupEvidence::default();
        let operation = async {
            let mut client = Client::open(&account).await?;
            let before = client.snapshot().await?;
            println!("quota_before={:?}", before.session);
            let prepared = client.prepare_warmup(&mut evidence).await?;
            println!("prepared_model={:?}", evidence.model);
            // Exactly one turn; no retry loop and no changes to the user's schedule.
            client.warm(prepared, &mut evidence).await?;
            let after = client.snapshot().await?;
            println!("quota_after={:?}", after.session);
            Ok::<(), Failure>(())
        };
        let result = timeout(Duration::from_secs(240), operation).await;
        println!("warmup_evidence={evidence:?}");
        assert!(
            matches!(result, Ok(Ok(()))),
            "authorized request result: {result:?}"
        );
        assert_eq!(evidence.stage, "completed");
        assert!(evidence.reply_received || evidence.output_tokens.is_some_and(|tokens| tokens > 0));
    }
}
