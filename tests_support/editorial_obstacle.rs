use pretext::{
    advanced::LayoutCursor, ParagraphDirection, PretextEngine,
    PretextParagraphOptions as PrepareOptions,
    PretextPreparedParagraph as PreparedTextWithSegments, PretextStyle as TextStyleSpec,
    WhiteSpaceMode,
};
use serde::{Deserialize, Serialize};

const BODY_TEXT: &str = "Browsers still treat text as something you ask the DOM about after the fact. \
If a layout needs the real height of a paragraph, it has to trigger measurement, and measurement is usually coupled to reflow. \
That is fine for a single article paragraph and disastrous for interactive systems that need to measure hundreds of blocks before deciding where anything goes. \
\n\n\
The argument for a dedicated text engine is not aesthetic first. It is computational. \
Once shaping and line breaking are prepared, the remaining work should be arithmetic: advance widths, break opportunities, obstacle intervals, and cursor movement. \
The cost should scale with the text you are laying out, not with every unrelated element on the page. \
\n\n\
Editorial composition exposes the problem immediately. \
As soon as a drop cap, a pull quote, or an animated object intrudes on the column, CSS either gives up or asks the browser to do expensive layout work you cannot directly control. \
The geometry becomes opaque precisely when you need it most. \
\n\n\
This demo keeps the text stream continuous across three columns. \
The first column owns the drop cap, the final column reserves space for a pull quote, and translucent orbs carve circular exclusion bands through whichever column they overlap. \
Each column resumes from the cursor returned by the previous one, so the article remains a single logical stream even though the visual layout is fragmented. \
\n\n\
The key engineering detail is that animation and reflow are decoupled. \
The orbs drift every frame, but a full text reflow only happens when an orb's coarse vertical band signature changes or when the window geometry changes. \
That keeps the page responsive without pretending the text can ignore moving obstacles forever. \
\n\n\
The point is not to mimic a print magazine down to the last pixel. \
It is to prove that stable, explicit line geometry makes layouts possible that the browser's default text stack still treats as exotic. \
Once you can compute line positions yourself, text stops being a rigid rectangle and starts behaving like a real visual material.";
const PULL_QUOTE: &str =
    "\"Animation and reflow should be coupled by geometry, not by incidental DOM measurement.\"";
const BODY_LINE_HEIGHT: f32 = 28.0;
const QUOTE_LINE_HEIGHT: f32 = 24.0;
const DROP_CAP_LINES: usize = 3;
const COLUMN_GAP: f32 = 26.0;
const MARGIN: f32 = 26.0;
const BOTTOM_GAP: f32 = 18.0;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct EditorialGolden {
    pub body_lines: Vec<GoldenLine>,
    pub pull_quote_lines: Vec<GoldenLine>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct GoldenLine {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub text: String,
}

#[derive(Clone, Copy)]
struct Point {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy)]
struct GeoRect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

#[derive(Clone, Copy)]
struct Interval {
    left: f32,
    right: f32,
}

#[derive(Clone, Copy)]
struct Orb {
    x: f32,
    y: f32,
    radius: f32,
}

#[derive(Clone, Copy)]
struct CircleObstacle {
    center: Point,
    radius: f32,
    horizontal_padding: f32,
    vertical_padding: f32,
}

#[derive(Clone, Copy)]
struct EditorialPageLayout {
    body_columns: [GeoRect; 3],
    pull_quote_rect: GeoRect,
    drop_cap_rect: GeoRect,
}

#[derive(Clone)]
struct PositionedLine {
    x: f32,
    y: f32,
    width: f32,
    text: String,
}

impl GeoRect {
    fn right(self) -> f32 {
        self.x + self.width
    }

    fn bottom(self) -> f32 {
        self.y + self.height
    }
}

