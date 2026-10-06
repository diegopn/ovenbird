import { _ } from './i18n.js';

function skipSpace(source, position) {
    while (position < source.length && /\s/.test(source[position])) position++;
    return position;
}

function readBalanced(source, position, open, close) {
    let depth = 0;
    let escaped = false;
    for (let i = position; i < source.length; i++) {
        const character = source[i];
        if (escaped) {
            escaped = false;
            continue;
        }
        if (character === '\\') {
            escaped = true;
            continue;
        }
        if (character === open) depth++;
        if (character === close && --depth === 0)
            return { value: source.slice(position + 1, i), end: i + 1 };
    }
    throw new Error(_('BibTeX contains an unclosed value.'));
}

function readExpression(source, position, closing) {
    const start = position;
    let braceDepth = 0;
    let quote = false;
    let escaped = false;
    while (position < source.length) {
        const character = source[position];
        if (escaped) {
            escaped = false;
            position++;
            continue;
        }
        if (character === '\\') {
            escaped = true;
            position++;
            continue;
        }
        if (character === '"' && braceDepth === 0) quote = !quote;
        else if (!quote && character === '{') braceDepth++;
        else if (!quote && character === '}' && braceDepth > 0) braceDepth--;
        else if (!quote && braceDepth === 0 && (character === ',' || character === closing)) break;
        position++;
    }
    return { raw: source.slice(start, position).trim(), end: position };
}

function stringMacros(directives) {
    const macros = new Map([
        ['jan', 'January'], ['feb', 'February'], ['mar', 'March'], ['apr', 'April'],
        ['may', 'May'], ['jun', 'June'], ['jul', 'July'], ['aug', 'August'],
        ['sep', 'September'], ['oct', 'October'], ['nov', 'November'], ['dec', 'December'],
    ]);
    for (const directive of directives) {
        const header = directive.match(/^\s*@string\s*([({])/i);
        if (!header) continue;
        const closing = header[1] === '{' ? '}' : ')';
        let position = skipSpace(directive, header[0].length);
        const definition = directive.slice(position).match(/^([\w-]+)\s*=\s*/);
        if (!definition) continue;
        const name = definition[1].toLowerCase();
        position += definition[0].length;
        const expression = readExpression(directive, position, closing);
        macros.set(name, expression.raw);
    }
    return macros;
}

function expressionValue(raw, macros = new Map(), resolving = new Set()) {
    const chunks = [];
    let i = 0;
    while (i < raw.length) {
        i = skipSpace(raw, i);
        if (raw[i] === '#') {
            i++;
            continue;
        }
        if (raw[i] === '{') {
            const group = readBalanced(raw, i, '{', '}');
            chunks.push(group.value);
            i = group.end;
        } else if (raw[i] === '"') {
            let j = i + 1;
            let escaped = false;
            while (j < raw.length) {
                if (!escaped && raw[j] === '"') break;
                if (!escaped && raw[j] === '\\') escaped = true;
                else escaped = false;
                j++;
            }
            chunks.push(raw.slice(i + 1, j));
            i = Math.min(j + 1, raw.length);
        } else {
            const match = raw.slice(i).match(/^[^#\s]+/);
            if (!match) break;
            const name = match[0].toLowerCase();
            if (macros.has(name) && !resolving.has(name)) {
                const nested = new Set(resolving);
                nested.add(name);
                chunks.push(expressionValue(macros.get(name), macros, nested));
            } else {
                chunks.push(match[0]);
            }
            i += match[0].length;
        }
    }
    return chunks.join('');
}

export function parseBibtex(source) {
    const entries = [];
    const directives = [];
    let position = 0;

    while (position < source.length) {
        const at = source.indexOf('@', position);
        if (at < 0) break;
        position = at + 1;

        const typeMatch = source.slice(position).match(/^\s*([\w-]+)/);
        if (!typeMatch) continue;
        const type = typeMatch[1].toLowerCase();
        position += typeMatch[0].length;
        position = skipSpace(source, position);

        const opening = source[position];
        const closing = opening === '{' ? '}' : opening === '(' ? ')' : null;
        if (!closing) continue;
        position++;

        if (['comment', 'preamble', 'string'].includes(type)) {
            const result = readBalanced(source, position - 1, opening, closing);
            directives.push(source.slice(at, result.end).trim());
            position = result.end;
            continue;
        }

        position = skipSpace(source, position);
        const keyStart = position;
        while (position < source.length && source[position] !== ',' && source[position] !== closing)
            position++;
        const key = source.slice(keyStart, position).trim();
        if (source[position] !== ',') continue;
        position++;

        const fields = {};
        const rawFields = {};
        while (position < source.length) {
            position = skipSpace(source, position);
            if (source[position] === ',' || source[position] === '\n') {
                position++;
                continue;
            }
            if (source[position] === closing) {
                position++;
                break;
            }

            const fieldMatch = source.slice(position).match(/^([\w-]+)\s*=\s*/);
            if (!fieldMatch) {
                while (position < source.length && source[position] !== ',' && source[position] !== closing)
                    position++;
                if (source[position] === ',') position++;
                continue;
            }
            const fieldName = fieldMatch[1].toLowerCase();
            position += fieldMatch[0].length;
            const expression = readExpression(source, position, closing);
            rawFields[fieldName] = expression.raw;
            fields[fieldName] = expressionValue(expression.raw);
            position = expression.end;
            if (source[position] === ',') position++;
        }

        if (key)
            entries.push({ type, key, fields, rawFields });
    }

    const macros = stringMacros(directives);
    for (const entry of entries) {
        for (const [fieldName, raw] of Object.entries(entry.rawFields))
            entry.fields[fieldName] = expressionValue(raw, macros);
    }

    Object.defineProperty(entries, 'directives', { value: directives, enumerable: false });
    return entries;
}

function bibValue(entry, fieldName, value, macros) {
    const raw = entry.rawFields?.[fieldName];
    if (raw !== undefined && expressionValue(raw, macros) === value)
        return raw;
    return `{${String(value).replace(/(?<!\\)%/g, '\\%')}}`;
}

export function serializeBibtex(entries, directives = entries.directives || []) {
    const macros = stringMacros(directives);
    const serializedEntries = entries.map(entry => {
        const fields = Object.entries(entry.fields || {})
            .filter(([, value]) => value !== null && value !== undefined && String(value).trim() !== '')
            .map(([name, value]) => `  ${name} = ${bibValue(entry, name, value, macros)}`);
        return `@${entry.type || 'misc'}{${entry.key},\n${fields.join(',\n')}\n}`;
    });
    const content = [...directives, ...serializedEntries].filter(Boolean).join('\n\n');
    return content ? content + '\n' : '';
}

export function createCitationKey(fields, usedKeys = new Set()) {
    const author = fields.author || fields.editor || 'ref';
    const firstAuthor = author.split(/\s+and\s+/i)[0].trim();
    const surname = firstAuthor.includes(',')
        ? firstAuthor.split(',', 1)[0].trim().replace(/[{}]/g, '')
        : firstAuthor.replace(/^\{|\}$/g, '').split(/\s+/).filter(Boolean).pop() || 'ref';
    const titleWord = (fields.title || '').replace(/[{}\\]/g, '').split(/\s+/).find(word => word.length > 3) || '';
    const base = `${surname}${fields.year || ''}${titleWord}`.normalize('NFKD')
        .replace(/\p{M}/gu, '').replace(/[^a-z\d]/gi, '') || 'ref';
    let key = base;
    let suffix = 2;
    while (usedKeys.has(key.toLowerCase())) key = `${base}${suffix++}`;
    return key;
}
