#![cfg(unix)]

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use host_kernel::{PtySessionFactory, SessionFactory, SpawnRequest};

#[test]
fn real_pty_preserves_raw_bytes_and_projects_split_unicode_and_redraws() {
    let dir = tempfile::tempdir().unwrap();
    let session = PtySessionFactory
        .spawn(SpawnRequest {
            argv: vec![
                "/bin/sh".into(),
                "-c".into(),
                // Synchronize between chunks without echoing input into the
                // incomplete UTF-8 sequence. This exercises the real reader.
                r"stty -echo; printf '\033[2J\033[HLoading\r\033[2K\033[32m\346'; IFS= read -r next; printf '\210\220\345\212\237 OUTPUT_OK\033[0m\r\n\033]0;private title\007'; IFS= read -r next".into(),
            ],
            cwd: dir.path().to_path_buf(),
            env: BTreeMap::from([("PATH".into(), "/usr/bin:/bin".into())]),
            cols: 80,
            rows: 24,
        })
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while !session
        .read_after(0, Duration::from_millis(20))
        .data
        .ends_with(&[0xe6])
    {
        assert!(
            Instant::now() < deadline,
            "PTY did not emit the first chunk"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    session.write(b"continue\n").unwrap();
    while !session.recent_output().contains("OUTPUT_OK") {
        assert!(
            Instant::now() < deadline,
            "PTY did not emit the second chunk"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    let raw = session.read_after(0, Duration::ZERO).data;
    assert!(raw.windows(7).any(|bytes| bytes == b"Loading"));
    assert!(raw.contains(&0x1b));
    let readable = session.recent_output();
    assert_eq!(readable.trim(), "成功 OUTPUT_OK");
    session.resize(100, 30);
    assert_eq!(session.recent_output().trim(), "成功 OUTPUT_OK");
    session.write(b"finish\n").unwrap();
    while session.exit_code().is_none() {
        assert!(Instant::now() < deadline, "PTY did not exit");
        std::thread::sleep(Duration::from_millis(5));
    }
}
