use super::*;

pub(super) fn snapshot(
    source: &sourceview5::Buffer,
    visual: &TextBuffer,
    mode: EditorMode,
) -> EditorState {
    EditorState {
        source: buffer_text(source),
        source_cursor: source.iter_at_mark(&source.get_insert()).offset() as usize,
        visual_cursor: visual.iter_at_mark(&visual.get_insert()).offset() as usize,
        mode,
    }
}

pub(super) fn record_history(
    runtime: &Rc<RefCell<Runtime>>,
    source: &sourceview5::Buffer,
    visual: &TextBuffer,
    group: &str,
) {
    let state = runtime.borrow();
    let snapshot = snapshot(source, visual, state.mode);
    drop(state);
    runtime
        .borrow_mut()
        .history
        .record(snapshot, Some(group), Instant::now());
}

pub(super) fn notify_history(runtime: &Rc<RefCell<Runtime>>, callback: &HistoryCallback) {
    let state = runtime.borrow();
    let can_undo = state.history.can_undo();
    let can_redo = state.history.can_redo();
    drop(state);
    if let Some(callback) = callback.borrow().as_ref() {
        callback(can_undo, can_redo);
    }
}

pub(super) fn update_dirty(runtime: &Rc<RefCell<Runtime>>, text: &str, callback: &DirtyCallback) {
    let changed = {
        let mut state = runtime.borrow_mut();
        let dirty = text != state.saved_text;
        if dirty == state.dirty {
            false
        } else {
            state.dirty = dirty;
            true
        }
    };
    if changed {
        if let Some(callback) = callback.borrow().as_ref() {
            callback(runtime.borrow().dirty);
        }
    }
}
