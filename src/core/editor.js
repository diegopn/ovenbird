import Gtk from 'gi://Gtk?version=4.0';
import GtkSource from 'gi://GtkSource?version=5';
import Pango from 'gi://Pango';
import GLib from 'gi://GLib';
import Adw from 'gi://Adw?version=1';
import { parseVisualBody, serializeVisualTokens, splitLatexDocument } from './latex-document.js';
import { createHeadingCommand, createInlineCommand } from './latex-commands.js';
import { EditorHistory } from './editor-history.js';
import { _ } from './i18n.js';

function characterCount(text) {
    return Array.from(text).length;
}

export class LatexEditor {
    constructor() {
        this._synchronizing = false;
        this._file = null;
        this._dirty = false;
        this._visualEdited = false;
        this._sourceCursorBeforeVisual = null;
        this._historyGroup = null;
        this.history = null;
        this._savedText = '';
        this.root = new Gtk.Box({ orientation: Gtk.Orientation.VERTICAL, spacing: 0,
            hexpand: true, vexpand: true });

        this.sourceBuffer = new GtkSource.Buffer();
        this.sourceBuffer.set_enable_undo(false);
        const languageManager = GtkSource.LanguageManager.get_default();
        const latex = languageManager.get_language('latex');
        if (latex) this.sourceBuffer.set_language(latex);
        this.sourceBuffer.set_highlight_syntax(true);
        this.styleManager = Adw.StyleManager.get_default();
        this._updateSourceStyleScheme();
        this.styleManager.connect('notify::dark', () => this._updateSourceStyleScheme());

        this.sourceView = new GtkSource.View({
            buffer: this.sourceBuffer,
            monospace: true,
            show_line_numbers: true,
            show_line_marks: false,
            auto_indent: true,
            indent_width: 4,
            tab_width: 4,
            highlight_current_line: true,
            wrap_mode: Gtk.WrapMode.NONE,
            hexpand: true,
            vexpand: true,
        });
        this.sourceView.add_css_class('source-view');

        this.visualBuffer = new Gtk.TextBuffer();
        this.visualView = new Gtk.TextView({
            buffer: this.visualBuffer,
            wrap_mode: Gtk.WrapMode.WORD_CHAR,
            left_margin: 32,
            right_margin: 32,
            top_margin: 28,
            bottom_margin: 28,
            accepts_tab: false,
            hexpand: true,
            vexpand: true,
        });
        this.visualView.add_css_class('visual-document');
        this.visualTags = new Map();
        this._createVisualTags();

        this.sourceScroll = new Gtk.ScrolledWindow({
            hscrollbar_policy: Gtk.PolicyType.AUTOMATIC,
            vscrollbar_policy: Gtk.PolicyType.AUTOMATIC,
            child: this.sourceView,
            hexpand: true,
            vexpand: true,
        });
        this.visualScroll = new Gtk.ScrolledWindow({
            hscrollbar_policy: Gtk.PolicyType.NEVER,
            vscrollbar_policy: Gtk.PolicyType.AUTOMATIC,
            child: this.visualView,
            hexpand: true,
            vexpand: true,
        });

        this.editorStack = new Gtk.Stack({ transition_type: Gtk.StackTransitionType.CROSSFADE,
            hexpand: true, vexpand: true });
        this.editorStack.add_named(this.sourceScroll, 'code');
        this.editorStack.add_named(this.visualScroll, 'visual');
        this.root.append(this.editorStack);

        this.sourceBuffer.connect('changed', () => this._sourceChanged());
        this.visualBuffer.connect('changed', () => this._visualChanged());
        this.loadText(LatexEditor.newDocument());
    }

