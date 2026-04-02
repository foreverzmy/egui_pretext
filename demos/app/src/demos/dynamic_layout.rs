use std::time::Duration;

use eframe::egui;
use egui::epaint::{Mesh, Vertex};
use egui::{
    Align2, Color32, CornerRadius, FontFamily, FontId, Rect, Sense, Shape, Stroke, StrokeKind,
};
#[cfg(test)]
use pretext::BidiDirection;
use pretext::{
    LayoutCursor, LayoutLineGlyphRun, LayoutLineVisualRun, PrepareOptions,
    PreparedTextWithSegments, PretextEngine,
    WhiteSpaceMode,
};
use pretext_egui::{AssetRegistry, SvgAssetId};

use crate::demos::text_runs::paint_glyph_runs;
use crate::demos::DemoWindow;
use crate::geometry::{
    carve_text_line_slots, get_polygon_interval_for_band, get_rect_intervals_for_band,
    is_point_in_polygon, svg_alpha_hull, transform_points, Interval, Point, Rect as GeoRect,
};

const HEADLINE: &str = "SITUATIONAL AWARENESS: THE DECADE AHEAD";
const CREDIT_TEXT: &str = "LEOPOLD ASCHENBRENNER";
const HINT_TEXT: &str =
    "Everything laid out in Rust. Resize horizontally and vertically, then click the logos.";
