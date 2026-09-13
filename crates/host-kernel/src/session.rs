use std::collections::BTreeMap;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

mod output;
use output::{bounded_size, TerminalOutput};

const ACTION_REQUIRED_TITLE: &[u8] = b"]0;[ ! ] Action Required |";

fn terminal_requests_action(probe: &mut Vec<u8>, chunk: &[u8]) -> bool {
    probe.extend_from_slice(chunk);
    let found = probe
        .windows(ACTION_REQUIRED_TITLE.len())
        .any(|window| window == ACTION_REQUIRED_TITLE);
    let keep = ACTION_REQUIRED_TITLE.len().saturating_sub(1);
    if probe.len() > keep {
        probe.drain(..probe.len() - keep);
    }
    found
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnRequest {
    pub argv: Vec<String>,
    pub cwd: PathBuf,
    pub env: BTreeMap<String, String>,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtyChunk {
    pub offset: usize,
    pub data: Vec<u8>,
    pub exit_code: Option<i32>,
}

pub trait AgentSession: Send + Sync {
    fn write(&self, data: &[u8]) -> io::Result<()>;
    fn resize(&self, cols: u16, rows: u16);
    fn stop(&self);
    fn exit_code(&self) -> Option<i32>;
    fn read_after(&self, after: usize, wait: Duration) -> PtyChunk;
    /// Read-only text for lightweight clients; control sequences stay in the PTY stream.
    fn recent_output(&self) -> String {
        readable_pty_output(&self.read_after(0, Duration::ZERO).data)
    }
    fn was_stopped(&self) -> bool {
        false
    }
    fn waiting_for_user(&self) -> bool {
        self.completion_signals().waiting_for_user
    }
    fn completion_signals(&self) -> crate::agent::CompletionSignals {
        crate::agent::CompletionSignals::default()
    }
}

pub(crate) fn readable_pty_output(bytes: &[u8]) -> String {
    let mut parser =
        TerminalOutput::new(crate::run::DEFAULT_PTY_COLS, crate::run::DEFAULT_PTY_ROWS);
    parser.process(bytes);
    parser.contents()
}

pub trait SessionFactory: Send + Sync {
    fn spawn(&self, request: SpawnRequest) -> Result<Arc<dyn AgentSession>, String>;
}

#[derive(Debug)]
pub struct MemorySessionFactory {
    fail: Mutex<Option<String>>,
    last: Mutex<Option<SpawnRequest>>,
    spawn_count: Mutex<usize>,
    live: Mutex<Vec<Arc<MemorySession>>>,
}

impl MemorySessionFactory {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            fail: Mutex::new(None),
            last: Mutex::new(None),
            spawn_count: Mutex::new(0),
            live: Mutex::new(Vec::new()),
        })
    }

    pub fn fail_next(&self, message: impl Into<String>) {
        *self.fail.lock().expect("memory sessions") = Some(message.into());
    }

    pub fn last_spawn(&self) -> Option<SpawnRequest> {
        self.last.lock().expect("memory sessions").clone()
    }

    pub fn spawn_count(&self) -> usize {
        *self.spawn_count.lock().expect("memory sessions")
    }

    pub fn last_session(&self) -> Option<Arc<MemorySession>> {
        self.live.lock().expect("memory sessions").last().cloned()
    }
}

impl SessionFactory for MemorySessionFactory {
    fn spawn(&self, request: SpawnRequest) -> Result<Arc<dyn AgentSession>, String> {
        *self.spawn_count.lock().expect("memory sessions") += 1;
        *self.last.lock().expect("memory sessions") = Some(request.clone());
        if let Some(message) = self.fail.lock().expect("memory sessions").take() {
            return Err(message);
        }
        let session = Arc::new(MemorySession::new());
        self.live
            .lock()
            .expect("memory sessions")
            .push(Arc::clone(&session));
        Ok(session)
    }
}

#[derive(Debug)]
pub struct MemorySession {
    output: Mutex<Vec<u8>>,
    exit: Mutex<Option<i32>>,
    stopped: AtomicBool,
    waiting: AtomicBool,
    session_end: AtomicBool,
    stop_failure: AtomicBool,
    write_fail: Mutex<Option<String>>,
    pulse: Condvar,
}

impl MemorySession {
    pub fn new() -> Self {
        Self {
            output: Mutex::new(Vec::new()),
            exit: Mutex::new(None),
            stopped: AtomicBool::new(false),
            waiting: AtomicBool::new(false),
            session_end: AtomicBool::new(false),
            stop_failure: AtomicBool::new(false),
            write_fail: Mutex::new(None),
            pulse: Condvar::new(),
        }
    }

    pub fn set_waiting(&self, waiting: bool) {
        self.waiting.store(waiting, Ordering::SeqCst);
    }

