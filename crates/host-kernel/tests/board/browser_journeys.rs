use super::*;

#[test]
fn browser_appearance_preference_storage_matrix() {
    let tmp = tempfile::tempdir().unwrap();
    let tracker = Arc::new(MemoryTracker::new());
    let host = boot(tmp.path(), tracker);
    run_browser_e2e(host, "appearance-preference.mjs", &[]);
}

#[test]
fn browser_dependency_graph_overview_renders_more_than_the_focused_canvas_batch() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = make_dir(tmp.path(), "work/garden");
    let tracker = Arc::new(MemoryTracker::new());
    tracker.add_issue(
        IssueRecord::open("you/garden", 1, "dependency origin").blocking(
            "you/garden",
            2,
            "dependency target",
        ),
    );
    tracker.add_issue(IssueRecord::open("you/garden", 2, "dependency target"));
    for number in 3..=60 {
        tracker.add_issue(IssueRecord::open(
            "you/garden",
            number,
            format!("open {number}"),
        ));
    }
    let mut host = boot(tmp.path(), tracker);
    register(&mut host, "garden", &dir, "you/garden");

    run_browser_e2e(host, "dependency-graph-overview.mjs", &[]);
}
