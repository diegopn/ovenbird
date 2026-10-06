import { _ } from './i18n.js';

const INLINE_COMMANDS = new Map([
    ['bold', 'textbf'],
    ['italic', 'emph'],
    ['underline', 'underline'],
    ['monospace', 'texttt'],
]);

const HEADING_COMMANDS = new Map([
    ['section', 'section'],
    ['subsection', 'subsection'],
    ['subsubsection', 'subsubsection'],
    ['paragraph', 'paragraph'],
    ['subparagraph', 'subparagraph'],
]);

function wrap(command, selectedText = '') {
    const prefix = `\\${command}{`;
    const text = `${prefix}${selectedText}}`;
    return {
        text,
        cursorOffset: prefix.length + selectedText.length,
        selectionStart: selectedText ? prefix.length : null,
        selectionEnd: selectedText ? prefix.length + selectedText.length : null,
    };
}

export function createInlineCommand(format, selectedText = '') {
    const command = INLINE_COMMANDS.get(format);
    if (!command) throw new Error(_('Unknown LaTeX formatting: %s').replace('%s', format));
    return wrap(command, selectedText);
}

export function createHeadingCommand(style, selectedText = '') {
    if (style === 'normal') {
        const heading = selectedText.match(/^\\(?:section|subsection|subsubsection|paragraph|subparagraph)\{([\s\S]*)\}$/);
        const text = heading ? heading[1] : selectedText;
        return {
            text,
            cursorOffset: text.length,
            selectionStart: selectedText ? 0 : null,
            selectionEnd: selectedText ? text.length : null,
        };
    }
    const command = HEADING_COMMANDS.get(style);
    if (!command) throw new Error(_('Unknown paragraph style: %s').replace('%s', style));
    return wrap(command, selectedText);
}

export function createListSnippet(kind, selectedText = '') {
    if (!['itemize', 'enumerate', 'quote'].includes(kind))
        throw new Error(_('Unknown list environment: %s').replace('%s', kind));

    if (kind === 'quote') {
        const prefix = String.raw`\begin{quote}` + '\n';
        const text = `${prefix}${selectedText}\n${String.raw`\end{quote}`}`;
        return { text, cursorOffset: prefix.length + selectedText.length };
    }

    const prefix = String.raw`\begin{${kind}}` + '\n';
    const lines = selectedText ? selectedText.split(/\r?\n/) : [''];
    const items = lines.map(line => `${String.raw`\item `}${line}`).join('\n');
    const text = `${prefix}${items}\n${String.raw`\end{${kind}}`}`;
    return {
        text,
        cursorOffset: selectedText ? text.length : prefix.length + String.raw`\item `.length,
    };
}

export function createMathSnippet(display = false, selectedText = '') {
    if (display) {
        const text = `\\[ ${selectedText} \\]`;
        return { text, cursorOffset: selectedText ? 3 + selectedText.length : 3 };
    }
    const text = `\\(${selectedText}\\)`;
    return { text, cursorOffset: selectedText ? 2 + selectedText.length : 2 };
}

export function createTableSnippet(rows = 2, columns = 2) {
    const rowCount = Math.max(1, Math.min(20, Math.trunc(rows) || 1));
    const columnCount = Math.max(1, Math.min(12, Math.trunc(columns) || 1));
    const layout = `|${Array(columnCount).fill('l').join('|')}|`;
    const body = Array(rowCount).fill(Array(columnCount).fill('').join(' & ') + ' \\\\');
    const lines = [String.raw`\hline`];
    for (const row of body) lines.push(row, String.raw`\hline`);
    const prefix = `${String.raw`\begin{tabular}{${layout}}`}\n`;
    const text = `${prefix}${lines.join('\n')}\n${String.raw`\end{tabular}`}`;
    return { text, cursorOffset: prefix.length + String.raw`\hline`.length + 1 };
}