    _createVisualTags() {
        const specs = {
            bold: { weight: Pango.Weight.BOLD },
            italic: { style: Pango.Style.ITALIC },
            underline: { underline: Pango.Underline.SINGLE },
            monospace: { family: 'monospace' },
            heading1: { weight: Pango.Weight.BOLD, scale: 1.55, pixels_above_lines: 14, pixels_below_lines: 6 },
            heading2: { weight: Pango.Weight.BOLD, scale: 1.3, pixels_above_lines: 11, pixels_below_lines: 5 },
            heading3: { weight: Pango.Weight.BOLD, scale: 1.15, pixels_above_lines: 9, pixels_below_lines: 4 },
            heading4: { weight: Pango.Weight.BOLD, scale: 1.08, pixels_above_lines: 7, pixels_below_lines: 3 },
            heading5: { weight: Pango.Weight.BOLD, scale: 1.03, pixels_above_lines: 6, pixels_below_lines: 2 },
            raw: { family: 'monospace', style: Pango.Style.ITALIC },
        };
        for (const [name, properties] of Object.entries(specs)) {
            const tag = new Gtk.TextTag({ name, ...properties });
            this.visualBuffer.get_tag_table().add(tag);
            this.visualTags.set(name, tag);
        }
    }

    _updateSourceStyleScheme() {
        const schemeId = this.styleManager.get_dark() ? 'Adwaita-dark' : 'Adwaita';
        const scheme = GtkSource.StyleSchemeManager.get_default().get_scheme(schemeId);
        if (scheme) this.sourceBuffer.set_style_scheme(scheme);
    }

    static newDocument() {
        return `\\documentclass{article}\n\\usepackage[utf8]{inputenc}\n\\usepackage{graphicx}\n\\usepackage{hyperref}\n\\title{${_('New document')}}\n\\author{}\n\\date{\\today}\n\n\\begin{document}\n\\maketitle\n\n\\section{${_('Introduction')}}\n${_('Start writing here.')}\n\n\\bibliographystyle{plain}\n\\bibliography{references}\n\\end{document}\n`;
    }

    get file() { return this._file; }
    get dirty() { return this._dirty; }

    setFile(file) {
        this._file = file;
        this._dirty = false;
        this._savedText = this.text();
    }

    loadText(text) {
        this._synchronizing = true;
        this.sourceBuffer.set_text(text, -1);
        this._synchronizing = false;
        this._visualEdited = false;
        this._sourceCursorBeforeVisual = null;
        this._savedText = text;
        this._setDirty(false);
        this._renderVisual();
        this.history = new EditorHistory(this._captureState());
        this._notifyHistoryChanged();
    }

    text() {
        const [start, end] = [this.sourceBuffer.get_start_iter(), this.sourceBuffer.get_end_iter()];
        return this.sourceBuffer.get_text(start, end, true);
    }

    _setDirty(dirty) {
        this._dirty = dirty;
        this.onDirtyChanged?.(dirty);
    }

    _sourceChanged() {
        if (this._synchronizing) return;
        this._setDirty(this.text() !== this._savedText);
        this._renderVisual();
        this._recordHistory();
    }

    _renderVisual() {
        const document = splitLatexDocument(this.text());
        this.visualView.set_editable(document.valid);
        this.visualView.set_cursor_visible(document.valid);
        this._synchronizing = true;
        this.visualBuffer.set_text(document.valid ? this._tokensText(parseVisualBody(document.body)) :
            _('Visual mode needs a document with \\begin{document} and \\end{document}.\n\nSwitch to Code mode to review the document structure.'), -1);
        if (document.valid) this._applyTokens(parseVisualBody(document.body));
        this._synchronizing = false;
    }

    _tokensText(tokens) {
        return tokens.map(token => token.text).join('');
    }

    _applyTokens(tokens) {
        for (const tag of this.visualTags.values())
            this.visualBuffer.remove_tag(tag, this.visualBuffer.get_start_iter(), this.visualBuffer.get_end_iter());

        let offset = 0;
        for (const token of tokens) {
            const length = characterCount(token.text);
            if (length === 0) continue;
            const start = this.visualBuffer.get_iter_at_offset(offset);
            const end = this.visualBuffer.get_iter_at_offset(offset + length);
            if (token.type === 'raw') this.visualBuffer.apply_tag(this.visualTags.get('raw'), start, end);
            for (const mark of token.marks || []) {
                const tag = this.visualTags.get(mark);
                if (tag) this.visualBuffer.apply_tag(tag, start, end);
            }
            offset += length;
        }
    }

