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
                r"stty -echo; stty size; printf '\033[2J\033[HLoading\r\033[2K\033[32m\346'; IFS= read -r next; printf '\210\220\345\212\237 OUTPUT_OK\033[0m\r\n\033]0;private title\007'; IFS= read -r next; stty size".into(),
            ],
            cwd: dir.path().to_path_buf(),
            env: BTreeMap::from([("PATH".into(), "/usr/bin:/bin".into())]),
            cols: u16::MAX,
            rows: u16::MAX,
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
    assert!(String::from_utf8_lossy(&raw).contains("256 512"));
    let readable = session.recent_output();
    assert_eq!(readable.trim(), "成功 OUTPUT_OK");
    session.resize(100, 30);
    assert_eq!(session.recent_output().trim(), "成功 OUTPUT_OK");
    session.resize(u16::MAX, u16::MAX);
    session.write(b"finish\n").unwrap();
    while session.exit_code().is_none() {
        assert!(Instant::now() < deadline, "PTY did not exit");
        std::thread::sleep(Duration::from_millis(5));
    }
    while String::from_utf8_lossy(&session.read_after(0, Duration::ZERO).data)
        .matches("256 512")
        .count()
        != 2
    {
        assert!(
            Instant::now() < deadline,
            "resize must use the same bounded OS size"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn real_pty_keeps_an_answer_when_the_tui_restores_its_primary_screen() {
    let dir = tempfile::tempdir().unwrap();
    let session = PtySessionFactory.spawn(SpawnRequest {
        argv: vec!["/bin/sh".into(), "-c".into(),
            r"stty -echo; printf 'startup prompt\033[?1049h\033[HFINAL ANSWER\033[?1049l'; IFS= read -r next; printf '\r\nEXIT_OK'".into()],
        cwd: dir.path().to_path_buf(),
        env: BTreeMap::from([("PATH".into(), "/usr/bin:/bin".into())]),
        cols: 80,
        rows: 24,
    }).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while !session
        .read_after(0, Duration::from_millis(20))
        .data
        .ends_with(b"\x1b[?1049l")
    {
        assert!(
            Instant::now() < deadline,
            "PTY did not restore its primary screen"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(session.recent_output(), "FINAL ANSWER");
    session.write(b"finish\n").unwrap();
    while !session.recent_output().contains("EXIT_OK") || session.exit_code().is_none() {
        assert!(
            Instant::now() < deadline,
            "PTY did not emit its exit output"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(session.recent_output().contains("FINAL ANSWER"));
}
