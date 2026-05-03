use std::sync::{Arc, OnceLock};
use std::time::Duration;

#[cfg(test)]
use crate::demo_assets::bundled_font_data;
use eframe::egui;
use egui::epaint::{Mesh, Vertex};
use egui::{
    Align2, Color32, CornerRadius, FontFamily, FontId, Rect, Sense, Shape, Stroke, StrokeKind,
};
use pretext::advanced::LayoutCursor;
#[cfg(test)]
use pretext::BidiDirection;
use pretext::{
    PretextEngine, PretextGlyphRun as LayoutLineGlyphRun,
    PretextParagraphOptions as PrepareOptions,
    PretextPreparedParagraph as PreparedTextWithSegments, PretextStyle as TextStyleSpec,
    PretextVisualRun as LayoutLineVisualRun, WhiteSpaceMode,
};
use pretext_egui::{
    advanced::PretextFragmentPainter, EguiPretextPaintOptions, EguiPretextRenderer,
};

use crate::demo_assets::{bundled_svg_texture, svg_bytes, SvgAssetId};
use crate::demos::{format_warmup_status, DemoPerfStats, DemoWarmupStatus, DemoWindow};
#[cfg(test)]
use crate::geometry::carve_text_line_slots;
use crate::geometry::{
    append_rect_intervals_for_band, carve_text_line_slots_into, is_point_in_polygon,
    svg_alpha_hull, transform_points, Interval, Point, PolygonBandCache, Rect as GeoRect,
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
const WINDOW_DEFAULT_WIDTH: f32 = 1180.0;
const WINDOW_DEFAULT_HEIGHT: f32 = 1480.0;
const HINT_PILL_SAFE_TOP: f32 = 72.0;
const NARROW_BREAKPOINT: f32 = 760.0;
const COMPACT_BREAKPOINT: f32 = 980.0;
const NARROW_COLUMN_MAX_WIDTH: f32 = 430.0;
const UNBOUNDED_WIDTH: f32 = 100_000.0;
const DYNAMIC_REFLOW_BUCKET_PX: f32 = 2.0;

pub struct DynamicLayoutDemo {
    open: bool,
    openai_logo: LogoAnimationState,
    claude_logo: LogoAnimationState,
    prepared_engine_revision: Option<u64>,
    headline_paint_style: Option<CachedSizedTextStyle>,
    body_prepared: Option<PreparedTextWithSegments>,
    headline_prepared: Option<PreparedTextWithSegments>,
    headline_prepared_size_q: Option<u32>,
    credit_prepared: Option<PreparedTextWithSegments>,
    credit_width: Option<f32>,
    hulls: Option<LogoHulls>,
    layout_cache: Option<CachedPageLayout>,
    openai_geometry_cache: Option<CachedLogoGeometry>,
    claude_geometry_cache: Option<CachedLogoGeometry>,
    projection_cache: Option<CachedDynamicProjection>,
    projection_cache_stats: DynamicProjectionCacheStats,
}

#[derive(Clone)]
struct LogoHulls {
    openai: Vec<Point>,
    claude: Vec<Point>,
}

struct TransformedLogoGeometry {
    polygon: Vec<Point>,
    scanlines: Arc<PolygonBandCache>,
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

#[derive(Clone, Debug, PartialEq)]
struct DynamicProjection {
    headline_lines: Vec<Arc<PositionedLine>>,
    credit_line: Option<PositionedLine>,
    left_lines: Vec<Arc<PositionedLine>>,
    right_lines: Vec<Arc<PositionedLine>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DynamicLayoutKey {
    engine_revision: u64,
    page_width_q: u32,
    page_height_q: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DynamicProjectionKey {
    layout_key: DynamicLayoutKey,
    openai_angle_q: i32,
    claude_angle_q: i32,
}

struct CachedPageLayout {
    key: DynamicLayoutKey,
    layout: PageLayout,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DynamicLogoGeometryKey {
    layout_key: DynamicLayoutKey,
    angle_q: i32,
}

struct CachedLogoGeometry {
    key: DynamicLogoGeometryKey,
    scanlines: Arc<PolygonBandCache>,
}

struct CachedDynamicProjection {
    key: DynamicProjectionKey,
    projection: DynamicProjection,
    headline_plan: DynamicColumnPlan,
    headline_column: CachedDynamicColumn,
    headline_title_bands: Arc<[Vec<Interval>]>,
    headline_bottom: f32,
    headline_title_span: Option<DynamicVerticalSpan>,
    primary_body_plan: DynamicColumnPlan,
    primary_body_column: CachedDynamicColumn,
    secondary_body_plan: Option<DynamicColumnPlan>,
    secondary_body_column: Option<CachedDynamicColumn>,
    openai_span: Option<DynamicVerticalSpan>,
    claude_span: Option<DynamicVerticalSpan>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct DynamicProjectionCacheStats {
    bucket_hits: usize,
    dirty_bands: usize,
    full_recomputes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DynamicVerticalSpan {
    top: f32,
    bottom: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct DynamicColumnCacheStats {
    dirty_bands: usize,
    full_recomputes: usize,
}

#[derive(Clone)]
struct CachedSizedTextStyle {
    size_q: u32,
    style: Arc<TextStyleSpec>,
}

#[derive(Clone)]
struct DynamicColumnPlan {
    bands: Vec<DynamicBandPlan>,
}

#[derive(Clone)]
struct DynamicBandPlan {
    line_top: f32,
    slot: Option<Interval>,
    signature: DynamicBandSignature,
}

#[derive(Clone)]
struct CachedDynamicColumn {
    bands: Vec<CachedDynamicBand>,
}

#[derive(Clone)]
struct CachedDynamicBand {
    input_cursor: LayoutCursor,
    signature: DynamicBandSignature,
    line_count_before: usize,
    line: Option<Arc<PositionedLine>>,
    output_cursor: LayoutCursor,
    exhausted_text: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DynamicBandSignature {
    line_top_q: u32,
    slot: Option<DynamicBandIntervalKey>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DynamicBandIntervalKey {
    left_bits: u32,
    right_bits: u32,
}

#[derive(Default)]
struct DynamicBandScratch {
    blocked: Vec<Interval>,
    slots: Vec<Interval>,
    scratch_slots: Vec<Interval>,
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
    is_compact: bool,
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
        scanlines: &'a PolygonBandCache,
        horizontal_padding: f32,
        vertical_padding: f32,
    },
    #[cfg_attr(not(test), allow(dead_code))]
    Rects {
        rects: &'a [GeoRect],
        horizontal_padding: f32,
        vertical_padding: f32,
    },
    BandIntervals {
        bands: &'a [Vec<Interval>],
    },
}

#[derive(Clone, Copy)]
enum ColumnSide {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DynamicWarmupStage {
    PrepareBody,
    PrepareCredit,
    PrepareHulls,
    Layout,
    Geometries,
    Projection,
    Ready,
}

impl Default for DynamicLayoutDemo {
    fn default() -> Self {
        Self {
            open: false,
            openai_logo: LogoAnimationState::default(),
            claude_logo: LogoAnimationState::default(),
            prepared_engine_revision: None,
            headline_paint_style: None,
            body_prepared: None,
            headline_prepared: None,
            headline_prepared_size_q: None,
            credit_prepared: None,
            credit_width: None,
            hulls: None,
            layout_cache: None,
            openai_geometry_cache: None,
            claude_geometry_cache: None,
            projection_cache: None,
            projection_cache_stats: DynamicProjectionCacheStats::default(),
        }
    }
}

impl DemoWindow for DynamicLayoutDemo {
    fn id(&self) -> &'static str {
        "dynamic_layout"
    }

    fn title(&self) -> &str {
        "Dynamic Layout"
    }

    fn is_open(&self) -> bool {
        self.open
    }

    fn set_open(&mut self, open: bool) {
        self.open = open;
    }

    fn warmup_status(&self) -> DemoWarmupStatus {
        let stage = self.warmup_stage();
        if stage == DynamicWarmupStage::Ready {
            return DemoWarmupStatus::ready();
        }

        DemoWarmupStatus::pending(
            dynamic_warmup_stage_label(stage),
            dynamic_warmup_stage_index(stage),
            dynamic_warmup_stage_index(DynamicWarmupStage::Ready),
        )
    }

    fn warmup_step(
        &mut self,
        _ctx: &egui::Context,
        engine: &PretextEngine,
        _assets: &mut EguiPretextRenderer,
        _budget: Duration,
    ) -> bool {
        self.invalidate_engine_caches_if_needed(engine);
        let page_rect = default_warmup_page_rect();

        match self.warmup_stage() {
            DynamicWarmupStage::PrepareBody => {
                let _ = self.ensure_body_prepared(engine);
            }
            DynamicWarmupStage::PrepareCredit => {
                let _ = self.ensure_credit_width(engine);
            }
            DynamicWarmupStage::PrepareHulls => {
                let _ = self.ensure_hulls();
            }
            DynamicWarmupStage::Layout => {
                let layout = self.ensure_layout(page_rect, engine);
                let _ = self.ensure_headline_prepared(engine, layout.headline_size);
            }
            DynamicWarmupStage::Geometries => {
                let layout = self.ensure_layout(page_rect, engine);
                let _ = self.frame_logo_geometries(engine, layout);
            }
            DynamicWarmupStage::Projection => {
                let layout = self.ensure_layout(page_rect, engine);
                let (openai_geometry, claude_geometry) = self.frame_logo_geometries(engine, layout);
                let _ = self.ensure_projection(
                    engine,
                    layout,
                    openai_geometry.scanlines.as_ref(),
                    claude_geometry.scanlines.as_ref(),
                );
            }
            DynamicWarmupStage::Ready => {}
        }

        self.warmup_stage() == DynamicWarmupStage::Ready
    }

    fn show_loading(
        &mut self,
        ctx: &egui::Context,
        _engine: &PretextEngine,
        _assets: &mut EguiPretextRenderer,
    ) {
        let mut open = self.open;
        let status = self.warmup_status();
        egui::Window::new(self.title())
            .open(&mut open)
            .resizable(true)
            .default_size(egui::vec2(WINDOW_DEFAULT_WIDTH, WINDOW_DEFAULT_HEIGHT))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(18.0);
                    ui.heading(self.title());
                    ui.label("Preparing headline fit, logo geometry, and reflow caches.");
                    ui.add_space(6.0);
                    ui.monospace(format_warmup_status(status));
                    ui.add_space(12.0);
                    ui.spinner();
                });
            });
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
            .open(&mut open)
            .resizable(true)
            .default_size(egui::vec2(WINDOW_DEFAULT_WIDTH, WINDOW_DEFAULT_HEIGHT))
            .show(ctx, |ui| {
                let now = ctx.input(|input| input.time);
                let animating = update_spin_state(&mut self.openai_logo, now)
                    | update_spin_state(&mut self.claude_logo, now);

                let available = ui.available_size();
                let page_width = available.x.max(360.0);
                let page_height = available.y.max(MIN_PAGE_HEIGHT);
                let (page_rect, _) =
                    ui.allocate_exact_size(egui::vec2(page_width, page_height), Sense::hover());

                self.invalidate_engine_caches_if_needed(engine);
                let layout = self.ensure_layout(page_rect, engine);
                let headline_text_style =
                    Arc::clone(self.ensure_headline_paint_style(layout.headline_size));
                let (openai_geometry, claude_geometry) = self.frame_logo_geometries(engine, layout);

                let painter = ui.painter().clone();
                paint_page_background(&painter, page_rect);

                {
                    let projection = self.ensure_projection(
                        engine,
                        layout,
                        openai_geometry.scanlines.as_ref(),
                        claude_geometry.scanlines.as_ref(),
                    );
                    let headline_options = fragment_paint_options(
                        headline_text_style.as_ref(),
                        layout.headline_line_height,
                        Color32::from_rgb(17, 16, 13),
                    );
                    let credit_options = fragment_paint_options(
                        credit_style(),
                        CREDIT_LINE_HEIGHT,
                        Color32::from_rgba_premultiplied(17, 16, 13, 148),
                    );
                    let body_options = fragment_paint_options(
                        body_style(),
                        BODY_LINE_HEIGHT,
                        Color32::from_rgb(17, 16, 13),
                    );
                    let mut fragment_painter = PretextFragmentPainter::new(assets);
                    queue_positioned_lines(
                        &mut fragment_painter,
                        projection.headline_lines.iter().map(Arc::as_ref),
                        &headline_options,
                        ctx,
                        engine,
                        assets,
                    );
                    if let Some(credit_line) = &projection.credit_line {
                        queue_positioned_lines(
                            &mut fragment_painter,
                            std::slice::from_ref(credit_line),
                            &credit_options,
                            ctx,
                            engine,
                            assets,
                        );
                    }
                    queue_positioned_lines(
                        &mut fragment_painter,
                        projection.left_lines.iter().map(Arc::as_ref),
                        &body_options,
                        ctx,
                        engine,
                        assets,
                    );
                    queue_positioned_lines(
                        &mut fragment_painter,
                        projection.right_lines.iter().map(Arc::as_ref),
                        &body_options,
                        ctx,
                        engine,
                        assets,
                    );
                    let _ = fragment_painter.finish(&painter, ctx, assets);
                }

                paint_logo_shadow(
                    &painter,
                    layout.openai_rect,
                    egui::vec2(0.0, layout.openai_rect.height * 0.12),
                    Color32::from_rgba_premultiplied(
                        16,
                        16,
                        12,
                        if layout.is_compact { 24 } else { 34 },
                    ),
                );
                paint_logo_shadow(
                    &painter,
                    layout.claude_rect,
                    egui::vec2(0.0, layout.claude_rect.height * 0.1),
                    Color32::from_rgba_premultiplied(
                        140,
                        86,
                        52,
                        if layout.is_compact { 30 } else { 42 },
                    ),
                );

                let openai_texture =
                    bundled_svg_texture(assets, SvgAssetId::OpenAiLogo, LOGO_RASTER_SIZE, ctx);
                let claude_texture =
                    bundled_svg_texture(assets, SvgAssetId::ClaudeLogo, LOGO_RASTER_SIZE, ctx);
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
                    &openai_geometry.polygon,
                    &mut self.openai_logo,
                    -1.0,
                );
                handle_logo_interaction(
                    ui,
                    now,
                    "claude-logo",
                    layout.claude_rect,
                    &claude_geometry.polygon,
                    &mut self.claude_logo,
                    1.0,
                );

                if animating {
                    ctx.request_repaint_after(FRAME_INTERVAL);
                }
            });
        self.open = open;
    }

    fn perf_stats(&self) -> DemoPerfStats {
        DemoPerfStats {
            dynamic_bucket_hits: self.projection_cache_stats.bucket_hits,
            dynamic_dirty_bands: self.projection_cache_stats.dirty_bands,
            dynamic_full_recomputes: self.projection_cache_stats.full_recomputes,
            ..DemoPerfStats::default()
        }
    }
}

impl DynamicLayoutDemo {
    fn warmup_stage(&self) -> DynamicWarmupStage {
        if self.projection_cache.is_some() {
            DynamicWarmupStage::Ready
        } else if self.openai_geometry_cache.is_some() && self.claude_geometry_cache.is_some() {
            DynamicWarmupStage::Projection
        } else if self.layout_cache.is_some() {
            DynamicWarmupStage::Geometries
        } else if self.hulls.is_some() {
            DynamicWarmupStage::Layout
        } else if self.body_prepared.is_some() && self.credit_width.is_some() {
            DynamicWarmupStage::PrepareHulls
        } else if self.body_prepared.is_some() {
            DynamicWarmupStage::PrepareCredit
        } else {
            DynamicWarmupStage::PrepareBody
        }
    }

    fn invalidate_engine_caches_if_needed(&mut self, engine: &PretextEngine) {
        let revision = engine.revision();
        if self.prepared_engine_revision == Some(revision) {
            return;
        }

        self.prepared_engine_revision = Some(revision);
        self.headline_paint_style = None;
        self.body_prepared = None;
        self.headline_prepared = None;
        self.headline_prepared_size_q = None;
        self.credit_prepared = None;
        self.credit_width = None;
        self.layout_cache = None;
        self.openai_geometry_cache = None;
        self.claude_geometry_cache = None;
        self.projection_cache = None;
        self.projection_cache_stats = DynamicProjectionCacheStats::default();
    }

    fn ensure_body_prepared(&mut self, engine: &PretextEngine) -> &PreparedTextWithSegments {
        if self.body_prepared.is_none() {
            self.body_prepared =
                Some(engine.prepare_paragraph(BODY_COPY, body_style(), &normal_options()));
        }
        self.body_prepared
            .as_ref()
            .expect("dynamic body should be prepared")
    }

    fn ensure_credit_prepared(&mut self, engine: &PretextEngine) -> &PreparedTextWithSegments {
        if self.credit_prepared.is_none() {
            self.credit_prepared =
                Some(engine.prepare_paragraph(CREDIT_TEXT, credit_style(), &normal_options()));
        }
        self.credit_prepared
            .as_ref()
            .expect("dynamic credit should be prepared")
    }

    fn ensure_credit_width(&mut self, engine: &PretextEngine) -> f32 {
        if self.credit_width.is_none() {
            let width =
                measure_single_line_width(engine, self.ensure_credit_prepared(engine)).ceil();
            self.credit_width = Some(width);
        }
        self.credit_width
            .expect("dynamic credit width should be prepared")
    }

    fn ensure_headline_prepared(
        &mut self,
        engine: &PretextEngine,
        headline_size: f32,
    ) -> &PreparedTextWithSegments {
        let size_q = quantize_dynamic_value(headline_size);
        if self.headline_prepared_size_q != Some(size_q) || self.headline_prepared.is_none() {
            self.headline_prepared = Some(engine.prepare_paragraph(
                HEADLINE,
                &headline_style(headline_size),
                &normal_options(),
            ));
            self.headline_prepared_size_q = Some(size_q);
        }
        self.headline_prepared
            .as_ref()
            .expect("dynamic headline should be prepared")
    }

    fn ensure_headline_paint_style(&mut self, headline_size: f32) -> &Arc<TextStyleSpec> {
        let size_q = quantize_dynamic_value(headline_size);
        if self
            .headline_paint_style
            .as_ref()
            .is_none_or(|cached| cached.size_q != size_q)
        {
            self.headline_paint_style = Some(CachedSizedTextStyle {
                size_q,
                style: Arc::new(headline_style(headline_size)),
            });
        }
        &self
            .headline_paint_style
            .as_ref()
            .expect("dynamic headline paint style should exist")
            .style
    }

    fn ensure_hulls(&mut self) -> &LogoHulls {
        if self.hulls.is_none() {
            let openai = svg_alpha_hull(svg_bytes(SvgAssetId::OpenAiLogo), LOGO_RASTER_SIZE)
                .expect("openai hull");
            let claude = svg_alpha_hull(svg_bytes(SvgAssetId::ClaudeLogo), LOGO_RASTER_SIZE)
                .expect("claude hull");
            self.hulls = Some(LogoHulls { openai, claude });
        }
        self.hulls.as_ref().expect("dynamic hulls should exist")
    }

    fn ensure_layout(&mut self, page_rect: Rect, engine: &PretextEngine) -> PageLayout {
        let key = DynamicLayoutKey {
            engine_revision: engine.revision(),
            page_width_q: quantize_dynamic_value(page_rect.width()),
            page_height_q: quantize_dynamic_value(page_rect.height()),
        };
        if self
            .layout_cache
            .as_ref()
            .is_none_or(|cached| cached.key != key)
        {
            self.layout_cache = Some(CachedPageLayout {
                key,
                layout: build_page_layout(page_rect, engine),
            });
            self.openai_geometry_cache = None;
            self.claude_geometry_cache = None;
            self.projection_cache = None;
            self.projection_cache_stats = DynamicProjectionCacheStats::default();
        }

        self.layout_cache
            .as_ref()
            .expect("dynamic layout cache should exist")
            .layout
    }

    fn ensure_projection(
        &mut self,
        engine: &PretextEngine,
        layout: PageLayout,
        openai_scanlines: &PolygonBandCache,
        claude_scanlines: &PolygonBandCache,
    ) -> &DynamicProjection {
        let layout_key = self.current_layout_key(engine, layout);
        let key = DynamicProjectionKey {
            layout_key,
            openai_angle_q: quantize_dynamic_reflow_angle(
                layout.openai_rect,
                self.openai_logo.angle,
            ),
            claude_angle_q: quantize_dynamic_reflow_angle(
                layout.claude_rect,
                self.claude_logo.angle,
            ),
        };
        if self
            .projection_cache
            .as_ref()
            .is_some_and(|cached| cached.key == key)
        {
            self.projection_cache_stats = DynamicProjectionCacheStats {
                bucket_hits: 1,
                ..DynamicProjectionCacheStats::default()
            };
            return &self
                .projection_cache
                .as_ref()
                .expect("dynamic projection cache should exist")
                .projection;
        }

        self.ensure_body_prepared(engine);
        self.ensure_headline_prepared(engine, layout.headline_size);
        let credit_width = self.ensure_credit_width(engine);
        let cached_projection = self
            .projection_cache
            .take()
            .filter(|cached| cached.key.layout_key == layout_key);
        let (next_projection, next_stats) = {
            let (body_prepared, headline_prepared, credit_prepared) = match (
                &self.body_prepared,
                &self.headline_prepared,
                &self.credit_prepared,
            ) {
                (Some(body), Some(headline), Some(credit)) => (body, headline, credit),
                _ => unreachable!("dynamic prepared text should exist"),
            };
            compute_dynamic_projection(
                engine,
                body_prepared,
                headline_prepared,
                credit_prepared,
                credit_width,
                layout,
                openai_scanlines,
                claude_scanlines,
                key,
                cached_projection,
            )
        };
        self.projection_cache_stats = next_stats;
        self.projection_cache = Some(next_projection);

        &self
            .projection_cache
            .as_ref()
            .expect("dynamic projection cache should exist")
            .projection
    }

    fn current_layout_key(&self, engine: &PretextEngine, layout: PageLayout) -> DynamicLayoutKey {
        self.layout_cache
            .as_ref()
            .map(|cached| cached.key)
            .unwrap_or(DynamicLayoutKey {
                engine_revision: engine.revision(),
                page_width_q: quantize_dynamic_value(layout.page.width),
                page_height_q: quantize_dynamic_value(layout.page.height),
            })
    }

    fn frame_logo_geometries(
        &mut self,
        engine: &PretextEngine,
        layout: PageLayout,
    ) -> (TransformedLogoGeometry, TransformedLogoGeometry) {
        self.ensure_hulls();
        let layout_key = self.current_layout_key(engine, layout);
        let openai_angle = self.openai_logo.angle;
        let claude_angle = self.claude_logo.angle;
        let DynamicLayoutDemo {
            hulls,
            openai_geometry_cache,
            claude_geometry_cache,
            ..
        } = self;
        let hulls = hulls.as_ref().expect("dynamic hulls should exist");
        let openai_geometry = cached_logo_geometry(
            openai_geometry_cache,
            layout_key,
            &hulls.openai,
            layout.openai_rect,
            openai_angle,
        );
        let claude_geometry = cached_logo_geometry(
            claude_geometry_cache,
            layout_key,
            &hulls.claude,
            layout.claude_rect,
            claude_angle,
        );
        (openai_geometry, claude_geometry)
    }
}

fn dynamic_warmup_stage_index(stage: DynamicWarmupStage) -> usize {
    match stage {
        DynamicWarmupStage::PrepareBody => 0,
        DynamicWarmupStage::PrepareCredit => 1,
        DynamicWarmupStage::PrepareHulls => 2,
        DynamicWarmupStage::Layout => 3,
        DynamicWarmupStage::Geometries => 4,
        DynamicWarmupStage::Projection => 5,
        DynamicWarmupStage::Ready => 6,
    }
}

fn dynamic_warmup_stage_label(stage: DynamicWarmupStage) -> &'static str {
    match stage {
        DynamicWarmupStage::PrepareBody => "body text",
        DynamicWarmupStage::PrepareCredit => "credit text",
        DynamicWarmupStage::PrepareHulls => "logo hulls",
        DynamicWarmupStage::Layout => "page layout",
        DynamicWarmupStage::Geometries => "logo geometry",
        DynamicWarmupStage::Projection => "projection",
        DynamicWarmupStage::Ready => "ready",
    }
}

fn default_warmup_page_rect() -> Rect {
    Rect::from_min_size(
        egui::Pos2::ZERO,
        egui::vec2(
            WINDOW_DEFAULT_WIDTH,
            WINDOW_DEFAULT_HEIGHT.max(MIN_PAGE_HEIGHT),
        ),
    )
}

fn normal_options() -> PrepareOptions {
    PrepareOptions {
        white_space: WhiteSpaceMode::Normal,
        word_break: pretext::WordBreakMode::Normal,
        paragraph_direction: pretext::ParagraphDirection::Auto,
        letter_spacing: 0.0,
    }
}

fn quantize_dynamic_value(value: f32) -> u32 {
    (value.max(0.0) * 4.0).round() as u32
}

fn quantize_dynamic_reflow_angle(rect: GeoRect, angle: f32) -> i32 {
    (angle / dynamic_reflow_bucket_angle(rect)).round() as i32
}

fn dynamic_reflow_bucket_angle(rect: GeoRect) -> f32 {
    let radius = (rect.width.max(rect.height) * 0.5).max(1.0);
    DYNAMIC_REFLOW_BUCKET_PX / radius
}

fn snapped_dynamic_reflow_angle(rect: GeoRect, angle_q: i32) -> f32 {
    angle_q as f32 * dynamic_reflow_bucket_angle(rect)
}

const DYNAMIC_SERIF_FAMILIES: &[&str] = &[
    "Iowan Old Style",
    "Palatino Linotype",
    "Book Antiqua",
    "Palatino",
    "Georgia",
    "Times New Roman",
    "Noto Serif",
    "Noto Sans",
];
const DYNAMIC_SANS_FAMILIES: &[&str] = &["Helvetica Neue", "Helvetica", "Arial", "Noto Sans"];

fn build_text_style(families: &[&str], size_px: f32, weight: u16, italic: bool) -> TextStyleSpec {
    TextStyleSpec {
        families: families.iter().map(|name| (*name).to_owned()).collect(),
        size_px,
        weight,
        italic,
    }
}

fn body_style() -> &'static TextStyleSpec {
    static STYLE: OnceLock<TextStyleSpec> = OnceLock::new();
    STYLE.get_or_init(|| build_text_style(DYNAMIC_SERIF_FAMILIES, 20.0, 450, false))
}

fn headline_style(size_px: f32) -> TextStyleSpec {
    build_text_style(DYNAMIC_SERIF_FAMILIES, size_px, 700, false)
}

fn credit_style() -> &'static TextStyleSpec {
    static STYLE: OnceLock<TextStyleSpec> = OnceLock::new();
    STYLE.get_or_init(|| build_text_style(DYNAMIC_SANS_FAMILIES, 12.0, 500, false))
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
            engine.prepare_paragraph(HEADLINE, &headline_style(size as f32), &normal_options());
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
    let is_narrow = page.width <= NARROW_BREAKPOINT;
    let is_compact = page.width <= COMPACT_BREAKPOINT;

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
            is_compact,
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

    let gutter = if is_compact {
        (44.0f32.max(page.width * 0.044)).round()
    } else {
        (52.0f32.max(page.width * 0.048)).round()
    };
    let center_gap = if is_compact {
        (18.0f32.max(page.width * 0.018)).round()
    } else {
        (28.0f32.max(page.width * 0.025)).round()
    };
    let column_width = ((page.width - gutter * 2.0 - center_gap) * 0.5).round();
    let headline_top = if is_compact {
        38.0f32
            .max(page.width * 0.036)
            .max(HINT_PILL_SAFE_TOP)
            .round()
    } else {
        42.0f32
            .max(page.width * 0.04)
            .max(HINT_PILL_SAFE_TOP)
            .round()
    };
    let headline_width = (page.width - gutter * 2.0)
        .min(column_width.max(page.width * if is_compact { 0.56 } else { 0.5 }))
        .round();
    let headline_size = fit_headline_font_size(engine, headline_width, page.width);
    let headline_size = if is_compact {
        headline_size.min(72.0)
    } else {
        headline_size
    };
    let headline_line_height = (headline_size * 0.92).round();
    let credit_gap = if is_compact {
        (12.0f32.max(BODY_LINE_HEIGHT * 0.55)).round()
    } else {
        (14.0f32.max(BODY_LINE_HEIGHT * 0.6)).round()
    };
    let copy_gap = if is_compact {
        (18.0f32.max(BODY_LINE_HEIGHT * 0.8)).round()
    } else {
        (20.0f32.max(BODY_LINE_HEIGHT * 0.9)).round()
    };
    let openai_size = if is_compact {
        336.0f32
            .max((page.width * 0.33).min(page.height * 0.38).min(360.0))
            .round()
    } else {
        let openai_shrink_t = ((960.0 - page.width) / 260.0).clamp(0.0, 1.0);
        (400.0 - openai_shrink_t * 56.0)
            .min(page.height * 0.43)
            .round()
    };
    let claude_size = if is_compact {
        220.0f32
            .max((page.width * 0.30).min(page.height * 0.34).min(320.0))
            .round()
    } else {
        276.0f32
            .max((page.width * 0.355).min(page.height * 0.45).min(500.0))
            .round()
    };

    PageLayout {
        page,
        is_narrow,
        is_compact,
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
            x: if is_compact {
                page.x + page.width - gutter - (claude_size * 0.82).round()
            } else {
                page.x + page.width - (claude_size * 0.69).round()
            },
            y: if is_compact {
                page.y + 6.0
            } else {
                page.y - (claude_size * 0.22).round()
            },
            width: claude_size,
            height: claude_size,
        },
    }
}

