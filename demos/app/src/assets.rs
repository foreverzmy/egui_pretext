use std::collections::HashMap;

use eframe::egui;
use egui::{ColorImage, FontData, FontDefinitions, FontFamily};
use image::{ImageBuffer, Rgba};
use resvg::usvg;

#[derive(Default)]
pub struct AssetRegistry {
    textures: HashMap<String, egui::TextureHandle>,
}

impl AssetRegistry {
    pub fn bundled_font_data() -> Vec<Vec<u8>> {
        vec![
            include_bytes!("../assets/fonts/NotoSans-Regular.ttf").to_vec(),
            include_bytes!("../assets/fonts/NotoSansArabic-Regular.ttf").to_vec(),
            include_bytes!("../assets/fonts/NotoSansCJK-Regular.ttc").to_vec(),
            include_bytes!("../assets/fonts/NotoSansMyanmar-Regular.ttf").to_vec(),
            include_bytes!("../assets/fonts/Noto-COLRv1.ttf").to_vec(),
            include_bytes!("../assets/fonts/NotoSansMono-Regular.ttf").to_vec(),
        ]
    }

    fn font_definitions() -> FontDefinitions {
        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert(
            "noto-sans".to_owned(),
            FontData::from_static(include_bytes!("../assets/fonts/NotoSans-Regular.ttf")).into(),
        );
        fonts.font_data.insert(
            "noto-sans-arabic".to_owned(),
            FontData::from_static(include_bytes!("../assets/fonts/NotoSansArabic-Regular.ttf"))
                .into(),
        );
        fonts.font_data.insert(
            "noto-sans-cjk".to_owned(),
            FontData::from_static(include_bytes!("../assets/fonts/NotoSansCJK-Regular.ttc")).into(),
        );
        fonts.font_data.insert(
            "noto-sans-myanmar".to_owned(),
            FontData::from_static(include_bytes!(
                "../assets/fonts/NotoSansMyanmar-Regular.ttf"
            ))
            .into(),
        );
        // Use the Google `noto-emoji` COLRv1 font so the engine and UI share the same emoji source.
        fonts.font_data.insert(
            "noto-colr-emoji".to_owned(),
            FontData::from_static(include_bytes!("../assets/fonts/Noto-COLRv1.ttf")).into(),
        );
        fonts.font_data.insert(
            "noto-sans-mono".to_owned(),
            FontData::from_static(include_bytes!("../assets/fonts/NotoSansMono-Regular.ttf"))
                .into(),
        );

        let proportional = fonts.families.entry(FontFamily::Proportional).or_default();
        proportional.insert(0, "noto-sans".to_owned());
        proportional.insert(1, "noto-sans-arabic".to_owned());
        proportional.insert(2, "noto-sans-cjk".to_owned());
        proportional.insert(3, "noto-sans-myanmar".to_owned());
        proportional.insert(4, "noto-colr-emoji".to_owned());

        let monospace = fonts.families.entry(FontFamily::Monospace).or_default();
        monospace.insert(0, "noto-sans-mono".to_owned());
        monospace.insert(1, "noto-sans-arabic".to_owned());
        monospace.insert(2, "noto-sans-cjk".to_owned());
        monospace.insert(3, "noto-sans-myanmar".to_owned());
        monospace.insert(4, "noto-colr-emoji".to_owned());

        fonts
    }

    pub fn install_fonts(&mut self, ctx: &egui::Context) {
        ctx.set_fonts(Self::font_definitions());
    }

