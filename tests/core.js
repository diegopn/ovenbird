import { parseBibtex, serializeBibtex } from '../src/core/bibtex.js';
import { parseVisualBody, serializeVisualTokens, splitLatexDocument } from '../src/core/latex-document.js';
import { createHeadingCommand, createInlineCommand, createListSnippet, createMathSnippet, createTableSnippet } from '../src/core/latex-commands.js';
import { EditorHistory } from '../src/core/editor-history.js';
import { findDocumentMatch } from '../src/core/document-search.js';

function assertEqual(actual, expected, message) {
    if (actual !== expected)
        throw new Error(`${message}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
}

function testDocumentWithOneEndMarkerIsVisualizable() {
    const source = String.raw`\documentclass{article}
\begin{document}
Texto para edição.
\end{document}
`;
    const document = splitLatexDocument(source);

    assertEqual(document.valid, true, 'A standard LaTeX document should be recognized');
    assertEqual(document.body, '\nTexto para edição.\n', 'The visual body should exclude the preamble and ending');
    assertEqual(document.ending, String.raw`\end{document}` + '\n', 'The document ending should be preserved');
}

function testStringMacrosExpandForDisplayAndRemainInBibtex() {
    const source = String.raw`@string{journaltitle = {Journal of Testing}}
@article{Smith2022,
  title = {A Citation},
  journal = journaltitle # { Review}
}`;
    const entries = parseBibtex(source);

    assertEqual(entries[0].fields.journal, 'Journal of Testing Review',
        'A string macro and concatenated literal should resolve in the field value');

    const serialized = serializeBibtex(entries, entries.directives);
    if (!serialized.includes('journal = journaltitle # { Review}'))
        throw new Error('An unchanged field should retain its BibTeX macro expression');

    const reparsed = parseBibtex(serialized);
    assertEqual(reparsed[0].fields.journal, 'Journal of Testing Review',
        'A serialized macro expression should still resolve after re-import');
}

function testToolbarCommandsCreateLatexAndPlaceCaret() {
    const bold = createInlineCommand('bold', 'palavra');
    assertEqual(bold.text, String.raw`\textbf{palavra}`, 'Bold should wrap selected text with LaTeX');
    assertEqual(bold.selectionStart, 8, 'The selected text should remain addressable inside the command');
    assertEqual(bold.selectionEnd, 15, 'The selected range should preserve its full length');

    const italic = createInlineCommand('italic');
    assertEqual(italic.text, String.raw`\emph{}`, 'Italic should insert an empty LaTeX command');
    assertEqual(italic.cursorOffset, 6, 'The caret should be inside the empty command');

    const normal = createHeadingCommand('normal', String.raw`\section{Introdução}`);
    assertEqual(normal.text, 'Introdução', 'Normal text should remove a selected heading wrapper');

    const heading = createHeadingCommand('subparagraph', 'Detalhes');
    assertEqual(heading.text, String.raw`\subparagraph{Detalhes}`, 'Subparagraph should map to LaTeX');
}

function testListMathAndTableSnippets() {
    const list = createListSnippet('enumerate', 'Primeiro\nSegundo');
    assertEqual(list.text, String.raw`\begin{enumerate}` + '\n' + String.raw`\item Primeiro` + '\n' +
        String.raw`\item Segundo` + '\n' + String.raw`\end{enumerate}`,
    'Selected lines should become list items');

    const math = createMathSnippet(true);
    assertEqual(math.text, String.raw`\[  \]`, 'Display math should use LaTeX display delimiters');
    assertEqual(math.cursorOffset, 3, 'The caret should be inside the display math delimiters');

    const table = createTableSnippet(2, 2);
    assertEqual(table.text, String.raw`\begin{tabular}{|l|l|}` + '\n' + String.raw`\hline` + '\n' +
        String.raw` &  \\` + '\n' + String.raw`\hline` + '\n' + String.raw` &  \\` + '\n' +
        String.raw`\hline` + '\n' +
        String.raw`\end{tabular}`, 'A table snippet should contain the requested rows and columns');
    assertEqual(table.text.slice(table.cursorOffset, table.cursorOffset + 1), ' ',
        'The caret should be placed in the first table cell');
}

function testParagraphHeadingsRoundTripThroughVisualTokens() {
    const tokens = parseVisualBody(String.raw`\paragraph{Contexto}`);
    assertEqual(tokens[0].marks.includes('heading4'), true,
        'Paragraph headings should be represented as visual heading formatting');
    assertEqual(serializeVisualTokens(tokens), String.raw`\paragraph{Contexto}` + '\n\n',
        'Visual serialization should preserve the paragraph command');
}

function testEditorHistoryCoalescesTypingAndSupportsRedo() {
    const history = new EditorHistory({ text: 'a', codeOffset: 1, visualOffset: 0, mode: 'code' });
    history.record({ text: 'ab', codeOffset: 2, visualOffset: 0, mode: 'code' }, 'typing:code', 100);
    history.record({ text: 'abc', codeOffset: 3, visualOffset: 0, mode: 'code' }, 'typing:code', 200);
    assertEqual(history.undo().text, 'a', 'One undo should reverse a burst of typing');
    assertEqual(history.redo().text, 'abc', 'Redo should restore the coalesced typing burst');
    history.record({ text: 'abcd', codeOffset: 4, visualOffset: 0, mode: 'code' }, 'toolbar:bold', 210);
    assertEqual(history.undo().text, 'abc', 'A toolbar action should remain a distinct undo step');
}

function testDocumentSearchFindsCaseInsensitiveMatchesAndWraps() {
    assertEqual(findDocumentMatch('Introdução e INTRODUÇÃO', 'introdução', 0, 'forward').start, 0,
        'A forward search should find the first case-insensitive match');
    assertEqual(findDocumentMatch('Introdução e INTRODUÇÃO', 'introdução', 10, 'forward').start, 13,
        'A forward search should find the next match using character offsets');
    assertEqual(findDocumentMatch('Introdução e INTRODUÇÃO', 'introdução', 23, 'forward').start, 0,
        'A forward search should wrap to the first result');
    assertEqual(findDocumentMatch('Introdução e INTRODUÇÃO', 'introdução', 13, 'backward').start, 0,
        'A backward search should find the prior result');
    assertEqual(findDocumentMatch('Introdução e INTRODUÇÃO', 'introdução', 0, 'backward').start, 13,
        'A backward search should wrap to the last result');
    assertEqual(findDocumentMatch('A seção {Intro}', 'seção {Intro}', 0, 'forward').start, 2,
        'Search terms containing punctuation should be treated literally');
    assertEqual(findDocumentMatch('😀 texto TEXTO', 'texto', 7, 'forward').start, 8,
        'Search offsets should count Unicode characters rather than UTF-16 code units');
    assertEqual(findDocumentMatch('Introdução', '   ', 0, 'forward'), null,
        'An empty query should not produce a match');
}

testDocumentWithOneEndMarkerIsVisualizable();
testStringMacrosExpandForDisplayAndRemainInBibtex();
testToolbarCommandsCreateLatexAndPlaceCaret();
testListMathAndTableSnippets();
testParagraphHeadingsRoundTripThroughVisualTokens();
testEditorHistoryCoalescesTypingAndSupportsRedo();
testDocumentSearchFindsCaseInsensitiveMatchesAndWraps();
print('Core tests passed');
