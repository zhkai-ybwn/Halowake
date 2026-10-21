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

    pub async fn warm(&mut self) -> Result<(), Failure> {
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
        self.request("turn/start", json!({"threadId":thread_id,"effort":effort,"input":[{"type":"text","text":"Reply exactly OK.","text_elements":[]}]})).await?;
        timeout(Duration::from_secs(60), async {
            loop {
                let message = match self.pending_completion.take() {
                    Some(value) => value,
                    None => self.receive().await?,
                };
                if message["method"] == "turn/completed"
                    && message.pointer("/params/threadId").and_then(Value::as_str)
                        == Some(&thread_id)
                {
                    return if message
                        .pointer("/params/turn/status")
                        .and_then(Value::as_str)
                        == Some("completed")
                    {
                        Ok(())
                    } else {
                        Err(Failure("warmupFailed", false))
                    };
                }
            }
        })
        .await
        .map_err(|_| Failure("requestTimeout", true))?
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
