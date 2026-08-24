use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::agent::{path_from_dirs, split_path};

const ENV_MARKER: &str = "AGENT_TASKBOARD_ENV_BEGIN";
const CAPTURE_TIMEOUT: Duration = Duration::from_secs(8);
pub const PINNED_TERM: &str = "xterm-256color";
pub const PINNED_COLORTERM: &str = "truecolor";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchEnvironment {
    pub cwd: PathBuf,
    pub vars: BTreeMap<String, String>,
}

impl LaunchEnvironment {
    pub fn from_vars(cwd: PathBuf, vars: BTreeMap<String, String>) -> Self {
        Self { cwd, vars }
    }

    pub fn path_raw(&self) -> String {
        self.vars
            .get("PATH")
            .cloned()
            .or_else(|| self.vars.get("Path").cloned())
            .unwrap_or_default()
    }

    pub fn path_dirs(&self) -> Vec<PathBuf> {
        split_path(&self.path_raw())
    }

    pub fn prepend_path_dirs(&mut self, dirs: &[PathBuf]) {
        if dirs.is_empty() {
            return;
        }
        let key = if self.vars.contains_key("Path") && !self.vars.contains_key("PATH") {
            "Path"
        } else {
            "PATH"
        };
        let next = path_from_dirs(dirs, &self.path_raw());
        self.vars.insert(key.to_string(), next);
    }

    pub fn pin_term(&mut self) {
        self.vars.insert("TERM".into(), PINNED_TERM.into());
        self.vars
            .insert("COLORTERM".into(), PINNED_COLORTERM.into());
    }
}

pub trait LaunchEnvPort: Send + Sync {
    fn capture(&self, cwd: &Path) -> Result<LaunchEnvironment, String>;

    fn refresh(&self, cwd: &Path) -> Result<LaunchEnvironment, String> {
        self.capture(cwd)
    }
}

#[derive(Debug)]
pub struct MemoryLaunchEnv {
    captures: Mutex<Vec<PathBuf>>,
    by_cwd: Mutex<BTreeMap<PathBuf, LaunchEnvironment>>,
    default: Mutex<LaunchEnvironment>,
}

impl MemoryLaunchEnv {
    pub fn new() -> Self {
        Self {
            captures: Mutex::new(Vec::new()),
            by_cwd: Mutex::new(BTreeMap::new()),
            default: Mutex::new(LaunchEnvironment::from_vars(
                PathBuf::from("/"),
                crate::agent::env_map_with_path("/opt/empty"),
            )),
        }
    }

    pub fn with_path(path: &str) -> Self {
        let env = Self::new();
        env.set_default(LaunchEnvironment::from_vars(
            PathBuf::from("/"),
            crate::agent::env_map_with_path(path),
        ));
        env
    }

    pub fn set_default(&self, env: LaunchEnvironment) {
        *self.default.lock().expect("memory launch env") = env;
    }

    pub fn set(&self, cwd: PathBuf, env: LaunchEnvironment) {
        self.by_cwd
            .lock()
            .expect("memory launch env")
            .insert(cwd, env);
    }

    pub fn capture_count(&self) -> usize {
        self.captures.lock().expect("memory launch env").len()
    }

    pub fn captured_dirs(&self) -> Vec<PathBuf> {
        self.captures.lock().expect("memory launch env").clone()
    }
}

impl Default for MemoryLaunchEnv {
    fn default() -> Self {
        Self::new()
    }
}

impl LaunchEnvPort for MemoryLaunchEnv {
    fn capture(&self, cwd: &Path) -> Result<LaunchEnvironment, String> {
        self.captures
            .lock()
            .expect("memory launch env")
            .push(cwd.to_path_buf());
        if let Some(env) = self
            .by_cwd
            .lock()
            .expect("memory launch env")
            .get(cwd)
            .cloned()
        {
            return Ok(env);
        }
        let mut env = self.default.lock().expect("memory launch env").clone();
        env.cwd = cwd.to_path_buf();
        Ok(env)
    }
}

#[derive(Debug)]
pub struct ShellLaunchEnv {
    shell: PathBuf,
    timeout: Duration,
    cache: Mutex<BTreeMap<PathBuf, (Instant, LaunchEnvironment)>>,
}

