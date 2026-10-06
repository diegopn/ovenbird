const INLINE_COMMANDS = new Map([
    ['textbf', 'bold'],
    ['textit', 'italic'],
    ['emph', 'italic'],
    ['underline', 'underline'],
    ['texttt', 'monospace'],
]);
const HEADING_COMMANDS = ['section', 'subsection', 'subsubsection', 'paragraph', 'subparagraph'];

function readGroup(source, start) {
    if (source[start] !== '{') return null;
    let depth = 0;
    let escaped = false;
    for (let i = start; i < source.length; i++) {
        const char = source[i];
        if (escaped) {
            escaped = false;
            continue;
        }
        if (char === '\\') {
            escaped = true;
            continue;
        }
        if (char === '{') depth++;
        if (char === '}' && --depth === 0)
            return { content: source.slice(start + 1, i), end: i + 1 };
    }
    return null;
}

function skipWhitespace(source, position) {
    while (/\s/.test(source[position] || '')) position++;
    return position;
}

function readOptionalGroup(source, start) {
    if (source[start] !== '[') return null;
    let squareDepth = 0;
    let braceDepth = 0;
    let escaped = false;
    for (let i = start; i < source.length; i++) {
        const char = source[i];
        if (escaped) {
            escaped = false;
            continue;
        }
        if (char === '\\') {
            escaped = true;
            continue;
        }
        if (char === '{') braceDepth++;
        else if (char === '}' && braceDepth > 0) braceDepth--;
        else if (braceDepth === 0 && char === '[') squareDepth++;
        else if (braceDepth === 0 && char === ']' && --squareDepth === 0)
            return { content: source.slice(start + 1, i), end: i + 1 };
    }
    return null;
}

function readCommandArguments(source, start) {
    let position = start;
    let end = start;
    const groups = [];
    const optionalGroups = [];
    let malformed = false;
    while (position < source.length) {
        position = skipWhitespace(source, position);
        const opening = source[position];
        const group = opening === '{' ? readGroup(source, position) :
            opening === '[' ? readOptionalGroup(source, position) : null;
        if ((opening === '{' || opening === '[') && !group) {
            malformed = true;
            end = source.length;
            break;
        }
        if (!group) break;
        if (opening === '{') groups.push(group);
        else optionalGroups.push(group);
        position = group.end;
        end = position;
    }
    return { groups, optionalGroups, end, malformed };
}

function matchingEnvironmentEnd(source, name, start) {
    const commands = /\\(begin|end)\s*\{([^{}]+)\}/g;
    let depth = 1;
    commands.lastIndex = start;
    let match;
    while ((match = commands.exec(source))) {
        const lineStart = source.lastIndexOf('\n', match.index - 1) + 1;
        let backslashes = 0;
        let inComment = false;
        for (let i = lineStart; i < match.index; i++) {
            if (source[i] === '\\') backslashes++;
            else {
                if (source[i] === '%' && backslashes % 2 === 0) {
                    inComment = true;
                    break;
                }
                backslashes = 0;
            }
        }
        if (inComment) continue;
        if (match[2].trim() !== name) continue;
        if (match[1] === 'begin') depth++;
        else if (--depth === 0) return commands.lastIndex;
    }
    return -1;
}

function pushText(tokens, text, marks) {
    if (!text) return;
    const previous = tokens[tokens.length - 1];
    const signature = marks.join(':');
    if (previous?.type === 'text' && previous.signature === signature) {
        previous.text += text;
        return;
    }
    tokens.push({ type: 'text', text, marks: [...marks], signature });
}

