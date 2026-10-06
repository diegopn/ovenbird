function copyState(state) {
    return {
        text: state.text,
        codeOffset: state.codeOffset || 0,
        visualOffset: state.visualOffset || 0,
        mode: state.mode || 'code',
    };
}

export class EditorHistory {
    constructor(initialState, limit = 200) {
        this.limit = Math.max(2, limit);
        this.states = [copyState(initialState)];
        this.index = 0;
        this.lastGroup = null;
        this.lastTime = 0;
    }

    record(state, group = 'edit', time = Date.now()) {
        const next = copyState(state);
        const current = this.states[this.index];
        if (current?.text === next.text) {
            this.states[this.index] = next;
            return false;
        }

        this.states.splice(this.index + 1);
        const coalesce = this.index === this.states.length - 1 &&
            group === this.lastGroup && time >= this.lastTime && time - this.lastTime <= 700;
        if (coalesce) this.states[this.index] = next;
        else {
            this.states.push(next);
            this.index++;
        }

        this.lastGroup = group;
        this.lastTime = time;
        if (this.states.length > this.limit) {
            const excess = this.states.length - this.limit;
            this.states.splice(0, excess);
            this.index = Math.max(0, this.index - excess);
        }
        return true;
    }

    undo() {
        if (this.index === 0) return null;
        this.index--;
        this.lastGroup = null;
        return copyState(this.states[this.index]);
    }

    redo() {
        if (this.index >= this.states.length - 1) return null;
        this.index++;
        this.lastGroup = null;
        return copyState(this.states[this.index]);
    }

    get canUndo() { return this.index > 0; }
    get canRedo() { return this.index < this.states.length - 1; }
}