pub fn compute_editorial_golden(engine: &PretextEngine) -> EditorialGolden {
    let body = engine.prepare_paragraph(&BODY_TEXT[1..], &body_style(), &normal_options());
    let quote = engine.prepare_paragraph(PULL_QUOTE, &quote_style(), &normal_options());
    let page = GeoRect {
        x: 0.0,
        y: 0.0,
        width: 980.0,
        height: 700.0,
    };
    let layout = build_editorial_page_layout(page);
    let circle_obstacles: Vec<CircleObstacle> = initial_orbs()
        .into_iter()
        .map(|orb| CircleObstacle {
            center: Point { x: orb.x, y: orb.y },
            radius: orb.radius,
            horizontal_padding: BODY_LINE_HEIGHT * 0.72,
            vertical_padding: BODY_LINE_HEIGHT * 0.12,
        })
        .collect();
    let quote_layout =
        engine.layout_paragraph(&quote, layout.pull_quote_rect.width, QUOTE_LINE_HEIGHT);
    let pull_quote_lines = quote_layout
        .lines
        .into_iter()
        .enumerate()
        .map(|(index, line)| PositionedLine {
            x: layout.pull_quote_rect.x,
            y: layout.pull_quote_rect.y + index as f32 * QUOTE_LINE_HEIGHT,
            width: line.line.width,
            text: line.line.text,
        })
        .collect::<Vec<_>>();
    let (mut col1, cursor1) = layout_column(
        engine,
        &body,
        LayoutCursor::default(),
        layout.body_columns[0],
        BODY_LINE_HEIGHT,
        &circle_obstacles,
        &[layout.drop_cap_rect],
    );
    let (col2, cursor2) = layout_column(
        engine,
        &body,
        cursor1,
        layout.body_columns[1],
        BODY_LINE_HEIGHT,
        &circle_obstacles,
        &[],
    );
    let (col3, _) = layout_column(
        engine,
        &body,
        cursor2,
        layout.body_columns[2],
        BODY_LINE_HEIGHT,
        &circle_obstacles,
        &[layout.pull_quote_rect],
    );
    let mut body_lines = Vec::new();
    body_lines.append(&mut col1);
    body_lines.extend(col2);
    body_lines.extend(col3);

    EditorialGolden {
        body_lines: body_lines.into_iter().map(to_golden_line).collect(),
        pull_quote_lines: pull_quote_lines.into_iter().map(to_golden_line).collect(),
    }
}

fn normal_options() -> PrepareOptions {
    PrepareOptions {
        white_space: WhiteSpaceMode::Normal,
        word_break: pretext::WordBreakMode::Normal,
        paragraph_direction: ParagraphDirection::Auto,
    }
}

fn body_style() -> TextStyleSpec {
    TextStyleSpec {
        families: vec![
            "Noto Sans".to_owned(),
            "Arial".to_owned(),
            "Helvetica".to_owned(),
        ],
        size_px: 18.0,
        weight: 400,
        italic: false,
    }
}

fn quote_style() -> TextStyleSpec {
    TextStyleSpec {
        families: vec![
            "Noto Sans".to_owned(),
            "Arial".to_owned(),
            "Helvetica".to_owned(),
        ],
        size_px: 19.0,
        weight: 500,
        italic: true,
    }
}

fn initial_orbs() -> [Orb; 3] {
    [
        Orb {
            x: 260.0,
            y: 190.0,
            radius: 58.0,
        },
        Orb {
            x: 590.0,
            y: 320.0,
            radius: 52.0,
        },
        Orb {
            x: 880.0,
            y: 230.0,
            radius: 48.0,
        },
    ]
}

fn build_editorial_page_layout(page: GeoRect) -> EditorialPageLayout {
    let headline_line_height = 38.0;
    let headline_height = headline_line_height * 2.0;
    let body_top = page.y + MARGIN + headline_height + 34.0;
    let col_width = (page.width - MARGIN * 2.0 - COLUMN_GAP * 2.0) / 3.0;

    let col1 = GeoRect {
        x: page.x + MARGIN,
        y: body_top,
        width: col_width,
        height: page.height - (body_top - page.y) - BOTTOM_GAP,
    };
    let col2 = GeoRect {
        x: col1.right() + COLUMN_GAP,
        ..col1
    };
    let col3 = GeoRect {
        x: col2.right() + COLUMN_GAP,
        ..col1
    };

    EditorialPageLayout {
        body_columns: [col1, col2, col3],
        pull_quote_rect: GeoRect {
            x: col3.x + col3.width * 0.08,
            y: body_top + col3.height * 0.18,
            width: col3.width * 0.82,
            height: 110.0,
        },
        drop_cap_rect: GeoRect {
            x: col1.x,
            y: body_top,
            width: 54.0,
            height: BODY_LINE_HEIGHT * DROP_CAP_LINES as f32,
        },
    }
}

