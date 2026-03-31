use ahash::AHashMap;

use crate::analysis::{
    is_cjk, is_cjk_line_end_prohibited, is_cjk_line_start_prohibited, slice_text, AnalyzedGrapheme,
    GraphemeKind, TextAnalysis,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BreakOpportunity {
    Allowed,
    Prohibited,
    Forced,
}

struct BreakContext<'a> {
    analysis: &'a TextAnalysis,
    grapheme_index: usize,
    current: &'a AnalyzedGrapheme,
    current_text: &'a str,
    next_text: Option<&'a str>,
    baseline: BreakOpportunity,
}

type BreakRule = fn(&BreakContext<'_>) -> BreakOpportunity;

const OVERRIDE_RULES: &[BreakRule] = &[
    rule_forced_newline,
    rule_nbsp,
    rule_wj,
    rule_zwsp,
    rule_soft_hyphen,
    rule_cjk_punctuation,
    rule_url_atom,
];

pub(crate) fn compute_breaks(analysis: &TextAnalysis) -> Vec<BreakOpportunity> {
    let baseline_by_byte = baseline_breaks(&analysis.normalized);
    let mut breaks = Vec::with_capacity(analysis.graphemes.len());

    for index in 0..analysis.graphemes.len() {
        let current = &analysis.graphemes[index];
        let current_text = slice_text(&analysis.normalized, &current.byte_range);
        let next_text = analysis
            .graphemes
            .get(index + 1)
            .map(|next| slice_text(&analysis.normalized, &next.byte_range));

        let mut opportunity = break_opportunity(&BreakContext {
            analysis,
            grapheme_index: index,
            current,
            current_text,
            next_text,
            baseline: baseline_by_byte
                .get(&current.byte_range.end)
                .copied()
                .unwrap_or(BreakOpportunity::Prohibited),
        });

        if index + 1 == analysis.graphemes.len() {
            opportunity = merge(opportunity, BreakOpportunity::Forced);
        }

        breaks.push(opportunity);
    }

    breaks
}

fn break_opportunity(ctx: &BreakContext<'_>) -> BreakOpportunity {
    let mut opportunity = uax14_baseline(ctx);
    for rule in OVERRIDE_RULES {
        opportunity = merge(opportunity, rule(ctx));
    }
    opportunity
}

fn merge(a: BreakOpportunity, b: BreakOpportunity) -> BreakOpportunity {
    use BreakOpportunity::*;
    match (a, b) {
        (Forced, _) | (_, Forced) => Forced,
        (Prohibited, _) | (_, Prohibited) => Prohibited,
        _ => Allowed,
    }
}

fn uax14_baseline(ctx: &BreakContext<'_>) -> BreakOpportunity {
    match ctx.current.kind {
        GraphemeKind::Text => {
            if ctx
                .next_text
                .map(|next_text| is_cjk(ctx.current_text) && is_cjk(next_text))
                .unwrap_or(false)
            {
                merge(ctx.baseline, BreakOpportunity::Allowed)
            } else {
                ctx.baseline
            }
        }
        GraphemeKind::Space | GraphemeKind::Tab => merge(ctx.baseline, BreakOpportunity::Allowed),
        GraphemeKind::Newline => BreakOpportunity::Forced,
        GraphemeKind::ZeroWidthBreak | GraphemeKind::SoftHyphen => BreakOpportunity::Allowed,
        GraphemeKind::WordJoiner => BreakOpportunity::Prohibited,
    }
}

fn rule_forced_newline(ctx: &BreakContext<'_>) -> BreakOpportunity {
    if ctx.current.kind == GraphemeKind::Newline {
        BreakOpportunity::Forced
    } else {
        BreakOpportunity::Allowed
    }
}

fn rule_nbsp(ctx: &BreakContext<'_>) -> BreakOpportunity {
    if contains_nbsp(ctx.current_text) || ctx.next_text.is_some_and(contains_nbsp) {
        BreakOpportunity::Prohibited
    } else {
        BreakOpportunity::Allowed
    }
}

fn rule_wj(ctx: &BreakContext<'_>) -> BreakOpportunity {
    if contains_word_joiner(ctx.current_text) || ctx.next_text.is_some_and(contains_word_joiner) {
        BreakOpportunity::Prohibited
    } else {
        BreakOpportunity::Allowed
    }
}

fn rule_zwsp(_ctx: &BreakContext<'_>) -> BreakOpportunity {
    BreakOpportunity::Allowed
}

fn rule_soft_hyphen(_ctx: &BreakContext<'_>) -> BreakOpportunity {
    BreakOpportunity::Allowed
}

fn rule_cjk_punctuation(ctx: &BreakContext<'_>) -> BreakOpportunity {
    if ctx
        .next_text
        .map(is_cjk_line_start_prohibited)
        .unwrap_or(false)
        || is_cjk_line_end_prohibited(ctx.current_text)
    {
        BreakOpportunity::Prohibited
    } else {
        BreakOpportunity::Allowed
    }
}

fn rule_url_atom(ctx: &BreakContext<'_>) -> BreakOpportunity {
    if boundary_is_inside_url(ctx.analysis, ctx.grapheme_index) {
        BreakOpportunity::Prohibited
    } else {
        BreakOpportunity::Allowed
    }
}

fn baseline_breaks(text: &str) -> AHashMap<usize, BreakOpportunity> {
    let mut map = AHashMap::new();
    for (byte_index, opportunity) in unicode_linebreak::linebreaks(text) {
        let mapped = match opportunity {
            unicode_linebreak::BreakOpportunity::Allowed => BreakOpportunity::Allowed,
            unicode_linebreak::BreakOpportunity::Mandatory => BreakOpportunity::Forced,
        };
        map.insert(byte_index, mapped);
    }
    map
}

fn boundary_is_inside_url(analysis: &TextAnalysis, grapheme_index: usize) -> bool {
    let boundary = analysis.graphemes[grapheme_index].byte_range.end;
    analysis
        .urls
        .iter()
        .any(|span| boundary > span.start && boundary < span.end)
}

fn contains_nbsp(text: &str) -> bool {
    text.contains('\u{00A0}')
}

fn contains_word_joiner(text: &str) -> bool {
    text.contains('\u{2060}')
}

#[cfg(test)]
mod tests {
    use super::{compute_breaks, merge, BreakOpportunity};
    use crate::analysis::WhiteSpaceMode;
    use crate::engine::PrepareOptions;

    #[test]
    fn nbsp_prohibits_adjacent_breaks() {
        let analysis = crate::analysis::analyze_text(
            "a\u{00A0}b",
            &PrepareOptions {
                white_space: WhiteSpaceMode::Normal,
                paragraph_direction: crate::bidi::ParagraphDirection::Auto,
            },
            None,
        );
        let breaks = compute_breaks(&analysis);
        assert_eq!(breaks[0], BreakOpportunity::Prohibited);
        assert_eq!(breaks[1], BreakOpportunity::Prohibited);
    }

    #[test]
    fn zero_width_space_allows_break_after_itself() {
        let analysis = crate::analysis::analyze_text(
            "a\u{200B}b",
            &PrepareOptions {
                white_space: WhiteSpaceMode::Normal,
                paragraph_direction: crate::bidi::ParagraphDirection::Auto,
            },
            None,
        );
        let breaks = compute_breaks(&analysis);
        assert_eq!(breaks[1], BreakOpportunity::Allowed);
    }

    #[test]
    fn forced_wins_over_prohibited() {
        let analysis = crate::analysis::analyze_text(
            "\n\u{00A0}a",
            &PrepareOptions {
                white_space: WhiteSpaceMode::PreWrap,
                paragraph_direction: crate::bidi::ParagraphDirection::Auto,
            },
            None,
        );
        let breaks = compute_breaks(&analysis);
        assert_eq!(breaks[0], BreakOpportunity::Forced);
    }

    #[test]
    fn url_remains_atomic() {
        let analysis = crate::analysis::analyze_text(
            "https://example.com/path",
            &PrepareOptions {
                white_space: WhiteSpaceMode::Normal,
                paragraph_direction: crate::bidi::ParagraphDirection::Auto,
            },
            None,
        );
        let breaks = compute_breaks(&analysis);
        assert!(breaks
            .iter()
            .take(breaks.len().saturating_sub(1))
            .all(|opportunity| *opportunity == BreakOpportunity::Prohibited));
    }

    #[test]
    fn cjk_punctuation_is_not_line_start() {
        let analysis = crate::analysis::analyze_text(
            "你。",
            &PrepareOptions {
                white_space: WhiteSpaceMode::Normal,
                paragraph_direction: crate::bidi::ParagraphDirection::Auto,
            },
            None,
        );
        let breaks = compute_breaks(&analysis);
        assert_eq!(breaks[0], BreakOpportunity::Prohibited);
    }

    #[test]
    fn merge_respects_priority() {
        assert_eq!(
            merge(BreakOpportunity::Forced, BreakOpportunity::Prohibited),
            BreakOpportunity::Forced
        );
        assert_eq!(
            merge(BreakOpportunity::Allowed, BreakOpportunity::Prohibited),
            BreakOpportunity::Prohibited
        );
    }
}