impl ShellLaunchEnv {
    pub fn live() -> Self {
        Self {
            shell: default_shell(),
            timeout: CAPTURE_TIMEOUT,
            cache: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn with_shell(shell: PathBuf) -> Self {
        Self {
            shell,
            timeout: CAPTURE_TIMEOUT,
            cache: Mutex::new(BTreeMap::new()),
        }
    }
}

impl LaunchEnvPort for ShellLaunchEnv {
    fn capture(&self, cwd: &Path) -> Result<LaunchEnvironment, String> {
        if let Some(cached) = self.cached(cwd) {
            return Ok(cached);
        }
        match self.capture_fresh(cwd) {
            Ok(env) => Ok(env),
            Err(err) => self.cached_any(cwd).ok_or(err),
        }
    }

    fn refresh(&self, cwd: &Path) -> Result<LaunchEnvironment, String> {
        self.capture_fresh(cwd)
    }
}

impl ShellLaunchEnv {
    fn capture_fresh(&self, cwd: &Path) -> Result<LaunchEnvironment, String> {
        let env = capture_shell(&self.shell, cwd, self.timeout)?;
        self.cache
            .lock()
            .expect("launch env cache")
            .insert(cwd.to_path_buf(), (Instant::now(), env.clone()));
        Ok(env)
    }

    fn cached(&self, cwd: &Path) -> Option<LaunchEnvironment> {
        let cache = self.cache.lock().expect("launch env cache");
        cache
            .get(cwd)
            .and_then(|(at, env)| (at.elapsed() < Duration::from_secs(30)).then(|| env.clone()))
    }

    fn cached_any(&self, cwd: &Path) -> Option<LaunchEnvironment> {
        self.cache
            .lock()
            .expect("launch env cache")
            .get(cwd)
            .map(|(_, env)| env.clone())
    }
}

fn default_shell() -> PathBuf {
    #[cfg(windows)]
    {
        which_windows_shell()
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("SHELL")
            .map(PathBuf::from)
            .filter(|path| !path.as_os_str().is_empty())
            .or_else(user_login_shell)
            .unwrap_or_else(|| PathBuf::from("/bin/zsh"))
    }
}

#[cfg(unix)]
fn user_login_shell() -> Option<PathBuf> {
    dscl_shell().or_else(passwd_shell)
}

#[cfg(unix)]
fn dscl_shell() -> Option<PathBuf> {
    let user = std::env::var("USER").ok()?;
    let output = Command::new("dscl")
        .args([".", "-read", &format!("/Users/{user}"), "UserShell"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .split_once(':')
        .map(|(_, value)| PathBuf::from(value.trim()))
        .filter(|path| !path.as_os_str().is_empty())
}

#[cfg(unix)]
fn passwd_shell() -> Option<PathBuf> {
    let user = std::env::var("USER").ok()?;
    let raw = std::fs::read_to_string("/etc/passwd").ok()?;
    raw.lines().find_map(|line| {
        let mut parts = line.split(':');
        let name = parts.next()?;
        if name != user {
            return None;
        }
        parts.nth(5).map(PathBuf::from)
    })
}

#[cfg(windows)]
fn which_windows_shell() -> PathBuf {
    ["pwsh", "powershell"]
        .into_iter()
        .find_map(|name| which_cmd(name))
        .unwrap_or_else(|| PathBuf::from("powershell.exe"))
}

#[cfg(windows)]
fn which_cmd(name: &str) -> Option<PathBuf> {
    let output = Command::new("where").arg(name).output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .map(|line| PathBuf::from(line.trim()))
}

fn capture_shell(shell: &Path, cwd: &Path, timeout: Duration) -> Result<LaunchEnvironment, String> {
    let mut command = Command::new(shell);
    command
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        command
            .arg("-NoLogo")
            .arg("-NoProfile")
            .arg("-Command")
            .arg(windows_env_script());
    }
    #[cfg(not(windows))]
    {
        command.arg("-lic").arg(unix_env_script());
    }
    let mut child = command
        .spawn()
        .map_err(|err| format!("could not capture launch environment: {err}"))?;
    let stdout = child.stdout.take();
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut bytes = Vec::new();
                if let Some(mut out) = stdout {
                    let _ = out.read_to_end(&mut bytes);
                }
                if !status.success() && bytes.is_empty() {
                    return Err("could not capture launch environment".into());
                }
                let vars = parse_env_output(&bytes)?;
                return Ok(LaunchEnvironment::from_vars(cwd.to_path_buf(), vars));
            }
            Ok(None) => {
                if started.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("timed out capturing launch environment".into());
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(err) => return Err(format!("could not capture launch environment: {err}")),
        }
    }
}

fn unix_env_script() -> String {
    format!("printf '%s\\0' {ENV_MARKER}; env -0")
}

#[cfg(windows)]
fn windows_env_script() -> String {
    format!(
        "Write-Output '{ENV_MARKER}'; Get-ChildItem Env: | ForEach-Object {{ '{{0}}={{1}}' -f $_.Name, $_.Value }}"
    )
}

fn parse_env_output(bytes: &[u8]) -> Result<BTreeMap<String, String>, String> {
    if let Some(vars) = parse_null_env(bytes) {
        return Ok(vars);
    }
    parse_line_env(bytes)
}

fn parse_null_env(bytes: &[u8]) -> Option<BTreeMap<String, String>> {
    let marker = ENV_MARKER.as_bytes();
    let pos = bytes
        .windows(marker.len())
        .position(|window| window == marker)?;
    let rest = &bytes[pos + marker.len()..];
    let rest = rest.strip_prefix(&[0]).unwrap_or(rest);
    let mut vars = BTreeMap::new();
    for chunk in rest.split(|byte| *byte == 0) {
        if chunk.is_empty() {
            continue;
        }
        let text = String::from_utf8_lossy(chunk);
        if let Some((key, value)) = split_env_pair(text.as_ref()) {
            vars.insert(key, value);
        }
    }
    (!vars.is_empty()).then_some(vars)
}

fn parse_line_env(bytes: &[u8]) -> Result<BTreeMap<String, String>, String> {
    let text = String::from_utf8_lossy(bytes);
    let rest = text.split(ENV_MARKER).nth(1).unwrap_or(&text);
    let mut vars = BTreeMap::new();
    for line in rest.lines() {
        if let Some((key, value)) = split_env_pair(line) {
            vars.insert(key, value);
        }
    }
    if vars.is_empty() {
        Err("could not parse launch environment".into())
    } else {
        Ok(vars)
    }
}

fn split_env_pair(text: &str) -> Option<(String, String)> {
    let text = text.trim();
    if text.is_empty() || text.starts_with('#') {
        return None;
    }
    let (key, value) = text.split_once('=')?;
    if key.is_empty() || key.contains(' ') {
        return None;
    }
    Some((key.to_string(), value.to_string()))
}
