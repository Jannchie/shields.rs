# shields-wasm

WebAssembly bindings for [shields.rs](../../). The core Rust engine is reused
unchanged; this crate only marshals a JS object into `BadgeParams` and returns
the rendered SVG. Runs anywhere WASM does — Node, Deno, Bun, browsers, and edge
runtimes (Cloudflare Workers, Vercel Edge).

## Build

```sh
wasm-pack build --target nodejs --release          # -> pkg/ (embeds ~3000 Simple Icons)
wasm-pack build --target nodejs --release \
  --no-default-features --out-dir pkg-slim          # -> pkg-slim/ (custom SVG logos only)
```

Use `--target web` / `--target bundler` for browser and bundler targets.

| build | `.wasm` raw | `.wasm` gzip | named logos (`logo: "rust"`) |
|-------|-------------|--------------|------------------------------|
| full  | 6.7 MB      | 2.7 MB       | yes                          |
| slim  | 0.9 MB      | 0.35 MB      | no (custom SVG data only)    |

The full build's size is almost entirely the embedded Simple Icons set. Prefer
the slim build unless you need named-logo lookup.

## Usage

```js
import { renderBadge } from 'shields-wasm'; // or require('./pkg/shields_wasm.js') on Node

const svg = renderBadge({
  style: 'flat',                 // flat | flat-square | plastic | social | for-the-badge
  label: 'build',
  message: 'passing',
  messageColor: 'brightgreen',
  logo: 'rust',                  // named (Simple Icons) or an SVG data URI; full build only
  logoColor: 'white',
  idSuffix: 'b1',                // dedupe SVG element ids when inlining several badges
  logoWidth: 14,
});
```

All fields are optional. Unknown keys are ignored (serde-wasm-bindgen does not
enforce `deny_unknown_fields`).

## Benchmark

`bench.mjs` compares throughput against [`badge-maker`](https://www.npmjs.com/package/badge-maker),
the JS renderer shields.io itself uses:

```sh
wasm-pack build --target nodejs --release
npm install          # dev dep: badge-maker
npm run bench
```

Representative run (Node v22, 200k iterations/cell):

| workload      | shields-wasm | badge-maker | speedup |
|---------------|--------------|-------------|---------|
| flat          | ~3.0 µs      | ~24 µs      | ~8×     |
| for-the-badge | ~2.7 µs      | ~15 µs      | ~6×     |
| plastic       | ~2.9 µs      | ~27 µs      | ~10×    |
| social        | ~2.8 µs      | ~32 µs      | ~12×    |

The per-call cost includes JS↔WASM marshaling (deserializing the options object
and copying the SVG string out), so native Rust is faster still; this is the
number a Node caller actually sees.