const BODY_COPY: &str = r#"You can see the future first in San Francisco. Over the past year, the
talk of the town has shifted from $10 billion compute clusters to $100 billion clusters to
trillion-dollar clusters. Every six months another zero is added to the boardroom plans. Behind
the scenes, there's a fierce scramble to secure every power contract still available for the rest
of the decade, every voltage transformer that can possibly be procured. American big business is
gearing up to pour trillions of dollars into a long-unseen mobilization of American industrial
might. By the end of the decade, American electricity production will have grown tens of percent;
from the shale fields of Pennsylvania to the solar farms of Nevada, hundreds of millions of GPUs
will hum. The AGI race has begun. We are building machines that can think and reason. By 2025 and
2026, these machines will outpace college graduates. By the end of the decade, they will be
smarter than you or I; we will have superintelligence, in the true sense of the word. Along the
way, national security forces not seen in half a century will be unleashed, and before long, The
Project will be on. If we're lucky, we'll be in an all-out race with the CCP; if we're unlucky, an
all-out war. Everyone is now talking about AI, but few have the faintest glimmer of what is about
to hit them. Nvidia analysts still think 2024 might be close to the peak. Mainstream pundits are
stuck on the willful blindness of "it's just predicting the next word". They see only hype and
business-as-usual; at most they entertain another internet-scale technological change. Before long,
the world will wake up. But right now, there are perhaps a few hundred people, most of them in San
Francisco and the AI labs, that have situational awareness. Through whatever peculiar forces of
fate, I have found myself amongst them. A few years ago, these people were derided as crazy, but
they trusted the trendlines, which allowed them to correctly predict the AI advances of the past
few years. Whether these people are also right about the next few years remains to be seen. But
these are very smart people, the smartest people I have ever met, and they are the ones building
this technology. Perhaps they will be an odd footnote in history, or perhaps they will go down in
history like Szilard and Oppenheimer and Teller. If they are seeing the future even close to
correctly, we are in for a wild ride. Let me tell you what we see. We have machines now that we
can basically talk to like humans. It's a remarkable testament to the human capacity to adjust
that this seems normal, that we've become inured to the pace of progress. But it's worth stepping
back and looking at the progress of just the last few years. Let me remind you of how far we came
in just the roughly four years leading up to GPT-4. GPT-2, circa 2019, was like a preschooler:
"Wow, it can string together a few plausible sentences." A very cherry-picked example of a
semi-coherent story about unicorns in the Andes was incredibly impressive at the time. And yet
GPT-2 could barely count to 5 without getting tripped up; when summarizing an article, it just
barely outperformed selecting three random sentences from the article. GPT-3, circa 2020, was like
an elementary schooler: "Wow, with just some few-shot examples it can do some simple useful
tasks." It started being cohesive over even multiple paragraphs much more consistently, and could
correct grammar and do some very basic arithmetic. For the first time, it was also commercially
useful in a few narrow ways: for example, GPT-3 could generate simple copy for SEO and marketing.
GPT-4, circa 2023, was like a smart high schooler: "Wow, it can write pretty sophisticated code
and iteratively debug, it can write intelligently and sophisticatedly about complicated subjects,
it can reason through difficult high-school competition math, it's beating the vast majority of
high schoolers on whatever tests we can give it, etc." From code to math to Fermi estimates, it
can think and reason. GPT-4 is now useful in my daily tasks, from helping write code to revising
drafts. On everything from AP exams to the SAT, GPT-4 scores better than the vast majority of high
schoolers. Of course, even GPT-4 is still somewhat uneven; for some tasks it's much better than
smart high-schoolers, while there are other tasks it can't yet do. That said, I tend to think most
of these limitations come down to obvious ways models are still hobbled, as I'll discuss in depth
later. The raw intelligence is mostly there, even if the models are still artificially constrained;
it'll take extra work to unlock models being able to fully apply that raw intelligence across
applications. The pace of deep learning progress in the last decade has simply been extraordinary.
A mere decade ago it was revolutionary for a deep learning system to identify simple images. Today,
we keep trying to come up with novel, ever harder tests, and yet each new benchmark is quickly
cracked. It used to take decades to crack widely-used benchmarks; now it feels like mere months.
We're literally running out of benchmarks. Over and over again, year after year, skeptics have
claimed "deep learning won't be able to do X" and have been quickly proven wrong. If there's one
lesson we've learned from the past decade of AI, it's that you should never bet against deep
learning. How did this happen? The magic of deep learning is that it just works, and the trendlines
have been astonishingly consistent, despite naysayers at every turn. With each order of magnitude
of effective compute, models predictably, reliably get better. If we can count the orders of
magnitude, we can roughly, qualitatively extrapolate capability improvements. That's how a few
prescient individuals saw GPT-4 coming. We can decompose the progress in the four years from GPT-2
to GPT-4 into three categories of scaleups: compute, algorithmic efficiencies, and "unhobbling"
gains. We can count the orders of magnitude of improvement along these axes, trace the scaleup for
each in units of effective compute, and look at what we should expect on top of GPT-4 from 2023 to
2027. I'll go through each one one by one, but the upshot is clear: we are rapidly racing through
the orders of magnitude. There are potential headwinds in the data wall, which I'll address, but
overall, it seems likely that we should expect another GPT-2-to-GPT-4-sized jump, on top of GPT-4,
by 2027. GPT-4's capabilities came as a shock to many: an AI system that could write code and
essays, could reason through difficult math problems, and ace college exams. A few years ago, most
thought these were impenetrable walls. But GPT-4 was merely the continuation of a decade of
breakneck progress in deep learning. A decade earlier, models could barely identify simple images
of cats and dogs; four years earlier, GPT-2 could barely string together semi-plausible sentences.
Now we are rapidly saturating all the benchmarks we can come up with. And yet this dramatic
progress has merely been the result of consistent trends in scaling up deep learning. There have
been people who have seen this for far longer. They were scoffed at, but all they did was trust
the trendlines. The trendlines are intense, and they were right. The models just want to learn;
you scale them up, and they learn more. I make the following claim: it is strikingly plausible that
by 2027, models will be able to do the work of an AI researcher or engineer. That doesn't require
believing in science fiction; it just requires believing in straight lines on a graph. The upshot
is pretty simple. GPT-2 to GPT-4, from models that were impressive for sometimes managing to string
together a few coherent sentences, to models that ace high-school exams, was not a one-time gain.
We are racing through the orders of magnitude extremely rapidly, and the numbers indicate we should
expect another roughly 100,000 times effective compute scaleup, resulting in another
GPT-2-to-GPT-4-sized qualitative jump, over four years. Moreover, and critically, that doesn't
just mean a better chatbot; picking the many obvious low-hanging fruit on "unhobbling" gains
should take us from chatbots to agents, from a tool to something that looks more like drop-in
remote worker replacements. While the inference is simple, the implication is striking. Another
jump like that very well could take us to AGI, to models as smart as PhDs or experts that can work
beside us as coworkers. Perhaps most importantly, if these AI systems could automate AI research
itself, that would set in motion intense feedback loops. Even now, barely anyone is pricing all
this in. But situational awareness on AI isn't actually that hard, once you step back and look at
the trends. If you keep being surprised by AI capabilities, just start counting the orders of
magnitude. Finally, the hardest to quantify, but no less important, category of improvements: what
I'll call "unhobbling". Imagine if when asked to solve a hard math problem, you had to instantly
answer with the very first thing that came to mind. It seems obvious that you would have a hard
time, except for the simplest problems. But until recently, that's how we had LLMs solve math
problems. Instead, most of us work through the problem step by step on a scratchpad, and are able
to solve much more difficult problems that way. Chain-of-thought prompting unlocked that for LLMs.
Despite excellent raw capabilities, they were much worse at math than they could be because they
were hobbled in an obvious way, and it took a small algorithmic tweak to unlock much greater
capabilities. We've made huge strides in unhobbling models over the past few years. These are
algorithmic improvements beyond just training better base models, and often only use a fraction of
pretraining compute, that unleash model capabilities. Reinforcement learning from human feedback:
base models have incredible latent capabilities, but they're raw and incredibly hard to work with."#;
const BODY_LINE_HEIGHT: f32 = 32.0;
const CREDIT_LINE_HEIGHT: f32 = 16.0;
const MIN_PAGE_HEIGHT: f32 = 520.0;
const LOGO_RASTER_SIZE: [usize; 2] = [512, 512];
const FRAME_INTERVAL: Duration = Duration::from_millis(16);
const HINT_PILL_SAFE_TOP: f32 = 72.0;
const NARROW_BREAKPOINT: f32 = 760.0;
const NARROW_COLUMN_MAX_WIDTH: f32 = 430.0;
const UNBOUNDED_WIDTH: f32 = 100_000.0;

pub struct DynamicLayoutDemo {
    open: bool,
    openai_logo: LogoAnimationState,
    claude_logo: LogoAnimationState,
    body_prepared: Option<PreparedTextWithSegments>,
    credit_prepared: Option<PreparedTextWithSegments>,
    hulls: Option<LogoHulls>,
}

#[derive(Clone)]
struct LogoHulls {
    openai: Vec<Point>,
    claude: Vec<Point>,
}

#[derive(Clone, Copy)]
struct SpinState {
    from: f32,
    to: f32,
    start: f64,
    duration: f64,
}

#[derive(Clone, Copy, Default)]
struct LogoAnimationState {
    angle: f32,
    spin: Option<SpinState>,
}

#[derive(Clone)]
struct DynamicProjection {
    headline_lines: Vec<PositionedLine>,
    credit_line: Option<PositionedLine>,
    left_lines: Vec<PositionedLine>,
    right_lines: Vec<PositionedLine>,
}