    _visualChanged() {
        if (this._synchronizing) return;
        const document = splitLatexDocument(this.text());
        if (!document.valid) return;
        const tokens = this._serializeBuffer();
        const visibleOffset = this.visualBuffer.get_iter_at_mark(this.visualBuffer.get_insert()).get_offset();
        const sourcePrefix = serializeVisualTokens(this._tokensBeforeOffset(tokens, visibleOffset));
        const body = serializeVisualTokens(tokens);
        const updated = `${document.preamble}${body}${document.ending}`;
        this._synchronizing = true;
        this.sourceBuffer.set_text(updated, -1);
        const sourceOffset = Math.min(characterCount(updated),
            characterCount(document.preamble) + characterCount(sourcePrefix));
        this.sourceBuffer.place_cursor(this.sourceBuffer.get_iter_at_offset(sourceOffset));
        this._synchronizing = false;
        this._visualEdited = true;
        this._setDirty(updated !== this._savedText);
        this._recordHistory();
    }

    _tokensBeforeOffset(tokens, offset) {
        const prefix = [];
        let remaining = offset;
        for (const token of tokens) {
            const characters = Array.from(token.text);
            if (remaining >= characters.length) {
                prefix.push(token);
                remaining -= characters.length;
                if (remaining === 0) break;
                continue;
            }
            if (remaining > 0) prefix.push({ ...token, text: characters.slice(0, remaining).join('') });
            break;
        }
        return prefix;
    }

    _placeVisualCursorFromSource(document) {
        const codeOffset = this.sourceBuffer.get_iter_at_mark(this.sourceBuffer.get_insert()).get_offset();
        const preambleLength = characterCount(document.preamble);
        const bodyLength = characterCount(document.body);
        const bodyOffset = Math.max(0, Math.min(codeOffset - preambleLength, bodyLength));
        const bodyPrefix = Array.from(document.body).slice(0, bodyOffset).join('');
        const visualOffset = characterCount(this._tokensText(parseVisualBody(bodyPrefix)));
        this.visualBuffer.place_cursor(this.visualBuffer.get_iter_at_offset(visualOffset));
    }

    _serializeBuffer() {
        const tokens = [];
        let iter = this.visualBuffer.get_start_iter();
        const end = this.visualBuffer.get_end_iter();
        while (iter.compare(end) < 0) {
            const tagObjects = iter.get_tags();
            const activeMarks = [];
            let raw = false;
            for (const [name, tag] of this.visualTags) {
                if (tagObjects.includes(tag)) {
                    if (name === 'raw') raw = true;
                    else activeMarks.push(name);
                }
            }
            const char = iter.get_char();
            if (raw) tokens.push({ type: 'raw', text: char });
            else tokens.push({ type: 'text', text: char, marks: activeMarks });
            iter.forward_char();
        }

        const merged = [];
        for (const token of tokens) {
            const previous = merged[merged.length - 1];
            if (previous && previous.type === token.type && previous.text !== undefined &&
                previous.marks?.join(':') === token.marks?.join(':')) previous.text += token.text;
            else merged.push({ ...token, marks: token.marks || [] });
        }
        return merged;
    }

    setMode(mode) {
        if (mode === 'visual') {
            this._sourceCursorBeforeVisual = this.sourceBuffer.get_iter_at_mark(
                this.sourceBuffer.get_insert()).get_offset();
            this._visualEdited = false;
            const document = splitLatexDocument(this.text());
            if (document.valid) this._placeVisualCursorFromSource(document);
        } else if (mode === 'code' && !this._visualEdited && this._sourceCursorBeforeVisual !== null) {
            this.sourceBuffer.place_cursor(this.sourceBuffer.get_iter_at_offset(this._sourceCursorBeforeVisual));
        }
        this.editorStack.set_visible_child_name(mode);
        this.focus();
    }

