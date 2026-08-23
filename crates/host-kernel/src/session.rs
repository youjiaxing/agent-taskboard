use std::collections::BTreeMap;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

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
    fn was_stopped(&self) -> bool {
        false
    }
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
    pulse: Condvar,
}

impl MemorySession {
    pub fn new() -> Self {
        Self {
            output: Mutex::new(Vec::new()),
            exit: Mutex::new(None),
            stopped: AtomicBool::new(false),
            pulse: Condvar::new(),
        }
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

    pub fn stopped(&self) -> bool {
        self.stopped.load(Ordering::SeqCst)
    }
}

impl AgentSession for MemorySession {
    fn write(&self, data: &[u8]) -> io::Result<()> {
        if self.exit_code().is_some() {
            return Err(io::Error::other("run has ended"));
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
    exit: Arc<Mutex<Option<i32>>>,
    pulse: Arc<Condvar>,
    writer: Mutex<Box<dyn Write + Send>>,
    master: Mutex<Box<dyn portable_pty::MasterPty + Send>>,
    child: Mutex<Box<dyn portable_pty::Child + Send + Sync>>,
    stopped: AtomicBool,
}

impl PtyLive {
    fn spawn(request: SpawnRequest) -> Result<Arc<Self>, String> {
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
                rows: request.rows.max(2),
                cols: request.cols.max(2),
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
        let exit = Arc::new(Mutex::new(None));
        let pulse = Arc::new(Condvar::new());
        let session = Arc::new(Self {
            output: Arc::clone(&output),
            exit: Arc::clone(&exit),
            pulse: Arc::clone(&pulse),
            writer: Mutex::new(writer),
            master: Mutex::new(pair.master),
            child: Mutex::new(child),
            stopped: AtomicBool::new(false),
        });
        let reader_out = Arc::clone(&output);
        let reader_pulse = Arc::clone(&pulse);
        std::thread::Builder::new()
            .name("run-pty-reader".into())
            .spawn(move || {
                let mut buf = [0u8; 4096];
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
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

impl AgentSession for PtyLive {
    fn write(&self, data: &[u8]) -> io::Result<()> {
        self.writer
            .lock()
            .map_err(|_| io::Error::other("pty writer"))?
            .write_all(data)
    }

    fn resize(&self, cols: u16, rows: u16) {
        if let Ok(master) = self.master.lock() {
            let _ = master.resize(portable_pty::PtySize {
                rows: rows.max(2),
                cols: cols.max(2),
                pixel_width: 0,
                pixel_height: 0,
            });
        }
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
