use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CompletionSignals {
    pub session_end: bool,
    pub stop_failure: bool,
    pub waiting_for_user: bool,
    pub native_session_id: Option<String>,
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
  *StopFailure*|*stop_failure*) rm -f "$sink/waiting-for-user"; : > "$sink/stop-failure" ;;
  *PermissionRequest*|*permission_request*) : > "$sink/waiting-for-user" ;;
  *UserPromptSubmit*|*user_prompt_submit*) rm -f "$sink/waiting-for-user" ;;
  *SessionEnd*|*session_end*) rm -f "$sink/waiting-for-user"; : > "$sink/session-end" ;;
  Stop|stop)
    rm -f "$sink/waiting-for-user"
    termination=`printf '%s' "$payload" | sed -n 's/.*"terminationReason"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1`
    error=`printf '%s' "$payload" | sed -n 's/.*"error"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1`
    if [ "$termination" = "error" ] || [ -n "$error" ]; then
      : > "$sink/stop-failure"
    else
      : > "$sink/session-end"
    fi
    ;;
esac
session_id=`printf '%s' "$payload" | sed -n 's/.*"session_id"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p; s/.*"sessionId"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p; s/.*"conversationId"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' | head -1`
if [ -n "$session_id" ]; then
  printf '%s' "$session_id" > "$sink/native-session-id.tmp"
  mv "$sink/native-session-id.tmp" "$sink/native-session-id"
fi
if [ "$event" = "Stop" ] || [ "$event" = "stop" ]; then
  printf '{"decision":"allow"}'
else
  printf '{}'
fi
exit 0
"#;

const RECORD_PS1: &str = r#"param([string]$Event)
$sink = $env:AGENT_TASKBOARD_HOOK_SINK
if ([string]::IsNullOrEmpty($sink)) { exit 0 }
$payload = [Console]::In.ReadToEnd()
$data = $null
try { $data = $payload | ConvertFrom-Json } catch {}
if ([string]::IsNullOrEmpty($Event)) {
  $Event = $env:GROK_HOOK_EVENT
  if ([string]::IsNullOrEmpty($Event) -and $null -ne $data) {
    $Event = $data.hookEventName
    if ([string]::IsNullOrEmpty($Event)) { $Event = $data.hook_event_name }
  }
}
New-Item -ItemType Directory -Force -Path $sink | Out-Null
switch -Regex ($Event) {
  'PermissionRequest|permission_request' {
    New-Item -ItemType File -Force -Path (Join-Path $sink 'waiting-for-user') | Out-Null
    break
  }
  'UserPromptSubmit|user_prompt_submit' {
    Remove-Item -Force (Join-Path $sink 'waiting-for-user') -ErrorAction SilentlyContinue
    break
  }
  '^(SessionEnd|session_end)$' {
    Remove-Item -Force (Join-Path $sink 'waiting-for-user') -ErrorAction SilentlyContinue
    New-Item -ItemType File -Force -Path (Join-Path $sink 'session-end') | Out-Null
    break
  }
  '^(Stop|stop)$' {
    Remove-Item -Force (Join-Path $sink 'waiting-for-user') -ErrorAction SilentlyContinue
    if ($data.terminationReason -eq 'error' -or -not [string]::IsNullOrEmpty($data.error)) {
      New-Item -ItemType File -Force -Path (Join-Path $sink 'stop-failure') | Out-Null
    } else {
      New-Item -ItemType File -Force -Path (Join-Path $sink 'session-end') | Out-Null
    }
    break
  }
  '^(StopFailure|stop_failure)$' {
    Remove-Item -Force (Join-Path $sink 'waiting-for-user') -ErrorAction SilentlyContinue
    New-Item -ItemType File -Force -Path (Join-Path $sink 'stop-failure') | Out-Null
    break
  }
}
if ($null -ne $data) {
  $sessionId = $data.session_id
  if ([string]::IsNullOrEmpty($sessionId)) { $sessionId = $data.sessionId }
  if ([string]::IsNullOrEmpty($sessionId)) { $sessionId = $data.conversationId }
  if (-not [string]::IsNullOrEmpty($sessionId)) {
    $tmp = Join-Path $sink 'native-session-id.tmp'
    $path = Join-Path $sink 'native-session-id'
    [System.IO.File]::WriteAllText($tmp, $sessionId, [System.Text.UTF8Encoding]::new($false))
    Move-Item -Force $tmp $path
  }
}
if ($Event -eq 'Stop' -or $Event -eq 'stop') {
  Write-Output '{"decision":"allow"}'
} else {
  Write-Output '{}'
}
exit 0
"#;

