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

```sh
npm install shields-wasm
```

The published package targets Node (CommonJS). For browser / bundler / edge
consumers, build the `--target web` or `--target bundler` variant locally (see
[Build](#build)).

```js
const { renderBadge } = require('shields-wasm'); // ESM: import { renderBadge } from 'shields-wasm'

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
pnpm install         # dev dep: badge-maker
pnpm bench
```

Representative run (Node v22, 200k iterations/cell):

| workload      | shields-wasm | badge-maker | speedup |
|---------------|--------------|-------------|---------|
| flat          | ~2.6 µs      | ~24 µs      | ~9×     |
| for-the-badge | ~2.3 µs      | ~15 µs      | ~6×     |
| plastic       | ~2.5 µs      | ~27 µs      | ~11×    |
| social        | ~2.4 µs      | ~31 µs      | ~13×    |

The per-call cost includes JS↔WASM marshaling (deserializing the options object
and copying the SVG string out), so native Rust is faster still; this is the
number a Node caller actually sees.

## Publishing

CI (`.github/workflows/publish-wasm.yml`) publishes `shields-wasm` to npm on
every `v*` tag, using **npm Trusted Publishing** (OIDC) — no `NPM_TOKEN` secret,
and every release carries provenance.

> The `publish` step uses the **npm CLI**, not pnpm, on purpose: pnpm's OIDC
> trusted-publishing support is currently broken (fails with 404 on pnpm 11,
> [pnpm#11513](https://github.com/pnpm/pnpm/issues/11513)). pnpm is fine for
> installs and scripts (`pnpm install`, `pnpm bench`) — the disk savings live
> there; the publish job installs no dependencies, so npm costs nothing extra.

Trusted Publishing can only be configured on a package that already exists, so
the **first** release needs a one-time manual bootstrap:

1. Build and publish once from your machine to create the package:
   ```sh
   npm login
   wasm-pack build bindings/wasm --target nodejs --release
   cd bindings/wasm/pkg && npm publish --access public
   ```
2. On npmjs.com → the `shields-wasm` package → **Settings → Trusted Publisher**,
   add a GitHub Actions publisher:
   - Repository: `Jannchie/shields.rs`
   - Workflow filename: `publish-wasm.yml`
3. Done. Every subsequent `git tag vX.Y.Z && git push --tags` publishes
   tokenlessly. The workflow stamps the npm version from the tag, so the crate's
   `Cargo.toml` version does not need bumping per release.
