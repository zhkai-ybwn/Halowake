mod codex;

use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

use chrono::{Datelike, Local, TimeZone};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use tokio::sync::Notify;

use crate::storage::AppDatabase;
use codex::{Client, Failure, Snapshot, WarmupEvidence};

const STORAGE_KEY: &str = "codex_quota_scheduler_v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub enabled: bool,
    pub times: Vec<String>,
    pub weekdays: Vec<u32>,
    pub check_interval_seconds: u32,
    pub max_wait_minutes: u32,
    pub account_id: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: false,
            times: vec!["08:00".into(), "13:00".into(), "18:00".into()],
            weekdays: vec![1, 2, 3, 4, 5],
            check_interval_seconds: 60,
            max_wait_minutes: 30,
            account_id: None,
        }
    }
}

impl Settings {
    fn validate(&mut self) -> Result<(), String> {
        if self.times.is_empty()
            || self.times.len() > 8
            || self.times.iter().any(|s| parse_time(s).is_none())
            || self.weekdays.is_empty()
            || self.weekdays.iter().any(|d| !(1..=7).contains(d))
            || ![30, 60, 120, 300].contains(&self.check_interval_seconds)
            || !(5..=60).contains(&self.max_wait_minutes)
        {
            return Err("invalidSettings".into());
        }
        self.times.sort();
        self.times.dedup();
        self.weekdays.sort();
        self.weekdays.dedup();
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    pub id: String,
    pub scheduled_at: i64,
    pub deadline: i64,
    pub status: String,
    pub detail: String,
    pub checks: u32,
    pub failures: u32,
    pub next_check_at: i64,
    pub requested_at: Option<i64>,
    pub request_completed: bool,
    #[serde(default)]
    pub request_finished_at: Option<i64>,
    #[serde(default)]
    pub request_error: Option<String>,
    #[serde(default)]
    pub request_evidence: WarmupEvidence,
    #[serde(default)]
    pub observation: Option<WindowObservation>,
    #[serde(default)]
    pub verified_reset: Option<i64>,
    #[serde(default)]
    pub verification_version: u32,
    pub baseline_reset: Option<i64>,
    pub baseline_used: f64,
    pub account_id: String,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowObservation {
    at: i64,
    resets_at: i64,
    used_percent: f64,
}

impl Run {
    fn active(&self) -> bool {
        matches!(
            self.status.as_str(),
            "waiting" | "checking" | "warming" | "verifying" | "backoff"
        )
    }
    fn finish(&mut self, status: &str, detail: &str, now: i64) {
        self.status = status.into();
        self.detail = detail.into();
        self.updated_at = now;
        self.next_check_at = 0;
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SchedulerData {
    pub settings: Settings,
    pub runs: Vec<Run>,
}

pub struct Scheduler {
    data: Mutex<SchedulerData>,
    changed: Notify,
}

fn parse_time(value: &str) -> Option<(u32, u32)> {
    let bytes = value.as_bytes();
    if bytes.len() != 5
        || bytes[2] != b':'
        || ![bytes[0], bytes[1], bytes[3], bytes[4]]
            .iter()
            .all(u8::is_ascii_digit)
    {
        return None;
    }
    let hour = value[..2].parse::<u32>().ok()?;
    let minute = value[3..].parse::<u32>().ok()?;
    (hour < 24 && minute < 60).then_some((hour, minute))
}

fn persist(db: &AppDatabase, data: &SchedulerData) -> Result<(), String> {
    let json = serde_json::to_string(data).map_err(|_| "storageError".to_string())?;
    db.connect()?.execute("INSERT INTO app_metadata(key,value,updated_at) VALUES(?1,?2,?3) ON CONFLICT(key) DO UPDATE SET value=excluded.value,updated_at=excluded.updated_at",
        rusqlite::params![STORAGE_KEY,json,Local::now().timestamp_millis()]).map_err(|_| "storageError".to_string())?;
    Ok(())
}

pub fn start(app: &AppHandle) -> Result<(), String> {
    let db = app.state::<AppDatabase>();
    let loaded = (|| -> Result<SchedulerData, String> {
        let saved: Option<String> = db
            .connect()?
            .query_row(
                "SELECT value FROM app_metadata WHERE key=?1",
                [STORAGE_KEY],
                |r| r.get(0),
            )
            .optional()
            .map_err(|_| "storageError".to_string())?;
        let mut data = match saved {
            Some(json) => serde_json::from_str::<SchedulerData>(&json)
                .map_err(|_| "invalidSchedulerData".to_string())?,
            None => SchedulerData::default(),
        };
        data.settings.validate()?;
        Ok(data)
    })();
    let mut data = loaded.unwrap_or_else(|_| {
        log::error!("Codex scheduler disabled: stored settings could not be loaded");
        let now = Local::now().timestamp();
        let mut run = new_run("storage-error".into(), now, now, String::new());
        run.finish("failed", "invalidSchedulerData", now);
        SchedulerData {
            settings: Settings::default(),
            runs: vec![run],
        }
    });
    // In-flight dispatches are durable. After a crash only verify, never resend them.
    for run in data.runs.iter_mut().filter(|r| r.active()) {
        run.status = if run.requested_at.is_some() {
            "verifying"
        } else {
            "waiting"
        }
        .into();
    }
    if repair_legacy_confirmations(&mut data) {
        persist(&db, &data)?;
    }
    app.manage(Scheduler {
        data: Mutex::new(data),
        changed: Notify::new(),
    });
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        worker(handle).await;
    });
    Ok(())
}

#[tauri::command]
pub fn load_codex_schedule(scheduler: State<'_, Scheduler>) -> Result<SchedulerData, String> {
    scheduler
        .data
        .lock()
        .map(|v| v.clone())
        .map_err(|_| "storageError".into())
}

#[tauri::command]
pub fn save_codex_schedule(
    app: AppHandle,
    scheduler: State<'_, Scheduler>,
    mut settings: Settings,
) -> Result<SchedulerData, String> {
    settings.validate()?;
    let mut guard = scheduler
        .data
        .lock()
        .map_err(|_| "storageError".to_string())?;
    if settings.enabled {
        let (_, account) = codex::local_login().map_err(|e| e.0.to_string())?;
        // An explicit save is the user's opportunity to bind the current local login.
        settings.account_id = Some(account);
    } else {
        settings.account_id = guard.settings.account_id.clone();
    }
    if settings.enabled
        && serde_json::to_value(&settings).ok() == serde_json::to_value(&guard.settings).ok()
    {
        return Ok(guard.clone());
    }
    let mut next = guard.clone();
    next.settings = settings;
    for run in next.runs.iter_mut().filter(|r| r.active()) {
        run.finish("cancelled", "settingsChanged", Local::now().timestamp());
    }
    persist(&app.state::<AppDatabase>(), &next)?;
    *guard = next;
    scheduler.changed.notify_one();
    Ok(guard.clone())
}

#[tauri::command]
pub fn run_codex_warmup_now(
    app: AppHandle,
    scheduler: State<'_, Scheduler>,
) -> Result<SchedulerData, String> {
    let (_, account) = codex::local_login().map_err(|e| e.0.to_string())?;
    let mut guard = scheduler
        .data
        .lock()
        .map_err(|_| "storageError".to_string())?;
    if guard.runs.iter().any(Run::active) {
        return Err("alreadyRunning".into());
    }
    if guard
        .settings
        .account_id
        .as_ref()
        .is_some_and(|id| id != &account)
    {
        return Err("accountChanged".into());
    }
    let mut next = guard.clone();
    let now = Local::now().timestamp();
    let id = format!("manual:{now}");
    if next.runs.iter().any(|r| r.id == id) {
        return Err("alreadyRunning".into());
    }
    let deadline = now + i64::from(next.settings.max_wait_minutes) * 60;
    next.runs.insert(0, new_run(id, now, deadline, account));
    next.runs.truncate(20);
    persist(&app.state::<AppDatabase>(), &next)?;
    *guard = next;
    scheduler.changed.notify_one();
    Ok(guard.clone())
}

fn new_run(id: String, at: i64, deadline: i64, account_id: String) -> Run {
    Run {
        id,
        scheduled_at: at,
        deadline,
        status: "waiting".into(),
        detail: "waiting".into(),
        checks: 0,
        failures: 0,
        next_check_at: at,
        requested_at: None,
        request_completed: false,
        request_finished_at: None,
        request_error: None,
        request_evidence: WarmupEvidence::default(),
        observation: None,
        verified_reset: None,
        verification_version: 2,
        baseline_reset: None,
        baseline_used: 0.0,
        account_id,
        updated_at: at,
    }
}

fn enqueue(data: &mut SchedulerData, now: chrono::DateTime<Local>) {
    if !data.settings.enabled
        || !data
            .settings
            .weekdays
            .contains(&now.weekday().number_from_monday())
    {
        return;
    }
    if data.runs.iter().any(Run::active) {
        return;
    }
    let Some(account) = data.settings.account_id.clone() else {
        return;
    };
    // Keep only the latest eligible slot; nearby nodes must never create concurrent warmups.
    let slot = data
        .settings
        .times
        .iter()
        .filter_map(|time| {
            let (hour, minute) = parse_time(time)?;
            let naive = now.date_naive().and_hms_opt(hour, minute, 0)?;
            let at = Local.from_local_datetime(&naive).earliest()?.timestamp();
            let deadline = at + i64::from(data.settings.max_wait_minutes) * 60;
            (now.timestamp() >= at && now.timestamp() < deadline)
                .then(|| (format!("{}@{}", now.date_naive(), time), at, deadline))
        })
        .max_by_key(|(_, at, _)| *at);
    if let Some((id, at, deadline)) = slot {
        if data.runs.iter().any(|r| r.id == id) {
            return;
        }
        for run in data.runs.iter_mut().filter(|r| r.active()) {
            run.finish("cancelled", "superseded", now.timestamp());
        }
        data.runs.insert(0, new_run(id, at, deadline, account));
        data.runs.truncate(20);
    }
}

async fn worker(app: AppHandle) {
    let scheduler = app.state::<Scheduler>();
    let db = app.state::<AppDatabase>();
    let mut budget: Option<(String, Instant)> = None;
    loop {
        // Register cancellation before reading state, so a save cannot race a dispatch.
        let changed = scheduler.changed.notified();
        tokio::pin!(changed);
        changed.as_mut().enable();
        let job = {
            let Ok(mut guard) = scheduler.data.lock() else {
                return;
            };
            let mut next = guard.clone();
            let now = Local::now();
            let maximum = Duration::from_secs(u64::from(next.settings.max_wait_minutes) * 60);
            for run in next.runs.iter_mut().filter(|r| r.active()) {
                let clock_expired = budget
                    .as_ref()
                    .is_some_and(|(id, start)| id == &run.id && start.elapsed() >= maximum);
                if now.timestamp() >= run.deadline || clock_expired {
                    run.finish("expired", "waitExpired", now.timestamp());
                } else if run
                    .request_finished_at
                    .or(run.requested_at)
                    .is_some_and(|at| now.timestamp() >= at + 180)
                {
                    run.finish("failed", "unconfirmed", now.timestamp());
                }
            }
            enqueue(&mut next, now);
            if let Some(run) = next.runs.iter().find(|r| r.active()) {
                if budget.as_ref().map_or(true, |(id, _)| id != &run.id) {
                    budget = Some((run.id.clone(), Instant::now()));
                }
            } else {
                budget = None;
            }
            let settings = next.settings.clone();
            let mut job = next
                .runs
                .iter_mut()
                .find(|r| r.active() && r.next_check_at <= now.timestamp())
                .map(|run| {
                    run.status = if run.requested_at.is_some() {
                        "verifying"
                    } else {
                        "checking"
                    }
                    .into();
                    run.checks += 1;
                    (run.clone(), settings)
                });
            if serde_json::to_string(&next).ok() != serde_json::to_string(&*guard).ok() {
                if persist(&db, &next).is_err() {
                    // No durable write means no inference dispatch. Disable instead of spinning.
                    guard.settings.enabled = false;
                    for run in guard.runs.iter_mut().filter(|r| r.active()) {
                        run.finish("failed", "storageError", now.timestamp());
                    }
                    log::error!("Codex scheduler paused: persistence failed");
                    job = None;
                } else {
                    *guard = next;
                }
            }
            job
        };
        if let Some((mut run, settings)) = job {
            let remaining = ((run.deadline - Local::now().timestamp()).max(1) as u64).min(
                u64::from(settings.max_wait_minutes)
                    .saturating_mul(60)
                    .saturating_sub(
                        budget
                            .as_ref()
                            .map_or(0, |(_, start)| start.elapsed().as_secs()),
                    )
                    .max(1),
            );
            let started = Instant::now();
            let result = tokio::select! {
                _ = &mut changed => { continue; }
                result = tokio::time::timeout(Duration::from_secs(remaining.min(240)), attempt(&app,&mut run)) => {
                    result.unwrap_or(Err(Failure("requestTimeout",true)))
                }
            };
            let now = Local::now().timestamp();
            if now >= run.deadline || started.elapsed().as_secs() >= remaining {
                run.finish("expired", "waitExpired", now);
            } else {
                match result {
                    Ok((status, detail)) => {
                        run.status = status.into();
                        run.detail = detail.into();
                        run.failures = 0;
                        run.updated_at = now;
                        run.next_check_at = if run.active() {
                            now + i64::from(if run.requested_at.is_some() {
                                settings.check_interval_seconds.min(30)
                            } else {
                                settings.check_interval_seconds
                            })
                        } else {
                            0
                        };
                    }
                    Err(Failure(code, retryable)) => {
                        if run.requested_at.is_some()
                            && !run.request_completed
                            && run.request_error.is_none()
                        {
                            run.request_error = Some(code.into());
                        }
                        log::warn!(
                            "Codex scheduler attempt stopped: stage={} code={}",
                            run.request_evidence.stage,
                            code
                        );
                        run.failures += 1;
                        if !retryable || code == "rateLimited" || run.failures >= 5 {
                            run.finish("failed", code, now);
                        } else {
                            run.status = "backoff".into();
                            run.detail = code.into();
                            run.updated_at = now;
                            let delay = (i64::from(settings.check_interval_seconds)
                                * (1_i64 << run.failures.min(4)))
                            .min(600);
                            run.next_check_at = now + delay;
                        }
                    }
                }
            }
            let Ok(mut guard) = scheduler.data.lock() else {
                return;
            };
            if let Some(current) = guard.runs.iter_mut().find(|r| r.id == run.id && r.active()) {
                *current = run;
                if persist(&db, &guard).is_err() {
                    guard.settings.enabled = false;
                    for run in guard.runs.iter_mut().filter(|r| r.active()) {
                        run.finish("failed", "storageError", now);
                    }
                }
            }
        }
        tokio::select! {
            _ = scheduler.changed.notified() => {}
            _ = tokio::time::sleep(Duration::from_secs(1)) => {}
        }
    }
}

enum Decision {
    Wait,
    Ready,
    Warm,
}

fn same_confirmed_window(previous: &Run, current: &Run, reset: i64, now: i64) -> bool {
    previous.id != current.id
        && previous.account_id == current.account_id
        && matches!(previous.status.as_str(), "success" | "ready")
        && previous.verification_version == 2
        && reset > now
        && reset - 18000 >= current.scheduled_at - 120
        && previous
            .verified_reset
            .is_some_and(|old| (old - reset).abs() <= 15)
}

fn repair_legacy_confirmations(data: &mut SchedulerData) -> bool {
    let mut changed = false;
    for run in &mut data.runs {
        if run.status == "success" && run.verification_version < 2 {
            run.status = "failed".into();
            run.detail = "legacyUnconfirmed".into();
            run.verified_reset = None;
            changed = true;
        }
    }
    changed
}

fn window_confirmed(snapshot: &Snapshot, run: &Run, now: i64) -> bool {
    let evidence = &run.request_evidence;
    if !run.request_completed
        || evidence.stage != "completed"
        || !(evidence.reply_received || evidence.output_tokens.is_some_and(|n| n > 0))
    {
        return false;
    }
    let window = &snapshot.session;
    if window.resets_at <= now || window.resets_at > now + 18015 || snapshot.allowed == Some(false)
    {
        return false;
    }
    if window.used_percent > run.baseline_used {
        return true;
    }
    // 0% can be a rounded value. Only a completed inference plus a fixed countdown
    // observed for a full minute can confirm it; a sliding now+5h reset never can.
    run.observation.as_ref().is_some_and(|old| {
        now - old.at >= 60
            && (window.resets_at - old.resets_at).abs() <= 5
            && window.resets_at <= now + 18000 - 60
    })
}

fn verify_window(
    snapshot: &Snapshot,
    run: &mut Run,
    now: i64,
) -> Result<(&'static str, &'static str), Failure> {
    if window_confirmed(snapshot, run, now) {
        run.verified_reset = Some(snapshot.session.resets_at);
        run.verification_version = 2;
        return Ok(("success", "confirmed"));
    }
    let expired = run
        .request_finished_at
        .or(run.requested_at)
        .is_some_and(|at| now >= at + 180);
    if expired {
        return Err(Failure("unconfirmed", false));
    }
    if run.observation.as_ref().map_or(true, |old| {
        (old.resets_at - snapshot.session.resets_at).abs() > 5
    }) {
        run.observation = Some(WindowObservation {
            at: now,
            resets_at: snapshot.session.resets_at,
            used_percent: snapshot.session.used_percent,
        });
    }
    Ok(("verifying", "verifying"))
}

fn decide(snapshot: &Snapshot, run: &Run, now: i64) -> Result<Decision, Failure> {
    if snapshot
        .weekly
        .as_ref()
        .is_some_and(|w| w.used_percent >= 100.0 && w.resets_at > now)
    {
        return Err(Failure("weeklyLimited", false));
    }
    let window = &snapshot.session;
    if snapshot.allowed == Some(false) && window.used_percent < 100.0 {
        return Err(Failure("quotaBlocked", false));
    }
    // A passed local timestamp alone is not evidence that the server reset quota.
    if window.used_percent >= 100.0 {
        return Ok(Decision::Wait);
    }
    let active = window.resets_at > now
        && (window.used_percent > 0.0 || window.resets_at < now + 18000 - 120);
    if active {
        // A new window started by the user already satisfies this node. An older window
        // expiring near the node is awaited; one expiring past the deadline is left alone.
        if window.resets_at - 18000 >= run.scheduled_at - 120 || window.resets_at > run.deadline {
            Ok(Decision::Ready)
        } else {
            Ok(Decision::Wait)
        }
    } else {
        Ok(Decision::Warm)
    }
}

async fn attempt(app: &AppHandle, run: &mut Run) -> Result<(&'static str, &'static str), Failure> {
    if run.requested_at.is_none() {
        run.request_evidence.stage = "connecting".into();
    }
    let mut client = Client::open(&run.account_id).await?;
    if run.requested_at.is_none() {
        run.request_evidence.stage = "readingQuota".into();
    }
    let snapshot = client.snapshot().await?;
    let now = Local::now().timestamp();
    if run.requested_at.is_some() {
        return verify_window(&snapshot, run, now);
    }
    let decision = decide(&snapshot, run, now)?;
    let already_confirmed = {
        let scheduler = app.state::<Scheduler>();
        let data = scheduler
            .data
            .lock()
            .map_err(|_| Failure("storageError", false))?;
        data.runs
            .iter()
            .any(|previous| same_confirmed_window(previous, run, snapshot.session.resets_at, now))
    };
    if already_confirmed && matches!(&decision, Decision::Warm) {
        run.baseline_reset = Some(snapshot.session.resets_at);
        run.verified_reset = Some(snapshot.session.resets_at);
        return Ok(("ready", "existingWindow"));
    }
    match decision {
        Decision::Ready => {
            run.baseline_reset = Some(snapshot.session.resets_at);
            run.verified_reset = Some(snapshot.session.resets_at);
            return Ok(("ready", "existingWindow"));
        }
        Decision::Wait => return Ok(("waiting", "waitingReset")),
        Decision::Warm => {}
    }
    let prepared = client.prepare_warmup(&mut run.request_evidence).await?;
    let now = Local::now().timestamp();
    if now >= run.deadline {
        return Err(Failure("waitExpired", false));
    }
    run.requested_at = Some(now);
    run.request_error = None;
    run.request_evidence.stage = "startingTurn".into();
    run.baseline_reset = Some(snapshot.session.resets_at);
    run.baseline_used = snapshot.session.used_percent;
    run.status = "warming".into();
    run.detail = "warming".into();
    run.updated_at = now;
    {
        let scheduler = app.state::<Scheduler>();
        let mut data = scheduler
            .data
            .lock()
            .map_err(|_| Failure("storageError", false))?;
        let current = data
            .runs
            .iter_mut()
            .find(|r| r.id == run.id && r.active())
            .ok_or(Failure("cancelled", false))?;
        *current = run.clone();
        persist(&app.state::<AppDatabase>(), &data).map_err(|_| Failure("storageError", false))?;
        // A separate insert-only claim also protects against two Halowake instances.
        let db = app.state::<AppDatabase>();
        let key = format!("codex_quota_dispatch:{}:{}", run.account_id, run.id);
        let inserted = db.connect().map_err(|_| Failure("storageError",false))?
            .execute("INSERT OR IGNORE INTO app_metadata(key,value,updated_at) VALUES(?1,'dispatched',?2)",rusqlite::params![key,now * 1000])
            .map_err(|_| Failure("storageError",false))?;
        if inserted == 0 {
            return Err(Failure("duplicateRequest", false));
        }
        // Dispatch receipts are only needed beyond the maximum catch-up/restart horizon.
        db.connect().map_err(|_| Failure("storageError",false))?.execute(
            "DELETE FROM app_metadata WHERE key LIKE 'codex_quota_dispatch:%' AND updated_at < ?1",[now.saturating_sub(7 * 86400) * 1000])
            .map_err(|_| Failure("storageError",false))?;
    }
    // The marker is persisted BEFORE dispatch. Even timeout/crash cannot cause a second request.
    let result = client.warm(prepared, &mut run.request_evidence).await;
    run.request_finished_at = Some(Local::now().timestamp());
    run.request_completed = result.is_ok();
    if let Err(error) = result {
        run.request_error = Some(error.0.into());
    }
    match result {
        Ok(()) => {
            let after = client.snapshot().await?;
            verify_window(&after, run, Local::now().timestamp())
        }
        Err(Failure("requestTimeout" | "connectionError", _)) => Ok(("verifying", "verifying")),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex::Window;

    fn snapshot(reset: i64, used: f64) -> Snapshot {
        Snapshot {
            session: Window {
                used_percent: used,
                window_duration_mins: 300,
                resets_at: reset,
            },
            weekly: None,
            allowed: None,
        }
    }

    #[test]
    fn validates_times_and_bounds() {
        for time in ["8:00", "24:00", "08:60", "xx:00", "08:0é"] {
            assert!(parse_time(time).is_none());
        }
        assert_eq!(parse_time("08:00"), Some((8, 0)));
        let mut config = Settings::default();
        config.check_interval_seconds = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn waits_for_old_window_and_accepts_user_started_window() {
        let node = new_run("node".into(), 20000, 21800, "account".into());
        assert!(matches!(
            decide(&snapshot(20180, 80.0), &node, 20000),
            Ok(Decision::Wait)
        ));
        assert!(matches!(
            decide(&snapshot(38000, 1.0), &node, 20000),
            Ok(Decision::Ready)
        ));
        assert!(matches!(
            decide(&snapshot(38000, 0.0), &node, 20000),
            Ok(Decision::Warm)
        ));
    }

    #[test]
    fn weekly_block_and_server_denial_prevent_warmup() {
        let node = new_run("node".into(), 20000, 21800, "account".into());
        let mut state = snapshot(38000, 0.0);
        state.weekly = Some(Window {
            used_percent: 100.0,
            window_duration_mins: 10080,
            resets_at: 99999,
        });
        assert_eq!(
            decide(&state, &node, 20000).err().unwrap().0,
            "weeklyLimited"
        );
        state.weekly = None;
        state.allowed = Some(false);
        assert_eq!(
            decide(&state, &node, 20000).err().unwrap().0,
            "quotaBlocked"
        );
    }

    #[test]
    fn expired_timestamp_with_exhausted_quota_still_waits_for_server() {
        let node = new_run("node".into(), 20000, 21800, "account".into());
        assert!(matches!(
            decide(&snapshot(19999, 100.0), &node, 20000),
            Ok(Decision::Wait)
        ));
    }

    #[test]
    fn active_manual_run_is_not_replaced_by_scheduled_node() {
        let now = Local::now();
        let at = Local
            .from_local_datetime(&now.date_naive().and_hms_opt(13, 0, 0).unwrap())
            .earliest()
            .unwrap();
        let mut data = SchedulerData::default();
        data.settings.enabled = true;
        data.settings.account_id = Some("account".into());
        data.settings.weekdays = vec![1, 2, 3, 4, 5, 6, 7];
        data.settings.times = vec!["13:00".into()];
        data.runs.push(new_run(
            "manual:1".into(),
            at.timestamp(),
            at.timestamp() + 1800,
            "account".into(),
        ));
        enqueue(&mut data, at + chrono::Duration::seconds(1));
        assert_eq!(data.runs.len(), 1);
        assert_eq!(data.runs[0].id, "manual:1");
    }

    #[test]
    fn morning_confirmation_cannot_satisfy_the_afternoon_slot() {
        let mut morning = new_run("morning".into(), 28800, 30600, "account".into());
        morning.status = "success".into();
        morning.baseline_reset = Some(46980); // 08:03 start, 13:03 reset.
        morning.verified_reset = Some(46980);
        let afternoon = new_run("afternoon".into(), 46800, 48600, "account".into());
        assert!(!same_confirmed_window(&morning, &afternoon, 46980, 46800));
        let nearby = new_run("nearby".into(), 28920, 30720, "account".into());
        assert!(same_confirmed_window(&morning, &nearby, 46980, 29000));
    }

    #[test]
    fn slot_is_deduplicated_and_expired_slots_are_not_replayed() {
        let now = Local::now();
        let at = Local
            .from_local_datetime(&now.date_naive().and_hms_opt(13, 0, 0).unwrap())
            .earliest()
            .unwrap();
        let mut data = SchedulerData::default();
        data.settings.enabled = true;
        data.settings.account_id = Some("account".into());
        data.settings.weekdays = vec![1, 2, 3, 4, 5, 6, 7];
        data.settings.times = vec!["13:00".into()];
        enqueue(&mut data, at + chrono::Duration::seconds(1));
        enqueue(&mut data, at + chrono::Duration::seconds(30));
        assert_eq!(data.runs.len(), 1);
        let mut expired = SchedulerData {
            settings: data.settings.clone(),
            runs: vec![],
        };
        enqueue(&mut expired, at + chrono::Duration::minutes(31));
        assert!(expired.runs.is_empty());
    }

    fn completed_run() -> Run {
        let mut run = new_run("node".into(), 20000, 21800, "account".into());
        run.requested_at = Some(20004);
        run.request_finished_at = Some(20010);
        run.request_completed = true;
        run.baseline_reset = Some(38004);
        run.request_evidence.stage = "completed".into();
        run.request_evidence.reply_received = true;
        run
    }

    #[test]
    fn regression_oct22_timeout_and_sliding_reset_cannot_confirm() {
        let mut run = completed_run();
        run.request_completed = false;
        run.request_evidence = WarmupEvidence::default();
        assert!(!window_confirmed(&snapshot(38103, 0.0), &run, 20104));
        assert!(!window_confirmed(&snapshot(38103, 1.0), &run, 20104));
        assert_eq!(
            verify_window(&snapshot(38103, 0.0), &mut run, 20104)
                .unwrap()
                .0,
            "verifying"
        );
        assert!(run.verified_reset.is_none());
    }

    #[test]
    fn completed_request_with_sliding_zero_usage_window_stays_unconfirmed() {
        let mut run = completed_run();
        for now in [20010, 20040, 20070, 20100, 20130, 20160] {
            assert_eq!(
                verify_window(&snapshot(now + 18000, 0.0), &mut run, now)
                    .unwrap()
                    .0,
                "verifying"
            );
        }
        assert_eq!(
            verify_window(&snapshot(38190, 0.0), &mut run, 20190)
                .err()
                .unwrap()
                .0,
            "unconfirmed"
        );
    }

    #[test]
    fn positive_quota_usage_requires_a_completed_inference_reply() {
        let mut run = completed_run();
        run.request_evidence.reply_received = false;
        assert!(!window_confirmed(&snapshot(38010, 1.0), &run, 20040));
        run.request_evidence.output_tokens = Some(2);
        assert!(window_confirmed(&snapshot(38010, 1.0), &run, 20040));
    }

    #[test]
    fn rounded_zero_usage_requires_an_observed_fixed_countdown() {
        let mut run = completed_run();
        assert_eq!(
            verify_window(&snapshot(38010, 0.0), &mut run, 20015)
                .unwrap()
                .0,
            "verifying"
        );
        assert_eq!(
            verify_window(&snapshot(38010, 0.0), &mut run, 20045)
                .unwrap()
                .0,
            "verifying"
        );
        assert_eq!(
            verify_window(&snapshot(38010, 0.0), &mut run, 20075)
                .unwrap()
                .0,
            "success"
        );
        assert_eq!(run.baseline_reset, Some(38004));
        assert_eq!(run.verified_reset, Some(38010));
    }

    #[test]
    fn repairs_legacy_success_without_replaying_or_changing_the_schedule() {
        let mut run = completed_run();
        run.verification_version = 0;
        run.status = "success".into();
        run.request_completed = false;
        let mut data = SchedulerData {
            settings: Settings::default(),
            runs: vec![run],
        };
        let times = data.settings.times.clone();
        assert!(repair_legacy_confirmations(&mut data));
        assert_eq!(data.runs[0].status, "failed");
        assert_eq!(data.runs[0].detail, "legacyUnconfirmed");
        assert_eq!(data.runs[0].requested_at, Some(20004));
        assert_eq!(data.settings.times, times);
        assert!(!data.runs[0].active());
        assert!(!repair_legacy_confirmations(&mut data));
    }
}
