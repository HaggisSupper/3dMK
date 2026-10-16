import assert from 'node:assert/strict';
import fs from 'node:fs';

const html = fs.readFileSync(new URL('./index.html', import.meta.url), 'utf8');
for (const id of ['surveyDrop', 'sectionsDrop', 'outputFolder', 'format', 'interpolation']) assert.match(html, new RegExp(`id="${id}"`));
assert.match(html, /localStorage/, 'settings persist across launches');
assert.match(html, /dragover/, 'drag and drop is available');
assert.match(html, /choose_csv_file/, 'native Explorer file choice is invoked');
assert.match(html, /choose_output_folder/, 'native Explorer folder choice is invoked');
assert.match(html, /--bg-dark:\s*#121212/i, '3DMK dark canvas token is used');
assert.match(html, /--bg-panel:\s*#1e1e1e/i, '3DMK panel token is used');
assert.match(html, /--teal-accent:\s*#14b8a6/i, '3DMK teal accent token is used');
assert.match(html, /font:\s*13px\/1\.45\s+"Segoe UI"/i, 'compact 3DMK typography is used');
assert.match(html, /class="panel-heading"/, 'technical panel headings are rendered');
console.log('PASS: persistent native file UI contract.');
