use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use rand::{RngExt, distr::Alphanumeric};
use shields::{BadgeParams, BadgeStyle, RenderOptions, render_badge_svg, render_badge_svg_into};
use std::hint::black_box;

const STYLES: [BadgeStyle; 4] = [
    BadgeStyle::Flat,
    BadgeStyle::Plastic,
    BadgeStyle::ForTheBadge,
    BadgeStyle::Social,
];

/// Pre-generated (text, style) inputs so the RNG cost stays out of the measured loop.
fn make_inputs(n: usize) -> Vec<(String, BadgeStyle)> {
    let mut rng = rand::rng();
    (0..n)
        .map(|_| {
            let len = rng.random_range(8..=12);
            let text: String = (&mut rng)
                .sample_iter(&Alphanumeric)
                .take(len)
                .map(char::from)
                .collect();
            let style = STYLES[rng.random_range(0..STYLES.len())];
            (text, style)
        })
        .collect()
}

// A. Traditional parameter struct
fn bench_params_badge(c: &mut Criterion) {
    let inputs = make_inputs(1024);
    let mut i = 0;
    c.bench_function("params_badge_svg", |b| {
        b.iter(|| {
            let (text, style) = &inputs[i % inputs.len()];
            i += 1;
            let params = BadgeParams {
                style: *style,
                label: Some(text.as_str()),
                message: Some(text.as_str()),
                label_color: Some("#555"),
                message_color: Some("#4c1"),
                link: Some("https://example.com"),
                extra_link: Some("https://example.org"),
                logo: Some("rust"),
                logo_color: Some("#FFF"),
            };
            black_box(render_badge_svg(&params));
        });
    });
}

// B. Builder pattern
fn bench_builder_badge(c: &mut Criterion) {
    let inputs = make_inputs(1024);
    let mut i = 0;
    c.bench_function("builder_badge_svg", |b| {
        b.iter(|| {
            let (text, style) = &inputs[i % inputs.len()];
            i += 1;
            let svg = shields::builder::Badge::style(*style)
                .label(text)
                .message(text)
                .label_color("#555")
                .message_color("#4c1")
                .logo("rust")
                .logo_color("#FFF")
                .link("https://example.com")
                .extra_link("https://example.org")
                .build();
            black_box(svg);
        });
    });
}

// C. Every style with and without a logo, so a change to one style's markup
// shows up on its own instead of averaged into a random mix.
fn bench_styles(c: &mut Criterion) {
    let mut group = c.benchmark_group("style");
    let styles = [
        ("flat", BadgeStyle::Flat),
        ("flat-square", BadgeStyle::FlatSquare),
        ("plastic", BadgeStyle::Plastic),
        ("social", BadgeStyle::Social),
        ("for-the-badge", BadgeStyle::ForTheBadge),
    ];
    for (name, style) in styles {
        for logo in [None, Some("rust")] {
            let params = BadgeParams {
                style,
                label: Some("build"),
                message: Some("passing"),
                message_color: Some("brightgreen"),
                logo,
                ..Default::default()
            };
            let variant = if logo.is_some() { "logo" } else { "text" };
            group.bench_with_input(BenchmarkId::new(name, variant), &params, |b, params| {
                b.iter(|| black_box(render_badge_svg(black_box(params))));
            });
        }
    }
    group.finish();
}

// D. Rendering into a reused buffer, which skips the per-badge allocation.
fn bench_render_into(c: &mut Criterion) {
    let params = BadgeParams {
        label: Some("build"),
        message: Some("passing"),
        message_color: Some("brightgreen"),
        ..Default::default()
    };
    let options = RenderOptions::default();
    let mut buf = String::new();
    c.bench_function("render_into_reused_buffer", |b| {
        b.iter(|| {
            buf.clear();
            render_badge_svg_into(black_box(&params), &options, &mut buf);
            black_box(&buf);
        });
    });
}

criterion_group!(
    benches,
    bench_params_badge,
    bench_builder_badge,
    bench_styles,
    bench_render_into
);
criterion_main!(benches);
