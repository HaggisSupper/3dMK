import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

const html = readFileSync(new URL('./surface-reconstruction-workflow.html', import.meta.url), 'utf8');
for (const text of [
  '255,330', '3,990', '3,134', '6,256', '6.083 s',
  'Difference review', 'Accept with warnings', 'The original scan remains unchanged.',
  'Textured mesh remeshing', 'id="run-state"', 'id="accept-revision"',
]) assert.match(html, new RegExp(text.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')));
assert.match(html, /id="accept-revision"[^>]*disabled/);
assert.doesNotMatch(html, /Rust|Poisson|gradient/i);
new Function(html.match(/<script>([\s\S]*?)<\/script>/)?.[1] ?? '');
console.log('surface reconstruction workflow checks passed');
