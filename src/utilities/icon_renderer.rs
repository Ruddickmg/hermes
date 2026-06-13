use serde::{Deserialize, Serialize};
use std::path::PathBuf;

include!(concat!(env!("OUT_DIR"), "/prerendered_icons.rs"));

use crate::acp::{Result, error::Error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PixelIcon {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct IconRenderer {
    cache_dir: PathBuf,
    size: u32,
}

impl IconRenderer {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            cache_dir,
            size: 16,
        }
    }

    pub async fn get_or_render(&self, url: &str) -> Result<Option<PixelIcon>> {
        if let Some((w, h, pixels)) = lookup_prerendered_icon(url) {
            return Ok(Some(PixelIcon {
                width: w,
                height: h,
                pixels: pixels.to_vec(),
            }));
        }

        let cache_path = self.cache_path(url);

        if cache_path.is_file() {
            if let Ok(data) = std::fs::read_to_string(&cache_path) {
                if let Ok(icon) = serde_json::from_str::<PixelIcon>(&data) {
                    return Ok(Some(icon));
                }
            }
        }

        let url = url.to_owned();
        let size = self.size;

        let icon = blocking::unblock(move || -> Result<PixelIcon> {
            let svg = download_svg(&url)?;
            let icon = render_svg(&svg, size)?;

            if let Some(parent) = cache_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| Error::Network(format!("Failed to create icon cache dir: {e}")))?;
            }
            let json = serde_json::to_string(&icon)
                .map_err(|e| Error::Network(format!("Failed to serialize icon: {e}")))?;
            std::fs::write(&cache_path, &json)
                .map_err(|e| Error::Network(format!("Failed to write icon cache: {e}")))?;

            Ok(icon)
        })
        .await?;

        Ok(Some(icon))
    }

    fn cache_key(url: &str) -> String {
        url.chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>()
            + ".json"
    }

    fn cache_path(&self, url: &str) -> PathBuf {
        self.cache_dir.join(Self::cache_key(url))
    }
}

fn download_svg(url: &str) -> Result<String> {
    let mut response = ureq::get(url)
        .call()
        .map_err(|e| Error::Network(format!("Failed to download SVG from {url}: {e}")))?;

    let body = response
        .body_mut()
        .read_to_vec()
        .map_err(|e| Error::Network(format!("Failed to read SVG response from {url}: {e}")))?;

    String::from_utf8(body)
        .map_err(|e| Error::Network(format!("SVG response from {url} is not valid UTF-8: {e}")))
}

fn make_svg_options() -> usvg::Options<'static> {
    let mut fontdb = usvg::fontdb::Database::new();
    let _ = fontdb.load_system_fonts();
    if !DEJAVU_SANS_MONO.is_empty() {
        fontdb.load_font_data(DEJAVU_SANS_MONO.to_vec());
    }

    let font_resolver = usvg::FontResolver {
        select_font: custom_font_selector(),
        select_fallback: usvg::FontResolver::default_fallback_selector(),
    };

    usvg::Options {
        fontdb: std::sync::Arc::new(fontdb),
        font_resolver,
        ..usvg::Options::default()
    }
}

fn custom_font_selector() -> usvg::FontSelectionFn<'static> {
    Box::new(
        move |font: &usvg::Font, fontdb: &mut std::sync::Arc<usvg::fontdb::Database>| {
            let families: Vec<usvg::fontdb::Family> = font
                .families()
                .iter()
                .map(|f| match f {
                    usvg::FontFamily::Serif => usvg::fontdb::Family::Serif,
                    usvg::FontFamily::SansSerif => usvg::fontdb::Family::SansSerif,
                    usvg::FontFamily::Cursive => usvg::fontdb::Family::Cursive,
                    usvg::FontFamily::Fantasy => usvg::fontdb::Family::Fantasy,
                    usvg::FontFamily::Monospace => usvg::fontdb::Family::Monospace,
                    usvg::FontFamily::Named(s) => usvg::fontdb::Family::Name(s),
                })
                .collect();

            let mut all_families = families;
            all_families.push(usvg::fontdb::Family::Serif);

            let stretch = match font.stretch() {
                usvg::FontStretch::UltraCondensed => usvg::fontdb::Stretch::UltraCondensed,
                usvg::FontStretch::ExtraCondensed => usvg::fontdb::Stretch::ExtraCondensed,
                usvg::FontStretch::Condensed => usvg::fontdb::Stretch::Condensed,
                usvg::FontStretch::SemiCondensed => usvg::fontdb::Stretch::SemiCondensed,
                usvg::FontStretch::Normal => usvg::fontdb::Stretch::Normal,
                usvg::FontStretch::SemiExpanded => usvg::fontdb::Stretch::SemiExpanded,
                usvg::FontStretch::Expanded => usvg::fontdb::Stretch::Expanded,
                usvg::FontStretch::ExtraExpanded => usvg::fontdb::Stretch::ExtraExpanded,
                usvg::FontStretch::UltraExpanded => usvg::fontdb::Stretch::UltraExpanded,
            };

            let style = match font.style() {
                usvg::FontStyle::Normal => usvg::fontdb::Style::Normal,
                usvg::FontStyle::Italic => usvg::fontdb::Style::Italic,
                usvg::FontStyle::Oblique => usvg::fontdb::Style::Oblique,
            };

            let query = usvg::fontdb::Query {
                families: &all_families,
                weight: usvg::fontdb::Weight(font.weight()),
                stretch,
                style,
            };

            if let Some(id) = fontdb.query(&query) {
                return Some(id);
            }

            let fallback = [usvg::fontdb::Family::Name("DejaVu Sans Mono")];
            fontdb.query(&usvg::fontdb::Query {
                families: &fallback,
                weight: usvg::fontdb::Weight(font.weight()),
                stretch,
                style,
            })
        },
    )
}

