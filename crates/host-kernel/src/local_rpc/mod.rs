use std::io::{self};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::{CommandOutcome, HostKernel, LoopbackKind, ProcessIntent};

mod http;
mod serve;

pub use http::local_client_origin_allowed;
use http::*;
use serve::*;

pub const LOCAL_RPC_PORT: u16 = 10529;

#[derive(Debug, Clone)]
pub enum LoopbackAssets {
    Builtin,
    Directory(PathBuf),
    DevProxy { origin: String },
}

const MAX_HEADER_BYTES: usize = 16 * 1024;
const MAX_BODY_BYTES: usize = 1024 * 1024;

pub fn bind_local_rpc(preferred_port: u16) -> io::Result<(TcpListener, String)> {
    let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, preferred_port))
        .or_else(|_| TcpListener::bind((Ipv4Addr::UNSPECIFIED, 0)))?;
    let port = listener.local_addr()?.port();
    Ok((listener, format!("http://127.0.0.1:{port}")))
}

fn bind_strict(port: u16) -> io::Result<(TcpListener, String)> {
    let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, port))?;
    let bound = listener.local_addr()?.port();
    Ok((listener, format!("http://127.0.0.1:{bound}")))
}

pub struct LoopbackServer {
    protocol_url: String,
    stop: Arc<AtomicBool>,
}

impl LoopbackServer {
    pub fn attach_client_transport(
        kernel: Arc<Mutex<HostKernel>>,
        on_outcome: impl Fn(CommandOutcome) + Send + Sync + 'static,
    ) -> io::Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        let port = listener.local_addr()?.port();
        let protocol_url = format!("http://127.0.0.1:{port}");
        spawn_local_rpc_inner(
            listener,
            kernel,
            LoopbackAssets::Builtin,
            Arc::clone(&stop),
            on_outcome,
            false,
        );
        Ok(Self { protocol_url, stop })
    }

    pub fn attach(
        kernel: Arc<Mutex<HostKernel>>,
        page_port: u16,
        on_outcome: impl Fn(CommandOutcome) + Send + Sync + 'static,
    ) -> io::Result<Self> {
        Self::attach_with(kernel, page_port, LoopbackAssets::Builtin, on_outcome)
    }

    pub fn attach_with(
        kernel: Arc<Mutex<HostKernel>>,
        page_port: u16,
        assets: LoopbackAssets,
        on_outcome: impl Fn(CommandOutcome) + Send + Sync + 'static,
    ) -> io::Result<Self> {
        Self::attach_configured(kernel, page_port, assets, on_outcome, true)
    }

    pub fn attach_without_host_tick(
        kernel: Arc<Mutex<HostKernel>>,
        page_port: u16,
        assets: LoopbackAssets,
        on_outcome: impl Fn(CommandOutcome) + Send + Sync + 'static,
    ) -> io::Result<Self> {
        Self::attach_configured(kernel, page_port, assets, on_outcome, false)
    }

    fn attach_configured(
        kernel: Arc<Mutex<HostKernel>>,
        page_port: u16,
        assets: LoopbackAssets,
        on_outcome: impl Fn(CommandOutcome) + Send + Sync + 'static,
        host_tick: bool,
    ) -> io::Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let running = kernel
            .lock()
            .map_err(|_| io::Error::other("kernel lock poisoned"))?
            .snapshot()
            .running;
        if !running {
            kernel
                .lock()
                .map_err(|_| io::Error::other("kernel lock poisoned"))?
                .note_loopback_page(
                    LoopbackKind::HostNotRunning,
                    if page_port == 0 {
                        LOCAL_RPC_PORT
                    } else {
                        page_port
                    },
                );
            return Ok(Self {
                protocol_url: String::new(),
                stop,
            });
        }

        match bind_strict(page_port) {
            Ok((listener, protocol_url)) => {
                let port = listener.local_addr()?.port();
                kernel
                    .lock()
                    .map_err(|_| io::Error::other("kernel lock poisoned"))?
                    .note_loopback_page(LoopbackKind::Serving, port);
                spawn_local_rpc_inner(
                    listener,
                    kernel,
                    assets,
                    Arc::clone(&stop),
                    on_outcome,
                    host_tick,
                );
                Ok(Self { protocol_url, stop })
            }
            Err(_) => {
                let (listener, protocol_url) = bind_local_rpc(0)?;
                kernel
                    .lock()
                    .map_err(|_| io::Error::other("kernel lock poisoned"))?
                    .note_loopback_page(
                        LoopbackKind::Occupied,
                        if page_port == 0 {
                            LOCAL_RPC_PORT
                        } else {
                            page_port
                        },
                    );
                spawn_local_rpc_inner(
                    listener,
                    kernel,
                    assets,
                    Arc::clone(&stop),
                    on_outcome,
                    host_tick,
                );
                Ok(Self { protocol_url, stop })
            }
        }
    }

    pub fn protocol_url(&self) -> &str {
        &self.protocol_url
    }

    pub fn shutdown(&self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Ok(addr) = self
            .protocol_url
            .trim_start_matches("http://")
            .parse::<SocketAddr>()
        {
            let _ = TcpStream::connect_timeout(&addr, Duration::from_millis(50));
        }
    }
}