fn append_obstacle_intervals(
    blocked: &mut Vec<Interval>,
    obstacle: &BandObstacle<'_>,
    band_index: usize,
    band_top: f32,
    band_bottom: f32,
) {
    match *obstacle {
        BandObstacle::Polygon {
            scanlines,
            horizontal_padding,
            vertical_padding,
        } => {
            if let Some(interval) = scanlines.interval_for_band(
                band_top,
                band_bottom,
                horizontal_padding,
                vertical_padding,
            ) {
                blocked.push(interval);
            }
        }
        BandObstacle::Rects {
            rects,
            horizontal_padding,
            vertical_padding,
        } => append_rect_intervals_for_band(
            blocked,
            rects,
            band_top,
            band_bottom,
            horizontal_padding,
            vertical_padding,
        ),
        BandObstacle::BandIntervals { bands } => {
            if let Some(intervals) = bands.get(band_index) {
                blocked.extend(intervals.iter().copied());
            }
        }
    }
}

fn build_rect_band_interval_cache(
    rects: &[GeoRect],
    region: GeoRect,
    line_height: f32,
    horizontal_padding: f32,
    vertical_padding: f32,
) -> Arc<[Vec<Interval>]> {
    let band_count = dynamic_band_count(region, line_height);
    let mut bands = Vec::with_capacity(band_count);
    for band_index in 0..band_count {
        let band_top = region.y + band_index as f32 * line_height;
        let band_bottom = band_top + line_height;
        let mut intervals = Vec::new();
        append_rect_intervals_for_band(
            &mut intervals,
            rects,
            band_top,
            band_bottom,
            horizontal_padding,
            vertical_padding,
        );
        bands.push(intervals);
    }
    Arc::from(bands)
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

#[cfg(test)]
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
    let mut band_index = 0;
    let mut lines = Vec::new();

    while line_top + line_height <= region.bottom() {
        let band_top = line_top;
        let band_bottom = line_top + line_height;
        let mut blocked = Vec::new();
        for obstacle in obstacles {
            append_obstacle_intervals(&mut blocked, obstacle, band_index, band_top, band_bottom);
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
            band_index += 1;
            continue;
        }

        let slot = choose_slot(&slots, side);
        let mut next_cursor = cursor;
        let Some(line) = engine.layout_next_line_with_runs(
            prepared,
            &mut next_cursor,
            (slot.right - slot.left).max(1.0),
        ) else {
            break;
        };
        if next_cursor == cursor {
            break;
        }

        lines.push(PositionedLine {
            x: slot.left.round(),
            y: line_top.round(),
            width: line.line.width,
            text: line.line.text,
            visual_runs: line.runs.visual_runs,
            glyph_runs: line.runs.glyph_runs,
        });
        cursor = next_cursor;
        line_top += line_height;
        band_index += 1;
    }

    (lines, cursor)
}