    pub fn openai_logo_svg() -> &'static [u8] {
        include_bytes!("../assets/logos/openai-symbol.svg")
    }

    pub fn claude_logo_svg() -> &'static [u8] {
        include_bytes!("../assets/logos/claude-symbol.svg")
    }

    pub fn rocket_emoji_svg() -> &'static [u8] {
        include_bytes!("../assets/emoji_u1f680.svg")
    }

    pub fn party_popper_emoji_svg() -> &'static [u8] {
        include_bytes!("../assets/emoji_u1f389.svg")
    }

    pub fn get_or_load_svg(
        &mut self,
        key: &str,
        svg_bytes: &[u8],
        size: [usize; 2],
        ctx: &egui::Context,
    ) -> &egui::TextureHandle {
        self.textures.entry(key.to_owned()).or_insert_with(|| {
            let image =
                rasterize_svg(svg_bytes, size, false).unwrap_or_else(|| transparent_image(size));
            ctx.load_texture(key.to_owned(), image, egui::TextureOptions::LINEAR)
        })
    }

    pub fn get_or_load_generated_image(
        &mut self,
        key: &str,
        size: [usize; 2],
        options: egui::TextureOptions,
        ctx: &egui::Context,
        build: impl FnOnce() -> Option<ColorImage>,
    ) -> &egui::TextureHandle {
        self.textures.entry(key.to_owned()).or_insert_with(|| {
            let image = build().unwrap_or_else(|| transparent_image(size));
            ctx.load_texture(key.to_owned(), image, options)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ui_fonts_prefer_local_colr_emoji_font() {
        let fonts = AssetRegistry::font_definitions();
        let proportional = fonts
            .families
            .get(&FontFamily::Proportional)
            .expect("proportional family");
        let local_emoji = proportional
            .iter()
            .position(|name| name == "noto-colr-emoji")
            .expect("expected local COLRv1 emoji font in proportional family");
        let builtin_emoji = proportional
            .iter()
            .position(|name| name == "NotoEmoji-Regular")
            .expect("expected builtin emoji fallback in proportional family");

        assert!(local_emoji < builtin_emoji);
        assert!(fonts.font_data.contains_key("noto-colr-emoji"));
    }

    #[test]
    fn installed_ui_fonts_cover_mixed_arabic_and_emoji_text() {
        let ctx = egui::Context::default();
        let mut assets = AssetRegistry::default();
        assets.install_fonts(&ctx);

        let mut probe = None;
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            let font_id = egui::FontId::new(16.0, FontFamily::Proportional);
            probe = Some(ctx.fonts_mut(|fonts| {
                (
                    fonts.has_glyphs(&font_id, "بدأت الرحلة 🚀"),
                    fonts.glyph_width(&font_id, '🚀'),
                )
            }));
        });
        let (supports_sample, rocket_width) = probe.expect("expected probe result");

        assert!(supports_sample);
        assert!(rocket_width > 0.0);
    }
}

fn rasterize_svg(
    svg_bytes: &[u8],
    size: [usize; 2],
    load_bundled_fonts: bool,
) -> Option<ColorImage> {
    let mut options = usvg::Options::default();
    if load_bundled_fonts {
        let fontdb = options.fontdb_mut();
        for data in AssetRegistry::bundled_font_data() {
            fontdb.load_font_data(data);
        }
        fontdb.set_sans_serif_family("Noto Sans");
        fontdb.set_monospace_family("Noto Sans Mono");
    }
    let tree = usvg::Tree::from_data(svg_bytes, &options).ok()?;
    let mut pixmap = tiny_skia::Pixmap::new(size[0] as u32, size[1] as u32)?;
    let svg_size = tree.size();
    let scale_x = size[0] as f32 / svg_size.width();
    let scale_y = size[1] as f32 / svg_size.height();
    let transform = tiny_skia::Transform::from_scale(scale_x, scale_y);

    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(
        size[0] as u32,
        size[1] as u32,
        pixmap.data().to_vec(),
    )?;
    let pixels = image
        .pixels()
        .map(|pixel| egui::Color32::from_rgba_premultiplied(pixel[0], pixel[1], pixel[2], pixel[3]))
        .collect();
    Some(ColorImage::new(size, pixels))
}

fn transparent_image(size: [usize; 2]) -> ColorImage {
    let pixels = vec![egui::Color32::from_rgba_premultiplied(0, 0, 0, 0); size[0] * size[1]];
    ColorImage::new(size, pixels)
}
