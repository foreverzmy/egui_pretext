use std::collections::HashMap;
use std::sync::OnceLock;

use eframe::egui;
use egui::{
    Align, Color32, CornerRadius, FontId, Frame, Layout, Margin, Rect, RichText, Sense, Stroke,
    StrokeKind,
};
use pretext::advanced::LayoutCursor;
use pretext::{
    ParagraphDirection, PretextEngine, PretextGlyphRun as LayoutLineGlyphRun,
    PretextParagraphOptions as PrepareOptions, PretextStyle as TextStyleSpec, WhiteSpaceMode,
};
use pretext_egui::{
    advanced::{paint_positioned_text_runs, PositionedTextRunRef},
    EguiPretextPaintOptions, EguiPretextRenderer,
};

use crate::demos::DemoWindow;

const TITLE: &str = "Justification Algorithms Compared";
const SUBTITLE: &str =
    "The same passage, three approaches. Narrow the column to reveal the differences.";
const CSS_HEADER: &str = "CSS / GREEDY";
const CSS_DESC: &str = "Native browser justification";
const CSS_NOTE: &str = "CSS and greedy line-breaking produce similar results — both fill each line left-to-right and break when the line overflows. The differences emerge with hyphenation and global optimization.";
const HYPHEN_HEADER: &str = "PRETEXT (HYPHENATION)";
const HYPHEN_DESC: &str = "Greedy with syllable-level hyphenation";
const OPTIMAL_HEADER: &str = "PRETEXT (KNUTH-PLASS)";
const OPTIMAL_DESC: &str = "Optimal global line-breaking with syllable hyphenation";

const PARAGRAPHS: [&str; 4] = [
    r#"The relationship between typographic colour and reading comfort has been studied extensively since the early twentieth century. When lines of justified text contain excessive inter-word spacing, the eye perceives pale horizontal streaks — "rivers" — that cut vertically through the paragraph, disrupting the smooth lateral scanning motion that skilled readers depend upon. These rivers are not merely an aesthetic blemish; they constitute a measurable impediment to reading speed and comprehension."#,
    r#"Traditional typesetting systems addressed this problem through a combination of techniques: hyphenation dictionaries that permitted words to break at syllable boundaries, letterspacing adjustments that distributed small amounts of additional space between individual characters, and — most significantly — global optimization algorithms that evaluated thousands of possible line-break combinations to find the arrangement minimizing total spacing deviation across the entire paragraph."#,
    r#"The Knuth-Plass algorithm, developed by Donald Knuth and Michael Plass for the TeX typesetting system in 1981, remains the gold standard for paragraph optimization. Rather than greedily filling each line from left to right, the algorithm constructs a graph of all feasible breakpoints and finds the shortest path — the combination of breaks that produces the most uniform spacing throughout. Even a simplified implementation produces dramatically better results than the greedy approach used by web browsers and most word processors."#,
    r#"Modern CSS justification operates on a strictly greedy, line-by-line basis: the browser fills each line with as many words as will fit, then distributes the remaining space uniformly between words. This approach requires no lookahead and executes quickly, but it produces wildly inconsistent spacing — particularly in narrow columns where a single long word can force enormous gaps across the preceding line. The result: rivers of white space that would have horrified any compositor working with metal type."#,
];

const COLUMN_WIDTH_MIN: f32 = 200.0;
const COLUMN_WIDTH_MAX: f32 = 600.0;
const DEFAULT_COLUMN_WIDTH: f32 = 364.0;
const PAGE_MIN_WIDTH: f32 = 1140.0;
const PAGE_PADDING_X: i8 = 24;
const PAGE_PADDING_Y: i8 = 18;
const COLUMN_GAP: f32 = 24.0;
const COLUMN_PAD: f32 = 12.0;
const LINE_HEIGHT: f32 = 24.0;
const PARA_GAP: f32 = LINE_HEIGHT * 0.6;
const TEXT_SIZE: f32 = 15.0;
const H1_SIZE: f32 = 32.0;
const SUBTITLE_SIZE: f32 = 13.0;
const LABEL_SIZE: f32 = 11.0;
const METRIC_VALUE_SIZE: f32 = 11.0;
const FOOTER_SIZE: f32 = 12.0;
const SLIDER_WIDTH: f32 = 220.0;
const CANVAS_RADIUS: u8 = 3;
const METRICS_RADIUS: u8 = 3;
const OVERFLOW_EPSILON: f32 = 0.01;
const INF_BADNESS: f32 = 100_000_000.0;

const PAGE_BG: Color32 = Color32::from_rgb(250, 248, 245);
const INK: Color32 = Color32::from_rgb(42, 37, 32);
const HEADING: Color32 = Color32::from_rgb(26, 23, 20);
const MUTED: Color32 = Color32::from_rgb(138, 127, 112);
const MUTED_SOFT: Color32 = Color32::from_rgb(160, 152, 136);
const ACCENT: Color32 = Color32::from_rgb(90, 79, 64);
const RULE: Color32 = Color32::from_rgb(232, 224, 212);
const CANVAS_BG: Color32 = Color32::WHITE;
const METRICS_BG: Color32 = Color32::from_rgb(245, 242, 237);
const GOOD: Color32 = Color32::from_rgb(42, 138, 74);
const OK: Color32 = Color32::from_rgb(184, 112, 32);
const BAD: Color32 = Color32::from_rgb(204, 68, 68);

const PREFIXES: &[&str] = &[
    "anti", "auto", "be", "bi", "co", "com", "con", "contra", "counter", "de", "dis", "en", "em",
    "ex", "extra", "fore", "hyper", "il", "im", "in", "inter", "intra", "ir", "macro", "mal",
    "micro", "mid", "mis", "mono", "multi", "non", "omni", "out", "over", "para", "poly", "post",
    "pre", "pro", "pseudo", "quasi", "re", "retro", "semi", "sub", "super", "sur", "syn", "tele",
    "trans", "tri", "ultra", "un", "under",
];

const SUFFIXES: &[&str] = &[
    "able", "ible", "tion", "sion", "ment", "ness", "ous", "ious", "eous", "ful", "less", "ive",
    "ative", "itive", "al", "ial", "ical", "ical", "ing", "ling", "ed", "er", "est", "ism", "ist",
    "ity", "ety", "ty", "ence", "ance", "ly", "fy", "ify", "ize", "ise", "ure", "ture",
];