    _visualDocumentAvailable() {
        return splitLatexDocument(this.text()).valid;
    }

    _ensureEditableMode() {
        if (this.editorStack.get_visible_child_name() === 'visual' && !this._visualDocumentAvailable()) {
            this.setMode('code');
            this.onModeChanged?.('code');
        }
        return this.editorStack.get_visible_child_name() || 'code';
    }

    _effectiveMode() {
        if (this.editorStack.get_visible_child_name() === 'visual' && !this._visualDocumentAvailable()) return 'code';
        return this.editorStack.get_visible_child_name() || 'code';
    }

    _captureState() {
        return {
            text: this.text(),
            codeOffset: this.sourceBuffer.get_iter_at_mark(this.sourceBuffer.get_insert()).get_offset(),
            visualOffset: this.visualBuffer.get_iter_at_mark(this.visualBuffer.get_insert()).get_offset(),
            mode: this.editorStack.get_visible_child_name() || 'code',
        };
    }

    _recordHistory(group = null) {
        if (!this.history) return;
        const mode = this.editorStack.get_visible_child_name() || 'code';
        const bucket = group || this._historyGroup || `typing:${mode}`;
        this.history.record(this._captureState(), bucket, GLib.get_monotonic_time() / 1000);
        this._notifyHistoryChanged();
    }

    _notifyHistoryChanged() {
        this.onHistoryChanged?.(this.history?.canUndo || false, this.history?.canRedo || false);
    }

    _withHistoryGroup(group, callback) {
        const previous = this._historyGroup;
        this._historyGroup = group;
        try { callback(); }
        finally { this._historyGroup = previous; }
    }

    _restoreHistoryState(state) {
        if (!state) return;
        const mode = this.editorStack.get_visible_child_name() || 'code';
        this._synchronizing = true;
        this.sourceBuffer.set_text(state.text, -1);
        this._synchronizing = false;
        this._renderVisual();
        const codeOffset = Math.max(0, Math.min(characterCount(state.text), state.codeOffset));
        this.sourceBuffer.place_cursor(this.sourceBuffer.get_iter_at_offset(codeOffset));
        const document = splitLatexDocument(state.text);
        if (document.valid) this._placeVisualCursorFromSource(document);
        this._setDirty(state.text !== this._savedText);
        this._visualEdited = mode === 'visual';
        this._notifyHistoryChanged();
        this.focus();
    }

    undo() { this._restoreHistoryState(this.history?.undo()); }
    redo() { this._restoreHistoryState(this.history?.redo()); }

    _replaceBufferSelection(buffer, view, text, cursorOffset = null, selectionStart = null, selectionEnd = null) {
        let [hasSelection, start, end] = buffer.get_selection_bounds();
        if (!hasSelection) {
            start = buffer.get_iter_at_mark(buffer.get_insert());
            end = start.copy();
        }
        const baseOffset = start.get_offset();
        buffer.delete(start, end);
        const insertAt = buffer.get_iter_at_offset(baseOffset);
        buffer.insert(insertAt, text, -1);
        if (selectionStart !== null && selectionEnd !== null && selectionEnd > selectionStart) {
            const selectedStart = buffer.get_iter_at_offset(baseOffset + characterCount(text.slice(0, selectionStart)));
            const selectedEnd = buffer.get_iter_at_offset(baseOffset + characterCount(text.slice(0, selectionEnd)));
            buffer.select_range(selectedEnd, selectedStart);
        } else {
            const offset = cursorOffset === null ? characterCount(text) : characterCount(text.slice(0, cursorOffset));
            buffer.place_cursor(buffer.get_iter_at_offset(baseOffset + offset));
        }
        view.grab_focus();
        return { baseOffset };
    }

