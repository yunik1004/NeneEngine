use nene::debug::Profiler;
use std::thread;
use std::time::Duration;

// ── Basic frame lifecycle ─────────────────────────────────────────────────────

#[test]
fn new_profiler_default_stats_zero() {
    let p = Profiler::new();
    assert_eq!(p.fps(), 0.0);
    assert_eq!(p.frame_ms(), 0.0);
    assert_eq!(p.min_ms(), 0.0);
    assert_eq!(p.max_ms(), 0.0);
    assert_eq!(p.avg_ms(), 0.0);
}

#[test]
fn frame_ms_positive_after_end_frame() {
    let mut p = Profiler::new();
    p.begin_frame();
    thread::sleep(Duration::from_millis(2));
    p.end_frame();
    assert!(p.frame_ms() > 0.0);
}

#[test]
fn fps_positive_after_end_frame() {
    let mut p = Profiler::new();
    p.begin_frame();
    p.end_frame();
    assert!(p.fps() > 0.0);
}

// ── Rolling stats ─────────────────────────────────────────────────────────────

#[test]
fn min_max_after_two_frames() {
    let mut p = Profiler::new();

    p.begin_frame();
    thread::sleep(Duration::from_millis(1));
    p.end_frame();

    p.begin_frame();
    thread::sleep(Duration::from_millis(5));
    p.end_frame();

    assert!(p.min_ms() < p.max_ms());
    assert!(p.avg_ms() >= p.min_ms());
    assert!(p.avg_ms() <= p.max_ms());
}

#[test]
fn history_iterator_length_matches_frames() {
    let mut p = Profiler::new();
    for _ in 0..10 {
        p.begin_frame();
        p.end_frame();
    }
    assert_eq!(p.frame_history().count(), 10);
}

#[test]
fn history_capped_at_128() {
    let mut p = Profiler::new();
    for _ in 0..200 {
        p.begin_frame();
        p.end_frame();
    }
    assert_eq!(p.frame_history().count(), 128);
}

// ── Scopes ────────────────────────────────────────────────────────────────────

#[test]
fn scope_ms_positive_after_sleep() {
    let mut p = Profiler::new();
    p.begin_frame();
    {
        let _s = p.scope("work");
        thread::sleep(Duration::from_millis(2));
    }
    p.end_frame();
    assert!(p.scope_ms("work") > 0.0);
}

#[test]
fn scope_ms_zero_for_unknown_scope() {
    let mut p = Profiler::new();
    p.begin_frame();
    p.end_frame();
    assert_eq!(p.scope_ms("nonexistent"), 0.0);
}

#[test]
fn multiple_scopes_tracked_independently() {
    let mut p = Profiler::new();
    p.begin_frame();
    {
        let _s = p.scope("a");
        thread::sleep(Duration::from_millis(1));
    }
    {
        let _s = p.scope("b");
        thread::sleep(Duration::from_millis(3));
    }
    p.end_frame();
    assert!(p.scope_ms("a") > 0.0);
    assert!(p.scope_ms("b") > p.scope_ms("a"));
}

#[test]
fn scope_accumulates_within_frame() {
    let mut p = Profiler::new();
    p.begin_frame();
    {
        let _s = p.scope("work");
        thread::sleep(Duration::from_millis(1));
    }
    {
        let _s = p.scope("work"); // same name again
        thread::sleep(Duration::from_millis(1));
    }
    p.end_frame();
    // Should be accumulated, not overwritten
    assert!(p.scope_ms("work") >= 2.0);
}

// ── Default ───────────────────────────────────────────────────────────────────

#[test]
fn default_equals_new() {
    let p = Profiler::default();
    assert_eq!(p.fps(), 0.0);
    assert_eq!(p.frame_ms(), 0.0);
}

// ── Frame history values ──────────────────────────────────────────────────────

#[test]
fn frame_history_values_positive() {
    let mut p = Profiler::new();
    for _ in 0..5 {
        p.begin_frame();
        thread::sleep(Duration::from_millis(1));
        p.end_frame();
    }
    assert!(p.frame_history().all(|ms| ms > 0.0));
}
