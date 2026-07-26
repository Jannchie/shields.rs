# shields.rs

[![CodeTime Badge](https://img.shields.io/endpoint?style=social&color=222&url=https%3A%2F%2Fapi.codetime.dev%2Fv3%2Fusers%2Fshield%3Fuid%3D2%26project%3Dshields)](https://codetime.dev)

![Crates.io Version](https://img.shields.io/crates/v/shields)
![Crates.io License](https://img.shields.io/crates/l/shields)
![Crates.io Total Downloads](https://img.shields.io/crates/d/shields)

A badge rendering engine in Rust, compatible with [shields.io](https://shields.io/).

For the same parameters, the generated SVG string is byte-identical to what
shields.io returns — verified against 5,761 reference SVGs fetched from the
live service, covering all five styles (`flat`, `flat-square`, `plastic`,
`social`, `for-the-badge`), colors, links, and logos. A weekly CI job re-checks
against upstream, so divergence is caught when shields.io changes its output.

Logos take [Simple Icons](https://simpleicons.org/) slugs or raw SVG strings.
Font metrics are embedded at compile time; rendering does no I/O, takes no
locks, and needs no runtime besides the standard library (wasm32 builds work).

## Benchmark: Rust vs Node.js badge-maker

| Library     | Language | Time per badge | Unit |
| ----------- | -------- | -------------- | ---- |
| shields     | Rust     | 0.94           | µs   |
| badge-maker | Node.js  | 43.7           | µs   |

Both render a badge with a Simple Icons logo; `cargo bench` reproduces the Rust
side. badge-maker takes an already-encoded logo, so it is handed the same data
URI shields resolves internally. Text-only badges render in 0.87 µs, and
rendering takes no locks, so it scales with cores.

Label and message text is XML-escaped at render time, so `&`, `<` and quotes in
badge text produce well-formed SVG and cannot escape their attribute or element.
Escaping runs after text measurement, which is why it belongs here and not in
calling code: pre-escaped input would be laid out five characters wide per `&`.

## Installation

```bash
cargo add shields
```

## Usage

```rust
use shields::BadgeStyle;
use shields::builder::Badge;

fn main() {
    // Simple flat badge
    let badge = Badge::style(BadgeStyle::Flat)
        .label("test")
        .message("passing")
        .build();
    println!("{badge}");

    // Plastic badge with custom colors
    let badge = Badge::style(BadgeStyle::Plastic)
        .label("version")
        .message("1.0.0")
        .label_color("#555")
        .message_color("#4c1")
        .build();
    println!("{badge}");

    // Social badge with logo and links
    let badge = Badge::style(BadgeStyle::Social)
        .label("github")
        .message("stars")
        .logo("github")
        .link("https://github.com/user/repo")
        .extra_link("https://github.com/user/repo/stargazers")
        .build();
    println!("{badge}");
}
```

Additional options beyond the shields.io URL parameters:

```rust
use shields::builder::Badge;

let svg = Badge::flat()
    .label("build")
    .message("passing")
    // Unique id suffix, required when several badges are inlined in one HTML
    // page (inline SVGs share the page's id namespace).
    .id_suffix("badge1")
    // Widen the logo box (default 14px) for wide logos.
    .logo_width(20)
    .build();
```

If you only render text badges or custom SVG logos, disable the embedded
Simple Icons set to cut compile time and binary size:

```toml
shields = { version = "1", default-features = false }
```

See `examples/server.rs` for a dependency-free HTTP badge service using the
serde-friendly `BadgeParamsOwned` type.

There is also a plain parameter-struct API if you prefer explicit construction:

```rust
use shields::{BadgeParams, BadgeStyle, render_badge_svg};

let svg = render_badge_svg(&BadgeParams {
    style: BadgeStyle::Flat,
    label: Some("build"),
    message: Some("passing"),
    label_color: None,
    message_color: Some("brightgreen"),
    link: None,
    extra_link: None,
    logo: None,
    logo_color: None,
});
```

## Documentation

API reference: [docs.rs/shields](https://docs.rs/shields)

## License

MIT. See [LICENSE](LICENSE).