const HYPHEN_EXCEPTIONS: &[(&str, &[&str])] = &[
    ("extensively", &["ex", "ten", "sive", "ly"]),
    ("relationship", &["re", "la", "tion", "ship"]),
    ("typographic", &["ty", "po", "graph", "ic"]),
    ("comfortable", &["com", "fort", "a", "ble"]),
    ("horizontal", &["hor", "i", "zon", "tal"]),
    ("vertically", &["ver", "ti", "cal", "ly"]),
    ("disrupting", &["dis", "rupt", "ing"]),
    ("comprehension", &["com", "pre", "hen", "sion"]),
    ("traditional", &["tra", "di", "tion", "al"]),
    ("combination", &["com", "bi", "na", "tion"]),
    ("techniques", &["tech", "niques"]),
    ("hyphenation", &["hy", "phen", "a", "tion"]),
    ("dictionaries", &["dic", "tion", "ar", "ies"]),
    ("permitted", &["per", "mit", "ted"]),
    ("syllable", &["syl", "la", "ble"]),
    ("boundaries", &["bound", "a", "ries"]),
    ("letterspacing", &["let", "ter", "spac", "ing"]),
    ("adjustments", &["ad", "just", "ments"]),
    ("distributed", &["dis", "trib", "u", "ted"]),
    ("additional", &["ad", "di", "tion", "al"]),
    ("individual", &["in", "di", "vid", "u", "al"]),
    ("characters", &["char", "ac", "ters"]),
    ("significantly", &["sig", "nif", "i", "cant", "ly"]),
    ("optimization", &["op", "ti", "mi", "za", "tion"]),
    ("evaluated", &["e", "val", "u", "at", "ed"]),
    ("thousands", &["thou", "sands"]),
    ("possible", &["pos", "si", "ble"]),
    ("arrangement", &["ar", "range", "ment"]),
    ("minimizing", &["min", "i", "miz", "ing"]),
    ("deviation", &["de", "vi", "a", "tion"]),
    ("paragraph", &["par", "a", "graph"]),
    ("algorithm", &["al", "go", "rithm"]),
    ("developed", &["de", "vel", "oped"]),
    ("typesetting", &["type", "set", "ting"]),
    ("constructs", &["con", "structs"]),
    ("feasible", &["fea", "si", "ble"]),
    ("breakpoints", &["break", "points"]),
    ("produces", &["pro", "du", "ces"]),
    ("uniform", &["u", "ni", "form"]),
    ("throughout", &["through", "out"]),
    ("simplified", &["sim", "pli", "fied"]),
    ("implementation", &["im", "ple", "men", "ta", "tion"]),
    ("dramatically", &["dra", "mat", "i", "cal", "ly"]),
    ("processors", &["proc", "es", "sors"]),
    ("justification", &["jus", "ti", "fi", "ca", "tion"]),
    ("operates", &["op", "er", "ates"]),
    ("strictly", &["strict", "ly"]),
    ("distributes", &["dis", "trib", "utes"]),
    ("remaining", &["re", "main", "ing"]),
    ("uniformly", &["u", "ni", "form", "ly"]),
    ("requires", &["re", "quires"]),
    ("lookahead", &["look", "a", "head"]),
    ("executes", &["ex", "e", "cutes"]),
    ("quickly", &["quick", "ly"]),
    ("inconsistent", &["in", "con", "sis", "tent"]),
    ("particularly", &["par", "tic", "u", "lar", "ly"]),
    ("enormous", &["e", "nor", "mous"]),
    ("preceding", &["pre", "ced", "ing"]),
    ("compositor", &["com", "pos", "i", "tor"]),
    ("twentieth", &["twen", "ti", "eth"]),
    ("century", &["cen", "tu", "ry"]),
    ("perceived", &["per", "ceived"]),
    ("streaks", &["streaks"]),
    ("scanning", &["scan", "ning"]),
    ("impediment", &["im", "ped", "i", "ment"]),
    ("addressed", &["ad", "dressed"]),
    ("combinations", &["com", "bi", "na", "tions"]),
    ("measuring", &["meas", "ur", "ing"]),
    ("measurable", &["meas", "ur", "a", "ble"]),
    ("reading", &["read", "ing"]),
    ("spacing", &["spac", "ing"]),
    ("between", &["be", "tween"]),
    ("excessive", &["ex", "ces", "sive"]),
    ("aesthetic", &["aes", "thet", "ic"]),
    ("merely", &["mere", "ly"]),
    ("constitute", &["con", "sti", "tute"]),
    ("lateral", &["lat", "er", "al"]),
    ("skilled", &["skilled"]),
    ("readers", &["read", "ers"]),
    ("depend", &["de", "pend"]),
    ("studying", &["stud", "y", "ing"]),
    ("studied", &["stud", "ied"]),
    ("comfort", &["com", "fort"]),
    ("colour", &["col", "our"]),
    ("working", &["work", "ing"]),
    ("horrified", &["hor", "ri", "fied"]),
    ("especially", &["es", "pe", "cial", "ly"]),
    ("precisely", &["pre", "cise", "ly"]),
    ("browsers", &["brows", "ers"]),
    ("modern", &["mod", "ern"]),
    ("approach", &["ap", "proach"]),
    ("wildly", &["wild", "ly"]),
    ("columns", &["col", "umns"]),
    ("single", &["sin", "gle"]),
    ("standard", &["stan", "dard"]),
    ("michael", &["Mi", "cha", "el"]),
    ("donald", &["Don", "ald"]),
    ("remains", &["re", "mains"]),
    ("system", &["sys", "tem"]),
    ("rather", &["rath", "er"]),
    ("greedily", &["greed", "i", "ly"]),
    ("filling", &["fill", "ing"]),
    ("shortest", &["short", "est"]),
    ("results", &["re", "sults"]),
    ("greedy", &["greed", "y"]),
    ("number", &["num", "ber"]),
    ("completely", &["com", "plete", "ly"]),
    ("different", &["dif", "fer", "ent"]),
    ("problem", &["prob", "lem"]),
    ("amounts", &["a", "mounts"]),
    ("entire", &["en", "tire"]),
    ("global", &["glob", "al"]),
    ("metal", &["met", "al"]),
    ("every", &["ev", "ery"]),
    ("inter", &["in", "ter"]),
];

pub struct JustificationAlgorithmsDemo {
    open: bool,
    requested_column_width: f32,
    prepared_engine_revision: Option<u64>,
    measurements: Option<DemoMeasurements>,
    layout_cache: Option<JustificationLayoutCache>,
}

#[derive(Clone)]
struct DemoMeasurements {
    raw_paragraphs: Vec<PreparedParagraph>,
    hyphenated_paragraphs: Vec<PreparedParagraph>,
    normal_space_width: f32,
    hyphen_width: f32,
    hyphen_glyph_runs: Vec<LayoutLineGlyphRun>,
}

