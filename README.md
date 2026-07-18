# shields.rs

[![CodeTime Badge](https://img.shields.io/endpoint?style=social&color=222&url=https%3A%2F%2Fapi.codetime.dev%2Fv3%2Fusers%2Fshield%3Fuid%3D2%26project%3Dshields)](https://codetime.dev)

![Crates.io Version](https://img.shields.io/crates/v/shields)
![Deps.rs Crate Dependencies (latest)](https://img.shields.io/deps-rs/shields/latest)
![Crates.io License](https://img.shields.io/crates/l/shields)
![Crates.io Size](https://img.shields.io/crates/size/shields)
![Crates.io Downloads (recent)](https://img.shields.io/crates/dr/shields)
![Crates.io Total Downloads](https://img.shields.io/crates/d/shields)

A high-performance badge rendering engine written in Rust, supporting SVG output and font parsing. This project is designed for developers and services that require fast, customizable, and reliable badge generation.

**🟢 Bitwise-Identical SVG Output**

Not only do we pursue pixel-level similarity, but we also guarantee that the generated SVG string is bitwise-identical to the output returned by shields.io for the same parameters. This ensures absolute compatibility and consistency for all use cases.

**⚡️ Fast & Efficient**

Over 10x faster than the Node.js badge-maker library, this Rust implementation is optimized for speed and efficiency. It can generate badges in microseconds, making it suitable for high-performance applications and services.

**🎨 Supported All Styles & Logos**

We support all major badge styles: `flat`, `flat-square`, `plastic`, `social` and `for-the-badge`. Each style can be customized with various properties such as label, message, color, logo, and more. You can easily use [Simple Icons](https://simpleicons.org/?q=5) slugs to set logos for your badges, and we also support custom logos with SVG strings.

## Benchmark: Rust vs Node.js badge-maker

| Library     | Language | Time per badge | Unit |
| ----------- | -------- | -------------- | ---- |
| shields     | Rust     | 3.69           | µs   |
| badge-maker | Node.js  | 49.52          | µs   |

The benchmark renders badges with a Simple Icons logo and links (`cargo bench`). Simple text-only badges render in well under 1 µs, and rendering is lock-free, so throughput scales with cores.

## Installation

```bash
cargo add shields
```

## Usage Example

The library provides a chainable API for customizing badges. You can set the label, message, color, and other properties using method chaining:

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

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Community & Contact

- GitHub: [https://github.com/Jannchie/shields.rs](https://github.com/Jannchie/shields.rs)
- Documentation: [https://docs.rs/shields](https://docs.rs/shields)
- Author: Jannchie (<jannchie@gmail.com>)
