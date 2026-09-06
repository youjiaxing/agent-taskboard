use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

mod common;

use common::{
    boot as boot_base, boot_local, boot_req, make_dir, register_project as register,
    start_unbound_grok,
};
use host_kernel::{
    bind_local_rpc, spawn_local_rpc, AuthFailureKind, CredentialSource, GitHubTracker, HostKernel,
    KernelError, LocalMarkdownTracker, MemoryTracker, ProbeContext, ProjectConnection,
    ScriptedGitHub, TrackerKind, TrackerPort,
};

fn boot_memory(root: &Path) -> HostKernel {
    boot_base(root, Arc::new(MemoryTracker::new()))
}

#[path = "projects/lifecycle.rs"]
mod lifecycle;
#[path = "projects/local_markdown.rs"]
mod local_markdown;
