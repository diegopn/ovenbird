import { buildOutputDirectory } from '../src/core/build.js';

function assertEqual(actual, expected, message) {
    if (actual !== expected)
        throw new Error(`${message}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
}

const first = buildOutputDirectory('/home/example/project/main.tex', '/cache');
const second = buildOutputDirectory('/home/example/project/main.tex', '/cache');
const otherProject = buildOutputDirectory('/home/example/other/main.tex', '/cache');

assertEqual(first.startsWith('/cache/ovenbird/build/'), true,
    'Build files should stay in the app cache instead of the selected project');
assertEqual(first, second, 'The same source should always use the same build directory');
assertEqual(first === otherProject, false, 'Separate projects should not share build files');
print('Build path tests passed');