fn render_svg(svg_data: &str, size: u32) -> Result<PixelIcon> {
    let opt = make_svg_options();
    let tree = usvg::Tree::from_str(svg_data, &opt)
        .map_err(|e| Error::InvalidInput(format!("Failed to parse SVG: {e}")))?;

    let viewport = tree.size();
    let vp_w = viewport.width();
    let vp_h = viewport.height();

    if vp_w <= 0.0 || vp_h <= 0.0 {
        return Err(Error::InvalidInput(format!(
            "SVG has invalid dimensions: {vp_w}x{vp_h}"
        )));
    }

    let scale_x = size as f32 / vp_w;
    let scale_y = size as f32 / vp_h;
    let scale = scale_x.min(scale_y);

    let mut pixmap = tiny_skia::Pixmap::new(size, size)
        .ok_or_else(|| Error::InvalidInput(format!("Failed to create {size}x{size} pixmap")))?;

    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let data = pixmap.data();
    let pixels: Vec<u8> = data.chunks(4).map(|rgba| rgba[3]).collect();

    Ok(PixelIcon {
        width: size,
        height: size,
        pixels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

    fn simple_svg() -> &'static str {
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
            <rect x="0" y="0" width="16" height="16" fill="#FF0000"/>
        </svg>"##
    }

    #[test]
    fn render_svg_with_simple_rect_returns_correct_dimensions() {
        let icon = render_svg(simple_svg(), 16).unwrap();
        assert_eq!(icon.width, 16);
        assert_eq!(icon.height, 16);
    }

    #[test]
    fn render_svg_with_simple_rect_has_correct_pixel_count() {
        let icon = render_svg(simple_svg(), 16).unwrap();
        assert_eq!(icon.pixels.len(), 256);
    }

    #[test]
    fn render_svg_with_opaque_fill_has_full_alpha() {
        let icon = render_svg(simple_svg(), 16).unwrap();
        assert_eq!(icon.pixels[0], 255);
    }

    #[test]
    fn render_svg_with_invalid_input_returns_error() {
        let result = render_svg("not valid svg at all", 16);
        assert!(result.is_err());
    }

    #[test]
    fn render_svg_with_empty_string_returns_error() {
        let result = render_svg("", 16);
        assert!(result.is_err());
    }

    #[test]
    fn pixel_icon_json_roundtrip() {
        let icon = render_svg(simple_svg(), 16).unwrap();
        let json = serde_json::to_string(&icon).unwrap();
        let deserialized: PixelIcon = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.width, icon.width);
        assert_eq!(deserialized.height, icon.height);
        assert_eq!(deserialized.pixels, icon.pixels);
    }

    #[test]
    fn cache_key_replaces_special_chars() {
        let url = "https://cdn.example.com/registry/v1/latest/test-agent.svg";
        let key = IconRenderer::cache_key(url);
        assert!(!key.contains('/'));
        assert!(!key.contains(':'));
        assert!(key.ends_with(".json"));
        assert!(key.contains("test-agent.svg"));
    }

    #[test]
    fn cache_key_preserves_alphanumeric_and_dots() {
        let url = "hello.world-123_test";
        let key = IconRenderer::cache_key(url);
        assert_eq!(key, "hello.world-123_test.json");
    }

    #[test]
    fn get_or_render_returns_cached_data_when_cache_file_exists() {
        let tmp = TempDir::new().unwrap();
        let renderer = IconRenderer::new(tmp.path().to_path_buf());
        let url = "https://cdn.example.com/icon.svg";

        let expected = PixelIcon {
            width: 16,
            height: 16,
            pixels: vec![255u8; 256],
        };
        let cache_path = renderer.cache_path(url);
        std::fs::create_dir_all(cache_path.parent().unwrap()).unwrap();
        std::fs::write(&cache_path, serde_json::to_string(&expected).unwrap()).unwrap();

        let result = smol::block_on(async { renderer.get_or_render(url).await.unwrap() });

        assert_eq!(result.unwrap().pixels, expected.pixels);
    }

    #[test]
    fn get_or_render_returns_error_for_invalid_url() {
        let tmp = TempDir::new().unwrap();
        let renderer = IconRenderer::new(tmp.path().to_path_buf());

        let result = smol::block_on(async {
            renderer
                .get_or_render("https://invalid-url-12345.example.com/icon.svg")
                .await
        });

        assert!(result.is_err());
    }

    #[test]
    fn render_svg_with_transparency_preserves_alpha() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
            <rect x="0" y="0" width="16" height="16" fill="#FF0000" opacity="0.5"/>
        </svg>"##;
        let icon = render_svg(svg, 16).unwrap();
        // resvg uses premultiplied alpha, so 50% opacity gives alpha=128
        assert_eq!(icon.pixels[0], 128);
    }

    #[test]
    fn render_svg_scales_to_fit_bounds() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">
            <rect x="0" y="0" width="32" height="32" fill="#00FF00"/>
        </svg>"##;
        let icon = render_svg(svg, 16).unwrap();
        assert_eq!(icon.width, 16);
        assert_eq!(icon.height, 16);
        assert_eq!(icon.pixels[0], 255);
    }
}