impl Drop for LoopbackServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub fn spawn_local_rpc(
    listener: TcpListener,
    kernel: Arc<Mutex<HostKernel>>,
    on_outcome: impl Fn(CommandOutcome) + Send + Sync + 'static,
) {
    spawn_local_rpc_inner(
        listener,
        kernel,
        LoopbackAssets::Builtin,
        Arc::new(AtomicBool::new(false)),
        on_outcome,
        false,
    );
}

fn spawn_local_rpc_inner(
    listener: TcpListener,
    kernel: Arc<Mutex<HostKernel>>,
    assets: LoopbackAssets,
    stop: Arc<AtomicBool>,
    on_outcome: impl Fn(CommandOutcome) + Send + Sync + 'static,
    host_tick: bool,
) {
    let on_outcome = Arc::new(on_outcome);
    if host_tick {
        let tick_kernel = Arc::clone(&kernel);
        let tick_stop = Arc::clone(&stop);
        let _ = std::thread::Builder::new()
            .name("host-refresh-tick".into())
            .spawn(move || {
                while !tick_stop.load(Ordering::Relaxed) && kernel_process_alive(&tick_kernel) {
                    std::thread::sleep(Duration::from_millis(1000));
                    if tick_stop.load(Ordering::Relaxed) {
                        break;
                    }
                    let (outcome, refreshes) = {
                        let Ok(mut host) = tick_kernel.lock() else {
                            break;
                        };
                        host.begin_deferred_refreshes();
                        let outcome = host.dispatch_background_tick(None);
                        let refreshes = host.take_deferred_refreshes();
                        (outcome, refreshes)
                    };
                    for refresh in refreshes {
                        let completed = HostKernel::execute_prepared_refresh(refresh);
                        let Ok(mut host) = tick_kernel.lock() else {
                            break;
                        };
                        host.finish_prepared_refresh(completed);
                    }
                    let should_exit = outcome
                        .ok()
                        .is_some_and(|process| process == ProcessIntent::Exit)
                        || !kernel_process_alive(&tick_kernel);
                    if should_exit {
                        tick_stop.store(true, Ordering::Relaxed);
                    }
                }
            });
    }
    std::thread::Builder::new()
        .name("host-local-rpc".into())
        .spawn(move || {
            let server_port = listener
                .local_addr()
                .map(|addr| addr.port())
                .unwrap_or(LOCAL_RPC_PORT);
            let _ = listener.set_nonblocking(true);
            while !stop.load(Ordering::Relaxed) && kernel_process_alive(&kernel) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let _ = stream.set_nonblocking(false);
                        let kernel = Arc::clone(&kernel);
                        let on_outcome = Arc::clone(&on_outcome);
                        let stop = Arc::clone(&stop);
                        let assets = assets.clone();
                        let _ = std::thread::Builder::new()
                            .name("host-local-rpc-conn".into())
                            .spawn(move || {
                                match serve_connection(
                                    stream,
                                    &kernel,
                                    &assets,
                                    server_port,
                                    Arc::clone(&kernel),
                                ) {
                                    Ok(Some(outcome)) => {
                                        if outcome.process == ProcessIntent::Exit {
                                            stop.store(true, Ordering::Relaxed);
                                        }
                                        on_outcome(outcome);
                                    }
                                    Ok(None) => {}
                                    Err(err) => eprintln!("local rpc: {err}"),
                                }
                            });
                    }
                    Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(20));
                    }
                    Err(_) => continue,
                }
            }
        })
        .expect("host local rpc thread");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        BootRequest, HostEvent, IssueRecord, MemoryTracker, RefreshStatus, SystemAppearance,
        DEFAULT_REFRESH_INTERVAL_MS,
    };

    #[test]
    fn background_ticks_keep_board_updates_until_a_client_receives_them() {
        let tmp = tempfile::tempdir().unwrap();
        let project_dir = tmp.path().join("work/garden");
        std::fs::create_dir_all(&project_dir).unwrap();
        let tracker = Arc::new(MemoryTracker::new());
        tracker.add_issue(IssueRecord::open("you/garden", 1, "ready"));
        let tracker_seam: Arc<dyn crate::TrackerSeam> = tracker.clone();
        let mut host = HostKernel::boot_with(
            BootRequest {
                app_local_data_dir: tmp.path().to_path_buf(),
                app_log_dir: tmp.path().join("logs"),
                system_locale: "zh-Hans-CN".into(),
                system_appearance: SystemAppearance::Light,
                host_display_name: "Studio".into(),
            },
            tracker_seam,
        )
        .unwrap();
        let project_id = host
            .handle(serde_json::json!({
                "op": "registerProject",
                "name": "garden",
                "localPath": project_dir,
                "repository": "you/garden",
            }))
            .unwrap()
            .snapshot
            .focused_project_id;
        host.handle(serde_json::json!({
            "op": "setClientView",
            "clientId": "desktop",
            "projectId": project_id,
            "visible": true,
        }))
        .unwrap();
        let fetched_at_ms = match host.snapshot().board.unwrap().refresh {
            RefreshStatus::Ready { fetched_at_ms, .. } => fetched_at_ms,
            other => panic!("expected ready, got {other:?}"),
        };

        tracker.set_issues(
            "you/garden",
            vec![
                IssueRecord::open("you/garden", 1, "ready"),
                IssueRecord::open("you/garden", 2, "new from background refresh"),
            ],
        );
        host.begin_deferred_refreshes();
        assert_eq!(
            host.dispatch_background_tick(Some(fetched_at_ms + DEFAULT_REFRESH_INTERVAL_MS))
                .unwrap(),
            ProcessIntent::KeepRunning
        );
        let refreshes = host.take_deferred_refreshes();
        assert_eq!(refreshes.len(), 1);
        for refresh in refreshes {
            let completed = HostKernel::execute_prepared_refresh(refresh);
            host.finish_prepared_refresh(completed);
        }

        host.begin_deferred_refreshes();
        host.dispatch_background_tick(Some(fetched_at_ms + DEFAULT_REFRESH_INTERVAL_MS + 1_000))
            .unwrap();
        assert!(host.take_deferred_refreshes().is_empty());

        let outcome = host
            .handle(serde_json::json!({
                "op": "snapshot",
                "clientInstanceId": "desktop",
            }))
            .unwrap();
        assert!(outcome.events.iter().any(|event| matches!(
            event,
            HostEvent::BoardUpdated { project_id: updated } if updated == &project_id
        )));
        assert_eq!(
            outcome
                .snapshot
                .board
                .unwrap()
                .columns
                .unwrap()
                .frontier
                .len(),
            2
        );
    }
}