    pub fn set_session_end(&self, value: bool) {
        self.session_end.store(value, Ordering::SeqCst);
    }

    pub fn set_stop_failure(&self, value: bool) {
        self.stop_failure.store(value, Ordering::SeqCst);
    }

    pub fn fail_next_write(&self, message: impl Into<String>) {
        *self.write_fail.lock().expect("memory session") = Some(message.into());
    }

    pub fn push_output(&self, bytes: &[u8]) {
        self.output
            .lock()
            .expect("memory session")
            .extend_from_slice(bytes);
        self.pulse.notify_all();
    }

    pub fn finish(&self, code: i32) {
        *self.exit.lock().expect("memory session") = Some(code);
        self.pulse.notify_all();
    }

    /// Simulate the PTY channel disappearing without a user-requested stop.
    pub fn disconnect(&self) {
        self.finish(1);
    }

    pub fn stopped(&self) -> bool {
        self.stopped.load(Ordering::SeqCst)
    }
}

impl AgentSession for MemorySession {
    fn write(&self, data: &[u8]) -> io::Result<()> {
        if self.exit_code().is_some() {
            return Err(io::Error::other("run has ended"));
        }
        if let Some(message) = self.write_fail.lock().expect("memory session").take() {
            return Err(io::Error::other(message));
        }
        self.push_output(data);
        Ok(())
    }

    fn resize(&self, _cols: u16, _rows: u16) {}

    fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
        self.finish(1);
    }

    fn exit_code(&self) -> Option<i32> {
        *self.exit.lock().expect("memory session")
    }

    fn was_stopped(&self) -> bool {
        self.stopped.load(Ordering::SeqCst)
    }

    fn waiting_for_user(&self) -> bool {
        self.waiting.load(Ordering::SeqCst)
    }

    fn completion_signals(&self) -> crate::agent::CompletionSignals {
        crate::agent::CompletionSignals {
            session_end: self.session_end.load(Ordering::SeqCst),
            stop_failure: self.stop_failure.load(Ordering::SeqCst),
            waiting_for_user: self.waiting.load(Ordering::SeqCst),
        }
    }

    fn read_after(&self, after: usize, wait: Duration) -> PtyChunk {
        let mut output = self.output.lock().expect("memory session");
        if output.len() <= after
            && self.exit.lock().expect("memory session").is_none()
            && !wait.is_zero()
        {
            let (guard, _) = self
                .pulse
                .wait_timeout(output, wait)
                .expect("memory session wait");
            output = guard;
        }
        let offset = output.len();
        let data = if after < offset {
            output[after..].to_vec()
        } else {
            Vec::new()
        };
        PtyChunk {
            offset,
            data,
            exit_code: *self.exit.lock().expect("memory session"),
        }
    }
}

pub struct PtySessionFactory;

impl SessionFactory for PtySessionFactory {
    fn spawn(&self, request: SpawnRequest) -> Result<Arc<dyn AgentSession>, String> {
        PtyLive::spawn(request).map(|session| session as Arc<dyn AgentSession>)
    }
}

struct PtyLive {
    output: Arc<Mutex<Vec<u8>>>,
    screen: Arc<Mutex<TerminalOutput>>,
    exit: Arc<Mutex<Option<i32>>>,
    pulse: Arc<Condvar>,
    writer: Mutex<Box<dyn Write + Send>>,
    master: Mutex<Box<dyn portable_pty::MasterPty + Send>>,
    child: Mutex<Box<dyn portable_pty::Child + Send + Sync>>,
    stopped: AtomicBool,
    waiting: Arc<AtomicBool>,
    hook_sink: Option<PathBuf>,
}

