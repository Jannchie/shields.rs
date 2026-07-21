// Head-to-head throughput: shields-wasm vs badge-maker (the JS renderer
// shields.io itself uses). Run from this directory:
//
//   wasm-pack build --target nodejs --release
//   npm install badge-maker
//   node bench.mjs
//
import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
const { renderBadge } = require('./pkg/shields_wasm.js');

let makeBadge = null;
try {
  ({ makeBadge } = require('badge-maker'));
} catch {
  console.log('(badge-maker not installed — showing shields-wasm only)\n');
}

// badge-maker keys differ: {label, message, color, labelColor, style}. Named
// logo lookup is a shields-only feature, so the comparison uses pure-render
// workloads where both libraries do the same work.
const workloads = [
  { name: 'flat (no logo)',
    wasm: { style: 'flat', label: 'build', message: 'passing', messageColor: 'brightgreen' },
    bm:   { label: 'build', message: 'passing', color: 'brightgreen', style: 'flat' } },
  { name: 'for-the-badge',
    wasm: { style: 'for-the-badge', label: 'coverage', message: '98%', messageColor: 'green' },
    bm:   { label: 'coverage', message: '98%', color: 'green', style: 'for-the-badge' } },
  { name: 'plastic',
    wasm: { style: 'plastic', label: 'version', message: 'v1.4.0', messageColor: 'blue' },
    bm:   { label: 'version', message: 'v1.4.0', color: 'blue', style: 'plastic' } },
  { name: 'social',
    wasm: { style: 'social', label: 'chat', message: 'on discord' },
    bm:   { label: 'chat', message: 'on discord', style: 'social' } },
];

function bench(fn, params, N) {
  for (let i = 0; i < 5000; i++) fn(params); // warmup / JIT
  const t0 = process.hrtime.bigint();
  let acc = 0;
  for (let i = 0; i < N; i++) acc += fn(params).length;
  const t1 = process.hrtime.bigint();
  return Number(t1 - t0) / N / 1000; // µs/op
}

const N = 200000;
console.log(`node ${process.version}   N=${N.toLocaleString()} per cell\n`);
const head = 'workload'.padEnd(18) + 'wasm µs/op'.padStart(12) +
  (makeBadge ? 'badge-maker µs/op'.padStart(20) + 'speedup'.padStart(10) : '');
console.log(head);
for (const w of workloads) {
  const rw = bench(renderBadge, w.wasm, N);
  let line = w.name.padEnd(18) + rw.toFixed(3).padStart(12);
  if (makeBadge) {
    const rb = bench(makeBadge, w.bm, N);
    line += rb.toFixed(3).padStart(20) + ((rb / rw).toFixed(2) + '×').padStart(10);
  }
  console.log(line);
}
