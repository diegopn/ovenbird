export function findDocumentMatch(text, query, fromOffset, direction = 'forward') {
    if (!query.trim()) return null;

    const characterOffsets = new Map([[0, 0]]);
    let utf16Offset = 0;
    let characterOffset = 0;
    for (const character of text) {
        utf16Offset += character.length;
        characterOffset++;
        characterOffsets.set(utf16Offset, characterOffset);
    }

    const boundedOffset = Math.max(0, Math.min(characterOffset, Math.trunc(fromOffset)));
    const escapedQuery = query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const expression = new RegExp(escapedQuery, 'giu');
    let first = null;
    let last = null;
    let next = null;
    let previous = null;
    let match;

    while ((match = expression.exec(text)) !== null) {
        const start = characterOffsets.get(match.index);
        const end = characterOffsets.get(match.index + match[0].length);
        const range = { start, end };
        first ??= range;
        last = range;

        if (start >= boundedOffset && next === null) next = range;
        if (end <= boundedOffset) previous = range;
    }

    if (direction === 'backward') return previous || last;
    return next || first;
}
