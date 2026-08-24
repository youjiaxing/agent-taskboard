use std::fs;
use std::path::Path;

use host_kernel::HostMode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopStartupSettings {
    pub client_only: bool,
}

pub fn requested_host_mode(path: &Path, args: impl IntoIterator<Item = String>) -> HostMode {
    let command_line = args.into_iter().find_map(|arg| match arg.as_str() {
        "--client-only" => Some(HostMode::ClientOnly),
        "--host-and-client" => Some(HostMode::HostAndClient),
        _ => None,
    });
    command_line.unwrap_or_else(|| {
        fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_json::from_str::<DesktopStartupSettings>(&raw).ok())
            .filter(|settings| settings.client_only)
            .map(|_| HostMode::ClientOnly)
            .unwrap_or(HostMode::HostAndClient)
    })
}

pub fn write_host_mode(path: &Path, mode: HostMode) -> Result<(), String> {
    let body = serde_json::to_vec_pretty(&DesktopStartupSettings {
        client_only: mode == HostMode::ClientOnly,
    })
    .map_err(|err| err.to_string())?;
    fs::write(path, body).map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_launch_defaults_to_host_and_client_and_persisted_client_only_survives_relogin() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("startup.json");
        assert_eq!(
            requested_host_mode(&path, Vec::<String>::new()),
            HostMode::HostAndClient
        );

        write_host_mode(&path, HostMode::ClientOnly).unwrap();
        assert_eq!(
            requested_host_mode(&path, Vec::<String>::new()),
            HostMode::ClientOnly
        );
        assert_eq!(
            requested_host_mode(&path, ["--host-and-client".into()]),
            HostMode::HostAndClient
        );
    }
}
