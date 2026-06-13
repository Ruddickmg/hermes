fn main() {
    if let Err(e) = nvim_oxi::tests::build() {
        panic!("nvim-oxi build failed: {e}");
    }

    if let Err(e) = prerender_icons() {
        println!(
            "cargo:warning=Icon pre-rendering failed: {e}. Icons will be rendered at runtime."
        );
    }
}

fn prerender_icons() -> Result<(), Box<dyn std::error::Error>> {
    let registry_json = include_str!("src/acp/registry/registry.json");
    let registry: serde_json::Value = serde_json::from_str(registry_json)?;

    let out_dir = std::env::var("OUT_DIR")?;
    let dest_path = std::path::Path::new(&out_dir).join("prerendered_icons.rs");

    let mut output = String::new();
    output.push_str(
        "pub fn lookup_prerendered_icon(url: &str) -> Option<(u32, u32, &'static [u8])> {\n\
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
        match render_agent_icon(icon_url) {
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

fn render_agent_icon(url: &str) -> Option<Vec<u8>> {
    let mut response = ureq::get(url).call().ok()?;
    let body = response.body_mut().read_to_vec().ok()?;
    let svg_data = String::from_utf8(body).ok()?;

    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(&svg_data, &opt).ok()?;

    let viewport = tree.size();
    let vp_w = viewport.width();
    let vp_h = viewport.height();
    if vp_w <= 0.0 || vp_h <= 0.0 {
        return None;
    }

    let size = 16u32;
    let scale_x = size as f32 / vp_w;
    let scale_y = size as f32 / vp_h;
    let scale = scale_x.min(scale_y);

    let mut pixmap = tiny_skia::Pixmap::new(size, size)?;
    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    Some(pixmap.data().chunks(4).map(|rgba| rgba[3]).collect())
}