#[derive(Clone, Debug, PartialEq)]
struct PreparedParagraph {
    segments: Vec<Segment>,
}

#[derive(Clone, Debug, PartialEq)]
struct Segment {
    text: String,
    width: f32,
    kind: SegmentKind,
    glyph_runs: Vec<LayoutLineGlyphRun>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SegmentKind {
    Word,
    Space,
    SoftHyphen,
}

#[derive(Clone, Debug, PartialEq)]
struct RenderSegment {
    text: String,
    width: f32,
    is_space: bool,
    glyph_runs: Vec<LayoutLineGlyphRun>,
}

#[derive(Clone, Debug, PartialEq)]
struct JustifiedLine {
    segments: Vec<RenderSegment>,
    line_width: f32,
    max_width: f32,
    is_last: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct ColumnMetrics {
    avg_deviation: f32,
    max_deviation: f32,
    river_count: usize,
    line_count: usize,
}

#[derive(Clone)]
struct ColumnState {
    header: &'static str,
    description: &'static str,
    note: Option<&'static str>,
    paragraphs: Vec<Vec<JustifiedLine>>,
    metrics: ColumnMetrics,
    normal_space_width: f32,
    canvas_height: f32,
}

#[derive(Clone)]
struct JustificationLayoutState {
    columns: Vec<ColumnState>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LayoutCacheKey {
    engine_revision: u64,
    column_width_q: u32,
}

struct JustificationLayoutCache {
    key: LayoutCacheKey,
    state: JustificationLayoutState,
}

#[derive(Clone, Copy)]
struct BreakCandidate {
    segment_index: usize,
    ends_with_hyphen: bool,
}

#[derive(Clone, Copy)]
struct LineInfo {
    word_width: f32,
    space_count: usize,
    ends_with_hyphen: bool,
}

impl Default for JustificationAlgorithmsDemo {
    fn default() -> Self {
        Self {
            open: false,
            requested_column_width: DEFAULT_COLUMN_WIDTH,
            prepared_engine_revision: None,
            measurements: None,
            layout_cache: None,
        }
    }
}

impl DemoWindow for JustificationAlgorithmsDemo {
    fn id(&self) -> &'static str {
        "justification_algorithms"
    }

    fn title(&self) -> &str {
        TITLE
    }

    fn is_open(&self) -> bool {
        self.open
    }

    fn set_open(&mut self, open: bool) {
        self.open = open;
    }

    fn show(
        &mut self,
        ctx: &egui::Context,
        engine: &PretextEngine,
        assets: &mut EguiPretextRenderer,
    ) {
        let mut open = self.open;
        egui::Window::new(self.title())
            .default_size(egui::vec2(1240.0, 1820.0))
            .open(&mut open)
            .resizable(true)
            .show(ctx, |ui| {
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.render_page(ui, ctx, engine, assets);
                    });
            });
        self.open = open;
    }
}

impl JustificationAlgorithmsDemo {
    fn render_page(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        engine: &PretextEngine,
        assets: &mut EguiPretextRenderer,
    ) {
        self.ensure_measurements(engine);

        let total_columns_width = self.requested_column_width * 3.0 + COLUMN_GAP * 2.0;
        let page_width = total_columns_width
            .max(PAGE_MIN_WIDTH)
            .max(ui.available_width() - PAGE_PADDING_X as f32 * 2.0);

        Frame::new()
            .fill(PAGE_BG)
            .inner_margin(Margin::symmetric(PAGE_PADDING_X, PAGE_PADDING_Y))
            .show(ui, |ui| {
                ui.set_min_width(page_width.max(1.0));
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);

                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(TITLE)
                            .size(H1_SIZE)
                            .color(HEADING)
                            .font(FontId::proportional(H1_SIZE)),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(SUBTITLE)
                            .size(SUBTITLE_SIZE)
                            .color(MUTED)
                            .font(FontId::proportional(SUBTITLE_SIZE)),
                    );
                });

                ui.add_space(10.0);
                let value_label = format!("{}px", self.requested_column_width as i32);
                let value_width = 44.0;
                let row_width = 96.0 + SLIDER_WIDTH + value_width;
                ui.horizontal(|ui| {
                    ui.add_space(((ui.available_width() - row_width).max(0.0)) * 0.5);
                    ui.label(
                        RichText::new("Column width")
                            .size(LABEL_SIZE)
                            .strong()
                            .color(MUTED),
                    );
                    ui.add_space(12.0);
                    ui.scope(|ui| {
                        ui.spacing_mut().slider_width = SLIDER_WIDTH;
                        ui.add_sized(
                            egui::vec2(SLIDER_WIDTH, 18.0),
                            egui::Slider::new(
                                &mut self.requested_column_width,
                                COLUMN_WIDTH_MIN..=COLUMN_WIDTH_MAX,
                            )
                            .step_by(1.0)
                            .show_value(false),
                        );
                    });
                    self.requested_column_width = self
                        .requested_column_width
                        .round()
                        .clamp(COLUMN_WIDTH_MIN, COLUMN_WIDTH_MAX);
                    ui.add_space(8.0);
                    ui.add_sized(
                        egui::vec2(value_width, 18.0),
                        egui::Label::new(
                            RichText::new(value_label).size(13.0).strong().color(ACCENT),
                        ),
                    );
                });

                ui.add_space(16.0);

                let column_width = self.requested_column_width;
                let layout_state = self.ensure_layout_state(engine);
                let left_gutter = ((ui.available_width() - total_columns_width).max(0.0)) * 0.5;
                ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                    ui.add_space(left_gutter);
                    for (index, column) in layout_state.columns.iter().enumerate() {
                        if index > 0 {
                            ui.add_space(COLUMN_GAP);
                        }
                        show_column(
                            ui,
                            ctx,
                            engine,
                            assets,
                            column_width,
                            justification_text_style(),
                            column,
                        );
                    }
                });

                ui.add_space(32.0);
                paint_footer(ui);
            });
    }

    fn ensure_measurements(&mut self, engine: &PretextEngine) {
        let engine_revision = engine.revision();
        if self.prepared_engine_revision == Some(engine_revision) && self.measurements.is_some() {
            return;
        }

        self.measurements = Some(build_measurements(
            engine,
            justification_text_style(),
            &PARAGRAPHS,
        ));
        self.prepared_engine_revision = Some(engine_revision);
        self.layout_cache = None;
    }

    fn ensure_layout_state(&mut self, engine: &PretextEngine) -> &JustificationLayoutState {
        self.ensure_measurements(engine);
        let key = LayoutCacheKey {
            engine_revision: engine.revision(),
            column_width_q: self.requested_column_width.round() as u32,
        };

        let should_rebuild = self
            .layout_cache
            .as_ref()
            .map(|cache| cache.key != key)
            .unwrap_or(true);
        if should_rebuild {
            let measurements = self
                .measurements
                .as_ref()
                .expect("measurements should exist before building layout");
            let inner_width = (self.requested_column_width - COLUMN_PAD * 2.0).max(1.0);
            let css_paragraphs = measurements
                .raw_paragraphs
                .iter()
                .map(|paragraph| {
                    greedy_layout(
                        &paragraph.segments,
                        inner_width,
                        measurements.hyphen_width,
                        &measurements.hyphen_glyph_runs,
                    )
                })
                .collect::<Vec<_>>();
            let hyphenated_paragraphs = measurements
                .hyphenated_paragraphs
                .iter()
                .map(|paragraph| {
                    greedy_layout(
                        &paragraph.segments,
                        inner_width,
                        measurements.hyphen_width,
                        &measurements.hyphen_glyph_runs,
                    )
                })
                .collect::<Vec<_>>();
            let optimal_paragraphs = measurements
                .hyphenated_paragraphs
                .iter()
                .map(|paragraph| {
                    optimal_layout(
                        &paragraph.segments,
                        inner_width,
                        measurements.normal_space_width,
                        measurements.hyphen_width,
                        &measurements.hyphen_glyph_runs,
                    )
                })
                .collect::<Vec<_>>();

            self.layout_cache = Some(JustificationLayoutCache {
                key,
                state: JustificationLayoutState {
                    columns: vec![
                        ColumnState::new(
                            CSS_HEADER,
                            CSS_DESC,
                            Some(CSS_NOTE),
                            css_paragraphs,
                            measurements.normal_space_width,
                        ),
                        ColumnState::new(
                            HYPHEN_HEADER,
                            HYPHEN_DESC,
                            None,
                            hyphenated_paragraphs,
                            measurements.normal_space_width,
                        ),
                        ColumnState::new(
                            OPTIMAL_HEADER,
                            OPTIMAL_DESC,
                            None,
                            optimal_paragraphs,
                            measurements.normal_space_width,
                        ),
                    ],
                },
            });
        }

        &self
            .layout_cache
            .as_ref()
            .expect("layout cache should exist")
            .state
    }
}