pub fn write_recorder(sink: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(sink).map_err(|err| err.to_string())?;
    let script = if cfg!(windows) {
        let path = sink.join("record.ps1");
        fs::write(&path, RECORD_PS1).map_err(|err| err.to_string())?;
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
        format!(
            "powershell.exe -NoProfile -ExecutionPolicy Bypass -File \"{}\" {event}",
            script.display()
        )
    } else {
        format!("\"{}\" {event}", script.display())
    }
}

pub fn read_signals(sink: &Path) -> CompletionSignals {
    CompletionSignals {
        session_end: sink.join("session-end").is_file(),
        stop_failure: sink.join("stop-failure").is_file(),
        waiting_for_user: sink.join("waiting-for-user").is_file(),
        native_session_id: fs::read_to_string(sink.join("native-session-id"))
            .ok()
            .map(|id| id.trim().to_string())
            .filter(|id| !id.is_empty()),
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
    let permission_request = recorder_command(recorder, "PermissionRequest");
    let user_prompt_submit = recorder_command(recorder, "UserPromptSubmit");
    let session_start = recorder_command(recorder, "SessionStart");
    let body = serde_json::json!({
        "hooks": {
            "SessionStart": [{"hooks": [{"type": "command", "command": session_start, "timeout": 5}]}],
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

const ANTIGRAVITY_HOOK_NAME_PREFIX: &str = "agent-taskboard-";

fn antigravity_hook_name(sink_dir: &Path) -> String {
    format!(
        "{ANTIGRAVITY_HOOK_NAME_PREFIX}{}",
        sink_dir
            .file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or_else(|| "run".into())
    )
}

fn write_json_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let temporary = path.with_extension("taskboard.tmp");
    let permissions = fs::metadata(path)
        .ok()
        .map(|metadata| metadata.permissions());
    fs::write(&temporary, bytes).map_err(|err| err.to_string())?;
    if let Some(permissions) = permissions {
        fs::set_permissions(&temporary, permissions).map_err(|err| err.to_string())?;
    }
    fs::rename(&temporary, path).map_err(|err| err.to_string())
}

pub fn write_antigravity_hooks(
    path: &Path,
    recorder: &Path,
    sink_dir: &Path,
) -> Result<(), String> {
    let original_path = sink_dir.join("antigravity-hooks.original");
    let original_missing_path = sink_dir.join("antigravity-hooks.original-missing");
    let installed_path = sink_dir.join("antigravity-hooks.installed");
    if !original_path.exists() && !original_missing_path.exists() {
        if path.is_file() {
            fs::write(
                &original_path,
                fs::read(path).map_err(|err| err.to_string())?,
            )
            .map_err(|err| err.to_string())?;
        } else {
            fs::write(&original_missing_path, []).map_err(|err| err.to_string())?;
        }
    }

    let mut body = if path.is_file() {
        serde_json::from_slice::<Value>(&fs::read(path).map_err(|err| err.to_string())?)
            .map_err(|err| format!("could not parse Antigravity hooks.json: {err}"))?
    } else {
        serde_json::json!({})
    };
    let object = body
        .as_object_mut()
        .ok_or_else(|| "Antigravity hooks.json must contain a JSON object".to_string())?;
    let command = |event: &str| recorder_command(recorder, event);
    object.insert(
        antigravity_hook_name(sink_dir),
        serde_json::json!({
            "PreInvocation": [{
                "type": "command",
                "command": command("PreInvocation"),
                "timeout": 5
            }],
            "PostInvocation": [{
                "type": "command",
                "command": command("PostInvocation"),
                "timeout": 5
            }],
            "Stop": [{
                "type": "command",
                "command": command("Stop"),
                "timeout": 5
            }]
        }),
    );
    let installed = serde_json::to_vec_pretty(&body).map_err(|err| err.to_string())?;
    write_json_file(path, &installed)?;
    if let Err(error) = fs::write(&installed_path, &installed) {
        let _ = cleanup_antigravity_hooks(path, sink_dir);
        return Err(error.to_string());
    }
    Ok(())
}

pub fn cleanup_antigravity_hooks(path: &Path, sink_dir: &Path) -> Result<(), String> {
    let original_path = sink_dir.join("antigravity-hooks.original");
    let original_missing_path = sink_dir.join("antigravity-hooks.original-missing");
    let installed_path = sink_dir.join("antigravity-hooks.installed");
    let installed = fs::read(&installed_path).ok();
    let current = fs::read(path).ok();
    let original_was_taskboard_only = original_path
        .is_file()
        .then(|| fs::read(&original_path).ok())
        .flatten()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|value| value.as_object().cloned())
        .is_some_and(|object| {
            !object.is_empty()
                && object
                    .keys()
                    .all(|key| key.starts_with(ANTIGRAVITY_HOOK_NAME_PREFIX))
        });
    if current.is_some() && installed.is_some() && current == installed {
        if original_path.is_file() {
            write_json_file(
                path,
                &fs::read(&original_path).map_err(|err| err.to_string())?,
            )?;
        } else if original_missing_path.exists() {
            match fs::remove_file(path) {
                Ok(()) => {}
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => return Err(err.to_string()),
            }
        }
    } else if let Some(current) = current {
        let mut body = serde_json::from_slice::<Value>(&current).map_err(|err| {
            format!("could not parse Antigravity hooks.json during cleanup: {err}")
        })?;
        if let Some(object) = body.as_object_mut() {
            object.remove(&antigravity_hook_name(sink_dir));
            if object.is_empty() && (original_missing_path.exists() || original_was_taskboard_only)
            {
                match fs::remove_file(path) {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.to_string()),
                }
            } else {
                let bytes = serde_json::to_vec_pretty(&body).map_err(|err| err.to_string())?;
                write_json_file(path, &bytes)?;
            }
        }
    }
    for sidecar in [original_path, original_missing_path, installed_path] {
        let _ = fs::remove_file(sidecar);
    }
    Ok(())
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

    fn command(recorder: &Path, event: &str) -> std::process::Command {
        let mut command = if cfg!(windows) {
            let mut command = std::process::Command::new("powershell.exe");
            command.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"]);
            command.arg(recorder);
            command
        } else {
            std::process::Command::new(recorder)
        };
        command.arg(event);
        command
    }

    #[cfg(unix)]
    #[test]
    fn recorder_tracks_and_clears_real_waiting_signal() {
        let tmp = tempfile::tempdir().unwrap();
        let sink = tmp.path().join("sink");
        let recorder = write_recorder(&sink).unwrap();
        let run = |event: &str| {
            assert!(command(&recorder, event)
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
        assert!(body.contains("SessionStart"));
        assert!(body.contains("SessionEnd"));
        assert!(body.contains("StopFailure"));
    }

    #[test]
    fn recorder_persists_native_session_id_from_hook_payload() {
        let tmp = tempfile::tempdir().unwrap();
        let sink = tmp.path().join("sink");
        let recorder = write_recorder(&sink).unwrap();
        let payload = br#"{"conversationId":"agy-session"}"#;
        command(&recorder, "SessionStart")
            .env("AGENT_TASKBOARD_HOOK_SINK", &sink)
            .stdin(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child.stdin.take().unwrap().write_all(payload)?;
                child.wait()
            })
            .unwrap();
        assert_eq!(
            read_signals(&sink).native_session_id.as_deref(),
            Some("agy-session")
        );
    }

    #[test]
    fn antigravity_hook_payload_records_id_and_stop_without_stop_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let sink = tmp.path().join("sink");
        let recorder = write_recorder(&sink).unwrap();
        let output = command(&recorder, "PreInvocation")
            .env("AGENT_TASKBOARD_HOOK_SINK", &sink)
            .stdin(std::process::Stdio::piped())
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, b"{}".to_vec());

        let payload = br#"{"conversationId":"agy-session"}"#;
        command(&recorder, "PreInvocation")
            .env("AGENT_TASKBOARD_HOOK_SINK", &sink)
            .stdin(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child.stdin.take().unwrap().write_all(payload)?;
                child.wait()
            })
            .unwrap();
        command(&recorder, "Stop")
            .env("AGENT_TASKBOARD_HOOK_SINK", &sink)
            .output()
            .map(|output| {
                assert!(output.status.success());
                assert_eq!(
                    String::from_utf8_lossy(&output.stdout).trim(),
                    "{\"decision\":\"allow\"}"
                );
            })
            .unwrap();
        let signals = read_signals(&sink);
        assert_eq!(signals.native_session_id.as_deref(), Some("agy-session"));
        assert!(signals.session_end);
        assert!(!signals.stop_failure);

        command(&recorder, "Stop")
            .env("AGENT_TASKBOARD_HOOK_SINK", &sink)
            .stdin(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child
                    .stdin
                    .take()
                    .unwrap()
                    .write_all(br#"{"terminationReason":"error","error":"failed"}"#)?;
                child.wait()
            })
            .unwrap();
        let signals = read_signals(&sink);
        assert!(signals.stop_failure);
    }
}
