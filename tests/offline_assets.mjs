import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const html = fs.readFileSync(path.join(root, 'public/index.html'), 'utf8');
assert.equal(/<script[^>]+src=["']https?:/i.test(html), false, 'Runtime scripts must be bundled');
const importMap = JSON.parse(html.match(/<script type="importmap">([\s\S]*?)<\/script>/)[1]);
for (const target of Object.values(importMap.imports)) {
  assert.ok(target.startsWith('./vendor/'), `External import target: ${target}`);
  assert.ok(fs.existsSync(path.join(root, 'public', target)), `Missing import target: ${target}`);
}
assert.ok(html.includes("setDecoderPath('./vendor/draco/1.5.6/')"), 'Pinned Draco decoder must be local');
const main = fs.readFileSync(path.join(root, 'src-tauri/src/main.rs'), 'utf8');
assert.ok(main.includes('publish_assets(&data_dir, EMBEDDED_FRONTEND_ASSETS)'), 'Native app must publish all embedded frontend assets atomically');
const manifest = JSON.parse(fs.readFileSync(path.join(root, 'public/vendor/runtime-assets.json'), 'utf8'));
assert.equal(manifest.schema_version, 1);
assert.equal(manifest.files.length, manifest.file_count);
for (const asset of manifest.files) {
  assert.ok(asset.path.startsWith('vendor/') && !asset.path.split('/').includes('..'));
  const bytes = fs.readFileSync(path.join(root, 'public', asset.path));
  assert.equal(bytes.length, asset.bytes, asset.path);
  assert.equal(createHash('sha256').update(bytes).digest('hex'), asset.sha256, asset.path);
}
assert.ok(manifest.files.some(asset => asset.kind === 'notice'));
console.log('offline_assets: PASS');