impl ColumnState {
    fn new(
        header: &'static str,
        description: &'static str,
        note: Option<&'static str>,
        paragraphs: Vec<Vec<JustifiedLine>>,
        normal_space_width: f32,
    ) -> Self {
        let metrics = compute_metrics(&paragraphs, normal_space_width);
        let line_count = paragraphs
            .iter()
            .map(|paragraph| paragraph.len())
            .sum::<usize>() as f32;
        let paragraph_count = paragraphs.len().saturating_sub(1) as f32;
        let canvas_height =
            COLUMN_PAD * 2.0 + line_count * LINE_HEIGHT + paragraph_count * PARA_GAP;
        Self {
            header,
            description,
            note,
            paragraphs,
            metrics,
            normal_space_width,
            canvas_height: canvas_height.max(COLUMN_PAD * 2.0 + LINE_HEIGHT),
        }
    }
}

fn build_measurements(
    engine: &PretextEngine,
    style: &TextStyleSpec,
    paragraphs: &[&str],
) -> DemoMeasurements {
    let mut width_cache = HashMap::new();
    let mut glyph_runs_cache = HashMap::new();
    let normal_space_width = measure_width_cached(engine, style, " ", &mut width_cache);
    let hyphen_width = measure_width_cached(engine, style, "-", &mut width_cache);
    let hyphen_glyph_runs = measure_glyph_runs_cached(engine, style, "-", &mut glyph_runs_cache);
    let raw_paragraphs = paragraphs
        .iter()
        .map(|paragraph| PreparedParagraph {
            segments: build_segments_for_paragraph(
                engine,
                style,
                paragraph,
                false,
                &mut width_cache,
                &mut glyph_runs_cache,
            ),
        })
        .collect::<Vec<_>>();
    let hyphenated_paragraphs = paragraphs
        .iter()
        .map(|paragraph| PreparedParagraph {
            segments: build_segments_for_paragraph(
                engine,
                style,
                paragraph,
                true,
                &mut width_cache,
                &mut glyph_runs_cache,
            ),
        })
        .collect::<Vec<_>>();

    DemoMeasurements {
        raw_paragraphs,
        hyphenated_paragraphs,
        normal_space_width,
        hyphen_width,
        hyphen_glyph_runs,
    }
}

fn build_segments_for_paragraph(
    engine: &PretextEngine,
    style: &TextStyleSpec,
    paragraph: &str,
    hyphenate: bool,
    width_cache: &mut HashMap<String, f32>,
    glyph_runs_cache: &mut HashMap<String, Vec<LayoutLineGlyphRun>>,
) -> Vec<Segment> {
    let mut output = Vec::new();

    for token in split_preserving_whitespace(paragraph) {
        if token.trim().is_empty() {
            output.push(Segment {
                width: measure_width_cached(engine, style, &token, width_cache),
                text: token,
                kind: SegmentKind::Space,
                glyph_runs: Vec::new(),
            });
            continue;
        }

        if !hyphenate {
            output.push(Segment {
                width: measure_width_cached(engine, style, &token, width_cache),
                glyph_runs: measure_glyph_runs_cached(engine, style, &token, glyph_runs_cache),
                text: token,
                kind: SegmentKind::Word,
            });
            continue;
        }

        let parts = hyphenate_word(&token);
        if parts.len() <= 1 {
            output.push(Segment {
                width: measure_width_cached(engine, style, &token, width_cache),
                glyph_runs: measure_glyph_runs_cached(engine, style, &token, glyph_runs_cache),
                text: token,
                kind: SegmentKind::Word,
            });
            continue;
        }

        let part_count = parts.len();
        for (index, part) in parts.into_iter().enumerate() {
            if !part.is_empty() {
                output.push(Segment {
                    width: measure_width_cached(engine, style, &part, width_cache),
                    glyph_runs: measure_glyph_runs_cached(engine, style, &part, glyph_runs_cache),
                    text: part,
                    kind: SegmentKind::Word,
                });
            }
            if index + 1 < part_count {
                output.push(Segment {
                    width: 0.0,
                    text: "\u{00AD}".to_owned(),
                    kind: SegmentKind::SoftHyphen,
                    glyph_runs: Vec::new(),
                });
            }
        }
    }

    output
}

