fn main() {
    if let Err(e) = nvim_oxi::tests::build() {
        panic!("nvim-oxi build failed: {e}");
    }

    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());

    if let Err(e) = prerender_icons(&out_dir) {
        println!(
            "cargo:warning=Icon pre-rendering failed: {e}. Icons will be rendered at runtime."
        );
    }
}

fn download_font(out_dir: &std::path::Path) -> Vec<u8> {
    let font_path = out_dir.join("DejaVuSansMono.ttf");

    if let Ok(bytes) = std::fs::read(&font_path) {
        if !bytes.is_empty() {
            return bytes;
        }
    }

    let url = "https://github.com/dejavu-fonts/dejavu-fonts/releases/download/version_2_37/dejavu-fonts-ttf-2.37.zip";
    let bytes = match ureq::get(url).call() {
        Ok(response) => {
            let mut body = response.into_body();
            match body.read_to_vec() {
                Ok(zip_bytes) => match extract_font_from_zip(&zip_bytes) {
                    Ok(font_data) => {
                        println!(
                            "cargo:warning=Downloaded DejaVuSansMono.ttf ({} bytes)",
                            font_data.len()
                        );
                        font_data
                    }
                    Err(e) => {
                        println!(
                            "cargo:warning=Failed to extract font from zip: {e}. Font fallback disabled."
                        );
                        Vec::new()
                    }
                },
                Err(e) => {
                    println!(
                        "cargo:warning=Failed to read font archive response: {e}. Font fallback disabled."
                    );
                    Vec::new()
                }
            }
        }
        Err(e) => {
            println!(
                "cargo:warning=Failed to download DejaVuSansMono.ttf: {e}. Font fallback disabled."
            );
            Vec::new()
        }
    };

    let _ = std::fs::write(font_path, &bytes);
    bytes
}

fn extract_font_from_zip(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use std::io::Read;
    let cursor = std::io::Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor)?;
    let target_name = "dejavu-fonts-ttf-2.37/ttf/DejaVuSansMono.ttf";
    let mut file = archive.by_name(target_name)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn make_svg_options(font_data: &[u8]) -> usvg::Options<'static> {
    let mut fontdb = fontdb::Database::new();
    fontdb.load_system_fonts();
    if !font_data.is_empty() {
        fontdb.load_font_data(font_data.to_vec());
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
        move |font: &usvg::Font, fontdb: &mut std::sync::Arc<fontdb::Database>| {
            let families: Vec<fontdb::Family> = font
                .families()
                .iter()
                .map(|f| match f {
                    usvg::FontFamily::Serif => fontdb::Family::Serif,
                    usvg::FontFamily::SansSerif => fontdb::Family::SansSerif,
                    usvg::FontFamily::Cursive => fontdb::Family::Cursive,
                    usvg::FontFamily::Fantasy => fontdb::Family::Fantasy,
                    usvg::FontFamily::Monospace => fontdb::Family::Monospace,
                    usvg::FontFamily::Named(s) => fontdb::Family::Name(s),
                })
                .collect();

            let mut all_families = families;
            all_families.push(fontdb::Family::Serif);

            let stretch = match font.stretch() {
                usvg::FontStretch::UltraCondensed => fontdb::Stretch::UltraCondensed,
                usvg::FontStretch::ExtraCondensed => fontdb::Stretch::ExtraCondensed,
                usvg::FontStretch::Condensed => fontdb::Stretch::Condensed,
                usvg::FontStretch::SemiCondensed => fontdb::Stretch::SemiCondensed,
                usvg::FontStretch::Normal => fontdb::Stretch::Normal,
                usvg::FontStretch::SemiExpanded => fontdb::Stretch::SemiExpanded,
                usvg::FontStretch::Expanded => fontdb::Stretch::Expanded,
                usvg::FontStretch::ExtraExpanded => fontdb::Stretch::ExtraExpanded,
                usvg::FontStretch::UltraExpanded => fontdb::Stretch::UltraExpanded,
            };

            let style = match font.style() {
                usvg::FontStyle::Normal => fontdb::Style::Normal,
                usvg::FontStyle::Italic => fontdb::Style::Italic,
                usvg::FontStyle::Oblique => fontdb::Style::Oblique,
            };

            let query = fontdb::Query {
                families: &all_families,
                weight: fontdb::Weight(font.weight()),
                stretch,
                style,
            };

            if let Some(id) = fontdb.query(&query) {
                return Some(id);
            }

            let fallback = [fontdb::Family::Name("DejaVu Sans Mono")];
            fontdb.query(&fontdb::Query {
                families: &fallback,
                weight: fontdb::Weight(font.weight()),
                stretch,
                style,
            })
        },
    )
}