#[derive(Clone, Debug, PartialEq)]
struct PositionedLine {
    x: f32,
    y: f32,
    width: f32,
    text: String,
    visual_runs: Vec<LayoutLineVisualRun>,
    glyph_runs: Vec<LayoutLineGlyphRun>,
}

#[derive(Clone, Copy)]
struct PageLayout {
    page: GeoRect,
    is_narrow: bool,
    gutter: f32,
    center_gap: f32,
    column_width: f32,
    headline_region: GeoRect,
    headline_size: f32,
    headline_line_height: f32,
    credit_gap: f32,
    copy_gap: f32,
    openai_rect: GeoRect,
    claude_rect: GeoRect,
}

#[derive(Clone, Copy)]
enum BandObstacle<'a> {
    Polygon {
        points: &'a [Point],
        horizontal_padding: f32,
        vertical_padding: f32,
    },
    Rects {
        rects: &'a [GeoRect],
        horizontal_padding: f32,
        vertical_padding: f32,
    },
}

#[derive(Clone, Copy)]
enum ColumnSide {
    Left,
    Right,
}

impl Default for DynamicLayoutDemo {
    fn default() -> Self {
        Self {
            open: false,
            openai_logo: LogoAnimationState::default(),
            claude_logo: LogoAnimationState::default(),
            body_prepared: None,
            credit_prepared: None,
            hulls: None,
        }
    }
}

impl DemoWindow for DynamicLayoutDemo {
    fn title(&self) -> &str {
        "Dynamic Layout"
    }

    fn is_open(&self) -> bool {
        self.open
    }

    fn set_open(&mut self, open: bool) {
        self.open = open;
    }

    fn show(&mut self, ctx: &egui::Context, engine: &PretextEngine, assets: &mut AssetRegistry) {
        let mut open = self.open;
        egui::Window::new(self.title())
            .open(&mut open)
            .resizable(true)
            .default_size(egui::vec2(1120.0, 780.0))
            .show(ctx, |ui| {
                let now = ctx.input(|input| input.time);
                let animating = update_spin_state(&mut self.openai_logo, now)
                    | update_spin_state(&mut self.claude_logo, now);

                let available = ui.available_size();
                let page_width = available.x.max(360.0);
                let page_height = available.y.max(MIN_PAGE_HEIGHT);
                let (page_rect, _) =
                    ui.allocate_exact_size(egui::vec2(page_width, page_height), Sense::hover());

                let body_prepared = self.ensure_body_prepared(engine).clone();
                let credit_prepared = self.ensure_credit_prepared(engine).clone();
                let hulls = self.ensure_hulls().clone();
                let layout = build_page_layout(page_rect, engine);

                let openai_poly =
                    transform_points(&hulls.openai, layout.openai_rect, self.openai_logo.angle);
                let claude_poly =
                    transform_points(&hulls.claude, layout.claude_rect, self.claude_logo.angle);
                let projection = evaluate_layout(
                    engine,
                    &body_prepared,
                    &credit_prepared,
                    layout,
                    &openai_poly,
                    &claude_poly,
                );

                let painter = ui.painter().clone();
                paint_page_background(&painter, page_rect);

                paint_positioned_lines(
                    &painter,
                    &projection.headline_lines,
                    &headline_style(layout.headline_size),
                    layout.headline_line_height,
                    headline_render_slack_y(layout.headline_size, layout.headline_line_height),
                    Color32::from_rgb(17, 16, 13),
                    ctx,
                    engine,
                    assets,
                );
                if let Some(credit_line) = &projection.credit_line {
                    paint_positioned_lines(
                        &painter,
                        std::slice::from_ref(credit_line),
                        &credit_style(),
                        CREDIT_LINE_HEIGHT,
                        2.0,
                        Color32::from_rgba_premultiplied(17, 16, 13, 148),
                        ctx,
                        engine,
                        assets,
                    );
                }
                paint_positioned_lines(
                    &painter,
                    &projection.left_lines,
                    &body_style(),
                    BODY_LINE_HEIGHT,
                    2.0,
                    Color32::from_rgb(17, 16, 13),
                    ctx,
                    engine,
                    assets,
                );
                paint_positioned_lines(
                    &painter,
                    &projection.right_lines,
                    &body_style(),
                    BODY_LINE_HEIGHT,
                    2.0,
                    Color32::from_rgb(17, 16, 13),
                    ctx,
                    engine,
                    assets,
                );

                paint_logo_shadow(
                    &painter,
                    layout.openai_rect,
                    egui::vec2(0.0, layout.openai_rect.height * 0.12),
                    Color32::from_rgba_premultiplied(16, 16, 12, 34),
                );
                paint_logo_shadow(
                    &painter,
                    layout.claude_rect,
                    egui::vec2(0.0, layout.claude_rect.height * 0.1),
                    Color32::from_rgba_premultiplied(140, 86, 52, 42),
                );

                let openai_texture =
                    assets.bundled_svg_texture(SvgAssetId::OpenAiLogo, LOGO_RASTER_SIZE, ctx);
                let claude_texture =
                    assets.bundled_svg_texture(SvgAssetId::ClaudeLogo, LOGO_RASTER_SIZE, ctx);
                paint_rotated_texture(
                    &painter,
                    layout.openai_rect,
                    self.openai_logo.angle,
                    &openai_texture,
                );
                paint_rotated_texture(
                    &painter,
                    layout.claude_rect,
                    self.claude_logo.angle,
                    &claude_texture,
                );
                paint_logo_hint(&painter, page_rect, layout.is_narrow);

                handle_logo_interaction(
                    ui,
                    now,
                    "openai-logo",
                    layout.openai_rect,
                    &openai_poly,
                    &mut self.openai_logo,
                    -1.0,
                );
                handle_logo_interaction(
                    ui,
                    now,
                    "claude-logo",
                    layout.claude_rect,
                    &claude_poly,
                    &mut self.claude_logo,
                    1.0,
                );

                if animating {
                    ctx.request_repaint_after(FRAME_INTERVAL);
                }
            });
        self.open = open;
    }
}

