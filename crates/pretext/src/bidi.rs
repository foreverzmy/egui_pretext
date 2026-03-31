use std::ops::Range;

use unicode_bidi::{BidiInfo, Level};

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub enum ParagraphDirection {
    #[default]
    Auto,
    Ltr,
    Rtl,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum BidiDirection {
    Ltr,
    Rtl,
}

#[derive(Clone, Debug)]
pub struct BidiRun {
    pub byte_range: Range<usize>,
    pub level: Level,
    pub direction: BidiDirection,
}

pub fn paragraph_to_bidi_runs(text: &str, base_direction: ParagraphDirection) -> Vec<BidiRun> {
    if text.is_empty() {
        return Vec::new();
    }

    let default_para_level = match base_direction {
        ParagraphDirection::Auto => None,
        ParagraphDirection::Ltr => Some(Level::ltr()),
        ParagraphDirection::Rtl => Some(Level::rtl()),
    };
    let bidi = BidiInfo::new(text, default_para_level);
    if bidi.levels.is_empty() {
        return vec![BidiRun {
            byte_range: 0..text.len(),
            level: Level::ltr(),
            direction: paragraph_direction(&bidi),
        }];
    }

    let mut runs = Vec::new();
    for para in &bidi.paragraphs {
        runs.extend(coalesce_byte_levels(&bidi.levels, text, para.range.clone()));
    }

    if runs.is_empty() {
        runs.push(BidiRun {
            byte_range: 0..text.len(),
            level: Level::ltr(),
            direction: paragraph_direction(&bidi),
        });
    }

    runs
}

fn coalesce_byte_levels(levels: &[Level], text: &str, byte_range: Range<usize>) -> Vec<BidiRun> {
    if byte_range.start >= byte_range.end || byte_range.end > text.len() {
        return Vec::new();
    }

    debug_assert_eq!(levels.len(), text.len());
    debug_assert!(text.is_char_boundary(byte_range.start));
    debug_assert!(text.is_char_boundary(byte_range.end));

    let mut starts = text[byte_range.clone()]
        .char_indices()
        .map(|(offset, _)| byte_range.start + offset);
    let Some(mut run_start) = starts.next() else {
        return Vec::new();
    };
    let mut current = levels[run_start];
    let mut runs = Vec::new();

    for byte_start in starts {
        let level = levels[byte_start];
        if level == current {
            continue;
        }

        runs.push(BidiRun {
            byte_range: run_start..byte_start,
            level: current,
            direction: direction_from_level(current),
        });
        run_start = byte_start;
        current = level;
    }

    runs.push(BidiRun {
        byte_range: run_start..byte_range.end,
        level: current,
        direction: direction_from_level(current),
    });
    runs
}

fn paragraph_direction(bidi: &BidiInfo<'_>) -> BidiDirection {
    bidi.paragraphs
        .first()
        .map(|para| direction_from_level(para.level))
        .unwrap_or(BidiDirection::Ltr)
}

fn direction_from_level(level: Level) -> BidiDirection {
    if level.is_rtl() {
        BidiDirection::Rtl
    } else {
        BidiDirection::Ltr
    }
}
