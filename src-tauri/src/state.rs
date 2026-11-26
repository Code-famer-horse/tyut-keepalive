use anyhow::Result;
use serde::Serialize;
use std::{
    sync::{atomic::{AtomicBool, Ordering}, mpsc::{self, Receiver, RecvTimeoutError, Sender}, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager};
use crate::core::{config, credentials, portal};

const WATCH_INTERVAL: Duration = Duration::from_secs(600);
const RETRY_INITIAL: Duration = Duration::from_secs(10);
const RETRY_MAX: Duration = Duration::from_secs(3600);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ConnectionState {
    #[default]
    Initializing,
    NeedSetup,
    CredentialError,
    Checking,
    LoggingIn,
    Online,
    Offline,
    InvalidCredentials,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Status {
    pub(crate) state: ConnectionState,
    pub(crate) account: String,
    pub(crate) last_check: String,
    pub(crate) last_login: String,
    pub(crate) message: String,
}

pub(crate) type SharedStatus = Arc<Mutex<Status>>;
pub(crate) type Reply = Sender<std::result::Result<(), String>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Task {
    Check,
    Login,
    Retry,
}
#[derive(Clone, Copy)]
pub(crate) enum Outcome {
    Checked(bool),
    Authenticated(portal::LoginOutcome),
    Failed,
}
pub(crate) enum Event {
    Reconnect,
    Save(config::Credentials, Reply),
    Clear(Reply),
    Finished { generation: u64, task: Task, outcome: Outcome },
}

pub(crate) struct Scheduler(Sender<Event>);
impl Scheduler {
    pub(crate) fn send(&self, event: Event) -> std::result::Result<(), String> {
        self.0.send(event).map_err(|_| "后台调度器不可用".into())
    }
}

fn now() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}
fn publish(app: &AppHandle, shared: &SharedStatus, status: &mut Status,
           state: ConnectionState, message: impl Into<String>) {
    status.state = state;
    status.message = message.into();
    if let Ok(mut guard) = shared.lock() {
        *guard = status.clone();
    }
    let _ = app.emit("status", status.clone());
}

fn due(task: Task, delay: Duration) -> Option<(Instant, Task)> {
    Some((Instant::now() + delay, task))
}
fn next_event(rx: &Receiver<Event>, deadline: Option<Instant>) -> std::result::Result<Event, RecvTimeoutError> {
    match deadline {
        Some(at) => rx.recv_timeout(at.saturating_duration_since(Instant::now())),
        None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
    }
}

fn current_result(generation: u64, current: u64, pending: bool) -> bool {
    generation == current && !pending
}

fn launch(tx: Sender<Event>, generation: u64, task: Task, value: config::Credentials, cancelled: Arc<AtomicBool>) {
    thread::spawn(move || {
        let outcome = (|| -> Result<Outcome> {
            anyhow::ensure!(!cancelled.load(Ordering::Acquire), "任务已取消");
            if matches!(task, Task::Check | Task::Retry) {
                let client = portal::client(None)?;
                let online = portal::check_network(&client);
                if task == Task::Check || online {
                    return Ok(Outcome::Checked(online));
                }
            }
            Ok(Outcome::Authenticated(portal::attempt_login(&value, None, &cancelled)?))
        })().unwrap_or(Outcome::Failed);
        // Network errors may include sensitive URLs. Only send a classified result.
        let _ = tx.send(Event::Finished { generation, task, outcome });
    });
}

pub(crate) fn start_worker(app: AppHandle, shared: SharedStatus) -> Result<()> {
    let dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&dir)?;
    let legacy = dir.join("config.enc");
    let (tx, rx) = mpsc::channel();
    app.manage(Scheduler(tx.clone()));
    thread::spawn(move || {
        let mut status = Status {
            account: "未配置".into(),
            last_login: "尚未登录".into(),
            ..Default::default()
        };
        let mut active = match credentials::load(&legacy) {
            Ok(value) => value,
            Err(error) => {
                publish(&app, &shared, &mut status, ConnectionState::CredentialError, error.to_string());
                None
            }
        };
        let mut generation = 0_u64;
        let mut retry = RETRY_INITIAL;
        let mut running: Option<(u64, Task)> = None;
        let mut cancellation: Option<Arc<AtomicBool>> = None;
        let mut next = None;
        if let Some(value) = &active {
            status.account = config::masked_account(&value.account);
            next = due(Task::Check, Duration::ZERO);
            publish(&app, &shared, &mut status, ConnectionState::Checking, "准备检查网络");
        } else if status.state != ConnectionState::CredentialError {
            publish(&app, &shared, &mut status, ConnectionState::NeedSetup, "请设置账号密码");
        }
        loop {
            // No timer or disk polling while idle; only one network job may run.
            let deadline = if running.is_none() { next.map(|(at, _)| at) } else { None };
            match next_event(&rx, deadline) {
                Err(RecvTimeoutError::Disconnected) => break,
                Err(RecvTimeoutError::Timeout) => {
                    let Some((_, task)) = next.take() else { continue; };
                    let Some(value) = active.clone() else { continue; };
                    running = Some((generation, task));
                    status.last_check = now();
                    let (state, message) = if task == Task::Check {
                        (ConnectionState::Checking, "正在检查网络")
                    } else {
                        (ConnectionState::LoggingIn, "正在尝试登录")
                    };
                    publish(&app, &shared, &mut status, state, message);
                    let cancelled = Arc::new(AtomicBool::new(false));
                    cancellation = Some(cancelled.clone());
                    launch(tx.clone(), generation, task, value, cancelled);
                }
                Ok(Event::Reconnect) => {
                    if active.is_none() {
                        // A failed reset must not reload a still-present old credential.
                        if status.state != ConnectionState::CredentialError {
                            publish(&app, &shared, &mut status, ConnectionState::NeedSetup, "请先设置凭据");
                        }
                    } else if running != Some((generation, Task::Login)) {
                        next = due(Task::Login, Duration::ZERO);
                    }
                }
                Ok(Event::Save(value, reply)) => {
                    // Invalidate results from prior credentials before touching storage.
                    if let Some(flag) = &cancellation { flag.store(true, Ordering::Release); }
                    generation = generation.wrapping_add(1);
                    active = None;
                    next = None;
                    retry = RETRY_INITIAL;
                    status.account = "未配置".into();
                    status.last_login = "尚未登录".into();
                    status.last_check.clear();
                    let result = credentials::save(&legacy, &value).map_err(|error| error.to_string());
                    match &result {
                        Ok(()) => {
                            status.account = config::masked_account(&value.account);
                            active = Some(value);
                            next = due(Task::Login, Duration::ZERO);
                            publish(&app, &shared, &mut status, ConnectionState::Checking, "凭据已保存，等待认证");
                        }
                        Err(error) => publish(&app, &shared, &mut status, ConnectionState::CredentialError, error.clone()),
                    }
                    let _ = reply.send(result);
                }
                Ok(Event::Clear(reply)) => {
                    if let Some(flag) = &cancellation { flag.store(true, Ordering::Release); }
                    generation = generation.wrapping_add(1);
                    active = None;
                    next = None;
                    retry = RETRY_INITIAL;
                    status.account = "未配置".into();
                    status.last_login = "尚未登录".into();
                    status.last_check.clear();
                    let result = credentials::clear(&legacy).map_err(|error| error.to_string());
                    match &result {
                        Ok(()) => publish(&app, &shared, &mut status, ConnectionState::NeedSetup, "凭据已清除"),
                        Err(error) => publish(&app, &shared, &mut status, ConnectionState::CredentialError, error.clone()),
                    }
                    let _ = reply.send(result);
                }
                Ok(Event::Finished { generation: completed, task, outcome }) => {
                    if running != Some((completed, task)) { continue; }
                    running = None;
                    cancellation = None;
                    // A new credential or queued manual action supersedes an old result.
                    if !current_result(completed, generation, next.is_some()) { continue; }
                    status.last_check = now();
                    match outcome {
                        Outcome::Checked(true) | Outcome::Authenticated(portal::LoginOutcome::Success) => {
                            if matches!(outcome, Outcome::Authenticated(_)) { status.last_login = now(); }
                            retry = RETRY_INITIAL;
                            next = due(Task::Check, WATCH_INTERVAL);
                            publish(&app, &shared, &mut status, ConnectionState::Online, "网络正常，10 分钟后复查");
                        }
                        Outcome::Authenticated(portal::LoginOutcome::BadPassword) => {
                            next = None;
                            publish(&app, &shared, &mut status, ConnectionState::InvalidCredentials, "密码错误，自动重试已停止，请更新凭据");
                        }
                        other => {
                            if !matches!(other, Outcome::Checked(false)) {
                                retry = (retry * 2).min(RETRY_MAX);
                            }
                            next = due(Task::Retry, retry);
                            publish(&app, &shared, &mut status, ConnectionState::Offline,
                                format!("网络异常，{} 秒后重试", retry.as_secs()));
                        }
                    }
                }
            }
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn command_interrupts_future_deadline() {
        let (tx, rx) = mpsc::channel();
        tx.send(Event::Reconnect).unwrap_or_else(|_| panic!("send failed"));
        assert!(matches!(next_event(&rx, Some(Instant::now() + WATCH_INTERVAL)), Ok(Event::Reconnect)));
    }
    #[test]
    fn old_generation_and_superseded_result_are_rejected() {
        assert!(!current_result(1, 2, false));
        assert!(!current_result(2, 2, true));
        assert!(current_result(2, 2, false));
    }
    #[test]
    fn no_deadline_waits_for_command() {
        let (tx, rx) = mpsc::channel();
        tx.send(Event::Reconnect).unwrap_or_else(|_| panic!("send failed"));
        assert!(matches!(next_event(&rx, None), Ok(Event::Reconnect)));
    }
}