fn prerender_icons(out_dir: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let registry_json = include_str!("src/acp/registry/registry.json");
    let registry: serde_json::Value = serde_json::from_str(registry_json)?;

    let font_data = download_font(out_dir);
    let svg_opt = make_svg_options(&font_data);

    let dest_path = out_dir.join("prerendered_icons.rs");

    let mut output = String::from(
        "pub static DEJAVU_SANS_MONO: &[u8] = include_bytes!(\"DejaVuSansMono.ttf\");\n\n\
         pub fn lookup_prerendered_icon(url: &str) -> Option<(u32, u32, &'static [u8])> {\n\
         match url {\n",
    );

    let agents = registry["agents"]
        .as_array()
        .ok_or("registry.agents is not an array")?;

    let mut rendered = 0u32;
    for agent in agents {
        let icon_url = match agent["icon"].as_str() {
            Some(u) => u,
            None => continue,
        };
        match render_agent_icon(icon_url, &svg_opt) {
            Some(pixels) => {
                let pixel_str = pixels
                    .iter()
                    .map(|b| b.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                output.push_str(&format!(
                    "        {url:?} => Some((16, 16, &[{pixels}])),\n",
                    url = icon_url,
                    pixels = pixel_str,
                ));
                rendered += 1;
            }
            None => {
                println!(
                    "cargo:warning=Failed to pre-render icon for {}: skipping",
                    icon_url
                );
            }
        }
    }

    output.push_str("        _ => None\n    }\n}\n");

    println!("cargo:rerun-if-changed=src/acp/registry/registry.json");
    println!(
        "cargo:warning=Pre-rendered {rendered} agent icons into binary ({size} bytes)",
        size = output.len(),
    );

    std::fs::write(&dest_path, &output)?;
    Ok(())
}

fn render_agent_icon(url: &str, opt: &usvg::Options) -> Option<Vec<u8>> {
    let mut response = ureq::get(url).call().ok()?;
    let body = response.body_mut().read_to_vec().ok()?;
    let svg_data = String::from_utf8(body).ok()?;

    let tree = usvg::Tree::from_str(&svg_data, opt).ok()?;

    let viewport = tree.size();
    let vp_w = viewport.width();
    let vp_h = viewport.height();
    if vp_w <= 0.0 || vp_h <= 0.0 {
        return None;
    }

    let final_size = 16u32;

    // Skip 4× supersampling when SVG viewport is already at or below target size.
    if vp_w <= final_size as f32 && vp_h <= final_size as f32 {
        let scale = final_size as f32 / vp_w.min(vp_h);
        let mut pixmap = tiny_skia::Pixmap::new(final_size, final_size)?;
        let transform = tiny_skia::Transform::from_scale(scale, scale);
        resvg::render(&tree, transform, &mut pixmap.as_mut());
        return Some(pixmap.data().chunks(4).map(|c| c[3]).collect());
    }

    // Render at 4× target resolution for better antialiasing,
    // then bicubic-downsample to 16×16 via tiny_skia's built-in filtering.
    let render_size = 64u32;

    let scale_x = render_size as f32 / vp_w;
    let scale_y = render_size as f32 / vp_h;
    let scale = scale_x.min(scale_y);

    let mut pixmap = tiny_skia::Pixmap::new(render_size, render_size)?;
    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Built-in bicubic downsampling via draw_pixmap
    let mut result_pixmap = tiny_skia::Pixmap::new(final_size, final_size)?;
    let mut paint = tiny_skia::PixmapPaint::default();
    paint.quality = tiny_skia::FilterQuality::Bicubic;
    result_pixmap.draw_pixmap(
        0,
        0,
        pixmap.as_ref(),
        &paint,
        tiny_skia::Transform::from_scale(
            final_size as f32 / render_size as f32,
            final_size as f32 / render_size as f32,
        ),
        None,
    );

    // Extract alpha channel (grayscale byte per pixel)
    Some(result_pixmap.data().chunks(4).map(|c| c[3]).collect())
}
