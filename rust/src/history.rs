use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorState {
    pub source: String,
    pub source_cursor: usize,
    pub visual_cursor: usize,
    pub mode: EditorMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMode {
    Code,
    Visual,
}

#[derive(Debug, Clone)]
struct Snapshot {
    state: EditorState,
    group: Option<String>,
    at: Instant,
}

#[derive(Debug, Clone)]
pub struct EditorHistory {
    snapshots: Vec<Snapshot>,
    cursor: usize,
    capacity: usize,
    coalesce_window: Duration,
}

impl EditorHistory {
    pub fn new(initial: EditorState) -> Self {
        Self {
            snapshots: vec![Snapshot {
                state: initial,
                group: None,
                at: Instant::now(),
            }],
            cursor: 0,
            capacity: 200,
            coalesce_window: Duration::from_millis(700),
        }
    }

    pub fn can_undo(&self) -> bool {
        self.cursor > 0
    }
    pub fn can_redo(&self) -> bool {
        self.cursor + 1 < self.snapshots.len()
    }

    pub fn record(&mut self, state: EditorState, group: Option<&str>, now: Instant) {
        if self.snapshots[self.cursor].state == state {
            return;
        }
        self.snapshots.truncate(self.cursor + 1);
        let previous = &self.snapshots[self.cursor];
        let coalesce = group.is_some()
            && previous.group.as_deref() == group
            && now.saturating_duration_since(previous.at) <= self.coalesce_window;
        if coalesce {
            self.snapshots[self.cursor] = Snapshot {
                state,
                group: group.map(str::to_owned),
                at: now,
            };
            return;
        }
        self.snapshots.push(Snapshot {
            state,
            group: group.map(str::to_owned),
            at: now,
        });
        if self.snapshots.len() > self.capacity {
            self.snapshots.remove(0);
        }
        self.cursor = self.snapshots.len() - 1;
    }

    pub fn undo(&mut self) -> Option<&EditorState> {
        if !self.can_undo() {
            return None;
        }
        self.cursor -= 1;
        Some(&self.snapshots[self.cursor].state)
    }

    pub fn redo(&mut self) -> Option<&EditorState> {
        if !self.can_redo() {
            return None;
        }
        self.cursor += 1;
        Some(&self.snapshots[self.cursor].state)
    }
}