fn split_preserving_whitespace(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }

    let mut tokens = Vec::new();
    let mut start = 0usize;
    let mut current_is_whitespace = None;

    for (index, ch) in text.char_indices() {
        let is_whitespace = ch.is_whitespace();
        match current_is_whitespace {
            None => {
                current_is_whitespace = Some(is_whitespace);
                start = index;
            }
            Some(current) if current != is_whitespace => {
                tokens.push(text[start..index].to_owned());
                current_is_whitespace = Some(is_whitespace);
                start = index;
            }
            _ => {}
        }
    }

    tokens.push(text[start..].to_owned());
    tokens
}

fn measure_width_cached(
    engine: &PretextEngine,
    style: &TextStyleSpec,
    text: &str,
    cache: &mut HashMap<String, f32>,
) -> f32 {
    if let Some(width) = cache.get(text).copied() {
        return width;
    }

    let width = engine
        .prefix_widths(text, style)
        .last()
        .copied()
        .unwrap_or(0.0);
    cache.insert(text.to_owned(), width);
    width
}

fn measure_glyph_runs_cached(
    engine: &PretextEngine,
    style: &TextStyleSpec,
    text: &str,
    cache: &mut HashMap<String, Vec<LayoutLineGlyphRun>>,
) -> Vec<LayoutLineGlyphRun> {
    if let Some(glyph_runs) = cache.get(text) {
        return glyph_runs.clone();
    }

    let prepared = engine.prepare_paragraph(text, style, &normal_options());
    let mut cursor = LayoutCursor::default();
    let glyph_runs = engine
        .layout_next_line_with_glyph_runs(&prepared, &mut cursor, f32::INFINITY)
        .map(|line| line.glyph_runs)
        .unwrap_or_default();
    cache.insert(text.to_owned(), glyph_runs.clone());
    glyph_runs
}

fn greedy_layout(
    segments: &[Segment],
    max_width: f32,
    hyphen_width: f32,
    hyphen_glyph_runs: &[LayoutLineGlyphRun],
) -> Vec<JustifiedLine> {
    if segments.is_empty() {
        return Vec::new();
    }

    let mut lines = Vec::new();
    let mut line_start = 0usize;
    let mut cursor = 0usize;
    let mut line_width = 0.0f32;
    let mut last_break: Option<(usize, usize, bool)> = None;

    while cursor < segments.len() {
        let segment = &segments[cursor];
        let next_width = line_width + segment.width;
        if next_width > max_width + OVERFLOW_EPSILON && cursor != line_start {
            if let Some((line_end, next_start, append_hyphen)) = last_break {
                let is_last = next_start >= segments.len();
                lines.push(build_line(
                    &segments[line_start..line_end],
                    max_width,
                    append_hyphen,
                    is_last,
                    hyphen_width,
                    hyphen_glyph_runs,
                ));
                line_start = next_start;
                cursor = next_start;
                line_width = 0.0;
                last_break = None;
                continue;
            }
        }

        line_width = next_width;
        match segment.kind {
            SegmentKind::Space => last_break = Some((cursor + 1, cursor + 1, false)),
            SegmentKind::SoftHyphen => last_break = Some((cursor, cursor + 1, true)),
            SegmentKind::Word => {}
        }
        cursor += 1;
    }

    lines.push(build_line(
        &segments[line_start..],
        max_width,
        false,
        true,
        hyphen_width,
        hyphen_glyph_runs,
    ));
    lines
}

fn optimal_layout(
    segments: &[Segment],
    max_width: f32,
    normal_space_width: f32,
    hyphen_width: f32,
    hyphen_glyph_runs: &[LayoutLineGlyphRun],
) -> Vec<JustifiedLine> {
    if segments.is_empty() {
        return Vec::new();
    }

    let break_candidates = collect_break_candidates(segments);
    if break_candidates.len() <= 1 {
        return vec![build_line(
            segments,
            max_width,
            false,
            true,
            hyphen_width,
            hyphen_glyph_runs,
        )];
    }

    let mut word_prefix = vec![0.0f32; segments.len() + 1];
    let mut space_prefix = vec![0usize; segments.len() + 1];
    for (index, segment) in segments.iter().enumerate() {
        word_prefix[index + 1] = word_prefix[index]
            + if segment.kind == SegmentKind::Word {
                segment.width
            } else {
                0.0
            };
        space_prefix[index + 1] =
            space_prefix[index] + usize::from(segment.kind == SegmentKind::Space);
    }

    let mut dp = vec![f32::INFINITY; break_candidates.len()];
    let mut prev = vec![usize::MAX; break_candidates.len()];
    dp[0] = 0.0;

    for to_index in 1..break_candidates.len() {
        let is_last = to_index + 1 == break_candidates.len();
        for from_index in (0..to_index).rev() {
            if !dp[from_index].is_finite() {
                continue;
            }

            let info = line_info(
                segments,
                &break_candidates,
                &word_prefix,
                &space_prefix,
                from_index,
                to_index,
                hyphen_width,
            );
            let total_width = info.word_width + info.space_count as f32 * normal_space_width;
            if total_width > max_width * 2.0 {
                break;
            }

            let total_badness =
                dp[from_index] + line_badness(info, is_last, max_width, normal_space_width);
            if total_badness < dp[to_index] {
                dp[to_index] = total_badness;
                prev[to_index] = from_index;
            }
        }
    }

    let mut break_path = Vec::new();
    let mut cursor = break_candidates.len() - 1;
    if prev[cursor] == usize::MAX {
        return greedy_layout(segments, max_width, hyphen_width, hyphen_glyph_runs);
    }

    while cursor > 0 {
        let from = prev[cursor];
        if from == usize::MAX {
            return greedy_layout(segments, max_width, hyphen_width, hyphen_glyph_runs);
        }
        break_path.push(cursor);
        cursor = from;
    }
    break_path.reverse();

    let mut lines = Vec::new();
    let mut from_candidate = 0usize;
    for &to_candidate in &break_path {
        let from = break_candidates[from_candidate].segment_index;
        let to = break_candidates[to_candidate].segment_index;
        let append_hyphen = break_candidates[to_candidate].ends_with_hyphen;
        let is_last = to_candidate + 1 == break_candidates.len();
        lines.push(build_line(
            &segments[from..to],
            max_width,
            append_hyphen,
            is_last,
            hyphen_width,
            hyphen_glyph_runs,
        ));
        from_candidate = to_candidate;
    }
    lines
}