impl DynamicLayoutDemo {
    fn ensure_body_prepared(&mut self, engine: &PretextEngine) -> &PreparedTextWithSegments {
        if self.body_prepared.is_none() {
            self.body_prepared =
                Some(engine.prepare_with_segments(BODY_COPY, &body_style(), &normal_options()));
        }
        self.body_prepared
            .as_ref()
            .expect("dynamic body should be prepared")
    }

    fn ensure_credit_prepared(&mut self, engine: &PretextEngine) -> &PreparedTextWithSegments {
        if self.credit_prepared.is_none() {
            self.credit_prepared =
                Some(engine.prepare_with_segments(CREDIT_TEXT, &credit_style(), &normal_options()));
        }
        self.credit_prepared
            .as_ref()
            .expect("dynamic credit should be prepared")
    }

    fn ensure_hulls(&mut self) -> &LogoHulls {
        if self.hulls.is_none() {
            let openai = svg_alpha_hull(
                AssetRegistry::svg_bytes(SvgAssetId::OpenAiLogo),
                LOGO_RASTER_SIZE,
            )
            .expect("openai hull");
            let claude = svg_alpha_hull(
                AssetRegistry::svg_bytes(SvgAssetId::ClaudeLogo),
                LOGO_RASTER_SIZE,
            )
            .expect("claude hull");
            self.hulls = Some(LogoHulls { openai, claude });
        }
        self.hulls.as_ref().expect("dynamic hulls should exist")
    }
}

fn normal_options() -> PrepareOptions {
    PrepareOptions {
        white_space: WhiteSpaceMode::Normal,
        paragraph_direction: pretext::ParagraphDirection::Auto,
    }
}

fn serif_families() -> Vec<String> {
    vec![
        "Iowan Old Style".to_owned(),
        "Palatino Linotype".to_owned(),
        "Book Antiqua".to_owned(),
        "Palatino".to_owned(),
        "Georgia".to_owned(),
        "Times New Roman".to_owned(),
        "Noto Serif".to_owned(),
        "Noto Sans".to_owned(),
    ]
}

fn sans_families() -> Vec<String> {
    vec![
        "Helvetica Neue".to_owned(),
        "Helvetica".to_owned(),
        "Arial".to_owned(),
        "Noto Sans".to_owned(),
    ]
}

fn body_style() -> pretext::TextStyleSpec {
    pretext::TextStyleSpec {
        families: serif_families(),
        size_px: 20.0,
        weight: 450,
        italic: false,
    }
}

fn headline_style(size_px: f32) -> pretext::TextStyleSpec {
    pretext::TextStyleSpec {
        families: serif_families(),
        size_px,
        weight: 700,
        italic: false,
    }
}

fn credit_style() -> pretext::TextStyleSpec {
    pretext::TextStyleSpec {
        families: sans_families(),
        size_px: 12.0,
        weight: 500,
        italic: false,
    }
}

fn measure_single_line_width(engine: &PretextEngine, prepared: &PreparedTextWithSegments) -> f32 {
    let mut max_width = 0.0f32;
    engine.walk_line_ranges(prepared, UNBOUNDED_WIDTH, |line| {
        max_width = max_width.max(line.width);
    });
    max_width
}

fn headline_breaks_inside_word(
    engine: &PretextEngine,
    prepared: &PreparedTextWithSegments,
    max_width: f32,
) -> bool {
    let mut breaks_inside_word = false;
    engine.walk_line_ranges(prepared, max_width, |line| {
        if line.end.grapheme_index != 0 {
            breaks_inside_word = true;
        }
    });
    breaks_inside_word
}

fn fit_headline_font_size(engine: &PretextEngine, headline_width: f32, page_width: f32) -> f32 {
    let mut low = (22.0f32.max(page_width * 0.026)).ceil() as i32;
    let mut high = (94.4f32.min(55.2f32.max(page_width * 0.055))).floor() as i32;
    let mut best = low;

    while low <= high {
        let size = (low + high) / 2;
        let prepared =
            engine.prepare_with_segments(HEADLINE, &headline_style(size as f32), &normal_options());
        if !headline_breaks_inside_word(engine, &prepared, headline_width) {
            best = size;
            low = size + 1;
        } else {
            high = size - 1;
        }
    }

    best as f32
}

