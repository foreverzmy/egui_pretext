use eframe::egui::{Context, FontData, FontDefinitions, FontFamily, TextureHandle};
use pretext_egui::{experimental, EguiPretextRenderer};

pub use experimental::demo_assets::SvgAssetId;

macro_rules! include_demo_asset {
    ($path:literal) => {
        include_bytes!(concat!("../assets/", $path))
    };
}

pub fn bundled_font_data() -> Vec<Vec<u8>> {
    vec![
        include_demo_asset!("fonts/NotoSans-Regular.ttf").to_vec(),
        include_demo_asset!("fonts/NotoSerif-Regular.ttf").to_vec(),
        include_demo_asset!("fonts/NotoSerif-Italic.ttf").to_vec(),
        include_demo_asset!("fonts/NotoSerif-Bold.ttf").to_vec(),
        include_demo_asset!("fonts/NotoSansArabic-Regular.ttf").to_vec(),
        include_demo_asset!("fonts/NotoSansCJK-Regular.ttc").to_vec(),
        include_demo_asset!("fonts/NotoSansMyanmar-Regular.ttf").to_vec(),
        include_demo_asset!("fonts/NotoEmoji-Regular.ttf").to_vec(),
        include_demo_asset!("fonts/NotoColorEmoji.ttf").to_vec(),
        include_demo_asset!("fonts/Noto-COLRv1.ttf").to_vec(),
        include_demo_asset!("fonts/NotoSansMono-Regular.ttf").to_vec(),
    ]
}

pub fn install_demo_fonts(ctx: &Context) {
    ctx.set_fonts(demo_font_definitions());
}

pub fn svg_bytes(asset_id: SvgAssetId) -> &'static [u8] {
    experimental::demo_assets::svg_bytes(asset_id)
}

pub fn bundled_svg_texture(
    renderer: &mut EguiPretextRenderer,
    asset_id: SvgAssetId,
    size: [usize; 2],
    ctx: &Context,
) -> TextureHandle {
    experimental::demo_assets::bundled_svg_texture(renderer, asset_id, size, ctx)
}

fn demo_font_definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "noto-sans".to_owned(),
        FontData::from_static(include_demo_asset!("fonts/NotoSans-Regular.ttf")).into(),
    );
    fonts.font_data.insert(
        "noto-sans-arabic".to_owned(),
        FontData::from_static(include_demo_asset!("fonts/NotoSansArabic-Regular.ttf")).into(),
    );
    fonts.font_data.insert(
        "noto-sans-cjk".to_owned(),
        FontData::from_static(include_demo_asset!("fonts/NotoSansCJK-Regular.ttc")).into(),
    );
    fonts.font_data.insert(
        "noto-sans-myanmar".to_owned(),
        FontData::from_static(include_demo_asset!("fonts/NotoSansMyanmar-Regular.ttf")).into(),
    );
    fonts.font_data.insert(
        "noto-emoji-regular-local".to_owned(),
        FontData::from_static(include_demo_asset!("fonts/NotoEmoji-Regular.ttf")).into(),
    );
    fonts.font_data.insert(
        "noto-color-emoji".to_owned(),
        FontData::from_static(include_demo_asset!("fonts/NotoColorEmoji.ttf")).into(),
    );
    fonts.font_data.insert(
        "noto-colr-emoji".to_owned(),
        FontData::from_static(include_demo_asset!("fonts/Noto-COLRv1.ttf")).into(),
    );
    fonts.font_data.insert(
        "noto-sans-mono".to_owned(),
        FontData::from_static(include_demo_asset!("fonts/NotoSansMono-Regular.ttf")).into(),
    );

    let proportional = fonts.families.entry(FontFamily::Proportional).or_default();
    proportional.insert(0, "noto-sans".to_owned());
    proportional.insert(1, "noto-sans-arabic".to_owned());
    proportional.insert(2, "noto-sans-cjk".to_owned());
    proportional.insert(3, "noto-sans-myanmar".to_owned());
    proportional.insert(4, "noto-emoji-regular-local".to_owned());
    proportional.insert(5, "noto-color-emoji".to_owned());
    proportional.insert(6, "noto-colr-emoji".to_owned());

    let monospace = fonts.families.entry(FontFamily::Monospace).or_default();
    monospace.insert(0, "noto-sans-mono".to_owned());
    monospace.insert(1, "noto-sans-arabic".to_owned());
    monospace.insert(2, "noto-sans-cjk".to_owned());
    monospace.insert(3, "noto-sans-myanmar".to_owned());
    monospace.insert(4, "noto-emoji-regular-local".to_owned());
    monospace.insert(5, "noto-color-emoji".to_owned());
    monospace.insert(6, "noto-colr-emoji".to_owned());

    fonts
}
