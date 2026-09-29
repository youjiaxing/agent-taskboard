use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CompletionSignals {
    pub session_end: bool,
    pub stop_failure: bool,
    pub waiting_for_user: bool,
    pub session_id: Option<String>,
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
session_id=`printf '%s' "$payload" | sed -n 's/.*"session_id"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1`
lock="$sink/hook-state.lock"
lock_acquired=false
for attempt in `seq 1 150`; do
  if mkdir "$lock" 2>/dev/null; then
    printf '%s\n' "$$" > "$lock/pid"
    lock_acquired=true
    break
  fi
  owner=`cat "$lock/pid" 2>/dev/null`
  if [ -n "$owner" ] && ! kill -0 "$owner" 2>/dev/null; then
    rm -rf "$lock" 2>/dev/null
  fi
  sleep 0.02
done
if [ "$lock_acquired" != true ]; then
  exit 0
fi
trap 'rm -rf "$lock" 2>/dev/null' EXIT
old_session_id=`sed -n 's/.*"sessionId"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$sink/hook-state.json" 2>/dev/null | head -1`
session_end=`sed -n 's/.*"sessionEnd"[[:space:]]*:[[:space:]]*\(true\|false\).*/\1/p' "$sink/hook-state.json" 2>/dev/null | head -1`
stop_failure=`sed -n 's/.*"stopFailure"[[:space:]]*:[[:space:]]*\(true\|false\).*/\1/p' "$sink/hook-state.json" 2>/dev/null | head -1`
waiting=`sed -n 's/.*"waitingForUser"[[:space:]]*:[[:space:]]*\(true\|false\).*/\1/p' "$sink/hook-state.json" 2>/dev/null | head -1`
[ -n "$session_end" ] || session_end=false
[ -n "$stop_failure" ] || stop_failure=false
[ -n "$waiting" ] || waiting=false
case "$event" in
  *PermissionRequest*|*permission_request*) waiting=true ;;
  *UserPromptSubmit*|*user_prompt_submit*) waiting=false ;;
  *SessionEnd*|*session_end*) waiting=false; session_end=true ;;
  *StopFailure*|*stop_failure*) waiting=false; stop_failure=true ;;
esac
[ -n "$session_id" ] || session_id="$old_session_id"
tmp="$sink/hook-state.json.tmp.$$"
if [ -n "$session_id" ]; then
  printf '{"sessionEnd":%s,"stopFailure":%s,"waitingForUser":%s,"sessionId":"%s"}\n' "$session_end" "$stop_failure" "$waiting" "$session_id" > "$tmp"
else
  printf '{"sessionEnd":%s,"stopFailure":%s,"waitingForUser":%s}\n' "$session_end" "$stop_failure" "$waiting" > "$tmp"
fi
mv -f "$tmp" "$sink/hook-state.json"
case "$event" in
  *PermissionRequest*|*permission_request*) : > "$sink/waiting-for-user" ;;
  *UserPromptSubmit*|*user_prompt_submit*|*SessionEnd*|*session_end*|*StopFailure*|*stop_failure*) rm -f "$sink/waiting-for-user" ;;
  *SessionEnd*|*session_end*) : > "$sink/session-end" ;;
  *StopFailure*|*stop_failure*) : > "$sink/stop-failure" ;;
esac
exit 0
"#;

const RECORD_CMD: &str = r#"@echo off
powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "%~dp0record.ps1" "%~1"
exit /b %ERRORLEVEL%
"#;

const RECORD_PS1: &str = r#"$ErrorActionPreference = 'SilentlyContinue'
$sink = $env:AGENT_TASKBOARD_HOOK_SINK
if ([string]::IsNullOrWhiteSpace($sink)) { exit 0 }
New-Item -ItemType Directory -Force -Path $sink | Out-Null
$payload = [Console]::In.ReadToEnd()
$event = $args[0]
$parsed = $null
try { $parsed = $payload | ConvertFrom-Json } catch {}
$lock = Join-Path $sink 'hook-state.lock'
$lockAcquired = $false
for ($attempt = 0; $attempt -lt 150; $attempt++) {
  try {
    New-Item -ItemType Directory -Path $lock -ErrorAction Stop | Out-Null
    Set-Content -Path (Join-Path $lock 'pid') -Value $PID -Encoding ASCII
    $lockAcquired = $true
    break
  } catch {
    $ownerPath = Join-Path $lock 'pid'
    if (Test-Path $ownerPath) {
      $owner = 0
      [int]::TryParse((Get-Content $ownerPath -Raw), [ref]$owner) | Out-Null
      if ($owner -gt 0 -and $null -eq (Get-Process -Id $owner -ErrorAction SilentlyContinue)) {
        Remove-Item -Force -Recurse $lock
      }
    }
    Start-Sleep -Milliseconds 20
  }
}
if (-not $lockAcquired) { exit 0 }
try {
  $statePath = Join-Path $sink 'hook-state.json'
  $state = $null
  if (Test-Path $statePath) {
    try { $state = Get-Content $statePath -Raw | ConvertFrom-Json } catch {}
  }
  if ($null -eq $state) {
    $state = [pscustomobject]@{ sessionEnd = $false; stopFailure = $false; waitingForUser = $false; sessionId = $null }
  }
  $sessionId = $null
  if ($null -ne $parsed) { $sessionId = $parsed.session_id; if ($null -eq $sessionId) { $sessionId = $parsed.sessionId } }
  if ($sessionId) { $state.sessionId = [string]$sessionId }
  if ($event -match 'PermissionRequest|permission_request') { $state.waitingForUser = $true }
  if ($event -match 'UserPromptSubmit|user_prompt_submit|SessionEnd|session_end|StopFailure|stop_failure') { $state.waitingForUser = $false }
  if ($event -match 'SessionEnd|session_end') { $state.sessionEnd = $true }
  if ($event -match 'StopFailure|stop_failure') { $state.stopFailure = $true }
  $tmp = "$statePath.tmp.$PID"
  $state | ConvertTo-Json -Compress | Set-Content -Encoding UTF8 $tmp
  Move-Item -Force $tmp $statePath
  if ($event -match 'SessionEnd|session_end') { New-Item -ItemType File -Force (Join-Path $sink 'session-end') | Out-Null }
  if ($event -match 'StopFailure|stop_failure') { New-Item -ItemType File -Force (Join-Path $sink 'stop-failure') | Out-Null }
  if ($event -match 'PermissionRequest|permission_request') { New-Item -ItemType File -Force (Join-Path $sink 'waiting-for-user') | Out-Null }
  if ($event -match 'UserPromptSubmit|user_prompt_submit|SessionEnd|session_end|StopFailure|stop_failure') { Remove-Item -Force (Join-Path $sink 'waiting-for-user') }
} finally {
  Remove-Item -Force -Recurse $lock
}
"#;

