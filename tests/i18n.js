import GLib from 'gi://GLib';
import { LatexEditor } from '../src/core/editor.js';
import { _, ngettext } from '../src/core/i18n.js';

function assertEqual(actual, expected, message) {
    if (actual !== expected)
        throw new Error(`${message}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
}

const expectedTitle = GLib.getenv('OVENBIRD_EXPECTED_TITLE');
const expectedIntroduction = GLib.getenv('OVENBIRD_EXPECTED_INTRODUCTION');
const expectedReferenceSingular = GLib.getenv('OVENBIRD_EXPECTED_REFERENCE_SINGULAR');
const expectedReferencePlural = GLib.getenv('OVENBIRD_EXPECTED_REFERENCE_PLURAL');
assertEqual(_('New document'), expectedTitle, 'The interface should use the translation for the system language');
assertEqual(_('Introduction'), expectedIntroduction, 'The document template should use the selected language');
assertEqual(ngettext('%d local reference', '%d local references', 1).replace('%d', '1'),
    expectedReferenceSingular, 'Singular quantities should use the selected language');
assertEqual(ngettext('%d local reference', '%d local references', 2).replace('%d', '2'),
    expectedReferencePlural, 'Plural quantities should use the selected language');

const source = LatexEditor.newDocument();
if (!source.includes(`\\title{${expectedTitle}}`) || !source.includes(`\\section{${expectedIntroduction}}`))
    throw new Error('The new LaTeX document should use the selected language');

print('Localization tests passed');
