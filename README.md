# shields.rs

A high-performance badge rendering engine written in Rust, supporting SVG output and font parsing. This project is designed for developers and services that require fast, customizable, and reliable badge generation.

**🎯 Pixel-Perfect Consistency with shields.io**

Our goal is to achieve pixel-perfect, 100% identical rendering results to [shields.io](https://shields.io/). We utilize precisely the same text length calculation data to ensure full consistency, while delivering significantly improved efficiency.

**🟢 Bitwise-Identical SVG Output**

Not only do we pursue pixel-level similarity, but we also guarantee that the generated SVG string is bitwise-identical to the output returned by shields.io for the same parameters. This ensures absolute compatibility and consistency for all use cases.

**🎨 Supported Styles**

We support all major badge styles (except "for the badge"): `flat`, `flat-square`, `plastic`, and `social`.

## Installation

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (edition 2024 or later)
- Cargo package manager (comes with Rust)

### Build

Clone the repository and build the project:

```bash
git clone https://github.com/Jannchie/shields.git
cd shields
cargo build --release
```

### Run

To run the badge rendering engine:

```bash
cargo run --release
```

## Usage Example

The library provides a chainable API for customizing badges. You can set the label, message, color, and other properties using method chaining:

```rust
use shields::Badge;

fn main() {
    let svg = Badge::flat()
        .set_label("build")
        .set_message("passing")
        .render();
    std::fs::write("badge.svg", svg).unwrap();
}
```

### Advanced Example

A more comprehensive example demonstrating advanced customization:

```rust
use shields::Badge;

fn main() {
    let svg = Badge::plastic()
        .set_label("coverage")
        .set_message("98%")
        .set_color("#4c1")
        .set_label_color("#555")
        .set_logo("github")
        .set_logo_color("white")
        .set_logo_width(18)
        .set_link("https://docs.rs/shields")
        .render();
    std::fs::write("badge-advanced.svg", svg).unwrap();
}
```

## Contributing

Contributions are welcome! To get started:

1. Fork the repository on [GitHub](https://github.com/Jannchie/shields).
2. Create a new branch for your feature or bugfix.
3. Commit your changes with clear messages.
4. Open a pull request describing your changes.
5. For issues, please use the [GitHub Issues](https://github.com/Jannchie/shields/issues) page.

Before submitting a PR, ensure all tests pass:

```bash
cargo test
```

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Community & Contact

- GitHub: [https://github.com/Jannchie/shields](https://github.com/Jannchie/shields)
- Documentation: [https://docs.rs/shields](https://docs.rs/shields)
- Author: Jannchie (<jannchie@gmail.com>)
