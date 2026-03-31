use pretext::{PretextEngine, TextStyleSpec};

pub fn bundled_engine() -> PretextEngine {
    PretextEngine::with_font_data_and_system_fonts(bundled_font_data(), false)
}

pub fn bundled_font_data() -> Vec<Vec<u8>> {
    vec![
        include_bytes!("../../../../demos/app/assets/fonts/NotoSans-Regular.ttf").to_vec(),
        include_bytes!("../../../../demos/app/assets/fonts/NotoSansArabic-Regular.ttf").to_vec(),
        include_bytes!("../../../../demos/app/assets/fonts/NotoSansCJK-Regular.ttc").to_vec(),
        include_bytes!("../../../../demos/app/assets/fonts/NotoSansMyanmar-Regular.ttf").to_vec(),
        include_bytes!("../../../../demos/app/assets/fonts/Noto-COLRv1.ttf").to_vec(),
        include_bytes!("../../../../demos/app/assets/fonts/NotoSansMono-Regular.ttf").to_vec(),
    ]
}

pub fn default_style() -> TextStyleSpec {
    TextStyleSpec {
        families: vec![
            "Noto Sans".to_owned(),
            "Noto Sans Arabic".to_owned(),
            "Arial".to_owned(),
            "Helvetica".to_owned(),
        ],
        size_px: 16.0,
        weight: 400,
        italic: false,
    }
}