fn collect_break_candidates(segments: &[Segment]) -> Vec<BreakCandidate> {
    let mut output = vec![BreakCandidate {
        segment_index: 0,
        ends_with_hyphen: false,
    }];

    for (index, segment) in segments.iter().enumerate() {
        match segment.kind {
            SegmentKind::SoftHyphen if index + 1 < segments.len() => output.push(BreakCandidate {
                segment_index: index + 1,
                ends_with_hyphen: true,
            }),
            SegmentKind::Space if index + 1 < segments.len() => output.push(BreakCandidate {
                segment_index: index + 1,
                ends_with_hyphen: false,
            }),
            _ => {}
        }
    }

    output.push(BreakCandidate {
        segment_index: segments.len(),
        ends_with_hyphen: false,
    });
    output
}

fn line_info(
    segments: &[Segment],
    break_candidates: &[BreakCandidate],
    word_prefix: &[f32],
    space_prefix: &[usize],
    from_index: usize,
    to_index: usize,
    hyphen_width: f32,
) -> LineInfo {
    let from = break_candidates[from_index].segment_index;
    let to = break_candidates[to_index].segment_index;
    let ends_with_hyphen = break_candidates[to_index].ends_with_hyphen;

    let mut space_count = space_prefix[to].saturating_sub(space_prefix[from]);
    if to > from && segments[to - 1].kind == SegmentKind::Space {
        space_count = space_count.saturating_sub(1);
    }

    let mut word_width = word_prefix[to] - word_prefix[from];
    if ends_with_hyphen {
        word_width += hyphen_width;
    }

    LineInfo {
        word_width,
        space_count,
        ends_with_hyphen,
    }
}

fn line_badness(
    info: LineInfo,
    is_last_line: bool,
    max_width: f32,
    normal_space_width: f32,
) -> f32 {
    if is_last_line {
        if info.word_width > max_width {
            return INF_BADNESS;
        }
        return 0.0;
    }

    if info.space_count == 0 {
        let slack = max_width - info.word_width;
        if slack < 0.0 {
            return INF_BADNESS;
        }
        return slack * slack * 10.0;
    }

    let justified_space = (max_width - info.word_width) / info.space_count as f32;
    if justified_space < 0.0 {
        return INF_BADNESS;
    }
    if justified_space < normal_space_width * 0.4 {
        return INF_BADNESS;
    }

    let ratio = (justified_space - normal_space_width) / normal_space_width.max(0.0001);
    let abs_ratio = ratio.abs();
    let badness = abs_ratio * abs_ratio * abs_ratio * 1000.0;
    let river_excess = justified_space / normal_space_width.max(0.0001) - 1.5;
    let river_penalty = if river_excess > 0.0 {
        5000.0 + river_excess * river_excess * 10_000.0
    } else {
        0.0
    };
    let tight_threshold = normal_space_width * 0.65;
    let tight_penalty = if justified_space < tight_threshold {
        3000.0
            + (tight_threshold - justified_space) * (tight_threshold - justified_space) * 10_000.0
    } else {
        0.0
    };
    let hyphen_penalty = if info.ends_with_hyphen { 50.0 } else { 0.0 };

    badness + river_penalty + tight_penalty + hyphen_penalty
}

fn build_line(
    segments: &[Segment],
    max_width: f32,
    append_hyphen: bool,
    is_last: bool,
    hyphen_width: f32,
    hyphen_glyph_runs: &[LayoutLineGlyphRun],
) -> JustifiedLine {
    let mut render_segments = Vec::new();
    let mut line_width = 0.0f32;

    for segment in segments {
        match segment.kind {
            SegmentKind::Word => {
                line_width += segment.width;
                render_segments.push(RenderSegment {
                    text: segment.text.clone(),
                    width: segment.width,
                    is_space: false,
                    glyph_runs: segment.glyph_runs.clone(),
                });
            }
            SegmentKind::Space => {
                line_width += segment.width;
                render_segments.push(RenderSegment {
                    text: segment.text.clone(),
                    width: segment.width,
                    is_space: true,
                    glyph_runs: Vec::new(),
                });
            }
            SegmentKind::SoftHyphen => {}
        }
    }

    while render_segments
        .last()
        .is_some_and(|segment| segment.is_space)
    {
        if let Some(removed) = render_segments.pop() {
            line_width -= removed.width;
        }
    }

    if append_hyphen && !is_last {
        line_width += hyphen_width;
        render_segments.push(RenderSegment {
            text: "-".to_owned(),
            width: hyphen_width,
            is_space: false,
            glyph_runs: hyphen_glyph_runs.to_vec(),
        });
    }

    JustifiedLine {
        segments: render_segments,
        line_width,
        max_width,
        is_last,
    }
}

fn compute_metrics(paragraphs: &[Vec<JustifiedLine>], normal_space_width: f32) -> ColumnMetrics {
    let mut total_deviation = 0.0f32;
    let mut max_deviation = 0.0f32;
    let mut measured_lines = 0usize;
    let mut river_count = 0usize;
    let mut line_count = 0usize;

    for paragraph in paragraphs {
        line_count += paragraph.len();
        for line in paragraph {
            if line.is_last {
                continue;
            }

            let space_count = line.space_count();
            if space_count == 0 {
                continue;
            }

            let word_width = line.word_width();
            let justified_space = (line.max_width - word_width) / space_count as f32;
            let deviation =
                ((justified_space - normal_space_width).abs()) / normal_space_width.max(0.0001);
            total_deviation += deviation;
            max_deviation = max_deviation.max(deviation);
            if justified_space > normal_space_width * 1.5 {
                river_count += 1;
            }
            measured_lines += 1;
        }
    }

    ColumnMetrics {
        avg_deviation: if measured_lines > 0 {
            total_deviation / measured_lines as f32
        } else {
            0.0
        },
        max_deviation,
        river_count,
        line_count,
    }
}

impl JustifiedLine {
    fn space_count(&self) -> usize {
        self.segments
            .iter()
            .filter(|segment| segment.is_space)
            .count()
    }

    fn word_width(&self) -> f32 {
        self.segments
            .iter()
            .filter(|segment| !segment.is_space)
            .map(|segment| segment.width)
            .sum()
    }
}

