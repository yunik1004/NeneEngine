/// A double-buffered, single-type event queue.
///
/// Events emitted with [`emit`](Self::emit) are readable via [`read`](Self::read)
/// for the remainder of the current frame **and** the entire next frame.
/// Call [`update`](Self::update) once per frame to advance the buffer.
///
/// # Quick start
/// ```no_run
/// use nene::event::Events;
///
/// #[derive(Debug)]
/// enum GameEvent { PlayerDied, LevelUp(u32) }
///
/// let mut events: Events<GameEvent> = Events::new();
/// events.emit(GameEvent::LevelUp(2));
/// for ev in events.read() { println!("{ev:?}"); }
/// events.update();
/// ```
pub struct Events<E> {
    old: Vec<E>,
    current: Vec<E>,
}

impl<E> Events<E> {
    pub fn new() -> Self {
        Self {
            old: Vec::new(),
            current: Vec::new(),
        }
    }

    /// Emit an event. Readable until the frame after next.
    pub fn emit(&mut self, event: E) {
        self.current.push(event);
    }

    /// Iterate all events from the current and previous frame.
    pub fn read(&self) -> impl Iterator<Item = &E> {
        self.old.iter().chain(self.current.iter())
    }

    /// Advance the buffer. Call once per frame.
    pub fn update(&mut self) {
        self.old.clear();
        std::mem::swap(&mut self.old, &mut self.current);
    }

    /// Drain all events from both buffers immediately.
    pub fn clear(&mut self) {
        self.old.clear();
        self.current.clear();
    }

    /// Number of events currently readable (both buffers).
    pub fn len(&self) -> usize {
        self.old.len() + self.current.len()
    }

    pub fn is_empty(&self) -> bool {
        self.old.is_empty() && self.current.is_empty()
    }
}

impl<E> Default for Events<E> {
    fn default() -> Self {
        Self::new()
    }
}