function parseInline(source, marks = [], tokens = []) {
    let position = 0;
    while (position < source.length) {
        if (source[position] === '$') {
            const delimiter = source.startsWith('$$', position) ? '$$' : '$';
            const end = source.indexOf(delimiter, position + delimiter.length);
            const finish = end < 0 ? source.length : end + delimiter.length;
            tokens.push({ type: 'raw', text: source.slice(position, finish) });
            position = finish;
            continue;
        }

        if (source.startsWith('\\(', position) || source.startsWith('\\[', position)) {
            const delimiter = source.slice(position, position + 2) === '\\(' ? '\\)' : '\\]';
            const end = source.indexOf(delimiter, position + 2);
            const finish = end < 0 ? source.length : end + 2;
            tokens.push({ type: 'raw', text: source.slice(position, finish) });
            position = finish;
            continue;
        }

        if (source[position] === '%') {
            const end = source.indexOf('\n', position);
            const finish = end < 0 ? source.length : end;
            tokens.push({ type: 'raw', text: source.slice(position, finish) });
            position = finish;
            continue;
        }

        if (source[position] === '\\') {
            const command = source.slice(position).match(/^\\([a-zA-Z]+|.)/);
            if (!command) {
                pushText(tokens, source[position], marks);
                position++;
                continue;
            }
            const commandName = command[1];
            const afterCommand = position + command[0].length;

            if (commandName === 'par') {
                tokens.push({ type: 'paragraph', text: '\n\n' });
                position = afterCommand;
                continue;
            }
            if (commandName === '\\') {
                let finish = afterCommand;
                if (source[finish] === '*') finish++;
                const optional = readOptionalGroup(source, skipWhitespace(source, finish));
                if (optional) finish = optional.end;
                tokens.push({ type: 'raw', text: source.slice(position, finish) });
                position = finish;
                continue;
            }

            if (commandName === 'verb') {
                let start = afterCommand;
                if (source[start] === '*') start++;
                const delimiter = source[start];
                const end = delimiter ? source.indexOf(delimiter, start + 1) : -1;
                const finish = end < 0 ? source.length : end + 1;
                tokens.push({ type: 'raw', text: source.slice(position, finish) });
                position = finish;
                continue;
            }

            if (HEADING_COMMANDS.includes(commandName) &&
                source[afterCommand] === '*') {
                const args = readCommandArguments(source, afterCommand + 1);
                const group = args.groups[0];
                const finish = group ? args.end : afterCommand + 1;
                tokens.push({ type: 'raw', text: source.slice(position, finish) });
                position = finish;
                continue;
            }

            const args = readCommandArguments(source, afterCommand);
            const group = args.groups[0];

            if (args.malformed) {
                tokens.push({ type: 'raw', text: source.slice(position) });
                position = source.length;
                continue;
            }

            if (commandName === 'begin' && group) {
                const environment = group.content.trim();
                const end = matchingEnvironmentEnd(source, environment, group.end);
                const finish = end < 0 ? source.length : end;
                tokens.push({ type: 'raw', text: source.slice(position, finish) });
                position = finish;
                continue;
            }

            if (HEADING_COMMANDS.includes(commandName) &&
                args.optionalGroups.length === 0 && group) {
                const level = HEADING_COMMANDS.indexOf(commandName) + 1;
                const headingMarks = [...marks, 'heading' + level];
                parseInline(group.content, headingMarks, tokens);
                tokens.push({ type: 'paragraph', text: '\n\n' });
                position = group.end;
                continue;
            }
            if (INLINE_COMMANDS.has(commandName) && args.optionalGroups.length === 0 && group) {
                parseInline(group.content, [...marks, INLINE_COMMANDS.get(commandName)], tokens);
                position = group.end;
                continue;
            }
            if (['cite', 'parencite', 'textcite', 'autocite', 'footcite', 'citeauthor',
                'citeyear', 'nocite'].includes(commandName) && group) {
                tokens.push({ type: 'raw', text: source.slice(position, args.end) });
                position = args.end;
                continue;
            }

            if (args.end > afterCommand) {
                tokens.push({ type: 'raw', text: source.slice(position, args.end) });
                position = args.end;
            } else {
                tokens.push({ type: 'raw', text: command[0] });
                position = afterCommand;
            }
            continue;
        }

        const next = source.slice(position).search(/[\\$%]/);
        const end = next < 0 ? source.length : position + next;
        const plain = source.slice(position, end);
        const parts = plain.split(/(\n[ \t]*\n+)/);
        for (const part of parts) {
            if (/^\n[ \t]*\n+$/.test(part)) tokens.push({ type: 'paragraph', text: part });
            else pushText(tokens, part, marks);
        }
        position = end;
    }
    return tokens;
}

export function splitLatexDocument(source) {
    const begin = /\\begin\s*\{document\}/.exec(source);
    const end = /\\end\s*\{document\}/g;
    let endMatch = null;
    let match;
    while ((match = end.exec(source))) endMatch = match;
    if (!begin || !endMatch || endMatch.index < begin.index)
        return { valid: false, source };

    const bodyStart = begin.index + begin[0].length;
    return {
        valid: true,
        preamble: source.slice(0, bodyStart),
        body: source.slice(bodyStart, endMatch.index),
        ending: source.slice(endMatch.index),
    };
}

export function parseVisualBody(source) {
    return parseInline(source);
}

function escapeLatexText(text) {
    return text.replace(/[#$%&_{}]/g, char => `\\${char}`)
        .replace(/~/g, '\\textasciitilde{}')
        .replace(/\^/g, '\\textasciicircum{}');
}

export function serializeVisualTokens(tokens) {
    const output = [];
    let index = 0;
    while (index < tokens.length) {
        const token = tokens[index];
        if (token.type === 'raw') {
            output.push(token.text);
            index++;
            continue;
        }
        if (token.type === 'paragraph' || token.type === 'linebreak') {
            output.push(token.text);
            index++;
            continue;
        }

        const marks = token.marks || [];
        let text = '';
        while (index < tokens.length && tokens[index].type === 'text' &&
            (tokens[index].marks || []).join(':') === marks.join(':')) {
            text += escapeLatexText(tokens[index].text);
            index++;
        }

        const heading = marks.find(mark => /^heading[1-5]$/.test(mark));
        const formats = marks.filter(mark => !/^heading[1-5]$/.test(mark));
        for (const format of formats.reverse()) {
            const command = { bold: 'textbf', italic: 'emph', underline: 'underline', monospace: 'texttt' }[format];
            if (command) text = `\\${command}{${text}}`;
        }
        if (heading) {
            const command = {
                heading1: 'section', heading2: 'subsection', heading3: 'subsubsection',
                heading4: 'paragraph', heading5: 'subparagraph',
            }[heading];
            text = `\\${command}{${text}}`;
        }
        output.push(text);
    }
    return output.join('');
}