fn show_column(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut EguiPretextRenderer,
    column_width: f32,
    style: &TextStyleSpec,
    column: &ColumnState,
) {
    ui.vertical(|ui| {
        ui.set_min_width(column_width);
        ui.set_max_width(column_width);
        ui.set_width(column_width);

        ui.label(
            RichText::new(column.header)
                .size(LABEL_SIZE)
                .strong()
                .color(ACCENT),
        );
        ui.add_space(2.0);
        ui.label(
            RichText::new(column.description)
                .size(LABEL_SIZE)
                .color(MUTED_SOFT),
        );

        ui.add_space(12.0);
        let (canvas_rect, _) = ui.allocate_exact_size(
            egui::vec2(column_width, column.canvas_height),
            Sense::hover(),
        );
        paint_column_canvas(
            &ui.painter_at(canvas_rect),
            canvas_rect,
            &column,
            ctx,
            engine,
            assets,
            style,
        );

        ui.add_space(8.0);
        show_metrics(ui, &column.metrics);

        if let Some(note) = column.note {
            ui.add_space(10.0);
            ui.label(RichText::new(note).size(LABEL_SIZE).color(MUTED_SOFT));
        }
    });
}

fn paint_column_canvas(
    painter: &egui::Painter,
    rect: Rect,
    column: &ColumnState,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut EguiPretextRenderer,
    style: &TextStyleSpec,
) {
    painter.rect_filled(rect, CornerRadius::same(CANVAS_RADIUS), CANVAS_BG);
    painter.rect_stroke(
        rect,
        CornerRadius::same(CANVAS_RADIUS),
        Stroke::new(1.0, RULE),
        StrokeKind::Inside,
    );

    let mut y = rect.top() + COLUMN_PAD;
    for (paragraph_index, paragraph) in column.paragraphs.iter().enumerate() {
        for line in paragraph {
            paint_line(
                painter,
                rect.left() + COLUMN_PAD,
                y,
                line,
                column.normal_space_width,
                ctx,
                engine,
                assets,
                style,
            );
            y += LINE_HEIGHT;
        }
        if paragraph_index + 1 < column.paragraphs.len() {
            y += PARA_GAP;
        }
    }
}

fn paint_line(
    painter: &egui::Painter,
    x: f32,
    y: f32,
    line: &JustifiedLine,
    natural_space: f32,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut EguiPretextRenderer,
    style: &TextStyleSpec,
) {
    let should_justify =
        !line.is_last && line.line_width >= line.max_width * 0.6 && line.space_count() > 0;
    let word_width = line.word_width();
    let space_count = line.space_count();
    let raw_justified_space = if space_count > 0 {
        (line.max_width - word_width) / space_count as f32
    } else {
        0.0
    };

    let use_justified_space = should_justify && raw_justified_space >= natural_space * 0.2;
    let justified_space = raw_justified_space.max(natural_space * 0.75);
    let river_color = river_highlight_color(justified_space, natural_space);
    let options = EguiPretextPaintOptions::new(style, LINE_HEIGHT)
        .color(INK)
        .fallback_font(FontId::new(style.size_px, egui::FontFamily::Proportional))
        .fallback_align(egui::Align2::LEFT_TOP);

    let _ = paint_positioned_text_runs(
        painter,
        line.segments
            .iter()
            .scan(x, |cursor_x, segment| {
                if segment.is_space {
                    let advance = if use_justified_space {
                        justified_space
                    } else {
                        segment.width
                    };
                    if let Some(color) = river_color.filter(|_| use_justified_space) {
                        let river_width = (advance - 2.0).max(1.0);
                        painter.rect_filled(
                            Rect::from_min_size(
                                egui::pos2(*cursor_x + 1.0, y),
                                egui::vec2(river_width, LINE_HEIGHT),
                            ),
                            CornerRadius::ZERO,
                            color,
                        );
                    }
                    *cursor_x += advance;
                    return Some(None);
                }

                let positioned = PositionedTextRunRef {
                    x: *cursor_x,
                    y,
                    text: &segment.text,
                    glyph_runs: &segment.glyph_runs,
                    emoji_overlays: &[],
                };
                *cursor_x += segment.width;
                Some(Some(positioned))
            })
            .flatten(),
        &options,
        ctx,
        engine,
        assets,
    );
}

fn show_metrics(ui: &mut egui::Ui, metrics: &ColumnMetrics) {
    Frame::new()
        .fill(METRICS_BG)
        .corner_radius(CornerRadius::same(METRICS_RADIUS))
        .inner_margin(Margin::symmetric(10, 8))
        .show(ui, |ui| {
            metric_row(ui, "Lines", &metrics.line_count.to_string(), ACCENT);
            metric_row(
                ui,
                "Avg deviation",
                &format!("{:.1}%", metrics.avg_deviation * 100.0),
                deviation_color(metrics.avg_deviation),
            );
            metric_row(
                ui,
                "Max deviation",
                &format!("{:.1}%", metrics.max_deviation * 100.0),
                deviation_color(metrics.max_deviation * 0.5),
            );
            metric_row(
                ui,
                "River spaces",
                &metrics.river_count.to_string(),
                if metrics.river_count == 0 { GOOD } else { BAD },
            );
        });
}

fn metric_row(ui: &mut egui::Ui, label: &str, value: &str, value_color: Color32) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(LABEL_SIZE).color(MUTED));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(value)
                    .size(METRIC_VALUE_SIZE)
                    .strong()
                    .color(value_color),
            );
        });
    });
}

fn paint_footer(ui: &mut egui::Ui) {
    ui.horizontal_centered(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        ui.label(
            RichText::new("Built with Pretext")
                .size(FOOTER_SIZE)
                .color(MUTED_SOFT),
        );
        ui.label(RichText::new("·").size(FOOTER_SIZE).color(MUTED_SOFT));
        ui.hyperlink_to("GitHub", "https://github.com/somnai-dreams");
        ui.label(RichText::new("·").size(FOOTER_SIZE).color(MUTED_SOFT));
        ui.hyperlink_to("@somnai_dreams", "https://twitter.com/somnai_dreams");
    });
}

fn deviation_color(value: f32) -> Color32 {
    if value < 0.15 {
        GOOD
    } else if value < 0.35 {
        OK
    } else {
        BAD
    }
}

fn river_highlight_color(space_width: f32, normal_space_width: f32) -> Option<Color32> {
    if space_width <= normal_space_width * 1.5 {
        return None;
    }

    let intensity = ((space_width / normal_space_width.max(0.0001) - 1.5) / 1.5).clamp(0.0, 1.0);
    let r = (220.0 + intensity * 35.0).round() as u8;
    let g = (180.0 - intensity * 80.0).round() as u8;
    let b = (180.0 - intensity * 80.0).round() as u8;
    let alpha = (0.25 + intensity * 0.35) * 255.0;
    Some(Color32::from_rgba_premultiplied(
        r,
        g,
        b,
        alpha.round() as u8,
    ))
}

