//! Frame-buffered event queue.
//!
//! `Events<E>` is a double-buffered queue: events emitted this frame are
//! visible to all readers, and are automatically cleared on the next
//! [`update`](Events::update) call. This means every system that reads
//! during the same frame sees all events, and nothing is dropped between
//! consecutive frames.
//!
//! # Quick start
//! ```no_run
//! use nene::event::Events;
//!
//! #[derive(Debug)]
//! enum GameEvent { PlayerDied, LevelUp(u32) }
//!
//! let mut events: Events<GameEvent> = Events::new();
//!
//! // Emit (typically in update):
//! events.emit(GameEvent::LevelUp(2));
//!
//! // Read (any system this frame):
//! for ev in events.read() {
//!     println!("{ev:?}");
//! }
//!
//! // End of frame — swap buffers so next frame starts clean:
//! events.update();
//! ```

// ── Events ────────────────────────────────────────────────────────────────────

/// A double-buffered, single-type event queue.
///
/// Events emitted with [`emit`](Self::emit) are readable via [`read`](Self::read)
/// for the remainder of the current frame **and** the entire next frame.
/// Call [`update`](Self::update) once per frame (e.g. at the start of your
/// update loop) to advance the buffer.
pub struct Events<E> {
    /// Events from the previous frame (still readable this frame).
    old: Vec<E>,
    /// Events emitted this frame.
    current: Vec<E>,
}

impl<E> Events<E> {
    /// Create an empty event queue.
    pub fn new() -> Self {
        Self {
            old: Vec::new(),
            current: Vec::new(),
        }
    }

    /// Emit an event. Readable via [`read`](Self::read) until the frame after next.
    pub fn emit(&mut self, event: E) {
        self.current.push(event);
    }

    /// Iterate all events from the current and previous frame.
    pub fn read(&self) -> impl Iterator<Item = &E> {
        self.old.iter().chain(self.current.iter())
    }

    /// Advance the buffer. Call once per frame.
    ///
    /// Events from `current` move to `old`; `old` is cleared.
    /// Events not read before this call are dropped after the next `update`.
    pub fn update(&mut self) {
        self.old.clear();
        std::mem::swap(&mut self.old, &mut self.current);
    }

    /// Drain all events from both buffers immediately.
    ///
    /// Useful for flushing after a scene transition or reset.
    pub fn clear(&mut self) {
        self.old.clear();
        self.current.clear();
    }

    /// Number of events currently readable (both buffers).
    pub fn len(&self) -> usize {
        self.old.len() + self.current.len()
    }

    /// Returns `true` if there are no readable events.
    pub fn is_empty(&self) -> bool {
        self.old.is_empty() && self.current.is_empty()
    }
}

impl<E> Default for Events<E> {
    fn default() -> Self {
        Self::new()
    }
}