    _insertVisualLatex(snippet, cursorOffset = null) {
        const buffer = this.visualBuffer;
        this._synchronizing = true;
        const range = this._replaceBufferSelection(buffer, this.visualView, snippet, cursorOffset);
        const start = buffer.get_iter_at_offset(range.baseOffset);
        const end = buffer.get_iter_at_offset(range.baseOffset + characterCount(snippet));
        buffer.apply_tag(this.visualTags.get('raw'), start, end);
        this._synchronizing = false;
        this._visualChanged();
    }

    _applyCodeCommand(spec, group) {
        this._withHistoryGroup(group, () => {
            this._synchronizing = true;
            this._replaceBufferSelection(this.sourceBuffer, this.sourceView, spec.text,
                spec.cursorOffset, spec.selectionStart, spec.selectionEnd);
            this._synchronizing = false;
            this._sourceChanged();
        });
    }

    applyFormat(name) {
        const mode = this._ensureEditableMode();
        if (/^heading[1-5]$/.test(name)) {
            this.applyHeading(name.replace('heading', ''));
            return;
        }
        const buffer = mode === 'visual' ? this.visualBuffer : this.sourceBuffer;
        const [hasSelection, start, end] = buffer.get_selection_bounds();
        const selected = hasSelection ? buffer.get_text(start, end, true) : '';
        const spec = createInlineCommand(name, selected);
        if (mode === 'code') {
            this._applyCodeCommand(spec, `toolbar:${name}`);
            return;
        }
        if (!hasSelection) {
            this._withHistoryGroup(`toolbar:${name}`, () => this._insertVisualLatex(spec.text, spec.cursorOffset));
            return;
        }
        const tag = this.visualTags.get(name);
        if (!tag) return;
        this._withHistoryGroup(`toolbar:${name}`, () => {
            if (start.has_tag(tag)) this.visualBuffer.remove_tag(tag, start, end);
            else this.visualBuffer.apply_tag(tag, start, end);
            this._visualChanged();
        });
    }

    applyHeading(style) {
        const mode = this._ensureEditableMode();
        if (mode !== 'visual') {
            const [hasSelection, start, end] = this.sourceBuffer.get_selection_bounds();
            const selected = hasSelection ? this.sourceBuffer.get_text(start, end, true) : '';
            if (style === 'normal' && !selected) return;
            this._applyCodeCommand(createHeadingCommand(style, selected), `toolbar:heading:${style}`);
            return;
        }

        const buffer = this.visualBuffer;
        const [hasSelection, selectedStart, selectedEnd] = buffer.get_selection_bounds();
        let start = selectedStart;
        let end = selectedEnd;
        if (!hasSelection) {
            start = buffer.get_iter_at_mark(buffer.get_insert());
            end = start.copy();
            start.backward_to_line_start();
            end.forward_to_line_end();
        }
        const tagName = style === 'normal' ? null : `heading${['section', 'subsection', 'subsubsection', 'paragraph', 'subparagraph'].indexOf(style) + 1}`;
        if (tagName && !this.visualTags.has(tagName)) return;
        if (start.equal(end)) {
            const spec = createHeadingCommand(style);
            if (spec.text) this._withHistoryGroup(`toolbar:heading:${style}`, () => this._insertVisualLatex(spec.text, spec.cursorOffset));
            return;
        }
        this._withHistoryGroup(`toolbar:heading:${style}`, () => {
            for (let level = 1; level <= 5; level++)
                this.visualBuffer.remove_tag(this.visualTags.get(`heading${level}`), start, end);
            if (tagName) this.visualBuffer.apply_tag(this.visualTags.get(tagName), start, end);
            this._visualChanged();
        });
    }

    insertLatexSnippet(snippet, cursorOffset = null, group = 'insert:latex') {
        const mode = this._ensureEditableMode();
        this._withHistoryGroup(group, () => {
            if (mode === 'visual') this._insertVisualLatex(snippet, cursorOffset);
            else {
                this._synchronizing = true;
                this._replaceBufferSelection(this.sourceBuffer, this.sourceView, snippet, cursorOffset);
                this._synchronizing = false;
                this._sourceChanged();
            }
        });
    }