fn normal_options() -> PrepareOptions {
    PrepareOptions {
        white_space: WhiteSpaceMode::Normal,
        word_break: pretext::WordBreakMode::Normal,
        paragraph_direction: ParagraphDirection::Auto,
        letter_spacing: 0.0,
    }
}

fn justification_text_style() -> &'static TextStyleSpec {
    static STYLE: OnceLock<TextStyleSpec> = OnceLock::new();
    STYLE.get_or_init(|| TextStyleSpec {
        families: vec![
            "Noto Serif".to_owned(),
            "Georgia".to_owned(),
            "Times New Roman".to_owned(),
            "Noto Sans".to_owned(),
        ],
        size_px: TEXT_SIZE,
        weight: 400,
        italic: false,
    })
}

fn hyphenate_word(word: &str) -> Vec<String> {
    let lower = word
        .to_lowercase()
        .chars()
        .filter(|ch| {
            !matches!(
                ch,
                '.' | ','
                    | ';'
                    | ':'
                    | '!'
                    | '?'
                    | '"'
                    | '\''
                    | '“'
                    | '”'
                    | '‘'
                    | '’'
                    | '—'
                    | '–'
                    | '-'
            )
        })
        .collect::<String>();

    if lower.chars().count() < 5 {
        return vec![word.to_owned()];
    }

    if let Some(parts) = hyphen_exception_parts(&lower) {
        let mut output = Vec::with_capacity(parts.len());
        let mut byte_cursor = 0usize;
        for part in parts {
            let next = byte_cursor + part.len();
            output.push(word[byte_cursor..next.min(word.len())].to_owned());
            byte_cursor = next.min(word.len());
        }
        if byte_cursor < word.len() {
            if let Some(last) = output.last_mut() {
                last.push_str(&word[byte_cursor..]);
            }
        }
        output.retain(|part| !part.is_empty());
        if !output.is_empty() {
            return output;
        }
    }

    for prefix in PREFIXES {
        if lower.starts_with(prefix) && lower.len().saturating_sub(prefix.len()) >= 3 {
            let cut = prefix.len().min(word.len());
            return vec![word[..cut].to_owned(), word[cut..].to_owned()];
        }
    }

    for suffix in SUFFIXES {
        if lower.ends_with(suffix) && lower.len().saturating_sub(suffix.len()) >= 3 {
            let cut = word.len().saturating_sub(suffix.len());
            return vec![word[..cut].to_owned(), word[cut..].to_owned()];
        }
    }

    vec![word.to_owned()]
}

fn hyphen_exception_parts(word: &str) -> Option<&'static [&'static str]> {
    HYPHEN_EXCEPTIONS
        .iter()
        .find_map(|(entry, parts)| (*entry == word).then_some(*parts))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bundled_engine() -> PretextEngine {
        PretextEngine::builder()
            .with_font_data(crate::demo_assets::bundled_font_data())
            .include_system_fonts(false)
            .build()
    }

    fn word(text: &str, width: f32) -> Segment {
        Segment {
            text: text.to_owned(),
            width,
            kind: SegmentKind::Word,
            glyph_runs: Vec::new(),
        }
    }

    fn space(width: f32) -> Segment {
        Segment {
            text: " ".to_owned(),
            width,
            kind: SegmentKind::Space,
            glyph_runs: Vec::new(),
        }
    }

    fn shy() -> Segment {
        Segment {
            text: "\u{00AD}".to_owned(),
            width: 0.0,
            kind: SegmentKind::SoftHyphen,
            glyph_runs: Vec::new(),
        }
    }

    fn line_text(line: &JustifiedLine) -> String {
        line.segments
            .iter()
            .map(|segment| segment.text.as_str())
            .collect::<String>()
    }

    #[test]
    fn hyphenate_word_uses_known_exception_parts() {
        assert_eq!(
            hyphenate_word("typographic"),
            vec![
                "ty".to_owned(),
                "po".to_owned(),
                "graph".to_owned(),
                "ic".to_owned()
            ]
        );
    }

    #[test]
    fn greedy_layout_appends_hyphen_when_soft_break_wins() {
        let segments = vec![
            word("alpha", 30.0),
            shy(),
            word("beta", 30.0),
            space(10.0),
            word("tail", 20.0),
        ];
        let hyphen_glyph_runs = Vec::new();
        let lines = greedy_layout(&segments, 65.0, 8.0, &hyphen_glyph_runs);
        assert_eq!(lines.len(), 2);
        assert_eq!(line_text(&lines[0]), "alpha-");
        assert_eq!(line_text(&lines[1]), "beta tail");
    }

    #[test]
    fn optimal_layout_improves_default_metrics() {
        let engine = bundled_engine();
        let measurements = build_measurements(&engine, justification_text_style(), &PARAGRAPHS);
        let inner_width = DEFAULT_COLUMN_WIDTH - COLUMN_PAD * 2.0;

        let greedy = measurements
            .hyphenated_paragraphs
            .iter()
            .map(|paragraph| {
                greedy_layout(
                    &paragraph.segments,
                    inner_width,
                    measurements.hyphen_width,
                    &measurements.hyphen_glyph_runs,
                )
            })
            .collect::<Vec<_>>();
        let optimal = measurements
            .hyphenated_paragraphs
            .iter()
            .map(|paragraph| {
                optimal_layout(
                    &paragraph.segments,
                    inner_width,
                    measurements.normal_space_width,
                    measurements.hyphen_width,
                    &measurements.hyphen_glyph_runs,
                )
            })
            .collect::<Vec<_>>();

        let greedy_metrics = compute_metrics(&greedy, measurements.normal_space_width);
        let optimal_metrics = compute_metrics(&optimal, measurements.normal_space_width);

        assert!(optimal_metrics.avg_deviation <= greedy_metrics.avg_deviation + 0.0001);
        assert!(optimal_metrics.river_count <= greedy_metrics.river_count);
    }

    #[test]
    fn justification_render_uses_atlas_without_shaped_text_textures() {
        let ctx = egui::Context::default();
        let engine = bundled_engine();
        let mut assets = EguiPretextRenderer::default();
        let mut demo = JustificationAlgorithmsDemo::default();
        demo.set_open(true);

        let _ = ctx.run_ui(egui::RawInput::default(), |ctx| {
            demo.show(ctx, &engine, &mut assets);
        });
        let stats = assets.stats();

        assert!(stats.atlas_entries > 0);
        assert_eq!(stats.shaped_text_textures, 0);
    }
}
