use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use super::{AgentField, AgentFieldOptionFilter};
use crate::LaunchEnvironment;

pub(super) const PROBE_TIMEOUT: Duration = Duration::from_secs(8);

pub(super) fn set_options(fields: &mut [AgentField], id: &str, options: Vec<String>) {
    if let Some(field) = fields.iter_mut().find(|field| field.id == id) {
        field.options = options;
    }
}

pub(super) fn set_options_if_found(fields: &mut [AgentField], id: &str, options: Vec<String>) {
    if !options.is_empty() {
        set_options(fields, id, options);
    }
}

pub(super) fn set_option_filter(
    fields: &mut [AgentField],
    id: &str,
    field_id: &str,
    options_by_value: BTreeMap<String, Vec<String>>,
) {
    if options_by_value.is_empty() {
        return;
    }
    if let Some(field) = fields.iter_mut().find(|field| field.id == id) {
        field.option_filter = Some(AgentFieldOptionFilter {
            field_id: field_id.to_string(),
            options_by_value,
        });
    }
}

pub(super) fn run_cli(
    executable: &Path,
    args: &[&str],
    env: &LaunchEnvironment,
) -> Result<String, String> {
    let mut command = configured_command(executable, env);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|err| format!("could not run {}: {err}", executable.display()))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdout_reader = std::thread::spawn(move || read_all(stdout));
    let stderr_reader = std::thread::spawn(move || read_all(stderr));
    let status = wait_for_child(&mut child, PROBE_TIMEOUT)?;
    let stdout = stdout_reader.join().unwrap_or_default();
    let stderr = stderr_reader.join().unwrap_or_default();
    if !status.success() {
        let detail = String::from_utf8_lossy(&stderr).trim().to_string();
        return Err(if detail.is_empty() {
            format!("{} {:?} failed", executable.display(), args)
        } else {
            detail
        });
    }
    String::from_utf8(stdout).map_err(|err| format!("CLI output was not UTF-8: {err}"))
}

pub(super) fn configured_command(executable: &Path, env: &LaunchEnvironment) -> Command {
    let mut command = Command::new(executable);
    command.current_dir(&env.cwd).env_clear().envs(&env.vars);
    command
}

pub(super) fn wait_for_child(
    child: &mut Child,
    timeout: Duration,
) -> Result<std::process::ExitStatus, String> {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) if started.elapsed() < timeout => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("CLI option discovery timed out".into());
            }
            Err(err) => return Err(format!("could not wait for CLI option discovery: {err}")),
        }
    }
}

pub(super) fn read_all<R: Read>(reader: Option<R>) -> Vec<u8> {
    let mut bytes = Vec::new();
    if let Some(mut reader) = reader {
        let _ = reader.read_to_end(&mut bytes);
    }
    bytes
}

pub(super) fn stderr_suffix(stderr: &[u8]) -> String {
    let stderr = String::from_utf8_lossy(stderr);
    let stderr = stderr.trim();
    if stderr.is_empty() {
        String::new()
    } else {
        format!(": {stderr}")
    }
}

pub(super) fn prefixed_value(output: &str, prefix: &str) -> Option<String> {
    output
        .lines()
        .find_map(|line| line.trim().strip_prefix(prefix).map(str::trim))
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub(super) fn option_values(help: &str, flag: &str) -> Vec<String> {
    let lines = help.lines().collect::<Vec<_>>();
    let Some(index) = lines.iter().position(|line| line.contains(flag)) else {
        return Vec::new();
    };
    for open in ['(', '['] {
        if let Some(values) = enclosed_values(lines[index], open) {
            return values;
        }
    }
    let listed = listed_values(&lines[index..lines.len().min(index + 9)]);
    if !listed.is_empty() {
        return listed;
    }
    let nearby = lines[index..lines.len().min(index + 7)].join(" ");
    for marker in ["possible values:", "Possible values:", "choices:"] {
        if let Some(values) = delimited_values(&nearby, marker) {
            return values;
        }
    }
    for open in ['(', '['] {
        if let Some(values) = enclosed_values(&nearby, open) {
            return values;
        }
    }
    Vec::new()
}

fn listed_values(lines: &[&str]) -> Vec<String> {
    let Some(marker) = lines
        .iter()
        .position(|line| line.contains("possible values:") || line.contains("Possible values:"))
    else {
        return Vec::new();
    };
    lines[marker + 1..]
        .iter()
        .map(|line| line.trim())
        .take_while(|line| line.starts_with("- "))
        .filter_map(|line| {
            line.strip_prefix("- ")
                .map(|value| value.trim_end_matches(':').trim())
                .filter(|value| !value.is_empty() && !value.contains(' '))
                .map(ToOwned::to_owned)
        })
        .collect()
}

fn enclosed_values(text: &str, open: char) -> Option<Vec<String>> {
    let start = text.find(open)?;
    let close = if open == '(' { ')' } else { ']' };
    let end = text[start + 1..].find(close)?;
    let raw = &text[start + 1..start + 1 + end];
    let raw = raw.split_once(':').map(|(_, values)| values).unwrap_or(raw);
    let values = split_values(raw);
    (values.len() > 1).then_some(values)
}

fn delimited_values(text: &str, marker: &str) -> Option<Vec<String>> {
    let start = text.find(marker)? + marker.len();
    let rest = &text[start..];
    let end = rest.find([']', ')']).unwrap_or(rest.len());
    let values = split_values(&rest[..end]);
    (!values.is_empty()).then_some(values)
}

fn split_values(raw: &str) -> Vec<String> {
    raw.split([',', '|'])
        .map(|value| {
            value
                .trim()
                .trim_matches(['"', '\'', '[', ']', '(', ')'])
                .trim_end_matches(':')
        })
        .filter(|value| !value.is_empty() && !value.contains(' '))
        .map(ToOwned::to_owned)
        .collect()
}

pub(super) fn quoted_values_near(help: &str, flag: &str) -> Vec<String> {
    let lines = help.lines().collect::<Vec<_>>();
    let Some(index) = lines.iter().position(|line| line.contains(flag)) else {
        return Vec::new();
    };
    let nearby = lines[index..lines.len().min(index + 5)].join(" ");
    let mut values = Vec::new();
    let mut rest = nearby.as_str();
    while let Some(start) = rest.find('\'') {
        rest = &rest[start + 1..];
        let Some(end) = rest.find('\'') else {
            break;
        };
        let value = &rest[..end];
        if !value.is_empty() {
            values.push(value.to_string());
        }
        rest = &rest[end + 1..];
    }
    values
}