fn layout_column(
    engine: &PretextEngine,
    prepared: &PreparedTextWithSegments,
    start: LayoutCursor,
    region: GeoRect,
    line_height: f32,
    circles: &[CircleObstacle],
    rects: &[GeoRect],
) -> (Vec<PositionedLine>, LayoutCursor) {
    let mut cursor = start;
    let mut line_top = region.y;
    let mut lines = Vec::new();

    while line_top + line_height <= region.bottom() {
        let mut blocked = Vec::new();
        for circle in circles {
            if let Some(interval) = circle_interval_for_band(
                circle.center.x,
                circle.center.y,
                circle.radius,
                line_top,
                line_top + line_height,
                circle.horizontal_padding,
                circle.vertical_padding,
            ) {
                blocked.push(interval);
            }
        }
        for rect in rects {
            if line_top + line_height <= rect.y || line_top >= rect.bottom() {
                continue;
            }
            blocked.push(Interval {
                left: rect.x,
                right: rect.right(),
            });
        }

        let slots = carve_text_line_slots(
            Interval {
                left: region.x,
                right: region.right(),
            },
            &blocked,
        );
        if slots.is_empty() {
            line_top += line_height;
            continue;
        }

        for slot in slots {
            let mut next_cursor = cursor;
            let Some(line) = engine.layout_next_line(
                prepared,
                &mut next_cursor,
                (slot.right - slot.left).max(1.0),
            ) else {
                return (lines, cursor);
            };
            if next_cursor == cursor {
                return (lines, cursor);
            }
            lines.push(PositionedLine {
                x: slot.left.round(),
                y: line_top.round(),
                width: line.width,
                text: line.text,
            });
            cursor = next_cursor;
        }
        line_top += line_height;
    }

    (lines, cursor)
}

fn circle_interval_for_band(
    cx: f32,
    cy: f32,
    radius: f32,
    band_top: f32,
    band_bottom: f32,
    horizontal_padding: f32,
    vertical_padding: f32,
) -> Option<Interval> {
    let top = band_top - vertical_padding;
    let bottom = band_bottom + vertical_padding;
    if top >= cy + radius || bottom <= cy - radius {
        return None;
    }

    let min_dy = if (top..=bottom).contains(&cy) {
        0.0
    } else if cy < top {
        top - cy
    } else {
        cy - bottom
    };
    if min_dy >= radius {
        return None;
    }

    let max_dx = (radius * radius - min_dy * min_dy).sqrt();
    Some(Interval {
        left: cx - max_dx - horizontal_padding,
        right: cx + max_dx + horizontal_padding,
    })
}

fn carve_text_line_slots(base: Interval, blocked: &[Interval]) -> Vec<Interval> {
    let mut slots = vec![base];

    for interval in blocked {
        let mut next = Vec::new();
        for slot in slots {
            if interval.right <= slot.left || interval.left >= slot.right {
                next.push(slot);
                continue;
            }
            if interval.left > slot.left {
                next.push(Interval {
                    left: slot.left,
                    right: interval.left,
                });
            }
            if interval.right < slot.right {
                next.push(Interval {
                    left: interval.right,
                    right: slot.right,
                });
            }
        }
        slots = next;
    }

    slots
        .into_iter()
        .filter(|slot| slot.right - slot.left >= 24.0)
        .collect()
}

fn to_golden_line(line: PositionedLine) -> GoldenLine {
    GoldenLine {
        x: round3(line.x),
        y: round3(line.y),
        width: round3(line.width),
        text: line.text,
    }
}

fn round3(value: f32) -> f32 {
    (value * 1000.0).round() / 1000.0
}
