mod support;
mod editorial_obstacle {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tests_support/editorial_obstacle.rs"
    ));
}

use std::env;
use std::fs;
use std::path::PathBuf;

use pretext::{ParagraphDirection, PretextParagraphOptions, WhiteSpaceMode};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy)]
struct GoldenCase {
    file: &'static str,
    text: &'static str,
    white_space: WhiteSpaceMode,
    width: f32,
    line_height: f32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct GoldenLayout {
    height: f32,
    line_count: usize,
    lines: Vec<GoldenLine>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct GoldenLine {
    text: String,
    width: f32,
}

const CASES: &[GoldenCase] = &[
    GoldenCase {
        file: "01_english_normal.json",
        text: "The layout engine should keep English prose stable across wraps.",
        white_space: WhiteSpaceMode::Normal,
        width: 180.0,
        line_height: 22.0,
    },
    GoldenCase {
        file: "02_english_prewrap.json",
        text: "alpha\tbeta\ngamma delta",
        white_space: WhiteSpaceMode::PreWrap,
        width: 140.0,
        line_height: 22.0,
    },
    GoldenCase {
        file: "03_arabic_rtl.json",
        text: "مرحبا بكم في محرك التخطيط العربي المتدفق",
        white_space: WhiteSpaceMode::Normal,
        width: 160.0,
        line_height: 22.0,
    },
    GoldenCase {
        file: "04_cjk.json",
        text: "这是一个用于测试换行行为的纯中文段落没有空格但是需要稳定几何",
        white_space: WhiteSpaceMode::Normal,
        width: 140.0,
        line_height: 22.0,
    },
    GoldenCase {
        file: "05_myanmar.json",
        text: "မြန်မာစာစီကုံးမှုကို စမ်းသပ်ရန် ဒီစာပိုဒ်ကို သုံးထားသည်",
        white_space: WhiteSpaceMode::Normal,
        width: 150.0,
        line_height: 22.0,
    },
    GoldenCase {
        file: "06_mixed_direction.json",
        text: "English قبل العربية and then back again",
        white_space: WhiteSpaceMode::Normal,
        width: 170.0,
        line_height: 22.0,
    },
    GoldenCase {
        file: "07_nbsp_wj_zwsp.json",
        text: "Keep\u{00A0}NBSP and word\u{2060}joiner plus zero\u{200B}width breaks available",
        white_space: WhiteSpaceMode::Normal,
        width: 150.0,
        line_height: 22.0,
    },
    GoldenCase {
        file: "08_soft_hyphen.json",
        text: "hy\u{00AD}phenation demonstration keeps soft hyphen hidden until needed",
        white_space: WhiteSpaceMode::Normal,
        width: 110.0,
        line_height: 22.0,
    },
    GoldenCase {
        file: "09_emoji_zwj.json",
        text: "Family emoji 👨‍👩‍👧‍👦 should stay atomic in the output",
        white_space: WhiteSpaceMode::Normal,
        width: 160.0,
        line_height: 22.0,
    },
    GoldenCase {
        file: "10_url.json",
        text: "See https://example.com/some/really/long/path?query=value for details",
        white_space: WhiteSpaceMode::Normal,
        width: 170.0,
        line_height: 22.0,
    },
    GoldenCase {
        file: "11_numbers_punctuation.json",
        text: "v2.0, 3,141.59; [alpha] {beta} -> 42% done.",
        white_space: WhiteSpaceMode::Normal,
        width: 145.0,
        line_height: 22.0,
    },
];

#[test]
fn layout_paragraph_matches_goldens() {
    let engine = support::bundled_engine();
    let style = support::default_style();
    let update = env::var_os("UPDATE_GOLDENS").is_some();
    let fixtures_dir = fixtures_dir();
    let _ = fs::create_dir_all(&fixtures_dir);

    for case in CASES {
        let prepared = engine.prepare_paragraph(
            case.text,
            &style,
            &PretextParagraphOptions {
                white_space: case.white_space,
                paragraph_direction: ParagraphDirection::Auto,
                ..PretextParagraphOptions::default()
            },
        );
        let result = engine.layout_paragraph(&prepared, case.width, case.line_height);
        let actual = GoldenLayout {
            height: round3(result.height),
            line_count: result.line_count,
            lines: result
                .lines
                .into_iter()
                .map(|line| GoldenLine {
                    text: line.line.text,
                    width: round3(line.line.width),
                })
                .collect(),
        };

        let path = fixtures_dir.join(case.file);
        if update {
            fs::write(&path, serde_json::to_string_pretty(&actual).unwrap()).unwrap();
            continue;
        }

        let expected: GoldenLayout = serde_json::from_str(
            &fs::read_to_string(&path)
                .unwrap_or_else(|_| panic!("missing golden fixture: {}", path.display())),
        )
        .unwrap_or_else(|_| panic!("invalid golden fixture: {}", path.display()));

        assert_eq!(
            actual, expected,
            "golden mismatch for {}. Re-run with UPDATE_GOLDENS=1 to refresh fixtures.",
            case.file
        );
    }
}

#[test]
fn editorial_obstacle_matches_golden() {
    let engine = support::bundled_engine();
    let update = env::var_os("UPDATE_GOLDENS").is_some();
    let fixtures_dir = fixtures_dir();
    let _ = fs::create_dir_all(&fixtures_dir);
    let actual = editorial_obstacle::compute_editorial_golden(&engine);
    let path = fixtures_dir.join("12_editorial_obstacle.json");

    if update {
        fs::write(&path, serde_json::to_string_pretty(&actual).unwrap()).unwrap();
        return;
    }

    let expected: editorial_obstacle::EditorialGolden = serde_json::from_str(
        &fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("missing golden fixture: {}", path.display())),
    )
    .unwrap_or_else(|_| panic!("invalid golden fixture: {}", path.display()));

    assert_eq!(
        actual, expected,
        "golden mismatch for 12_editorial_obstacle.json. Re-run with UPDATE_GOLDENS=1 to refresh fixtures."
    );
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/goldens")
}

fn round3(value: f32) -> f32 {
    (value * 1000.0).round() / 1000.0
}
