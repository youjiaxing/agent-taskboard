use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::{Language, LaunchEnvironment};

pub const GROK_BUILD_ID: &str = "grok-build";
pub const GROK_BUILD_NAME: &str = "Grok Build";
pub const GROK_BIN: &str = "grok";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeResult {
    Found {
        executable: PathBuf,
    },
    Missing {
        command: String,
        searched_path: String,
        known_locations: Vec<PathBuf>,
    },
}

pub trait AgentPort: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn bin(&self) -> &str;
    fn known_install_locations(&self) -> Vec<PathBuf>;
    fn probe(&self, env: &LaunchEnvironment) -> ProbeResult;
    fn assemble_argv(&self, executable: &Path) -> Vec<String>;
    fn recent_action(&self) -> Option<String> {
        None
    }
}

#[derive(Debug, Clone)]
pub struct GrokAdapter;

impl GrokAdapter {
    pub fn known_location() -> Option<PathBuf> {
        home_dir().map(|home| home.join(".grok").join("bin"))
    }
}

impl AgentPort for GrokAdapter {
    fn id(&self) -> &str {
        GROK_BUILD_ID
    }

    fn name(&self) -> &str {
        GROK_BUILD_NAME
    }

    fn bin(&self) -> &str {
        GROK_BIN
    }

    fn known_install_locations(&self) -> Vec<PathBuf> {
        Self::known_location().into_iter().collect()
    }

    fn probe(&self, env: &LaunchEnvironment) -> ProbeResult {
        probe_binary(self.bin(), env, &self.known_install_locations())
    }

    fn assemble_argv(&self, executable: &Path) -> Vec<String> {
        vec![executable.to_string_lossy().into_owned()]
    }
}

#[derive(Debug)]
pub struct MemoryAgent {
    id: String,
    name: String,
    bin: String,
    executable: PathBuf,
    known_locations: Vec<PathBuf>,
    installed: Mutex<bool>,
    recent_action: Mutex<Option<String>>,
}

impl MemoryAgent {
    pub fn installed_grok() -> Self {
        Self {
            id: GROK_BUILD_ID.into(),
            name: GROK_BUILD_NAME.into(),
            bin: GROK_BIN.into(),
            executable: PathBuf::from("/mem/grok"),
            known_locations: vec![PathBuf::from("/mem/.grok/bin")],
            installed: Mutex::new(true),
            recent_action: Mutex::new(None),
        }
    }

    pub fn missing_grok() -> Self {
        let agent = Self::installed_grok();
        agent.set_installed(false);
        agent
    }

    pub fn set_installed(&self, installed: bool) {
        *self.installed.lock().expect("memory agent") = installed;
    }

    pub fn set_recent_action(&self, action: Option<String>) {
        *self.recent_action.lock().expect("memory agent") = action;
    }
}

impl AgentPort for MemoryAgent {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn bin(&self) -> &str {
        &self.bin
    }

    fn known_install_locations(&self) -> Vec<PathBuf> {
        self.known_locations.clone()
    }

    fn probe(&self, env: &LaunchEnvironment) -> ProbeResult {
        if *self.installed.lock().expect("memory agent") {
            ProbeResult::Found {
                executable: self.executable.clone(),
            }
        } else {
            ProbeResult::Missing {
                command: self.bin.clone(),
                searched_path: env.path_raw(),
                known_locations: self.known_locations.clone(),
            }
        }
    }

    fn assemble_argv(&self, executable: &Path) -> Vec<String> {
        vec![executable.to_string_lossy().into_owned()]
    }

    fn recent_action(&self) -> Option<String> {
        self.recent_action.lock().expect("memory agent").clone()
    }
}

pub fn probe_binary(bin: &str, env: &LaunchEnvironment, known: &[PathBuf]) -> ProbeResult {
    let mut dirs = known.to_vec();
    dirs.extend(env.path_dirs());
    for dir in &dirs {
        if let Some(found) = executable_in(dir, bin) {
            return ProbeResult::Found { executable: found };
        }
    }
    ProbeResult::Missing {
        command: bin.to_string(),
        searched_path: env.path_raw(),
        known_locations: known.to_vec(),
    }
}

pub fn format_not_found(
    language: Language,
    command: &str,
    searched_path: &str,
    known_locations: &[PathBuf],
) -> String {
    let known = if known_locations.is_empty() {
        "—".to_string()
    } else {
        known_locations
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(path_sep())
    };
    let searched = if searched_path.is_empty() {
        "—"
    } else {
        searched_path
    };
    match language {
        Language::ZhCn => {
            format!("找不到 {command}。\n已搜 PATH：{searched}\n已知安装位置：{known}")
        }
        Language::En => format!(
            "Could not find {command}.\nSearched PATH: {searched}\nKnown install locations: {known}"
        ),
    }
}

pub fn prepare_launch_env(
    mut env: LaunchEnvironment,
    host_path_prefix: &[PathBuf],
    known_locations: &[PathBuf],
) -> LaunchEnvironment {
    let mut prepend = Vec::new();
    prepend.extend(host_path_prefix.iter().cloned());
    prepend.extend(known_locations.iter().cloned());
    env.prepend_path_dirs(&prepend);
    env.pin_term();
    env
}

fn executable_in(dir: &Path, bin: &str) -> Option<PathBuf> {
    let candidate = dir.join(bin);
    if is_executable(&candidate) {
        return Some(candidate);
    }
    #[cfg(windows)]
    {
        let exe = dir.join(format!("{bin}.exe"));
        if is_executable(&exe) {
            return Some(exe);
        }
    }
    None
}

fn is_executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path)
            .map(|meta| meta.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn path_sep() -> &'static str {
    if cfg!(windows) {
        ";"
    } else {
        ":"
    }
}

pub fn path_from_dirs(dirs: &[PathBuf], existing: &str) -> String {
    let mut parts = dirs
        .iter()
        .map(|dir| dir.to_string_lossy().into_owned())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if !existing.is_empty() {
        parts.push(existing.to_string());
    }
    parts.join(path_sep())
}

pub fn split_path(raw: &str) -> Vec<PathBuf> {
    raw.split(if cfg!(windows) { ';' } else { ':' })
        .filter(|part| !part.is_empty())
        .map(PathBuf::from)
        .collect()
}

pub fn env_map_with_path(path: &str) -> BTreeMap<String, String> {
    let mut vars = BTreeMap::new();
    vars.insert("PATH".into(), path.to_string());
    vars
}