    insertLatexAtCursor(snippet, cursorOffset = null, group = 'insert:latex') {
        const mode = this._ensureEditableMode();
        const buffer = mode === 'visual' ? this.visualBuffer : this.sourceBuffer;
        const [hasSelection, _start, end] = buffer.get_selection_bounds();
        if (hasSelection) buffer.place_cursor(end);
        this.insertLatexSnippet(snippet, cursorOffset, group);
    }

    insertFootnote() {
        const selected = this.selectedText();
        if (selected) {
            const snippet = String.raw`\footnote{${selected}}`;
            this.insertLatexSnippet(snippet, snippet.length, 'insert:footnote');
        } else this.insertLatexSnippet(String.raw`\footnote{}`, 10, 'insert:footnote');
    }

    insertComment() {
        const selected = this.selectedText();
        if (selected) {
            const comment = selected.split(/\r?\n/).map(line => line ? `% ${line}` : '%').join('\n');
            this.insertLatexSnippet(comment, comment.length, 'insert:comment');
        } else this.insertLatexSnippet('% ', 2, 'insert:comment');
    }

    insertCommand(spec, group = 'insert:latex') {
        this.insertLatexSnippet(spec.text, spec.cursorOffset, group);
    }

    adjustIndent(increase) {
        const mode = this._ensureEditableMode();
        const visual = mode === 'visual';
        const buffer = visual ? this.visualBuffer : this.sourceBuffer;
        const [hasSelection, selectedStart, selectedEnd] = buffer.get_selection_bounds();
        const first = hasSelection ? selectedStart.copy() : buffer.get_iter_at_mark(buffer.get_insert());
        let last = hasSelection ? selectedEnd.copy() : first.copy();
        const firstLine = first.get_line();
        if (hasSelection && last.get_line() > firstLine && last.get_line_offset() === 0)
            last.backward_char();
        first.set_line_offset(0);
        const lastLine = last.get_line();
        const offsets = [];
        const line = first.copy();
        while (line.get_line() <= lastLine) {
            offsets.push(line.get_offset());
            if (!line.forward_line()) break;
        }

        const prefix = String.raw`\hspace{1em}`;
        this._withHistoryGroup(increase ? 'toolbar:indent-increase' : 'toolbar:indent-decrease', () => {
            this._synchronizing = true;
            for (const offset of offsets.reverse()) {
                const start = buffer.get_iter_at_offset(offset);
                if (increase) {
                    const lineEnd = start.copy();
                    lineEnd.forward_to_line_end();
                    if (!buffer.get_text(start, lineEnd, true).trim()) continue;
                    buffer.insert(start, prefix, -1);
                    if (visual) {
                        const prefixStart = buffer.get_iter_at_offset(offset);
                        const prefixEnd = buffer.get_iter_at_offset(offset + characterCount(prefix));
                        buffer.apply_tag(this.visualTags.get('raw'), prefixStart, prefixEnd);
                    }
                } else {
                    const prefixEnd = start.copy();
                    for (let index = 0; index < characterCount(prefix); index++) prefixEnd.forward_char();
                    if (buffer.get_text(start, prefixEnd, true) === prefix) buffer.delete(start, prefixEnd);
                }
            }
            this._synchronizing = false;
            if (visual) this._visualChanged();
            else this._sourceChanged();
        });
    }

    selectedText() {
        const mode = this._effectiveMode();
        const buffer = mode === 'visual' ? this.visualBuffer : this.sourceBuffer;
        const [hasSelection, start, end] = buffer.get_selection_bounds();
        return hasSelection ? buffer.get_text(start, end, true) : '';
    }

    focus() {
        const mode = this.editorStack.get_visible_child_name();
        if (mode === 'visual' && !this._visualDocumentAvailable()) return;
        (mode === 'visual' ? this.visualView : this.sourceView).grab_focus();
    }

    save() {
        if (!this._file) return false;
        GLib.file_set_contents(this._file.get_path(), this.text());
        this._savedText = this.text();
        this._setDirty(false);
        return true;
    }
}
