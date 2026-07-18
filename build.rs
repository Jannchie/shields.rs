//! Build script for shields crate.
use std::fs;
use std::io;
use std::path::Path;

const TEMPLATE_FILES: [&str; 5] = [
    "templates/flat_badge_template.svg",
    "templates/flat_square_badge_template.svg",
    "templates/plastic_badge_template.svg",
    "templates/social_badge_template.svg",
    "templates/for_the_badge_template.svg",
];

fn main() -> io::Result<()> {
    println!("cargo:rerun-if-changed=build.rs");

    for file in &TEMPLATE_FILES {
        println!("cargo:rerun-if-changed={}", file);

        let path = Path::new(file);
        let dest = path.with_extension("min.svg");

        let content = fs::read_to_string(path)?;
        let min_content = minify_svg(&content);
        // Skip the write when up to date: the packaged crate ships correct .min.svg files,
        // and its source tree may be read-only (docs.rs, Nix, shared registry caches).
        if fs::read_to_string(&dest).is_ok_and(|existing| existing == min_content) {
            continue;
        }
        fs::write(dest, min_content)?;
    }
    Ok(())
}

// Minify SVG content by trimming lines, joining whitespace, and removing unnecessary spaces
fn minify_svg(content: &str) -> String {
    let min_content = content.lines().map(str::trim).collect::<String>();
    let min_content = min_content.split_whitespace().collect::<Vec<_>>().join(" ");
    min_content.replace(" />", "/>").replace("> <", "><")
}
