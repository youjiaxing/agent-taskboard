use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CompletionSignals {
    pub session_end: bool,
    pub stop_failure: bool,
}

#[derive(Debug, Clone, Default)]
pub struct CompletionHookPlan {
    pub extra_argv: Vec<String>,
    pub extra_env: BTreeMap<String, String>,
}

const RECORD_SH: &str = r#"#!/bin/sh
sink="${AGENT_TASKBOARD_HOOK_SINK}"
payload=`cat 2>/dev/null`
event="$1"
if [ -z "$event" ]; then
  event="${GROK_HOOK_EVENT:-}"
fi
if [ -z "$event" ]; then
  event=`printf '%s' "$payload" | sed -n 's/.*"hookEventName"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1`
fi
if [ -z "$sink" ]; then
  exit 0
fi
mkdir -p "$sink" 2>/dev/null
case "$event" in
  *SessionEnd*|*session_end*) : > "$sink/session-end" ;;
  *StopFailure*|*stop_failure*) : > "$sink/stop-failure" ;;
esac
exit 0
"#;

const RECORD_CMD: &str = r#"@echo off
setlocal
set "SINK=%AGENT_TASKBOARD_HOOK_SINK%"
set "EVENT=%~1"
if "%SINK%"=="" exit /b 0
if not exist "%SINK%" mkdir "%SINK%" >nul 2>nul
echo %EVENT% | findstr /I "SessionEnd session_end" >nul && type nul > "%SINK%\session-end"
echo %EVENT% | findstr /I "StopFailure stop_failure" >nul && type nul > "%SINK%\stop-failure"
exit /b 0
"#;

pub fn write_recorder(sink: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(sink).map_err(|err| err.to_string())?;
    let script = if cfg!(windows) {
        let path = sink.join("record.cmd");
        fs::write(&path, RECORD_CMD).map_err(|err| err.to_string())?;
        path
    } else {
        let path = sink.join("record.sh");
        fs::write(&path, RECORD_SH).map_err(|err| err.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&path)
                .map_err(|err| err.to_string())?
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&path, perms).map_err(|err| err.to_string())?;
        }
        path
    };
    Ok(script)
}

pub fn recorder_command(script: &Path, event: &str) -> String {
    if cfg!(windows) {
        format!("\"{}\" {event}", script.display())
    } else {
        format!("\"{}\" {event}", script.display())
    }
}

pub fn read_signals(sink: &Path) -> CompletionSignals {
    CompletionSignals {
        session_end: sink.join("session-end").is_file(),
        stop_failure: sink.join("stop-failure").is_file(),
    }
}

pub fn sink_env(sink: &Path) -> BTreeMap<String, String> {
    BTreeMap::from([(
        "AGENT_TASKBOARD_HOOK_SINK".into(),
        sink.to_string_lossy().into_owned(),
    )])
}

pub fn write_json_hooks(path: &Path, recorder: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let session_end = recorder_command(recorder, "SessionEnd");
    let stop_failure = recorder_command(recorder, "StopFailure");
    let body = serde_json::json!({
        "hooks": {
            "SessionEnd": [{"hooks": [{"type": "command", "command": session_end, "timeout": 5}]}],
            "StopFailure": [{"hooks": [{"type": "command", "command": stop_failure, "timeout": 5}]}]
        }
    });
    fs::write(
        path,
        serde_json::to_vec_pretty(&body).map_err(|err| err.to_string())?,
    )
    .map_err(|err| err.to_string())
}

pub fn grok_home_overlay(sink: &Path) -> Result<PathBuf, String> {
    let overlay = sink.join("grok-home");
    fs::create_dir_all(overlay.join("hooks")).map_err(|err| err.to_string())?;
    if let Some(user) = super::home_dir().map(|home| home.join(".grok")) {
        if user.is_dir() {
            if let Ok(entries) = fs::read_dir(&user) {
                for entry in entries.flatten() {
                    let name = entry.file_name();
                    if name == "hooks" {
                        continue;
                    }
                    let dest = overlay.join(&name);
                    let _ = symlink_any(&entry.path(), &dest);
                }
            }
            let user_hooks = user.join("hooks");
            if user_hooks.is_dir() {
                if let Ok(entries) = fs::read_dir(user_hooks) {
                    for entry in entries.flatten() {
                        let dest = overlay.join("hooks").join(entry.file_name());
                        let _ = fs::copy(entry.path(), dest);
                    }
                }
            }
        }
    }
    Ok(overlay)
}

fn symlink_any(src: &Path, dest: &Path) -> std::io::Result<()> {
    if dest.exists() {
        return Ok(());
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(src, dest)
    }
    #[cfg(windows)]
    {
        if src.is_dir() {
            std::os::windows::fs::symlink_dir(src, dest)
        } else {
            std::os::windows::fs::symlink_file(src, dest)
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        if src.is_dir() {
            copy_dir(src, dest)
        } else {
            fs::copy(src, dest).map(|_| ())
        }
    }
}