fn build_page_layout(page_rect: Rect, engine: &PretextEngine) -> PageLayout {
    let page = GeoRect {
        x: page_rect.left(),
        y: page_rect.top(),
        width: page_rect.width(),
        height: page_rect.height(),
    };
    let is_narrow = page.width < NARROW_BREAKPOINT;

    if is_narrow {
        let gutter = (page.width * 0.06).clamp(18.0, 28.0).round();
        let column_width = (page.width - gutter * 2.0)
            .min(NARROW_COLUMN_MAX_WIDTH)
            .round();
        let headline_top = 28.0;
        let headline_width = (page.width - gutter * 2.0).round();
        let headline_size = fit_headline_font_size(engine, headline_width, page.width).min(48.0);
        let headline_line_height = (headline_size * 0.92).round();
        let credit_gap = (12.0f32.max(BODY_LINE_HEIGHT * 0.5)).round();
        let copy_gap = (18.0f32.max(BODY_LINE_HEIGHT * 0.7)).round();
        let claude_size = (page.width * 0.23)
            .min(page.height * 0.11)
            .min(92.0)
            .round();
        let openai_size = (page.width * 0.34).min(138.0).round();

        return PageLayout {
            page,
            is_narrow,
            gutter,
            center_gap: 0.0,
            column_width,
            headline_region: GeoRect {
                x: page.x + gutter,
                y: page.y + headline_top,
                width: headline_width,
                height: (page.height - headline_top - gutter).max(320.0),
            },
            headline_size,
            headline_line_height,
            credit_gap,
            copy_gap,
            openai_rect: GeoRect {
                x: page.x + gutter - (openai_size * 0.22).round(),
                y: page.y + page.height - gutter - openai_size + (openai_size * 0.08).round(),
                width: openai_size,
                height: openai_size,
            },
            claude_rect: GeoRect {
                x: page.x + page.width - gutter - (claude_size * 0.88).round(),
                y: page.y + 4.0,
                width: claude_size,
                height: claude_size,
            },
        };
    }

    let gutter = (52.0f32.max(page.width * 0.048)).round();
    let center_gap = (28.0f32.max(page.width * 0.025)).round();
    let column_width = ((page.width - gutter * 2.0 - center_gap) * 0.5).round();
    let headline_top = 42.0f32
        .max(page.width * 0.04)
        .max(HINT_PILL_SAFE_TOP)
        .round();
    let headline_width = (page.width - gutter * 2.0)
        .min(column_width.max(page.width * 0.5))
        .round();
    let headline_size = fit_headline_font_size(engine, headline_width, page.width);
    let headline_line_height = (headline_size * 0.92).round();
    let credit_gap = (14.0f32.max(BODY_LINE_HEIGHT * 0.6)).round();
    let copy_gap = (20.0f32.max(BODY_LINE_HEIGHT * 0.9)).round();
    let openai_shrink_t = ((960.0 - page.width) / 260.0).clamp(0.0, 1.0);
    let openai_size = (400.0 - openai_shrink_t * 56.0)
        .min(page.height * 0.43)
        .round();
    let claude_size = 276.0f32
        .max((page.width * 0.355).min(page.height * 0.45).min(500.0))
        .round();

    PageLayout {
        page,
        is_narrow,
        gutter,
        center_gap,
        column_width,
        headline_region: GeoRect {
            x: page.x + gutter,
            y: page.y + headline_top,
            width: headline_width,
            height: page.height - headline_top - gutter,
        },
        headline_size,
        headline_line_height,
        credit_gap,
        copy_gap,
        openai_rect: GeoRect {
            x: page.x + gutter - (openai_size * 0.3).round(),
            y: page.y + page.height - gutter - openai_size + (openai_size * 0.2).round(),
            width: openai_size,
            height: openai_size,
        },
        claude_rect: GeoRect {
            x: page.x + page.width - (claude_size * 0.69).round(),
            y: page.y - (claude_size * 0.22).round(),
            width: claude_size,
            height: claude_size,
        },
    }
}

fn get_obstacle_intervals(
    obstacle: &BandObstacle<'_>,
    band_top: f32,
    band_bottom: f32,
) -> Vec<Interval> {
    match *obstacle {
        BandObstacle::Polygon {
            points,
            horizontal_padding,
            vertical_padding,
        } => get_polygon_interval_for_band(
            points,
            band_top,
            band_bottom,
            horizontal_padding,
            vertical_padding,
        )
        .into_iter()
        .collect(),
        BandObstacle::Rects {
            rects,
            horizontal_padding,
            vertical_padding,
        } => get_rect_intervals_for_band(
            rects,
            band_top,
            band_bottom,
            horizontal_padding,
            vertical_padding,
        ),
    }
}

fn choose_slot(slots: &[Interval], side: ColumnSide) -> Interval {
    let mut best = slots[0];
    for &candidate in &slots[1..] {
        let best_width = best.right - best.left;
        let candidate_width = candidate.right - candidate.left;
        if candidate_width > best_width {
            best = candidate;
            continue;
        }
        if candidate_width < best_width {
            continue;
        }
        match side {
            ColumnSide::Left if candidate.left > best.left => best = candidate,
            ColumnSide::Right if candidate.left < best.left => best = candidate,
            _ => {}
        }
    }
    best
}

