use nene::event::Events;

#[derive(Debug, PartialEq, Clone)]
enum Ev {
    A,
    B(u32),
}

// ── emit / read ───────────────────────────────────────────────────────────────

#[test]
fn emit_then_read() {
    let mut e: Events<Ev> = Events::new();
    e.emit(Ev::A);
    let v: Vec<_> = e.read().collect();
    assert_eq!(v, [&Ev::A]);
}

#[test]
fn empty_on_create() {
    let e: Events<Ev> = Events::new();
    assert!(e.is_empty());
    assert_eq!(e.len(), 0);
}

#[test]
fn multiple_events() {
    let mut e: Events<Ev> = Events::new();
    e.emit(Ev::A);
    e.emit(Ev::B(1));
    e.emit(Ev::B(2));
    assert_eq!(e.len(), 3);
}

// ── update (double-buffer) ────────────────────────────────────────────────────

#[test]
fn events_visible_after_update() {
    let mut e: Events<Ev> = Events::new();
    e.emit(Ev::A);
    e.update(); // A moves to old buffer
    let v: Vec<_> = e.read().collect();
    assert_eq!(v, [&Ev::A]);
}

#[test]
fn events_dropped_after_two_updates() {
    let mut e: Events<Ev> = Events::new();
    e.emit(Ev::A);
    e.update();
    e.update(); // A is now gone
    assert!(e.is_empty());
}

#[test]
fn new_events_visible_alongside_old() {
    let mut e: Events<Ev> = Events::new();
    e.emit(Ev::A);
    e.update();
    e.emit(Ev::B(99));
    let v: Vec<_> = e.read().cloned().collect();
    assert!(v.contains(&Ev::A));
    assert!(v.contains(&Ev::B(99)));
}

#[test]
fn update_clears_old_keeps_current() {
    let mut e: Events<Ev> = Events::new();
    e.emit(Ev::A);
    e.update(); // A → old
    e.emit(Ev::B(1)); // B → current
    e.update(); // A dropped, B → old
    let v: Vec<_> = e.read().collect();
    assert_eq!(v, [&Ev::B(1)]);
}

// ── clear ─────────────────────────────────────────────────────────────────────

#[test]
fn clear_empties_both_buffers() {
    let mut e: Events<Ev> = Events::new();
    e.emit(Ev::A);
    e.update();
    e.emit(Ev::B(1));
    e.clear();
    assert!(e.is_empty());
}

// ── multiple readers ──────────────────────────────────────────────────────────

#[test]
fn two_readers_see_same_events() {
    let mut e: Events<Ev> = Events::new();
    e.emit(Ev::A);
    e.emit(Ev::B(7));

    let r1: Vec<_> = e.read().cloned().collect();
    let r2: Vec<_> = e.read().cloned().collect();
    assert_eq!(r1, r2);
}

// ── default ───────────────────────────────────────────────────────────────────

#[test]
fn default_is_empty() {
    let e: Events<u32> = Events::default();
    assert!(e.is_empty());
}