fn positioned_line_rects(lines: &[Arc<PositionedLine>], line_height: f32) -> Vec<GeoRect> {
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

fn headline_metadata(
    lines: &[Arc<PositionedLine>],
    line_height: f32,
    title_vertical_padding: f32,
    fallback_top: f32,
) -> (Vec<GeoRect>, f32, Option<DynamicVerticalSpan>) {
    let rects = positioned_line_rects(lines, line_height);
    let bottom = lines
        .iter()
        .map(|line| line.y + line_height)
        .fold(fallback_top, f32::max);
    let title_span = dynamic_rect_span(&rects, title_vertical_padding);
    (rects, bottom, title_span)
}

fn cached_column_output_cursor(
    column: &CachedDynamicColumn,
    fallback_cursor: LayoutCursor,
) -> LayoutCursor {
    column
        .bands
        .last()
        .map(|band| band.output_cursor)
        .unwrap_or(fallback_cursor)
}

fn dynamic_scanline_span(scanlines: &PolygonBandCache) -> Option<DynamicVerticalSpan> {
    scanlines
        .vertical_span()
        .map(|(top, bottom)| DynamicVerticalSpan { top, bottom })
}

fn dynamic_rect_span(rects: &[GeoRect], vertical_padding: f32) -> Option<DynamicVerticalSpan> {
    let first = rects.first().copied()?;
    let mut top = first.y - vertical_padding;
    let mut bottom = first.bottom() + vertical_padding;
    for rect in rects.iter().copied().skip(1) {
        top = top.min(rect.y - vertical_padding);
        bottom = bottom.max(rect.bottom() + vertical_padding);
    }
    Some(DynamicVerticalSpan { top, bottom })
}

fn dynamic_union_span(
    left: Option<DynamicVerticalSpan>,
    right: Option<DynamicVerticalSpan>,
) -> Option<DynamicVerticalSpan> {
    match (left, right) {
        (Some(left), Some(right)) => Some(DynamicVerticalSpan {
            top: left.top.min(right.top),
            bottom: left.bottom.max(right.bottom),
        }),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn dynamic_expand_span(
    span: Option<DynamicVerticalSpan>,
    vertical_padding: f32,
) -> Option<DynamicVerticalSpan> {
    span.map(|span| DynamicVerticalSpan {
        top: span.top - vertical_padding,
        bottom: span.bottom + vertical_padding,
    })
}

fn dynamic_dirty_span(
    current: Option<DynamicVerticalSpan>,
    previous: Option<DynamicVerticalSpan>,
    unchanged: bool,
    vertical_padding: f32,
) -> Option<DynamicVerticalSpan> {
    if unchanged {
        None
    } else {
        dynamic_union_span(
            dynamic_expand_span(current, vertical_padding),
            dynamic_expand_span(previous, vertical_padding),
        )
    }
}

fn dynamic_band_count(region: GeoRect, line_height: f32) -> usize {
    (((region.bottom() - region.y) / line_height)
        .floor()
        .max(0.0)) as usize
}

fn dynamic_band_range_for_span(
    region: GeoRect,
    line_height: f32,
    span: Option<DynamicVerticalSpan>,
) -> Option<(usize, usize)> {
    let span = span?;
    if span.bottom <= region.y || span.top >= region.bottom() {
        return None;
    }

    let band_count = dynamic_band_count(region, line_height);
    if band_count == 0 {
        return None;
    }

    let first = (((span.top - region.y) / line_height).floor() as isize - 1)
        .clamp(0, band_count as isize - 1) as usize;
    let last = (((span.bottom - region.y) / line_height).ceil() as isize - 1)
        .clamp(0, band_count as isize - 1) as usize;
    if first > last {
        None
    } else {
        Some((first, last))
    }
}

fn dynamic_union_band_range(
    left: Option<(usize, usize)>,
    right: Option<(usize, usize)>,
) -> Option<(usize, usize)> {
    match (left, right) {
        (Some((left_start, left_end)), Some((right_start, right_end))) => {
            Some((left_start.min(right_start), left_end.max(right_end)))
        }
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn dynamic_plan_matches_region(
    plan: &DynamicColumnPlan,
    region: GeoRect,
    line_height: f32,
) -> bool {
    let band_count = dynamic_band_count(region, line_height);
    if plan.bands.len() != band_count {
        return false;
    }
    if band_count == 0 {
        return true;
    }
    let first_line_top_q = quantize_dynamic_value(region.y);
    let last_line_top_q = quantize_dynamic_value(region.y + (band_count - 1) as f32 * line_height);
    plan.bands
        .first()
        .zip(plan.bands.last())
        .is_some_and(|(first, last)| {
            first.signature.line_top_q == first_line_top_q
                && last.signature.line_top_q == last_line_top_q
        })
}

fn build_dynamic_column_plan(
    region: GeoRect,
    line_height: f32,
    obstacles: &[BandObstacle<'_>],
    side: ColumnSide,
) -> DynamicColumnPlan {
    let mut bands = Vec::new();
    let mut scratch = DynamicBandScratch::default();
    let mut line_top = region.y;
    while line_top + line_height <= region.bottom() {
        bands.push(build_dynamic_band_plan(
            region,
            bands.len(),
            line_top,
            line_height,
            obstacles,
            side,
            &mut scratch,
        ));
        line_top += line_height;
    }
    DynamicColumnPlan { bands }
}

fn build_dynamic_column_plan_incremental(
    region: GeoRect,
    line_height: f32,
    obstacles: &[BandObstacle<'_>],
    side: ColumnSide,
    cached: Option<DynamicColumnPlan>,
    dirty_range: Option<(usize, usize)>,
) -> DynamicColumnPlan {
    let Some(mut cached) = cached else {
        return build_dynamic_column_plan(region, line_height, obstacles, side);
    };
    if !dynamic_plan_matches_region(&cached, region, line_height) {
        return build_dynamic_column_plan(region, line_height, obstacles, side);
    }
    let Some((dirty_start, dirty_end)) = dirty_range else {
        return cached;
    };
    let mut scratch = DynamicBandScratch::default();

    for band_index in dirty_start..=dirty_end {
        let line_top = region.y + band_index as f32 * line_height;
        cached.bands[band_index] = build_dynamic_band_plan(
            region,
            band_index,
            line_top,
            line_height,
            obstacles,
            side,
            &mut scratch,
        );
    }
    cached
}

fn build_dynamic_band_plan(
    region: GeoRect,
    band_index: usize,
    line_top: f32,
    line_height: f32,
    obstacles: &[BandObstacle<'_>],
    side: ColumnSide,
    scratch: &mut DynamicBandScratch,
) -> DynamicBandPlan {
    let band_top = line_top;
    let band_bottom = line_top + line_height;
    scratch.blocked.clear();
    for obstacle in obstacles {
        append_obstacle_intervals(
            &mut scratch.blocked,
            obstacle,
            band_index,
            band_top,
            band_bottom,
        );
    }

    let slot = {
        carve_text_line_slots_into(
            Interval {
                left: region.x,
                right: region.right(),
            },
            &scratch.blocked,
            &mut scratch.slots,
            &mut scratch.scratch_slots,
        );
        if scratch.slots.is_empty() {
            None
        } else {
            Some(choose_slot(&scratch.slots, side))
        }
    };

    DynamicBandPlan {
        line_top: line_top.round(),
        signature: DynamicBandSignature {
            line_top_q: quantize_dynamic_value(line_top),
            slot: slot.map(|slot| DynamicBandIntervalKey {
                left_bits: slot.left.to_bits(),
                right_bits: slot.right.to_bits(),
            }),
        },
        slot,
    }
}

fn layout_dynamic_band(
    engine: &PretextEngine,
    prepared: &PreparedTextWithSegments,
    input_cursor: LayoutCursor,
    line_count_before: usize,
    band: &DynamicBandPlan,
) -> CachedDynamicBand {
    let Some(slot) = band.slot else {
        return CachedDynamicBand {
            input_cursor,
            signature: band.signature,
            line_count_before,
            line: None,
            output_cursor: input_cursor,
            exhausted_text: false,
        };
    };

    let mut next_cursor = input_cursor;
    let line = engine.layout_next_line_with_runs(
        prepared,
        &mut next_cursor,
        (slot.right - slot.left).max(1.0),
    );
    let Some(line) = line else {
        return CachedDynamicBand {
            input_cursor,
            signature: band.signature,
            line_count_before,
            line: None,
            output_cursor: input_cursor,
            exhausted_text: true,
        };
    };
    if next_cursor == input_cursor {
        return CachedDynamicBand {
            input_cursor,
            signature: band.signature,
            line_count_before,
            line: None,
            output_cursor: input_cursor,
            exhausted_text: true,
        };
    }

    CachedDynamicBand {
        input_cursor,
        signature: band.signature,
        line_count_before,
        line: Some(Arc::new(PositionedLine {
            x: slot.left.round(),
            y: band.line_top,
            width: line.line.width,
            text: line.line.text,
            visual_runs: line.runs.visual_runs,
            glyph_runs: line.runs.glyph_runs,
        })),
        output_cursor: next_cursor,
        exhausted_text: false,
    }
}

fn compute_incremental_dynamic_column(
    engine: &PretextEngine,
    prepared: &PreparedTextWithSegments,
    start_cursor: LayoutCursor,
    plan: &DynamicColumnPlan,
    cached: Option<CachedDynamicColumn>,
    cached_lines: Option<Vec<Arc<PositionedLine>>>,
) -> (
    Vec<Arc<PositionedLine>>,
    CachedDynamicColumn,
    LayoutCursor,
    DynamicColumnCacheStats,
) {
    let mut cursor = start_cursor;
    let mut exhausted = false;
    let mut lines = cached_lines.unwrap_or_default();
    let mut bands = cached.map(|cached| cached.bands).unwrap_or_default();
    if bands.len() > plan.bands.len() {
        bands.truncate(plan.bands.len());
    } else if bands.len() < plan.bands.len() {
        bands.reserve(plan.bands.len() - bands.len());
    }
    let mut first_dirty = None;

    for (band_index, band_plan) in plan.bands.iter().enumerate() {
        let reused = if exhausted {
            bands
                .get(band_index)
                .filter(|band| {
                    band.input_cursor == cursor
                        && band.signature == band_plan.signature
                        && band.exhausted_text
                })
                .is_some()
        } else {
            bands.get(band_index).is_some_and(|band| {
                band.input_cursor == cursor && band.signature == band_plan.signature
            })
        };
        if reused {
            if first_dirty.is_some() {
                let band = &mut bands[band_index];
                band.line_count_before = lines.len();
                if let Some(line) = band.line.as_ref() {
                    lines.push(Arc::clone(line));
                }
                cursor = band.output_cursor;
                exhausted |= band.exhausted_text;
            } else {
                let band = &bands[band_index];
                cursor = band.output_cursor;
                exhausted |= band.exhausted_text;
            }
        } else {
            if first_dirty.is_none() {
                let line_count_before = bands
                    .get(band_index)
                    .map(|band| band.line_count_before)
                    .unwrap_or(lines.len());
                lines.truncate(line_count_before);
                first_dirty = Some(band_index);
            }
            let next_band = if exhausted {
                CachedDynamicBand {
                    input_cursor: cursor,
                    signature: band_plan.signature,
                    line_count_before: lines.len(),
                    line: None,
                    output_cursor: cursor,
                    exhausted_text: true,
                }
            } else {
                layout_dynamic_band(engine, prepared, cursor, lines.len(), band_plan)
            };
            if band_index < bands.len() {
                bands[band_index] = next_band;
            } else {
                bands.push(next_band);
            }
            let band = &bands[band_index];
            if let Some(line) = band.line.as_ref() {
                lines.push(Arc::clone(line));
            }
            cursor = band.output_cursor;
            exhausted |= band.exhausted_text;
        }
    }

    let stats = first_dirty.map_or(DynamicColumnCacheStats::default(), |first_dirty| {
        DynamicColumnCacheStats {
            dirty_bands: plan.bands.len().saturating_sub(first_dirty),
            full_recomputes: usize::from(first_dirty == 0),
        }
    });

    (lines, CachedDynamicColumn { bands }, cursor, stats)
}

fn compute_credit_line(
    engine: &PretextEngine,
    credit_prepared: &PreparedTextWithSegments,
    credit_width: f32,
    layout: PageLayout,
    headline_bottom: f32,
    obstacles: &[BandObstacle<'_>],
) -> Option<PositionedLine> {
    let credit_top = headline_bottom + layout.credit_gap;
    let credit_region = GeoRect {
        x: layout.page.x + layout.gutter + 4.0,
        y: credit_top,
        width: layout.headline_region.width,
        height: CREDIT_LINE_HEIGHT,
    };
    let mut scratch = DynamicBandScratch::default();
    for obstacle in obstacles {
        append_obstacle_intervals(
            &mut scratch.blocked,
            obstacle,
            0,
            credit_region.y,
            credit_region.bottom(),
        );
    }
    carve_text_line_slots_into(
        Interval {
            left: credit_region.x,
            right: credit_region.right(),
        },
        &scratch.blocked,
        &mut scratch.slots,
        &mut scratch.scratch_slots,
    );
    let credit_left = scratch
        .slots
        .iter()
        .find(|slot| slot.right - slot.left >= credit_width)
        .map(|slot| slot.left.round())
        .unwrap_or(credit_region.x.round());
    let mut credit_cursor = LayoutCursor::default();
    engine
        .layout_next_line_with_runs(credit_prepared, &mut credit_cursor, UNBOUNDED_WIDTH)
        .map(|line| PositionedLine {
            x: credit_left,
            y: credit_top.round(),
            width: line.line.width,
            text: line.line.text,
            visual_runs: line.runs.visual_runs,
            glyph_runs: line.runs.glyph_runs,
        })
}

fn compute_dynamic_projection(
    engine: &PretextEngine,
    body_prepared: &PreparedTextWithSegments,
    headline_prepared: &PreparedTextWithSegments,
    credit_prepared: &PreparedTextWithSegments,
    credit_width: f32,
    layout: PageLayout,
    openai_scanlines: &PolygonBandCache,
    claude_scanlines: &PolygonBandCache,
    key: DynamicProjectionKey,
    cached: Option<CachedDynamicProjection>,
) -> (CachedDynamicProjection, DynamicProjectionCacheStats) {
    let openai_vertical_padding = (BODY_LINE_HEIGHT * 0.26).round();
    let claude_vertical_padding = (BODY_LINE_HEIGHT * 0.12).round();
    let openai_obstacle = BandObstacle::Polygon {
        scanlines: openai_scanlines,
        horizontal_padding: (BODY_LINE_HEIGHT * 0.82).round(),
        vertical_padding: openai_vertical_padding,
    };
    let claude_obstacle = BandObstacle::Polygon {
        scanlines: claude_scanlines,
        horizontal_padding: (BODY_LINE_HEIGHT * 0.28).round(),
        vertical_padding: claude_vertical_padding,
    };
    let openai_span = dynamic_scanline_span(openai_scanlines);
    let claude_span = dynamic_scanline_span(claude_scanlines);
    let cached_openai_span = cached.as_ref().and_then(|cached| cached.openai_span);
    let cached_claude_span = cached.as_ref().and_then(|cached| cached.claude_span);
    let cached_headline_title_span = cached
        .as_ref()
        .and_then(|cached| cached.headline_title_span);
    let openai_bucket_unchanged = cached
        .as_ref()
        .is_some_and(|cached| cached.key.openai_angle_q == key.openai_angle_q);
    let claude_bucket_unchanged = cached
        .as_ref()
        .is_some_and(|cached| cached.key.claude_angle_q == key.claude_angle_q);
    let mut cached_headline_lines = None;
    let mut cached_credit_line = None;
    let mut cached_left_lines = None;
    let mut cached_right_lines = None;
    let mut cached_headline_plan = None;
    let mut cached_headline_column = None;
    let mut cached_headline_title_bands = None;
    let mut cached_headline_bottom = None;
    let mut cached_primary_body_plan = None;
    let mut cached_primary_body_column = None;
    let mut cached_secondary_body_plan = None;
    let mut cached_secondary_body_column = None;
    if let Some(cached) = cached {
        let CachedDynamicProjection {
            projection:
                DynamicProjection {
                    headline_lines,
                    credit_line,
                    left_lines,
                    right_lines,
                },
            headline_plan,
            headline_column,
            headline_title_bands,
            headline_bottom,
            headline_title_span: _,
            primary_body_plan,
            primary_body_column,
            secondary_body_plan,
            secondary_body_column,
            ..
        } = cached;
        cached_headline_lines = Some(headline_lines);
        cached_credit_line = credit_line;
        cached_left_lines = Some(left_lines);
        cached_right_lines = Some(right_lines);
        cached_headline_plan = Some(headline_plan);
        cached_headline_column = Some(headline_column);
        cached_headline_title_bands = Some(headline_title_bands);
        cached_headline_bottom = Some(headline_bottom);
        cached_primary_body_plan = Some(primary_body_plan);
        cached_primary_body_column = Some(primary_body_column);
        cached_secondary_body_plan = secondary_body_plan;
        cached_secondary_body_column = secondary_body_column;
    }
    let title_vertical_padding = (BODY_LINE_HEIGHT * 0.3).round();
    let title_horizontal_padding = (BODY_LINE_HEIGHT * 0.95).round();
    let right_region = GeoRect {
        x: layout.page.x + layout.gutter + layout.column_width + layout.center_gap,
        y: layout.headline_region.y,
        width: layout.column_width,
        height: (layout.page.bottom() - layout.headline_region.y - layout.gutter).max(0.0),
    };

    let (
        headline_lines,
        headline_plan,
        headline_column,
        headline_title_bands,
        headline_bottom,
        headline_title_span,
        headline_stats,
    ) = if openai_bucket_unchanged {
        (
            cached_headline_lines
                .take()
                .expect("cached headline lines should exist when openai bucket is unchanged"),
            cached_headline_plan
                .take()
                .expect("cached headline plan should exist when openai bucket is unchanged"),
            cached_headline_column
                .take()
                .expect("cached headline column should exist when openai bucket is unchanged"),
            cached_headline_title_bands
                .take()
                .expect("cached headline title bands should exist when openai bucket is unchanged"),
            cached_headline_bottom
                .expect("cached headline bottom should exist when openai bucket is unchanged"),
            cached_headline_title_span,
            DynamicColumnCacheStats::default(),
        )
    } else {
        let headline_obstacles = [openai_obstacle];
        let headline_dirty_range = dynamic_band_range_for_span(
            layout.headline_region,
            layout.headline_line_height,
            dynamic_dirty_span(
                openai_span,
                cached_openai_span,
                openai_bucket_unchanged,
                openai_vertical_padding,
            ),
        );
        let headline_plan = build_dynamic_column_plan_incremental(
            layout.headline_region,
            layout.headline_line_height,
            &headline_obstacles,
            ColumnSide::Left,
            cached_headline_plan.take(),
            headline_dirty_range,
        );
        let (headline_lines, headline_column, _, headline_stats) =
            compute_incremental_dynamic_column(
                engine,
                headline_prepared,
                LayoutCursor::default(),
                &headline_plan,
                cached_headline_column.take(),
                cached_headline_lines.take(),
            );
        let (headline_rects, headline_bottom, headline_title_span) = headline_metadata(
            &headline_lines,
            layout.headline_line_height,
            title_vertical_padding,
            layout.headline_region.y,
        );
        let headline_title_bands = if layout.is_narrow {
            Arc::<[Vec<Interval>]>::from(Vec::<Vec<Interval>>::new())
        } else {
            build_rect_band_interval_cache(
                &headline_rects,
                right_region,
                BODY_LINE_HEIGHT,
                title_horizontal_padding,
                title_vertical_padding,
            )
        };
        (
            headline_lines,
            headline_plan,
            headline_column,
            headline_title_bands,
            headline_bottom,
            headline_title_span,
            headline_stats,
        )
    };

    let credit_line = if !layout.is_narrow && openai_bucket_unchanged {
        cached_credit_line
    } else if layout.is_narrow {
        compute_credit_line(
            engine,
            credit_prepared,
            credit_width,
            layout,
            headline_bottom,
            &[openai_obstacle, claude_obstacle],
        )
    } else {
        compute_credit_line(
            engine,
            credit_prepared,
            credit_width,
            layout,
            headline_bottom,
            &[openai_obstacle],
        )
    };

    let copy_top = headline_bottom + layout.credit_gap + CREDIT_LINE_HEIGHT + layout.copy_gap;
    if layout.is_narrow {
        let body_region = GeoRect {
            x: (layout.page.x + (layout.page.width - layout.column_width) * 0.5).round(),
            y: copy_top,
            width: layout.column_width,
            height: (layout.page.bottom() - copy_top - layout.gutter).max(0.0),
        };
        let body_obstacles = [claude_obstacle, openai_obstacle];
        let body_dirty_range = dynamic_union_band_range(
            dynamic_band_range_for_span(
                body_region,
                BODY_LINE_HEIGHT,
                dynamic_dirty_span(
                    openai_span,
                    cached_openai_span,
                    openai_bucket_unchanged,
                    openai_vertical_padding,
                ),
            ),
            dynamic_band_range_for_span(
                body_region,
                BODY_LINE_HEIGHT,
                dynamic_dirty_span(
                    claude_span,
                    cached_claude_span,
                    claude_bucket_unchanged,
                    claude_vertical_padding,
                ),
            ),
        );
        let body_plan = build_dynamic_column_plan_incremental(
            body_region,
            BODY_LINE_HEIGHT,
            &body_obstacles,
            ColumnSide::Left,
            cached_primary_body_plan.take(),
            body_dirty_range,
        );
        let (left_lines, primary_body_column, _, body_stats) = compute_incremental_dynamic_column(
            engine,
            body_prepared,
            LayoutCursor::default(),
            &body_plan,
            cached_primary_body_column.take(),
            cached_left_lines.take(),
        );
        let projection = CachedDynamicProjection {
            key,
            projection: DynamicProjection {
                headline_lines,
                credit_line,
                left_lines,
                right_lines: Vec::new(),
            },
            headline_plan,
            headline_column,
            headline_title_bands,
            headline_bottom,
            headline_title_span,
            primary_body_plan: body_plan,
            primary_body_column,
            secondary_body_plan: None,
            secondary_body_column: None,
            openai_span,
            claude_span,
        };
        return (
            projection,
            DynamicProjectionCacheStats {
                dirty_bands: headline_stats.dirty_bands + body_stats.dirty_bands,
                full_recomputes: headline_stats.full_recomputes + body_stats.full_recomputes,
                ..DynamicProjectionCacheStats::default()
            },
        );
    }

    let left_region = GeoRect {
        x: layout.page.x + layout.gutter,
        y: copy_top,
        width: layout.column_width,
        height: (layout.page.bottom() - copy_top - layout.gutter).max(0.0),
    };
    let title_obstacle = BandObstacle::BandIntervals {
        bands: headline_title_bands.as_ref(),
    };

    let (left_lines, left_plan, primary_body_column, cursor, left_stats) =
        if openai_bucket_unchanged {
            let primary_body_column = cached_primary_body_column
                .take()
                .expect("cached primary body column should exist when openai bucket is unchanged");
            let cursor = cached_column_output_cursor(&primary_body_column, LayoutCursor::default());
            (
                cached_left_lines
                    .take()
                    .expect("cached left lines should exist when openai bucket is unchanged"),
                cached_primary_body_plan.take().expect(
                    "cached primary body plan should exist when openai bucket is unchanged",
                ),
                primary_body_column,
                cursor,
                DynamicColumnCacheStats::default(),
            )
        } else {
            let left_obstacles = [openai_obstacle];
            let left_dirty_range = dynamic_band_range_for_span(
                left_region,
                BODY_LINE_HEIGHT,
                dynamic_dirty_span(
                    openai_span,
                    cached_openai_span,
                    openai_bucket_unchanged,
                    openai_vertical_padding,
                ),
            );
            let left_plan = build_dynamic_column_plan_incremental(
                left_region,
                BODY_LINE_HEIGHT,
                &left_obstacles,
                ColumnSide::Left,
                cached_primary_body_plan.take(),
                left_dirty_range,
            );
            let (left_lines, primary_body_column, cursor, left_stats) =
                compute_incremental_dynamic_column(
                    engine,
                    body_prepared,
                    LayoutCursor::default(),
                    &left_plan,
                    cached_primary_body_column.take(),
                    cached_left_lines.take(),
                );
            (
                left_lines,
                left_plan,
                primary_body_column,
                cursor,
                left_stats,
            )
        };
    let right_obstacles = [title_obstacle, claude_obstacle, openai_obstacle];
    let title_dirty_range = dynamic_band_range_for_span(
        right_region,
        BODY_LINE_HEIGHT,
        if openai_bucket_unchanged {
            None
        } else {
            dynamic_union_span(headline_title_span, cached_headline_title_span)
        },
    );
    let right_logo_dirty_range = dynamic_union_band_range(
        dynamic_band_range_for_span(
            right_region,
            BODY_LINE_HEIGHT,
            dynamic_dirty_span(
                openai_span,
                cached_openai_span,
                openai_bucket_unchanged,
                openai_vertical_padding,
            ),
        ),
        dynamic_band_range_for_span(
            right_region,
            BODY_LINE_HEIGHT,
            dynamic_dirty_span(
                claude_span,
                cached_claude_span,
                claude_bucket_unchanged,
                claude_vertical_padding,
            ),
        ),
    );
    let right_plan = build_dynamic_column_plan_incremental(
        right_region,
        BODY_LINE_HEIGHT,
        &right_obstacles,
        ColumnSide::Right,
        cached_secondary_body_plan.take(),
        dynamic_union_band_range(title_dirty_range, right_logo_dirty_range),
    );
    let (right_lines, secondary_body_column, _, right_stats) = compute_incremental_dynamic_column(
        engine,
        body_prepared,
        cursor,
        &right_plan,
        cached_secondary_body_column.take(),
        cached_right_lines.take(),
    );

    let projection = CachedDynamicProjection {
        key,
        projection: DynamicProjection {
            headline_lines,
            credit_line,
            left_lines,
            right_lines,
        },
        headline_plan,
        headline_column,
        headline_title_bands,
        headline_bottom,
        headline_title_span,
        primary_body_plan: left_plan,
        primary_body_column,
        secondary_body_plan: Some(right_plan),
        secondary_body_column: Some(secondary_body_column),
        openai_span,
        claude_span,
    };
    (
        projection,
        DynamicProjectionCacheStats {
            dirty_bands: headline_stats.dirty_bands
                + left_stats.dirty_bands
                + right_stats.dirty_bands,
            full_recomputes: headline_stats.full_recomputes
                + left_stats.full_recomputes
                + right_stats.full_recomputes,
            ..DynamicProjectionCacheStats::default()
        },
    )
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

fn cached_logo_geometry(
    cache: &mut Option<CachedLogoGeometry>,
    layout_key: DynamicLayoutKey,
    points: &[Point],
    rect: GeoRect,
    angle: f32,
) -> TransformedLogoGeometry {
    let polygon = transform_points(points, rect, angle);
    let angle_q = quantize_dynamic_reflow_angle(rect, angle);
    let key = DynamicLogoGeometryKey {
        layout_key,
        angle_q,
    };
    if cache.as_ref().is_none_or(|cached| cached.key != key) {
        let reflow_angle = snapped_dynamic_reflow_angle(rect, angle_q);
        let reflow_polygon = transform_points(points, rect, reflow_angle);
        *cache = Some(CachedLogoGeometry {
            key,
            scanlines: Arc::new(PolygonBandCache::new(&reflow_polygon)),
        });
    }
    let scanlines = cache
        .as_ref()
        .expect("dynamic logo geometry cache should exist")
        .scanlines
        .clone();
    TransformedLogoGeometry { polygon, scanlines }
}

fn fragment_paint_options(
    style: &TextStyleSpec,
    line_height: f32,
    color: Color32,
) -> EguiPretextPaintOptions<'_> {
    EguiPretextPaintOptions::new(style, line_height)
        .color(color)
        .fallback_font(egui::FontId::new(
            style.size_px,
            egui::FontFamily::Proportional,
        ))
        .fallback_align(egui::Align2::LEFT_TOP)
}

fn queue_positioned_lines<'a>(
    fragment_painter: &mut PretextFragmentPainter,
    lines: impl IntoIterator<Item = &'a PositionedLine>,
    options: &EguiPretextPaintOptions<'_>,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut EguiPretextRenderer,
) {
    for line in lines {
        fragment_painter.push_fragment(
            line.x,
            line.y,
            &line.text,
            &line.glyph_runs,
            &[],
            options,
            ctx,
            engine,
            assets,
        );
    }
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
        let openai = svg_alpha_hull(svg_bytes(SvgAssetId::OpenAiLogo), LOGO_RASTER_SIZE)
            .expect("openai hull");
        let claude = svg_alpha_hull(svg_bytes(SvgAssetId::ClaudeLogo), LOGO_RASTER_SIZE)
            .expect("claude hull");
        assert!(!openai.is_empty());
        assert!(!claude.is_empty());
    }

    #[test]
    fn dirty_span_skips_unchanged_obstacles_and_unions_changed_geometry() {
        let current = Some(DynamicVerticalSpan {
            top: 100.0,
            bottom: 180.0,
        });
        let previous = Some(DynamicVerticalSpan {
            top: 92.0,
            bottom: 164.0,
        });

        assert_eq!(dynamic_dirty_span(current, previous, true, 12.0), None);
        assert_eq!(
            dynamic_dirty_span(current, previous, false, 12.0),
            Some(DynamicVerticalSpan {
                top: 80.0,
                bottom: 192.0,
            })
        );
    }

    #[test]
    fn lines_route_around_obstacle_rect() {
        let engine = PretextEngine::builder()
            .with_font_data(bundled_font_data())
            .include_system_fonts(false)
            .build();
        let prepared = engine.prepare_paragraph(
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
    fn band_interval_cache_matches_rect_obstacle_layout() {
        let engine = PretextEngine::builder()
            .with_font_data(bundled_font_data())
            .include_system_fonts(false)
            .build();
        let prepared = engine.prepare_paragraph(
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
        let horizontal_padding = (BODY_LINE_HEIGHT * 0.72).round();
        let vertical_padding = (BODY_LINE_HEIGHT * 0.12).round();
        let rect_obstacles = [BandObstacle::Rects {
            rects: std::slice::from_ref(&obstacle),
            horizontal_padding,
            vertical_padding,
        }];
        let band_cache = build_rect_band_interval_cache(
            std::slice::from_ref(&obstacle),
            region,
            BODY_LINE_HEIGHT,
            horizontal_padding,
            vertical_padding,
        );
        let cached_obstacles = [BandObstacle::BandIntervals {
            bands: band_cache.as_ref(),
        }];

        let (rect_lines, rect_cursor) = layout_column(
            &engine,
            &prepared,
            LayoutCursor::default(),
            region,
            BODY_LINE_HEIGHT,
            &rect_obstacles,
            ColumnSide::Left,
        );
        let (cached_lines, cached_cursor) = layout_column(
            &engine,
            &prepared,
            LayoutCursor::default(),
            region,
            BODY_LINE_HEIGHT,
            &cached_obstacles,
            ColumnSide::Left,
        );

        assert_eq!(cached_lines, rect_lines);
        assert_eq!(cached_cursor, rect_cursor);
    }

    #[test]
    fn incremental_column_matches_fresh_after_dirty_suffix_with_skipped_band_prefix() {
        let engine = PretextEngine::builder()
            .with_font_data(bundled_font_data())
            .include_system_fonts(false)
            .build();
        let prepared = engine.prepare_paragraph(
            "This paragraph is long enough to fill several bands so we can move a fully blocking obstacle farther down the column and verify that incremental rebuilding truncates output lines at the right visible prefix instead of using the raw band index.",
            &body_style(),
            &normal_options(),
        );
        let region = GeoRect {
            x: 20.0,
            y: 20.0,
            width: 260.0,
            height: BODY_LINE_HEIGHT * 8.0,
        };
        let full_width_band_rect = |band_index: usize| GeoRect {
            x: region.x,
            y: region.y + BODY_LINE_HEIGHT * band_index as f32,
            width: region.width,
            height: BODY_LINE_HEIGHT,
        };
        let static_block = full_width_band_rect(1);
        let initial_moving_block = full_width_band_rect(3);
        let next_moving_block = full_width_band_rect(4);
        let initial_rects = [static_block, initial_moving_block];
        let next_rects = [static_block, next_moving_block];
        let horizontal_padding = 0.0;
        let vertical_padding = 0.0;
        let initial_obstacles = [BandObstacle::Rects {
            rects: &initial_rects,
            horizontal_padding,
            vertical_padding,
        }];
        let next_obstacles = [BandObstacle::Rects {
            rects: &next_rects,
            horizontal_padding,
            vertical_padding,
        }];

        let initial_plan = build_dynamic_column_plan(
            region,
            BODY_LINE_HEIGHT,
            &initial_obstacles,
            ColumnSide::Left,
        );
        let (initial_lines, initial_column, _, _) = compute_incremental_dynamic_column(
            &engine,
            &prepared,
            LayoutCursor::default(),
            &initial_plan,
            None,
            None,
        );
        let next_plan =
            build_dynamic_column_plan(region, BODY_LINE_HEIGHT, &next_obstacles, ColumnSide::Left);
        let (incremental_lines, _, incremental_cursor, _) = compute_incremental_dynamic_column(
            &engine,
            &prepared,
            LayoutCursor::default(),
            &next_plan,
            Some(initial_column),
            Some(initial_lines.clone()),
        );
        let (fresh_lines, _, fresh_cursor, _) = compute_incremental_dynamic_column(
            &engine,
            &prepared,
            LayoutCursor::default(),
            &next_plan,
            None,
            None,
        );

        assert_eq!(incremental_lines, fresh_lines);
        assert_eq!(incremental_cursor, fresh_cursor);
        assert!(initial_lines.len() >= 2);
        assert!(incremental_lines.len() >= 2);
        assert!(Arc::ptr_eq(&initial_lines[0], &incremental_lines[0]));
        assert!(Arc::ptr_eq(&initial_lines[1], &incremental_lines[1]));
    }

    #[test]
    fn positioned_lines_keep_visual_runs_for_mixed_direction_text() {
        let engine = PretextEngine::builder()
            .with_font_data(bundled_font_data())
            .include_system_fonts(false)
            .build();
        let prepared = engine.prepare_paragraph(
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
        let engine = PretextEngine::builder()
            .with_font_data(bundled_font_data())
            .include_system_fonts(false)
            .build();
        let size = fit_headline_font_size(&engine, 620.0, 1040.0);
        let prepared = engine.prepare_paragraph(HEADLINE, &headline_style(size), &normal_options());

        assert!(size >= 22.0);
        assert!(!headline_breaks_inside_word(&engine, &prepared, 620.0));
    }

    #[test]
    fn default_width_layout_uses_compact_mode() {
        let engine = PretextEngine::builder()
            .with_font_data(bundled_font_data())
            .include_system_fonts(false)
            .build();
        let layout = build_page_layout(
            Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(960.0, 760.0)),
            &engine,
        );

        assert!(!layout.is_narrow);
        assert!(layout.is_compact);
        assert!(layout.column_width >= 420.0);
        assert!(layout.openai_rect.width < 380.0);
        assert!(layout.claude_rect.width < 320.0);
    }

    #[test]
    fn projection_reuses_cached_headline_prepare_across_angle_updates() {
        let engine = PretextEngine::builder()
            .with_font_data(bundled_font_data())
            .include_system_fonts(false)
            .build();
        let mut demo = DynamicLayoutDemo::default();
        let page_rect = Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1100.0, 760.0));

        demo.invalidate_engine_caches_if_needed(&engine);
        let layout = demo.ensure_layout(page_rect, &engine);
        let (openai_geometry, claude_geometry) = demo.frame_logo_geometries(&engine, layout);
        let _ = demo.ensure_projection(
            &engine,
            layout,
            openai_geometry.scanlines.as_ref(),
            claude_geometry.scanlines.as_ref(),
        );
        let after_first = engine.runtime_stats();

        demo.openai_logo.angle = 0.01;
        let (openai_geometry, claude_geometry) = demo.frame_logo_geometries(&engine, layout);
        let _ = demo.ensure_projection(
            &engine,
            layout,
            openai_geometry.scanlines.as_ref(),
            claude_geometry.scanlines.as_ref(),
        );
        let after_second = engine.runtime_stats();

        assert_eq!(
            after_second.prepare_with_segments_calls,
            after_first.prepare_with_segments_calls
        );
    }

    #[test]
    fn projection_reuses_cached_geometry_within_reflow_bucket() {
        let engine = PretextEngine::builder()
            .with_font_data(bundled_font_data())
            .include_system_fonts(false)
            .build();
        let mut demo = DynamicLayoutDemo::default();
        let page_rect = Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1100.0, 760.0));

        demo.invalidate_engine_caches_if_needed(&engine);
        let layout = demo.ensure_layout(page_rect, &engine);
        let (openai_geometry, claude_geometry) = demo.frame_logo_geometries(&engine, layout);
        let _ = demo.ensure_projection(
            &engine,
            layout,
            openai_geometry.scanlines.as_ref(),
            claude_geometry.scanlines.as_ref(),
        );
        let after_first = engine.runtime_stats();

        demo.openai_logo.angle = dynamic_reflow_bucket_angle(layout.openai_rect) * 0.4;
        let (openai_geometry_next, claude_geometry_next) =
            demo.frame_logo_geometries(&engine, layout);
        let _ = demo.ensure_projection(
            &engine,
            layout,
            openai_geometry_next.scanlines.as_ref(),
            claude_geometry_next.scanlines.as_ref(),
        );
        let after_second = engine.runtime_stats();

        assert!(Arc::ptr_eq(
            &openai_geometry.scanlines,
            &openai_geometry_next.scanlines
        ));
        assert_eq!(
            after_second.layout_next_line_with_runs_calls,
            after_first.layout_next_line_with_runs_calls
        );
    }

    #[test]
    fn projection_reflows_only_dirty_suffix_across_bucket_change() {
        let engine = PretextEngine::builder()
            .with_font_data(bundled_font_data())
            .include_system_fonts(false)
            .build();
        let mut demo = DynamicLayoutDemo::default();
        let page_rect = Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1100.0, 760.0));

        demo.invalidate_engine_caches_if_needed(&engine);
        let layout = demo.ensure_layout(page_rect, &engine);
        let (openai_geometry, claude_geometry) = demo.frame_logo_geometries(&engine, layout);
        let _ = demo.ensure_projection(
            &engine,
            layout,
            openai_geometry.scanlines.as_ref(),
            claude_geometry.scanlines.as_ref(),
        );
        let after_first = engine.runtime_stats();

        demo.openai_logo.angle = dynamic_reflow_bucket_angle(layout.openai_rect) * 1.2;
        let (openai_geometry_next, claude_geometry_next) =
            demo.frame_logo_geometries(&engine, layout);
        let _ = demo.ensure_projection(
            &engine,
            layout,
            openai_geometry_next.scanlines.as_ref(),
            claude_geometry_next.scanlines.as_ref(),
        );
        let after_second = engine.runtime_stats();

        let second_frame_calls = after_second
            .layout_next_line_with_runs_calls
            .saturating_sub(after_first.layout_next_line_with_runs_calls);
        assert!(second_frame_calls > 0);
        assert!(second_frame_calls < after_first.layout_next_line_with_runs_calls);
    }

    #[test]
    fn projection_reuses_headline_credit_and_left_column_when_only_claude_changes() {
        let engine = PretextEngine::builder()
            .with_font_data(bundled_font_data())
            .include_system_fonts(false)
            .build();
        let mut demo = DynamicLayoutDemo::default();
        let page_rect = Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1100.0, 760.0));

        demo.invalidate_engine_caches_if_needed(&engine);
        let layout = demo.ensure_layout(page_rect, &engine);
        let (openai_geometry, claude_geometry) = demo.frame_logo_geometries(&engine, layout);
        let _ = demo.ensure_projection(
            &engine,
            layout,
            openai_geometry.scanlines.as_ref(),
            claude_geometry.scanlines.as_ref(),
        );

        let baseline_headline = demo
            .projection_cache
            .as_ref()
            .expect("baseline projection cache")
            .projection
            .headline_lines
            .clone();
        let baseline_left = demo
            .projection_cache
            .as_ref()
            .expect("baseline projection cache")
            .projection
            .left_lines
            .clone();
        let baseline_credit = demo
            .projection_cache
            .as_ref()
            .expect("baseline projection cache")
            .projection
            .credit_line
            .clone();
        let after_first = engine.runtime_stats();

        demo.claude_logo.angle = dynamic_reflow_bucket_angle(layout.claude_rect) * 1.2;
        let (openai_geometry_next, claude_geometry_next) =
            demo.frame_logo_geometries(&engine, layout);
        let _ = demo.ensure_projection(
            &engine,
            layout,
            openai_geometry_next.scanlines.as_ref(),
            claude_geometry_next.scanlines.as_ref(),
        );
        let after_second = engine.runtime_stats();
        let next_cache = demo
            .projection_cache
            .as_ref()
            .expect("updated projection cache");

        assert!(Arc::ptr_eq(
            &openai_geometry.scanlines,
            &openai_geometry_next.scanlines
        ));
        assert_eq!(baseline_credit, next_cache.projection.credit_line);
        assert_eq!(
            baseline_headline.len(),
            next_cache.projection.headline_lines.len()
        );
        assert_eq!(baseline_left.len(), next_cache.projection.left_lines.len());
        assert!(baseline_headline
            .iter()
            .zip(&next_cache.projection.headline_lines)
            .all(|(left, right)| Arc::ptr_eq(left, right)));
        assert!(baseline_left
            .iter()
            .zip(&next_cache.projection.left_lines)
            .all(|(left, right)| Arc::ptr_eq(left, right)));

        let second_frame_calls = after_second
            .layout_next_line_with_runs_calls
            .saturating_sub(after_first.layout_next_line_with_runs_calls);
        assert!(second_frame_calls > 0);
        assert!(second_frame_calls < after_first.layout_next_line_with_runs_calls);
    }

    fn line_signature(line: &PositionedLine) -> (i32, i32, i32, String) {
        (
            line.x.round() as i32,
            line.y.round() as i32,
            line.width.round() as i32,
            line.text.clone(),
        )
    }

    fn first_line_diff<T, U>(
        left: &[T],
        right: &[U],
    ) -> Option<(
        usize,
        Option<(i32, i32, i32, String)>,
        Option<(i32, i32, i32, String)>,
    )>
    where
        T: AsRef<PositionedLine>,
        U: AsRef<PositionedLine>,
    {
        let shared = left.len().min(right.len());
        for index in 0..shared {
            if left[index].as_ref() != right[index].as_ref() {
                return Some((
                    index,
                    Some(line_signature(left[index].as_ref())),
                    Some(line_signature(right[index].as_ref())),
                ));
            }
        }
        if left.len() == right.len() {
            None
        } else {
            Some((
                shared,
                left.get(shared).map(|line| line_signature(line.as_ref())),
                right.get(shared).map(|line| line_signature(line.as_ref())),
            ))
        }
    }

    #[test]
    fn incremental_plan_build_matches_fresh_projection_after_bucket_change() {
        let engine = PretextEngine::builder()
            .with_font_data(bundled_font_data())
            .include_system_fonts(false)
            .build();
        let mut demo = DynamicLayoutDemo::default();
        let page_rect = Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1100.0, 760.0));

        demo.invalidate_engine_caches_if_needed(&engine);
        let layout = demo.ensure_layout(page_rect, &engine);
        demo.ensure_body_prepared(&engine);
        demo.ensure_headline_prepared(&engine, layout.headline_size);
        demo.ensure_credit_prepared(&engine);
        let credit_width = demo.ensure_credit_width(&engine);
        let layout_key = demo.current_layout_key(&engine, layout);

        let (openai_geometry, claude_geometry) = demo.frame_logo_geometries(&engine, layout);
        let key = DynamicProjectionKey {
            layout_key,
            openai_angle_q: quantize_dynamic_reflow_angle(
                layout.openai_rect,
                demo.openai_logo.angle,
            ),
            claude_angle_q: quantize_dynamic_reflow_angle(
                layout.claude_rect,
                demo.claude_logo.angle,
            ),
        };
        let baseline = compute_dynamic_projection(
            &engine,
            demo.body_prepared.as_ref().unwrap(),
            demo.headline_prepared.as_ref().unwrap(),
            demo.credit_prepared.as_ref().unwrap(),
            credit_width,
            layout,
            openai_geometry.scanlines.as_ref(),
            claude_geometry.scanlines.as_ref(),
            key,
            None,
        )
        .0;

        demo.openai_logo.angle = dynamic_reflow_bucket_angle(layout.openai_rect) * 1.2;
        let (openai_geometry_next, claude_geometry_next) =
            demo.frame_logo_geometries(&engine, layout);
        let key_next = DynamicProjectionKey {
            layout_key,
            openai_angle_q: quantize_dynamic_reflow_angle(
                layout.openai_rect,
                demo.openai_logo.angle,
            ),
            claude_angle_q: quantize_dynamic_reflow_angle(
                layout.claude_rect,
                demo.claude_logo.angle,
            ),
        };
        let incremental = compute_dynamic_projection(
            &engine,
            demo.body_prepared.as_ref().unwrap(),
            demo.headline_prepared.as_ref().unwrap(),
            demo.credit_prepared.as_ref().unwrap(),
            credit_width,
            layout,
            openai_geometry_next.scanlines.as_ref(),
            claude_geometry_next.scanlines.as_ref(),
            key_next,
            Some(baseline),
        )
        .0;
        let fresh = compute_dynamic_projection(
            &engine,
            demo.body_prepared.as_ref().unwrap(),
            demo.headline_prepared.as_ref().unwrap(),
            demo.credit_prepared.as_ref().unwrap(),
            credit_width,
            layout,
            openai_geometry_next.scanlines.as_ref(),
            claude_geometry_next.scanlines.as_ref(),
            key_next,
            None,
        )
        .0;

        assert!(
            incremental.projection.headline_lines == fresh.projection.headline_lines,
            "headline diff: {:?}",
            first_line_diff(
                &incremental.projection.headline_lines,
                &fresh.projection.headline_lines,
            )
        );
        assert!(
            incremental.projection.credit_line == fresh.projection.credit_line,
            "credit diff: incremental={:?} fresh={:?}",
            incremental
                .projection
                .credit_line
                .as_ref()
                .map(line_signature),
            fresh.projection.credit_line.as_ref().map(line_signature),
        );
        assert!(
            incremental.projection.left_lines == fresh.projection.left_lines,
            "left diff: {:?}",
            first_line_diff(
                &incremental.projection.left_lines,
                &fresh.projection.left_lines
            )
        );
        assert!(
            incremental.projection.right_lines == fresh.projection.right_lines,
            "right diff: {:?}",
            first_line_diff(
                &incremental.projection.right_lines,
                &fresh.projection.right_lines
            )
        );
    }

    #[test]
    fn headline_paint_style_cache_reuses_arc_for_same_size() {
        let mut demo = DynamicLayoutDemo::default();
        let first = Arc::clone(demo.ensure_headline_paint_style(64.0));
        let second = Arc::clone(demo.ensure_headline_paint_style(64.0));
        let third = Arc::clone(demo.ensure_headline_paint_style(68.0));

        assert!(Arc::ptr_eq(&first, &second));
        assert!(!Arc::ptr_eq(&first, &third));
    }
}