pub fn write_recorder(sink: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(sink).map_err(|err| err.to_string())?;
    let script = if cfg!(windows) {
        let path = sink.join("record.cmd");
        fs::write(&path, RECORD_CMD).map_err(|err| err.to_string())?;
        fs::write(sink.join("record.ps1"), RECORD_PS1).map_err(|err| err.to_string())?;
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
    let state = fs::read_to_string(sink.join("hook-state.json"))
        .ok()
        .map(|raw| raw.trim_start_matches('\u{feff}').to_string())
        .and_then(|raw| serde_json::from_str::<HookState>(&raw).ok());
    CompletionSignals {
        session_end: state.as_ref().is_some_and(|state| state.session_end)
            || sink.join("session-end").is_file(),
        stop_failure: state.as_ref().is_some_and(|state| state.stop_failure)
            || sink.join("stop-failure").is_file(),
        waiting_for_user: sink.join("waiting-for-user").is_file(),
        session_id: state.and_then(|state| state.session_id),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct HookState {
    #[serde(default)]
    session_end: bool,
    #[serde(default)]
    stop_failure: bool,
    #[serde(default)]
    waiting_for_user: bool,
    #[serde(default)]
    session_id: Option<String>,
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
    let permission_request = recorder_command(recorder, "PermissionRequest");
    let user_prompt_submit = recorder_command(recorder, "UserPromptSubmit");
    let body = serde_json::json!({
        "hooks": {
            "PermissionRequest": [{"hooks": [{"type": "command", "command": permission_request, "timeout": 5}]}],
            "UserPromptSubmit": [{"hooks": [{"type": "command", "command": user_prompt_submit, "timeout": 5}]}],
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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn recorder_tracks_and_clears_real_waiting_signal() {
        let tmp = tempfile::tempdir().unwrap();
        let sink = tmp.path().join("sink");
        let recorder = write_recorder(&sink).unwrap();
        let run = |event: &str| {
            assert!(std::process::Command::new(&recorder)
                .arg(event)
                .env("AGENT_TASKBOARD_HOOK_SINK", &sink)
                .status()
                .unwrap()
                .success());
        };

        run("PermissionRequest");
        assert!(read_signals(&sink).waiting_for_user);
        run("UserPromptSubmit");
        assert!(!read_signals(&sink).waiting_for_user);
        run("PermissionRequest");
        run("SessionEnd");
        let signals = read_signals(&sink);
        assert!(!signals.waiting_for_user);
        assert!(signals.session_end);
    }

    #[cfg(unix)]
    #[test]
    fn recorder_persists_session_id_even_when_end_arrives_first() {
        use std::io::Write;
        let tmp = tempfile::tempdir().unwrap();
        let sink = tmp.path().join("sink");
        let recorder = write_recorder(&sink).unwrap();
        let run = |event: &str, payload: &str| {
            assert!(std::process::Command::new(&recorder)
                .arg(event)
                .env("AGENT_TASKBOARD_HOOK_SINK", &sink)
                .stdin(std::process::Stdio::piped())
                .spawn()
                .and_then(|mut child| {
                    if let Some(mut stdin) = child.stdin.take() {
                        stdin.write_all(payload.as_bytes())?;
                    }
                    child.wait()
                })
                .unwrap()
                .success());
        };

        run("SessionEnd", r#"{"session_id":"codex-session-1"}"#);
        let signals = read_signals(&sink);
        assert!(signals.session_end);
        assert_eq!(signals.session_id.as_deref(), Some("codex-session-1"));
    }

    #[test]
    fn read_signals_accepts_windows_utf8_bom() {
        let tmp = tempfile::tempdir().unwrap();
        let sink = tmp.path().join("sink");
        fs::create_dir_all(&sink).unwrap();
        fs::write(
            sink.join("hook-state.json"),
            b"\xef\xbb\xbf{\"sessionEnd\":true,\"sessionId\":\"windows-session\"}\n",
        )
        .unwrap();
        let signals = read_signals(&sink);
        assert!(signals.session_end);
        assert_eq!(signals.session_id.as_deref(), Some("windows-session"));
    }

    #[test]
    fn json_hook_plan_includes_waiting_lifecycle() {
        let tmp = tempfile::tempdir().unwrap();
        let sink = tmp.path().join("sink");
        let recorder = write_recorder(&sink).unwrap();
        let settings = tmp.path().join("settings.json");
        write_json_hooks(&settings, &recorder).unwrap();
        let body = fs::read_to_string(settings).unwrap();
        assert!(body.contains("PermissionRequest"));
        assert!(body.contains("UserPromptSubmit"));
        assert!(body.contains("SessionEnd"));
        assert!(body.contains("StopFailure"));
    }
}
