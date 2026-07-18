use criterion::{Criterion, criterion_group, criterion_main};
use rand::{Rng, distr::Alphanumeric};
use shields::{BadgeParams, BadgeStyle, render_badge_svg};
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

criterion_group!(benches, bench_params_badge, bench_builder_badge);
criterion_main!(benches);