impl PtyLive {
    fn spawn(request: SpawnRequest) -> Result<Arc<Self>, String> {
        let hook_sink = request
            .env
            .get("AGENT_TASKBOARD_HOOK_SINK")
            .map(PathBuf::from);
        let (cols, rows) = bounded_size(request.cols, request.rows);
        let program = request
            .argv
            .first()
            .cloned()
            .ok_or_else(|| "missing executable".to_string())?;
        if !PathBuf::from(&program).is_absolute() {
            return Err("agent executable must be an absolute path".into());
        }
        let pty_system = portable_pty::native_pty_system();
        let pair = pty_system
            .openpty(portable_pty::PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|err| err.to_string())?;
        let mut cmd = portable_pty::CommandBuilder::new(&program);
        for arg in request.argv.iter().skip(1) {
            cmd.arg(arg);
        }
        cmd.cwd(&request.cwd);
        cmd.env_clear();
        for (key, value) in &request.env {
            cmd.env(key, value);
        }
        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|err| err.to_string())?;
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|err| err.to_string())?;
        let writer = pair.master.take_writer().map_err(|err| err.to_string())?;
        let output = Arc::new(Mutex::new(Vec::new()));
        let screen = Arc::new(Mutex::new(TerminalOutput::new(cols, rows)));
        let exit = Arc::new(Mutex::new(None));
        let pulse = Arc::new(Condvar::new());
        let waiting = Arc::new(AtomicBool::new(false));
        let session = Arc::new(Self {
            output: Arc::clone(&output),
            screen: Arc::clone(&screen),
            exit: Arc::clone(&exit),
            pulse: Arc::clone(&pulse),
            writer: Mutex::new(writer),
            master: Mutex::new(pair.master),
            child: Mutex::new(child),
            stopped: AtomicBool::new(false),
            waiting: Arc::clone(&waiting),
            hook_sink,
        });
        let reader_out = Arc::clone(&output);
        let reader_pulse = Arc::clone(&pulse);
        std::thread::Builder::new()
            .name("run-pty-reader".into())
            .spawn(move || {
                let mut buf = [0u8; 4096];
                let mut waiting_probe = Vec::with_capacity(ACTION_REQUIRED_TITLE.len() * 2);
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            if terminal_requests_action(&mut waiting_probe, &buf[..n]) {
                                waiting.store(true, Ordering::SeqCst);
                            }
                            screen.lock().expect("pty screen").process(&buf[..n]);
                            reader_out
                                .lock()
                                .expect("pty output")
                                .extend_from_slice(&buf[..n]);
                            reader_pulse.notify_all();
                        }
                        Err(_) => break,
                    }
                }
            })
            .map_err(|err| err.to_string())?;
        let waiter = Arc::clone(&session);
        std::thread::Builder::new()
            .name("run-pty-wait".into())
            .spawn(move || {
                let code = loop {
                    let polled = waiter
                        .child
                        .lock()
                        .ok()
                        .and_then(|mut child| child.try_wait().ok());
                    match polled {
                        Some(Some(status)) => break status.exit_code() as i32,
                        Some(None) => std::thread::sleep(Duration::from_millis(30)),
                        None => break 1,
                    }
                };
                *waiter.exit.lock().expect("pty exit") = Some(code);
                waiter.pulse.notify_all();
            })
            .map_err(|err| err.to_string())?;
        Ok(session)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_required_title_is_detected_across_pty_chunks() {
        let mut probe = Vec::new();
        assert!(!terminal_requests_action(
            &mut probe,
            b"\x1b]0;[ ! ] Action Req"
        ));
        assert!(terminal_requests_action(
            &mut probe,
            b"uired | agent-taskboard\x07"
        ));
    }
}

impl AgentSession for PtyLive {
    fn write(&self, data: &[u8]) -> io::Result<()> {
        self.waiting.store(false, Ordering::SeqCst);
        if let Some(sink) = &self.hook_sink {
            let _ = std::fs::remove_file(sink.join("waiting-for-user"));
        }
        self.writer
            .lock()
            .map_err(|_| io::Error::other("pty writer"))?
            .write_all(data)
    }

    fn resize(&self, cols: u16, rows: u16) {
        let (cols, rows) = bounded_size(cols, rows);
        if let Ok(master) = self.master.lock() {
            if master
                .resize(portable_pty::PtySize {
                    rows,
                    cols,
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .is_ok()
            {
                self.screen.lock().expect("pty screen").resize(cols, rows);
            }
        }
    }

    fn recent_output(&self) -> String {
        self.screen.lock().expect("pty screen").contents()
    }

    fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
        }
    }

    fn exit_code(&self) -> Option<i32> {
        *self.exit.lock().expect("pty exit")
    }

    fn was_stopped(&self) -> bool {
        self.stopped.load(Ordering::SeqCst)
    }

    fn waiting_for_user(&self) -> bool {
        self.waiting.load(Ordering::SeqCst) || self.completion_signals().waiting_for_user
    }

    fn completion_signals(&self) -> crate::agent::CompletionSignals {
        self.hook_sink
            .as_deref()
            .map(crate::agent::read_completion_signals)
            .unwrap_or_default()
    }

    fn read_after(&self, after: usize, wait: Duration) -> PtyChunk {
        let mut output = self.output.lock().expect("pty output");
        if output.len() <= after && self.exit.lock().expect("pty exit").is_none() && !wait.is_zero()
        {
            let (guard, _) = self.pulse.wait_timeout(output, wait).expect("pty wait");
            output = guard;
        }
        let offset = output.len();
        let data = if after < offset {
            output[after..].to_vec()
        } else {
            Vec::new()
        };
        PtyChunk {
            offset,
            data,
            exit_code: *self.exit.lock().expect("pty exit"),
        }
    }
}
