use pretty_assertions::assert_eq;
use shields::{BadgeParams, BadgeStyle, render_badge_svg};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

fn shields_io_url(params: &BadgeParams) -> String {
    let style = match params.style {
        BadgeStyle::Flat => "flat",
        BadgeStyle::Plastic => "plastic",
        BadgeStyle::FlatSquare => "flat-square",
        BadgeStyle::Social => "social",
        BadgeStyle::ForTheBadge => "for-the-badge",
    };
    let url = if let Some(label) = &params.label {
        format!(
            "https://img.shields.io/badge/{}-{}-blue?style={}",
            label,
            params.message.unwrap_or("").replace(" ", "%20"),
            style
        )
    } else {
        format!(
            "https://img.shields.io/badge/{}-blue?style={}",
            params.message.unwrap_or("").replace(" ", "%20"),
            style
        )
    };
    let queries = [
        ("labelColor", params.label_color.unwrap_or("")),
        ("color", params.message_color.unwrap_or("")),
        ("link", params.link.unwrap_or("")),
        ("link", params.extra_link.unwrap_or("")),
        ("logo", params.logo.unwrap_or("")),
        ("logoColor", params.logo_color.unwrap_or("")),
    ];
    let mut url = format!("{url}&");
    for (key, value) in queries.iter() {
        if !value.is_empty() {
            url.push_str(&format!("{}={}&", key, urlencoding::encode(value)));
        }
    }
    url.pop();
    url
}

/**
 * Generate a unique cache file name (based on parameter text, avoiding illegal characters)
 */
fn cache_file_name_from_params(params: &BadgeParams) -> String {
    let url = shields_io_url(params);
    urlencoding::encode(&url)
        .replace("%", "_")
        .replace("/", "_")
        .replace(":", "_")
        .replace("?", "_")
        .replace("&", "_")
        .replace("=", "_")
        + ".svg"
}

/// Get shields.io SVG, with local cache
fn get_shields_svg_with_cache(params: &BadgeParams, url: &str) -> String {
    let cache_dir = Path::new("target/tmp/cache");
    if !cache_dir.exists() {
        fs::create_dir_all(cache_dir).expect("Failed to create cache directory");
    }
    let file_name = cache_file_name_from_params(params);
    // Encode file name to MD5
    let file_name = format!("{:x}.svg", md5::compute(file_name));
    let cache_path = cache_dir.join(file_name);
    // Read cache first if exists
    if cache_path.exists() {
        let mut file = fs::File::open(&cache_path).expect("Failed to read cache file");
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .expect("Failed to read cache content");
        return contents;
    }

    // If no cache, request and write to cache
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("Failed to create HTTP client");
    let resp = client.get(url).send();
    let resp = match resp {
        Ok(r) => r,
        Err(e) => panic!(
            "HTTP request failed: {url}\nPlease check your network connection, or manually visit shields.io to generate the cache.\nError details: {e}"
        ),
    };
    assert!(
        resp.status().is_success(),
        "shields.io request failed: {}\nHTTP status: {}\nPlease check if shields.io is available.",
        url,
        resp.status()
    );
    let svg = resp.text().unwrap_or_else(|e| {
        panic!(
            "Failed to read SVG: {url}\nPlease check the shields.io response content.\nError details: {e}"
        )
    });

    let mut file = fs::File::create(&cache_path)
        .unwrap_or_else(|_| panic!("Failed to create cache file: {:?}", cache_path.display()));
    file.write_all(svg.as_bytes())
        .expect("Failed to write cache content");

    svg
}

#[test]
fn test_svg_compare() {
    let label_selections = [Some("label"), Some(""), None];
    let message_selections = ["message", ""];
    let label_color_selections = [Some("blue"), Some("#4c1"), Some(""), None, Some("#FFF")];
    let message_color_selections = ["blue", "#4c3232", "", "#FFF"];
    let links_selections = [
        vec![None, None],
        vec![Some(""), None],
        vec![Some("https://example.com"), None],
        vec![Some("https://example.com"), Some("https://example2.com")],
        vec![Some("https://example.com"), Some("")],
    ];
    let logo_selections = [Some("rust"), Some(""), None];
    let style_selections = [
        BadgeStyle::Flat,
        BadgeStyle::Plastic,
        BadgeStyle::FlatSquare,
        BadgeStyle::Social,
    ];
    let logo_color_selections = [Some("blue"), None];
    let mut test_cases = vec![];
    for label in label_selections.iter() {
        for message in message_selections.iter() {
            for label_color in label_color_selections.iter() {
                for message_color in message_color_selections.iter() {
                    for links in links_selections.iter() {
                        for logo in logo_selections.iter() {
                            for logo_color in logo_color_selections.iter() {
                                for style in style_selections.iter() {
                                    if links.len() < 2 {
                                        continue;
                                    }
                                    let link = links[0];
                                    let extra_link = links[1];
                                    if link.is_none() && extra_link.is_none() {
                                        continue;
                                    }
                                    let params = BadgeParams {
                                        style: *style,
                                        label: *label,
                                        message: Some(message),
                                        label_color: *label_color,
                                        message_color: Some(message_color),
                                        link: links[0],
                                        extra_link: links[1],
                                        logo: *logo,
                                        logo_color: *logo_color,
                                    };
                                    test_cases.push(params);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // let mut test_case_count = 0;
    for params in test_cases {
        let local_svg = render_badge_svg(&params);
        let url = shields_io_url(&params);
        let shields_svg = get_shields_svg_with_cache(&params, &url);

        // Save
        let cache_dir = Path::new("target/tmp/cache");
        if !cache_dir.exists() {
            fs::create_dir_all(cache_dir).expect("Failed to create cache directory");
        }

        assert_eq!(
            local_svg, shields_svg,
            "SVG mismatch\nParams: {:?}\nLocal SVG:\n{}\nshields.io SVG:\n{}",
            params, local_svg, shields_svg
        );
    }
}
#[test]
fn test_svg_fast_compare() {
    let params = BadgeParams {
        style: BadgeStyle::Flat,
        label: Some("label"),
        message: Some("message"),
        label_color: Some("white"),
        message_color: Some("fff"),
        link: None,
        extra_link: None,
        logo: Some("rust"),
        logo_color: Some("blue"),
    };
    let local_svg = render_badge_svg(&params);
    let url = shields_io_url(&params);
    let shields_svg = get_shields_svg_with_cache(&params, &url);
    let file_name_local = "target/tmp/svg_local.svg".to_string();
    let file_name_shields = "target/tmp/svg_shields.svg".to_string();
    let mut file_local =
        fs::File::create(&file_name_local).expect("Failed to create local SVG file");
    file_local
        .write_all(local_svg.as_bytes())
        .expect("Failed to write local SVG file");
    let mut file_shields =
        fs::File::create(&file_name_shields).expect("Failed to create shields SVG file");
    file_shields
        .write_all(shields_svg.as_bytes())
        .expect("Failed to write shields SVG file");
    assert_eq!(
        local_svg, shields_svg,
        "SVG mismatch\nParams: {:?}\nLocal SVG:\n{}\nshields.io SVG:\n{}",
        params, local_svg, shields_svg
    );
}