fn layout_column(
    engine: &PretextEngine,
    prepared: &PreparedTextWithSegments,
    start_cursor: LayoutCursor,
    region: GeoRect,
    line_height: f32,
    obstacles: &[BandObstacle<'_>],
    side: ColumnSide,
) -> (Vec<PositionedLine>, LayoutCursor) {
    let mut cursor = start_cursor;
    let mut line_top = region.y;
    let mut lines = Vec::new();

    while line_top + line_height <= region.bottom() {
        let band_top = line_top;
        let band_bottom = line_top + line_height;
        let mut blocked = Vec::new();
        for obstacle in obstacles {
            blocked.extend(get_obstacle_intervals(obstacle, band_top, band_bottom));
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

        let slot = choose_slot(&slots, side);
        let mut next_cursor = cursor;
        let Some(line) = engine.layout_next_line(
            prepared,
            &mut next_cursor,
            (slot.right - slot.left).max(1.0),
        ) else {
            break;
        };
        if next_cursor == cursor {
            break;
        }

        let visual_runs = engine.line_visual_runs(prepared, &line);
        let glyph_runs = engine.line_glyph_runs(prepared, &line);
        lines.push(PositionedLine {
            x: slot.left.round(),
            y: line_top.round(),
            width: line.width,
            text: line.text,
            visual_runs,
            glyph_runs,
        });
        cursor = next_cursor;
        line_top += line_height;
    }

    (lines, cursor)
}

fn positioned_line_rects(lines: &[PositionedLine], line_height: f32) -> Vec<GeoRect> {
    lines
        .iter()
        .map(|line| GeoRect {
            x: line.x,
            y: line.y,
            width: line.width.ceil(),
            height: line_height,
        })
        .collect()
}

fn evaluate_layout(
    engine: &PretextEngine,
    body_prepared: &PreparedTextWithSegments,
    credit_prepared: &PreparedTextWithSegments,
    layout: PageLayout,
    openai_polygon: &[Point],
    claude_polygon: &[Point],
) -> DynamicProjection {
    let openai_obstacle = BandObstacle::Polygon {
        points: openai_polygon,
        horizontal_padding: (BODY_LINE_HEIGHT * 0.82).round(),
        vertical_padding: (BODY_LINE_HEIGHT * 0.26).round(),
    };
    let claude_obstacle = BandObstacle::Polygon {
        points: claude_polygon,
        horizontal_padding: (BODY_LINE_HEIGHT * 0.28).round(),
        vertical_padding: (BODY_LINE_HEIGHT * 0.12).round(),
    };

    let headline_prepared = engine.prepare_with_segments(
        HEADLINE,
        &headline_style(layout.headline_size),
        &normal_options(),
    );
    let headline_obstacles = [openai_obstacle];
    let (headline_lines, _) = layout_column(
        engine,
        &headline_prepared,
        LayoutCursor::default(),
        layout.headline_region,
        layout.headline_line_height,
        &headline_obstacles,
        ColumnSide::Left,
    );
    let headline_rects = positioned_line_rects(&headline_lines, layout.headline_line_height);
    let headline_bottom = headline_lines
        .iter()
        .map(|line| line.y + layout.headline_line_height)
        .fold(layout.headline_region.y, f32::max);

    let credit_top = headline_bottom + layout.credit_gap;
    let credit_region = GeoRect {
        x: layout.page.x + layout.gutter + 4.0,
        y: credit_top,
        width: layout.headline_region.width,
        height: CREDIT_LINE_HEIGHT,
    };
    let mut credit_blocked =
        get_obstacle_intervals(&openai_obstacle, credit_region.y, credit_region.bottom());
    if layout.is_narrow {
        credit_blocked.extend(get_obstacle_intervals(
            &claude_obstacle,
            credit_region.y,
            credit_region.bottom(),
        ));
    }
    let credit_slots = carve_text_line_slots(
        Interval {
            left: credit_region.x,
            right: credit_region.right(),
        },
        &credit_blocked,
    );
    let credit_width = measure_single_line_width(engine, credit_prepared).ceil();
    let credit_left = credit_slots
        .iter()
        .find(|slot| slot.right - slot.left >= credit_width)
        .map(|slot| slot.left.round())
        .unwrap_or(credit_region.x.round());
    let mut credit_cursor = LayoutCursor::default();
    let credit_line = engine
        .layout_next_line(credit_prepared, &mut credit_cursor, UNBOUNDED_WIDTH)
        .map(|line| {
            let visual_runs = engine.line_visual_runs(credit_prepared, &line);
            let glyph_runs = engine.line_glyph_runs(credit_prepared, &line);
            PositionedLine {
                x: credit_left,
                y: credit_top.round(),
                width: line.width,
                text: line.text,
                visual_runs,
                glyph_runs,
            }
        });

    let copy_top = credit_top + CREDIT_LINE_HEIGHT + layout.copy_gap;
    if layout.is_narrow {
        let body_region = GeoRect {
            x: (layout.page.x + (layout.page.width - layout.column_width) * 0.5).round(),
            y: copy_top,
            width: layout.column_width,
            height: (layout.page.bottom() - copy_top - layout.gutter).max(0.0),
        };
        let body_obstacles = [claude_obstacle, openai_obstacle];
        let (left_lines, _) = layout_column(
            engine,
            body_prepared,
            LayoutCursor::default(),
            body_region,
            BODY_LINE_HEIGHT,
            &body_obstacles,
            ColumnSide::Left,
        );
        return DynamicProjection {
            headline_lines,
            credit_line,
            left_lines,
            right_lines: Vec::new(),
        };
    }

    let left_region = GeoRect {
        x: layout.page.x + layout.gutter,
        y: copy_top,
        width: layout.column_width,
        height: (layout.page.bottom() - copy_top - layout.gutter).max(0.0),
    };
    let right_region = GeoRect {
        x: layout.page.x + layout.gutter + layout.column_width + layout.center_gap,
        y: layout.headline_region.y,
        width: layout.column_width,
        height: (layout.page.bottom() - layout.headline_region.y - layout.gutter).max(0.0),
    };
    let title_obstacle = BandObstacle::Rects {
        rects: &headline_rects,
        horizontal_padding: (BODY_LINE_HEIGHT * 0.95).round(),
        vertical_padding: (BODY_LINE_HEIGHT * 0.3).round(),
    };

    let left_obstacles = [openai_obstacle];
    let (left_lines, cursor) = layout_column(
        engine,
        body_prepared,
        LayoutCursor::default(),
        left_region,
        BODY_LINE_HEIGHT,
        &left_obstacles,
        ColumnSide::Left,
    );
    let right_obstacles = [title_obstacle, claude_obstacle, openai_obstacle];
    let (right_lines, _) = layout_column(
        engine,
        body_prepared,
        cursor,
        right_region,
        BODY_LINE_HEIGHT,
        &right_obstacles,
        ColumnSide::Right,
    );

    DynamicProjection {
        headline_lines,
        credit_line,
        left_lines,
        right_lines,
    }
}

fn paint_page_background(painter: &egui::Painter, page_rect: Rect) {
    painter.rect_filled(
        page_rect,
        CornerRadius::same(22),
        Color32::from_rgb(246, 240, 230),
    );

    let clipped = painter.with_clip_rect(page_rect);
    clipped.circle_filled(
        egui::pos2(
            page_rect.left() + page_rect.width() * 0.16,
            page_rect.top() + page_rect.height() * 0.82,
        ),
        page_rect.width() * 0.34,
        Color32::from_rgba_premultiplied(45, 88, 128, 32),
    );
    clipped.circle_filled(
        egui::pos2(
            page_rect.left() + page_rect.width() * 0.28,
            page_rect.top() + page_rect.height() * 0.64,
        ),
        page_rect.width() * 0.22,
        Color32::from_rgba_premultiplied(57, 78, 124, 16),
    );
    clipped.circle_filled(
        egui::pos2(
            page_rect.right() - page_rect.width() * 0.14,
            page_rect.top() + page_rect.height() * 0.16,
        ),
        page_rect.width() * 0.3,
        Color32::from_rgba_premultiplied(217, 119, 87, 36),
    );
    clipped.add(Shape::convex_polygon(
        vec![
            egui::pos2(page_rect.left() + page_rect.width() * 0.52, page_rect.top()),
            egui::pos2(page_rect.right(), page_rect.top()),
            egui::pos2(
                page_rect.right(),
                page_rect.top() + page_rect.height() * 0.78,
            ),
            egui::pos2(
                page_rect.left() + page_rect.width() * 0.7,
                page_rect.top() + page_rect.height() * 0.42,
            ),
        ],
        Color32::from_rgba_premultiplied(217, 119, 87, 12),
        Stroke::NONE,
    ));

    painter.rect_stroke(
        page_rect,
        CornerRadius::same(22),
        Stroke::new(1.0, Color32::from_rgb(221, 212, 198)),
        StrokeKind::Inside,
    );
}

fn paint_positioned_lines(
    painter: &egui::Painter,
    lines: &[PositionedLine],
    style: &pretext::TextStyleSpec,
    line_height: f32,
    _slack_y: f32,
    color: Color32,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut AssetRegistry,
) {
    for line in lines {
        paint_glyph_runs(
            painter,
            line.x,
            line.y,
            &line.text,
            &line.glyph_runs,
            style,
            line_height,
            color,
            ctx,
            engine,
            assets,
        );
    }
}

fn headline_render_slack_y(font_size: f32, line_height: f32) -> f32 {
    ((font_size - line_height).max(0.0) + 8.0).round()
}

fn paint_logo_shadow(painter: &egui::Painter, rect: GeoRect, offset: egui::Vec2, color: Color32) {
    let center = egui::pos2(rect.x + rect.width * 0.5, rect.y + rect.height * 0.5) + offset;
    painter.circle_filled(center, rect.width.max(rect.height) * 0.26, color);
}

fn paint_logo_hint(painter: &egui::Painter, page_rect: Rect, is_narrow: bool) {
    if is_narrow {
        return;
    }

    let width = (page_rect.width() - 48.0).min(520.0).max(280.0);
    let hint_rect = Rect::from_center_size(
        egui::pos2(page_rect.center().x, page_rect.top() + 22.0),
        egui::vec2(width, 34.0),
    );
    painter.rect_filled(
        hint_rect,
        CornerRadius::same(17),
        Color32::from_rgba_premultiplied(17, 16, 13, 240),
    );
    painter.text(
        hint_rect.center(),
        Align2::CENTER_CENTER,
        HINT_TEXT,
        FontId::new(12.0, FontFamily::Proportional),
        Color32::from_rgba_premultiplied(246, 240, 230, 245),
    );
}

fn paint_rotated_texture(
    painter: &egui::Painter,
    rect: GeoRect,
    angle: f32,
    texture: &egui::TextureHandle,
) {
    let egui_rect = Rect::from_min_size(
        egui::pos2(rect.x, rect.y),
        egui::vec2(rect.width, rect.height),
    );
    if angle.abs() < 0.001 {
        painter.image(
            texture.id(),
            egui_rect,
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        return;
    }

    let center = egui_rect.center();
    let half_w = rect.width * 0.5;
    let half_h = rect.height * 0.5;
    let cos = angle.cos();
    let sin = angle.sin();

    let corners = [
        (-half_w, -half_h),
        (half_w, -half_h),
        (half_w, half_h),
        (-half_w, half_h),
    ];
    let uvs = [
        egui::pos2(0.0, 0.0),
        egui::pos2(1.0, 0.0),
        egui::pos2(1.0, 1.0),
        egui::pos2(0.0, 1.0),
    ];

    let mut mesh = Mesh::with_texture(texture.id());
    for (index, (dx, dy)) in corners.into_iter().enumerate() {
        mesh.vertices.push(Vertex {
            pos: egui::pos2(
                center.x + dx * cos - dy * sin,
                center.y + dx * sin + dy * cos,
            ),
            uv: uvs[index],
            color: Color32::WHITE,
        });
    }
    mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
    painter.add(Shape::mesh(mesh));
}

fn handle_logo_interaction(
    ui: &mut egui::Ui,
    now: f64,
    id: &'static str,
    rect: GeoRect,
    polygon: &[Point],
    logo: &mut LogoAnimationState,
    direction: f32,
) {
    let response = ui.interact(
        Rect::from_min_size(
            egui::pos2(rect.x, rect.y),
            egui::vec2(rect.width, rect.height),
        ),
        ui.id().with(id),
        Sense::click(),
    );
    let pointer_pos = response.interact_pointer_pos();
    let hovered =
        pointer_pos.is_some_and(|pos| is_point_in_polygon(polygon, Point { x: pos.x, y: pos.y }));
    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    if response.clicked() && hovered {
        logo.spin = Some(SpinState {
            from: logo.angle,
            to: logo.angle + direction * core::f32::consts::PI,
            start: now,
            duration: 0.9,
        });
    }
}

fn update_spin_state(logo: &mut LogoAnimationState, now: f64) -> bool {
    let Some(spin) = logo.spin else {
        return false;
    };
    let progress = ((now - spin.start) / spin.duration).clamp(0.0, 1.0) as f32;
    let eased = ease_out_cubic(progress);
    logo.angle = spin.from + (spin.to - spin.from) * eased;
    if progress >= 1.0 {
        logo.spin = None;
        logo.angle = spin.to;
        return false;
    }
    true
}

fn ease_out_cubic(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dynamic_logo_hulls_load_from_svg() {
        let openai = svg_alpha_hull(
            AssetRegistry::svg_bytes(SvgAssetId::OpenAiLogo),
            LOGO_RASTER_SIZE,
        )
        .expect("openai hull");
        let claude = svg_alpha_hull(
            AssetRegistry::svg_bytes(SvgAssetId::ClaudeLogo),
            LOGO_RASTER_SIZE,
        )
        .expect("claude hull");
        assert!(!openai.is_empty());
        assert!(!claude.is_empty());
    }

    #[test]
    fn lines_route_around_obstacle_rect() {
        let engine = PretextEngine::with_font_data_and_system_fonts(
            AssetRegistry::bundled_font_data(),
            false,
        );
        let prepared = engine.prepare_with_segments(
            "This paragraph should be forced to route around an obstacle in the middle of the column so the first few lines shift to the right before the flow returns to the left edge.",
            &body_style(),
            &normal_options(),
        );
        let region = GeoRect {
            x: 20.0,
            y: 20.0,
            width: 260.0,
            height: 220.0,
        };
        let obstacle = GeoRect {
            x: 20.0,
            y: 20.0,
            width: 120.0,
            height: 86.0,
        };
        let obstacles = [BandObstacle::Rects {
            rects: std::slice::from_ref(&obstacle),
            horizontal_padding: (BODY_LINE_HEIGHT * 0.72).round(),
            vertical_padding: (BODY_LINE_HEIGHT * 0.12).round(),
        }];
        let (lines, _) = layout_column(
            &engine,
            &prepared,
            LayoutCursor::default(),
            region,
            BODY_LINE_HEIGHT,
            &obstacles,
            ColumnSide::Left,
        );
        assert!(!lines.is_empty());
        let overlapping: Vec<_> = lines
            .iter()
            .filter(|line| line.y < obstacle.bottom() && line.y + BODY_LINE_HEIGHT > obstacle.y)
            .collect();
        assert!(!overlapping.is_empty());
        assert!(overlapping.iter().all(|line| line.x >= obstacle.right()));
    }

    #[test]
    fn positioned_lines_keep_visual_runs_for_mixed_direction_text() {
        let engine = PretextEngine::with_font_data_and_system_fonts(
            AssetRegistry::bundled_font_data(),
            false,
        );
        let prepared = engine.prepare_with_segments(
            "English قبل العربية and back again",
            &body_style(),
            &normal_options(),
        );
        let obstacles: [BandObstacle<'_>; 0] = [];
        let (lines, _) = layout_column(
            &engine,
            &prepared,
            LayoutCursor::default(),
            GeoRect {
                x: 24.0,
                y: 32.0,
                width: 320.0,
                height: 160.0,
            },
            BODY_LINE_HEIGHT,
            &obstacles,
            ColumnSide::Left,
        );
        let line = lines
            .iter()
            .find(|line| {
                line.visual_runs
                    .iter()
                    .any(|run| run.direction == BidiDirection::Rtl)
            })
            .expect("at least one dynamic-layout line should contain an RTL run");

        assert!(line.visual_runs.len() >= 2);
        assert!(line
            .visual_runs
            .iter()
            .any(|run| run.direction == BidiDirection::Ltr));
        assert!(line
            .visual_runs
            .iter()
            .any(|run| run.direction == BidiDirection::Rtl));
    }

    #[test]
    fn fitted_headline_size_avoids_mid_word_breaks() {
        let engine = PretextEngine::with_font_data_and_system_fonts(
            AssetRegistry::bundled_font_data(),
            false,
        );
        let size = fit_headline_font_size(&engine, 620.0, 1040.0);
        let prepared =
            engine.prepare_with_segments(HEADLINE, &headline_style(size), &normal_options());

        assert!(size >= 22.0);
        assert!(!headline_breaks_inside_word(&engine, &prepared, 620.0));
    }
}
