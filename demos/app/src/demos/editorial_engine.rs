use std::collections::HashMap;
use std::time::Duration;

use eframe::egui;
use egui::{
    Color32, ColorImage, CornerRadius, CursorIcon, Rect, Sense, Stroke, TextureHandle,
    TextureOptions,
};
#[cfg(test)]
use pretext::BidiDirection;
use pretext::{
    LayoutCursor, LayoutLineGlyphRun, LayoutLineVisualRun, PrepareOptions,
    PreparedTextWithSegments, PretextEngine, WhiteSpaceMode,
};
use pretext_egui::AssetRegistry;

use crate::demos::text_runs::paint_glyph_runs;
use crate::demos::DemoWindow;
use crate::geometry::{Interval, Point, Rect as GeoRect};

const HEADLINE: &str = "THE FUTURE OF TEXT\u{2002}LAYOUT IS NOT CSS";
const HINT_TEXT: &str = "Drag the orbs \u{00b7} Click to pause \u{00b7} Zero DOM reads";
const CREDIT_TEXT: &str = "Made by @somnai_dreams";
const BODY_TEXT: &str = r#"The web renders text through a pipeline that was designed thirty years ago for static documents. A browser loads a font, shapes the text into glyphs, measures their combined width, determines where lines break, and positions each line vertically. Every step depends on the previous one. Every step requires the rendering engine to consult its internal layout tree — a structure so expensive to maintain that browsers guard access to it behind synchronous reflow barriers that can freeze the main thread for tens of milliseconds at a time.

For a paragraph in a blog post, this pipeline is invisible. The browser loads, lays out, and paints before the reader’s eye has traveled from the address bar to the first word. But the web is no longer a collection of static documents. It is a platform for applications, and those applications need to know about text in ways the original pipeline never anticipated.

A messaging application needs to know the exact height of every message bubble before rendering a virtualized list. A masonry layout needs the height of every card to position them without overlap. An editorial page needs text to flow around images, advertisements, and interactive elements. A responsive dashboard needs to resize and reflow text in real time as the user drags a panel divider.

Every one of these operations requires text measurement. And every text measurement on the web today requires a synchronous layout reflow. The cost is devastating. Measuring the height of a single text block forces the browser to recalculate the position of every element on the page. When you measure five hundred text blocks in sequence, you trigger five hundred full layout passes. This pattern, known as layout thrashing, is the single largest source of jank on the modern web.

Chrome DevTools will flag it with angry red bars. Lighthouse will dock your performance score. But the developer has no alternative — CSS provides no API for computing text height without rendering it. The information is locked behind the DOM, and the DOM makes you pay for every answer.

Developers have invented increasingly desperate workarounds. Estimated heights replace real measurements with guesses, causing content to visibly jump when the guess is wrong. ResizeObserver watches elements for size changes, but it fires asynchronously and always at least one frame too late. IntersectionObserver tracks visibility but says nothing about dimensions. Content-visibility allows the browser to skip rendering off-screen elements, but it breaks scroll position and accessibility. Each workaround addresses one symptom while introducing new problems.

The CSS Shapes specification, finalized in 2014, was supposed to bring magazine-style text wrap to the web. It allows text to flow around a defined shape — a circle, an ellipse, a polygon, even an image alpha channel. On paper, it was the answer. In practice, it is remarkably limited. CSS Shapes only works with floated elements. Text can only wrap on one side of the shape. The shape must be defined statically in CSS — you cannot animate it or change it dynamically without triggering a full layout reflow. And because it operates within the browser’s layout engine, you have no access to the resulting line geometry. You cannot determine where each line of text starts and ends, how many lines were generated, or what the total height of the shaped text block is.

The editorial layouts we see in print magazines — text flowing around photographs, pull quotes interrupting the column, multiple columns with seamless text handoff — have remained out of reach for the web. Not because they are conceptually difficult, but because the performance cost of implementing them with DOM measurement makes them impractical. A two-column editorial layout that reflows text around three obstacle shapes requires measuring and positioning hundreds of text lines. At thirty milliseconds per measurement, this would take seconds — an eternity for a render frame.

What if text measurement did not require the DOM at all? What if you could compute exactly where every line of text would break, exactly how wide each line would be, and exactly how tall the entire text block would be, using nothing but arithmetic?

This is the core insight of pretext. The browser’s canvas API includes a measureText method that returns the width of any string in any font without triggering a layout reflow. Canvas measurement uses the same font engine as DOM rendering — the results are identical. But because it operates outside the layout tree, it carries no reflow penalty.

Pretext exploits this asymmetry. When text first appears, pretext measures every word once via canvas and caches the widths. After this preparation phase, layout is pure arithmetic: walk the cached widths, track the running line width, insert line breaks when the width exceeds the maximum, and sum the line heights. No DOM. No reflow. No layout tree access.

The performance improvement is not incremental. Measuring five hundred text blocks with DOM methods costs fifteen to thirty milliseconds and triggers five hundred layout reflows. With pretext, the same operation costs 0.05 milliseconds and triggers zero reflows. This is a three hundred to six hundred times improvement. But even that number understates the impact, because pretext’s cost does not scale with page complexity — it is independent of how many other elements exist on the page.

With DOM-free text measurement, an entire class of previously impractical interfaces becomes trivial. Text can flow around arbitrary shapes, not because the browser’s layout engine supports it, but because you control the line widths directly. For each line of text, you compute which horizontal intervals are blocked by obstacles, subtract them from the available width, and pass the remaining width to the layout engine. The engine returns the text that fits, and you position the line at the correct offset.

This is exactly what CSS Shapes tried to accomplish, but with none of its limitations. Obstacles can be any shape — rectangles, circles, arbitrary polygons, even the alpha channel of an image. Text wraps on both sides simultaneously. Obstacles can move, animate, or be dragged by the user, and the text reflows instantly because the layout computation takes less than a millisecond.

Shrinkwrap is another capability that CSS cannot express. Given a block of multiline text, what is the narrowest width that preserves the current line count? CSS offers fit-content, which works for single lines but always leaves dead space for multiline text. Pretext solves this with a binary search over widths: narrow until the line count increases, then back off. The result is the tightest possible bounding box — perfect for chat message bubbles, image captions, and tooltip text.

Virtualized text rendering becomes exact rather than estimated. A virtual list needs to know the height of items before they enter the viewport, so it can position them correctly and calculate scroll extent. Without pretext, you must either render items off-screen to measure them (defeating the purpose of virtualization) or estimate heights and accept visual jumping when items enter the viewport with different heights than predicted. Pretext computes exact heights without creating any DOM elements, enabling perfect virtualization with zero visual artifacts.

Multi-column text flow with cursor handoff is perhaps the most striking capability. The left column consumes text until it reaches the bottom, then hands its cursor to the right column. The right column picks up exactly where the left column stopped, with no duplication, no gap, and perfect line breaking at the column boundary. This is how newspapers and magazines work on paper, but it has never been achievable on the web without extreme hacks involving multiple elements, hidden overflow, and JavaScript-managed content splitting.

Pretext makes it trivial. Call layoutNextLine in a loop for the first column, using the column width. When the column is full, take the returned cursor and start a new loop for the second column. The cursor carries the exact position in the prepared text — which segment, which grapheme within that segment. The second column continues seamlessly from the first.

Adaptive headline sizing is a detail that separates professional typography from amateur layout. The headline should be as large as possible without breaking any word across lines. This requires a binary search: try a font size, measure the text, check if any line breaks occur within a word, and adjust. With DOM measurement, each iteration costs a reflow. With pretext, each iteration is a microsecond of arithmetic.

Real-time text reflow around animated obstacles is the ultimate stress test. The demonstration you are reading right now renders text that flows around multiple moving objects simultaneously, every frame, at sixty frames per second. Each frame, the layout engine computes obstacle intersections for every line of text, determines the available horizontal slots, lays out each line at the correct width and position, and updates the DOM with the results. The total computation time is typically under half a millisecond.

The glowing orbs drifting across this page are not decorative — they are the demonstration. Each orb is a circular obstacle. For every line of text, the engine checks whether the line’s vertical band intersects each orb. If it does, it computes the blocked horizontal interval and subtracts it from the available width. The remaining width might be split into two or more segments — and the engine fills every viable slot, flowing text on both sides of the obstacle simultaneously. This is something CSS Shapes cannot do at all.

All of this runs without a single DOM measurement. The line positions, widths, and text contents are computed entirely in JavaScript using cached font metrics. The only DOM writes are setting the left, top, and textContent of each line element — the absolute minimum required to show text on screen. The browser never needs to compute layout because all positioning is explicit.

This performance characteristic has profound implications for the web platform. For thirty years, the browser has been the gatekeeper of text information. If you wanted to know anything about how text would render — its width, its height, where its lines break — you had to ask the browser, and the browser made you pay for the answer with a layout reflow. This created an artificial scarcity of text information that constrained what interfaces could do.

Pretext removes that constraint. Text information becomes abundant and cheap. You can ask how text would look at a thousand different widths in the time it used to take to ask about one. You can recompute text layout every frame, every drag event, every pixel of window resize, without any performance concern.

The implications extend beyond layout into composition. When you have instant text measurement, you can build compositing engines that combine text with graphics, animation, and interaction in ways that were previously reserved for game engines and native applications. Text becomes a first-class participant in the visual composition, not a static block that the rest of the interface must work around.

Imagine a data visualization where labels reflow around chart elements as the user zooms and pans. Imagine a collaborative document editor where text flows around embedded widgets, images, and annotations placed by other users, updating live as they move things around. Imagine a map application where place names wrap intelligently around geographic features rather than overlapping them. These are not hypothetical — they are engineering problems that become solvable when text measurement costs a microsecond instead of thirty milliseconds.

The open web deserves typography that matches its ambition. We build applications that rival native software in every dimension except text. Our animations are smooth, our interactions are responsive, our graphics are stunning — but our text sits in rigid boxes, unable to flow around obstacles, unable to adapt to dynamic layouts, unable to participate in the fluid compositions that define modern interface design.

This is what changes when text measurement becomes free. Not slightly better — categorically different. The interfaces that were too expensive to build become trivial. The layouts that existed only in print become interactive. The text that sat in boxes begins to flow.

The web has been waiting thirty years for this. A fifteen kilobyte library with zero dependencies delivers it. No browser API changes needed. No specification process. No multi-year standardization timeline. Just math, cached measurements, and the audacity to ask: what if we simply stopped asking the DOM?

Fifteen kilobytes. Zero dependencies. Zero DOM reads. And the text flows."#;
const PULL_QUOTE_TEXTS: [&str; 2] = [
    "\u{201c}The performance improvement is not incremental \u{2014} it is categorical. 0.05ms versus 30ms. Zero reflows versus five hundred.\u{201d}",
    "\u{201c}Text becomes a first-class participant in the visual composition \u{2014} not a static block, but a fluid material that adapts in real time.\u{201d}",
];

const BODY_LINE_HEIGHT: f32 = 30.0;
const HEADLINE_MAX_WIDTH: f32 = 1000.0;
const HEADLINE_MIN_SIZE: i32 = 20;
const HEADLINE_MAX_SIZE: i32 = 92;
const HEADLINE_NARROW_MAX_SIZE: i32 = 38;
const QUOTE_LINE_HEIGHT: f32 = 27.0;
const QUOTE_TEXT_SIZE: f32 = 19.0;
const DROP_CAP_LINES: usize = 3;
const MIN_SLOT_WIDTH: f32 = 50.0;
const GUTTER: f32 = 48.0;
const COL_GAP: f32 = 40.0;
const BOTTOM_GAP: f32 = 20.0;
const NARROW_BREAKPOINT: f32 = 760.0;
const NARROW_GUTTER: f32 = 20.0;
const NARROW_COL_GAP: f32 = 20.0;
const NARROW_BOTTOM_GAP: f32 = 16.0;
const NARROW_ORB_SCALE: f32 = 0.58;
const NARROW_ACTIVE_ORBS: usize = 3;
const PAGE_MIN_HEIGHT: f32 = 520.0;
const FRAME_INTERVAL: Duration = Duration::from_millis(16);
const REFLOW_BUCKET_PX: f32 = 2.0;
const UNBOUNDED_WIDTH: f32 = 100_000.0;
const ORB_SHADOW_1_BLUR: f32 = 60.0;
const ORB_SHADOW_1_SPREAD: f32 = 15.0;
const ORB_SHADOW_1_ALPHA: f32 = 0.18;
const ORB_SHADOW_2_BLUR: f32 = 120.0;
const ORB_SHADOW_2_SPREAD: f32 = 40.0;
const ORB_SHADOW_2_ALPHA: f32 = 0.07;
const BODY_START_CURSOR: LayoutCursor = LayoutCursor {
    segment_index: 0,
    grapheme_index: 1,
};

pub struct EditorialEngineDemo {
    open: bool,
    orbs: Vec<Orb>,
    drag: Option<DragState>,
    last_time: Option<f64>,
    body_prepared: Option<PreparedTextWithSegments>,
    pull_quote_prepared: Option<Vec<PreparedTextWithSegments>>,
    drop_cap_prepared: Option<PreparedTextWithSegments>,
    drop_cap_total_width: Option<f32>,
    layout_cache: Option<CachedEditorialLayout>,
    projection_cache: Option<CachedEditorialProjection>,
    background_texture: Option<SizedTexture>,
    orb_textures: HashMap<OrbTextureKey, SizedTexture>,
}

#[derive(Clone, Copy)]
struct OrbDefinition {
    fx: f32,
    fy: f32,
    radius: f32,
    vx: f32,
    vy: f32,
    rgb: [u8; 3],
}

#[derive(Clone, Copy)]
struct Orb {
    x: f32,
    y: f32,
    radius: f32,
    vx: f32,
    vy: f32,
    rgb: [u8; 3],
    paused: bool,
}

#[derive(Clone, Copy)]
struct DragState {
    orb_index: usize,
    start_pointer: Point,
    last_pointer: Point,
    start_orb: Point,
}

#[derive(Clone, Copy)]
struct PullQuotePlacement {
    col_idx: usize,
    y_frac: f32,
    w_frac: f32,
    side: PullQuoteSide,
}

#[derive(Clone)]
struct SizedTexture {
    size: [usize; 2],
    texture: TextureHandle,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct OrbTextureKey {
    diameter_px: u16,
    rgb: [u8; 3],
}

#[derive(Clone, Copy)]
enum PullQuoteSide {
    Left,
    Right,
}

#[derive(Clone, Copy)]
struct CircleObstacle {
    center: Point,
    radius: f32,
    horizontal_padding: f32,
    vertical_padding: f32,
}

#[derive(Clone, Copy)]
struct RectObstacle {
    rect: GeoRect,
}

#[derive(Clone)]
struct HeadlineFit {
    font_size: f32,
    line_height: f32,
    lines: Vec<PositionedLine>,
}

#[derive(Clone)]
struct PullQuoteBox {
    rect: GeoRect,
    lines: Vec<PositionedLine>,
    col_idx: usize,
}

#[derive(Clone)]
struct EditorialProjection {
    headline_lines: Vec<PositionedLine>,
    body_lines: Vec<PositionedLine>,
    pull_quotes: Vec<PullQuoteBox>,
    drop_cap_line: PositionedLine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct EditorialLayoutKey {
    engine_revision: u64,
    page_width_q: u32,
    page_height_q: u32,
    drop_cap_width_q: u32,
}

#[derive(Clone)]
struct CachedEditorialLayout {
    key: EditorialLayoutKey,
    layout: EditorialLayout,
}

#[derive(Clone)]
struct CachedEditorialProjection {
    layout_key: EditorialLayoutKey,
    reflow_signature: u64,
    projection: EditorialProjection,
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

#[derive(Clone)]
struct EditorialLayout {
    page: GeoRect,
    is_narrow: bool,
    gutter: f32,
    col_gap: f32,
    bottom_gap: f32,
    orb_radius_scale: f32,
    active_orb_count: usize,
    column_count: usize,
    content_left: f32,
    column_width: f32,
    body_top: f32,
    body_height: f32,
    headline_origin: Point,
    headline_fit: HeadlineFit,
    body_columns: Vec<GeoRect>,
    drop_cap_rect: GeoRect,
}

impl Default for EditorialEngineDemo {
    fn default() -> Self {
        Self {
            open: false,
            orbs: Vec::new(),
            drag: None,
            last_time: None,
            body_prepared: None,
            pull_quote_prepared: None,
            drop_cap_prepared: None,
            drop_cap_total_width: None,
            layout_cache: None,
            projection_cache: None,
            background_texture: None,
            orb_textures: HashMap::new(),
        }
    }
}

impl DemoWindow for EditorialEngineDemo {
    fn title(&self) -> &str {
        "Editorial Engine"
    }

    fn is_open(&self) -> bool {
        self.open
    }

    fn set_open(&mut self, open: bool) {
        self.open = open;
        if !open {
            self.last_time = None;
            self.drag = None;
        }
    }

    fn show(&mut self, ctx: &egui::Context, engine: &PretextEngine, assets: &mut AssetRegistry) {
        let mut open = self.open;
        egui::Window::new(self.title())
            .open(&mut open)
            .resizable(true)
            .default_size(egui::vec2(1220.0, 820.0))
            .show(ctx, |ui| {
                let now = ctx.input(|input| input.time);
                let available = ui.available_size();
                let page_width = available.x.max(360.0);
                let page_height = available.y.max(PAGE_MIN_HEIGHT);
                let (page_rect, _) = ui.allocate_exact_size(
                    egui::vec2(page_width, page_height),
                    Sense::click_and_drag(),
                );
                let page = GeoRect {
                    x: page_rect.left(),
                    y: page_rect.top(),
                    width: page_rect.width(),
                    height: page_rect.height(),
                };

                self.ensure_orbs(page);
                let drop_cap_total_width = self.ensure_drop_cap_total_width(engine);
                let body_prepared = self.ensure_body_prepared(engine).clone();
                let pull_quote_prepared = self.ensure_pull_quote_prepared(engine).clone();
                let drop_cap_prepared = self.ensure_drop_cap_prepared(engine).clone();

                let layout = self.ensure_layout(engine, page, drop_cap_total_width);
                let dragged_orb_index =
                    handle_orb_interaction(ui, page_rect, &layout, &mut self.orbs, &mut self.drag);
                let animating = update_orbs(
                    now,
                    &layout,
                    &mut self.orbs,
                    &mut self.last_time,
                    dragged_orb_index,
                );
                let current_orbs = self.orbs.clone();
                let projection = self.ensure_projection(
                    engine,
                    &body_prepared,
                    &pull_quote_prepared,
                    &drop_cap_prepared,
                    &layout,
                    &current_orbs,
                );

                let painter = ui.painter().clone();
                let background_texture = self.ensure_background_texture(
                    ctx,
                    [
                        page_rect.width().round().max(1.0) as usize,
                        page_rect.height().round().max(1.0) as usize,
                    ],
                );
                let orb_requests = self
                    .orbs
                    .iter()
                    .take(layout.active_orb_count)
                    .map(|orb| (orb.radius * layout.orb_radius_scale, orb.rgb))
                    .collect::<Vec<_>>();
                let orb_textures = orb_requests
                    .into_iter()
                    .map(|(radius, rgb)| self.ensure_orb_texture(ctx, radius, rgb))
                    .collect::<Vec<_>>();

                paint_editorial_background(&painter, page_rect, &background_texture);
                paint_projection(&painter, &projection, &layout, ctx, engine, assets);
                paint_orbs(&painter, &layout, &self.orbs, &orb_textures);
                paint_editorial_chrome(&painter, page_rect, layout.is_narrow, ctx, engine, assets);

                if animating || self.drag.is_some() {
                    ctx.request_repaint_after(FRAME_INTERVAL);
                }
            });
        self.open = open;
    }
}

impl EditorialEngineDemo {
    fn ensure_body_prepared(&mut self, engine: &PretextEngine) -> &PreparedTextWithSegments {
        if self.body_prepared.is_none() {
            self.body_prepared =
                Some(engine.prepare_with_segments(BODY_TEXT, &body_style(), &normal_options()));
        }
        self.body_prepared
            .as_ref()
            .expect("editorial body should exist")
    }

    fn ensure_pull_quote_prepared(
        &mut self,
        engine: &PretextEngine,
    ) -> &Vec<PreparedTextWithSegments> {
        if self.pull_quote_prepared.is_none() {
            self.pull_quote_prepared = Some(
                PULL_QUOTE_TEXTS
                    .iter()
                    .map(|text| {
                        engine.prepare_with_segments(text, &quote_style(), &normal_options())
                    })
                    .collect(),
            );
        }
        self.pull_quote_prepared
            .as_ref()
            .expect("editorial pull quotes should exist")
    }

    fn ensure_drop_cap_prepared(&mut self, engine: &PretextEngine) -> &PreparedTextWithSegments {
        if self.drop_cap_prepared.is_none() {
            self.drop_cap_prepared = Some(engine.prepare_with_segments(
                &BODY_TEXT.chars().next().unwrap_or('T').to_string(),
                &drop_cap_style(),
                &normal_options(),
            ));
        }
        self.drop_cap_prepared
            .as_ref()
            .expect("editorial drop cap should exist")
    }

    fn ensure_drop_cap_total_width(&mut self, engine: &PretextEngine) -> f32 {
        if self.drop_cap_total_width.is_none() {
            let width = measure_single_line_width(engine, self.ensure_drop_cap_prepared(engine));
            self.drop_cap_total_width = Some(width.ceil() + 10.0);
        }
        self.drop_cap_total_width
            .expect("editorial drop cap width should exist")
    }

    fn ensure_orbs(&mut self, page: GeoRect) {
        if !self.orbs.is_empty() {
            return;
        }

        self.orbs = orb_definitions()
            .iter()
            .map(|definition| Orb {
                x: page.x + page.width * definition.fx,
                y: page.y + page.height * definition.fy,
                radius: definition.radius,
                vx: definition.vx,
                vy: definition.vy,
                rgb: definition.rgb,
                paused: false,
            })
            .collect();
    }

    fn ensure_background_texture(
        &mut self,
        ctx: &egui::Context,
        logical_size: [usize; 2],
    ) -> TextureHandle {
        let clamped_size = [logical_size[0].max(1), logical_size[1].max(1)];
        if let Some(cached) = &self.background_texture {
            if cached.size == clamped_size {
                return cached.texture.clone();
            }
        }

        let image = editorial_background_image(clamped_size);
        let texture = ctx.load_texture(
            format!(
                "editorial/background/{}x{}",
                clamped_size[0], clamped_size[1]
            ),
            image,
            TextureOptions::LINEAR,
        );
        self.background_texture = Some(SizedTexture {
            size: clamped_size,
            texture: texture.clone(),
        });
        texture
    }

    fn ensure_orb_texture(
        &mut self,
        ctx: &egui::Context,
        radius: f32,
        rgb: [u8; 3],
    ) -> SizedTexture {
        let diameter_px = (radius * 2.0).round().clamp(2.0, u16::MAX as f32) as u16;
        let key = OrbTextureKey { diameter_px, rgb };
        if let Some(texture) = self.orb_textures.get(&key) {
            return texture.clone();
        }

        let image = orb_color_image(radius.max(1.0), rgb);
        let texture = ctx.load_texture(
            format!(
                "editorial/orb/{}-{}-{}-{}",
                diameter_px, rgb[0], rgb[1], rgb[2]
            ),
            image,
            TextureOptions::LINEAR,
        );
        let sized = SizedTexture {
            size: [texture.size()[0], texture.size()[1]],
            texture,
        };
        self.orb_textures.insert(key, sized.clone());
        sized
    }

    fn ensure_layout(
        &mut self,
        engine: &PretextEngine,
        page: GeoRect,
        drop_cap_total_width: f32,
    ) -> EditorialLayout {
        let key = EditorialLayoutKey {
            engine_revision: engine.revision(),
            page_width_q: quantize_editorial_value(page.width),
            page_height_q: quantize_editorial_value(page.height),
            drop_cap_width_q: quantize_editorial_value(drop_cap_total_width),
        };
        let should_rebuild = self
            .layout_cache
            .as_ref()
            .is_none_or(|cached| cached.key != key);
        if should_rebuild {
            self.layout_cache = Some(CachedEditorialLayout {
                key,
                layout: build_editorial_layout(page, engine, drop_cap_total_width),
            });
            self.projection_cache = None;
        }

        self.layout_cache
            .as_ref()
            .expect("editorial layout cache should exist")
            .layout
            .clone()
    }

    fn ensure_projection(
        &mut self,
        engine: &PretextEngine,
        body_prepared: &PreparedTextWithSegments,
        pull_quote_prepared: &[PreparedTextWithSegments],
        drop_cap_prepared: &PreparedTextWithSegments,
        layout: &EditorialLayout,
        orbs: &[Orb],
    ) -> EditorialProjection {
        let layout_key = self
            .layout_cache
            .as_ref()
            .map(|cached| cached.key)
            .unwrap_or(EditorialLayoutKey {
                engine_revision: engine.revision(),
                page_width_q: quantize_editorial_value(layout.page.width),
                page_height_q: quantize_editorial_value(layout.page.height),
                drop_cap_width_q: quantize_editorial_value(layout.drop_cap_rect.width),
            });
        let reflow_signature = editorial_reflow_signature(layout, orbs);
        if let Some(cached) = &self.projection_cache {
            if cached.layout_key == layout_key && cached.reflow_signature == reflow_signature {
                return cached.projection.clone();
            }
        }

        let projection = compute_editorial_projection(
            engine,
            body_prepared,
            pull_quote_prepared,
            drop_cap_prepared,
            layout,
            orbs,
        );
        self.projection_cache = Some(CachedEditorialProjection {
            layout_key,
            reflow_signature,
            projection: projection.clone(),
        });
        projection
    }
}

fn normal_options() -> PrepareOptions {
    PrepareOptions {
        white_space: WhiteSpaceMode::Normal,
        paragraph_direction: pretext::ParagraphDirection::Auto,
    }
}

fn quantize_editorial_value(value: f32) -> u32 {
    (value.max(0.0) * 4.0).round() as u32
}

fn quantize_reflow_bucket(value: f32) -> i32 {
    (value / REFLOW_BUCKET_PX).round() as i32
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
        size_px: 18.0,
        weight: 400,
        italic: false,
    }
}

fn quote_style() -> pretext::TextStyleSpec {
    pretext::TextStyleSpec {
        families: serif_families(),
        size_px: QUOTE_TEXT_SIZE,
        weight: 400,
        italic: true,
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

fn drop_cap_style() -> pretext::TextStyleSpec {
    pretext::TextStyleSpec {
        families: serif_families(),
        size_px: BODY_LINE_HEIGHT * DROP_CAP_LINES as f32 - 4.0,
        weight: 700,
        italic: false,
    }
}

fn chrome_style(size_px: f32) -> pretext::TextStyleSpec {
    pretext::TextStyleSpec {
        families: sans_families(),
        size_px,
        weight: 400,
        italic: false,
    }
}

fn editorial_background_image(size: [usize; 2]) -> ColorImage {
    let width = size[0].max(1);
    let height = size[1].max(1);
    let cx = width as f32 * 0.5;
    let cy = height as f32 * 0.4;
    let base_rx = cx.max(width as f32 - cx);
    let base_ry = cy.max(height as f32 - cy);

    let corners = [
        (0.0, 0.0),
        (width as f32, 0.0),
        (0.0, height as f32),
        (width as f32, height as f32),
    ];
    let scale = corners
        .into_iter()
        .map(|(x, y)| {
            let dx = (x - cx) / base_rx.max(1.0);
            let dy = (y - cy) / base_ry.max(1.0);
            (dx * dx + dy * dy).sqrt()
        })
        .fold(1.0f32, f32::max);
    let rx = base_rx * scale;
    let ry = base_ry * scale;

    let inner = [15u8, 15u8, 20u8];
    let outer = [10u8, 10u8, 12u8];
    let pixels = (0..height)
        .flat_map(|y| {
            (0..width).map(move |x| {
                let dx = (x as f32 + 0.5 - cx) / rx.max(1.0);
                let dy = (y as f32 + 0.5 - cy) / ry.max(1.0);
                let t = (dx * dx + dy * dy).sqrt().clamp(0.0, 1.0);
                Color32::from_rgb(
                    mix_u8(inner[0], outer[0], t),
                    mix_u8(inner[1], outer[1], t),
                    mix_u8(inner[2], outer[2], t),
                )
            })
        })
        .collect();
    ColorImage::new([width, height], pixels)
}

fn orb_color_image(radius: f32, rgb: [u8; 3]) -> ColorImage {
    let shadow_extent = (ORB_SHADOW_2_SPREAD + ORB_SHADOW_2_BLUR).ceil();
    let orb_diameter = (radius * 2.0).ceil();
    let size = (orb_diameter + shadow_extent * 2.0).max(4.0) as usize;
    let center = size as f32 * 0.5;
    let element_left = center - radius;
    let element_top = center - radius;
    let gradient_center_x = element_left + radius * 0.7;
    let gradient_center_y = element_top + radius * 0.7;
    let gradient_radius = [
        (gradient_center_x - element_left).hypot(gradient_center_y - element_top),
        (gradient_center_x - (element_left + radius * 2.0)).hypot(gradient_center_y - element_top),
        (gradient_center_x - element_left).hypot(gradient_center_y - (element_top + radius * 2.0)),
        (gradient_center_x - (element_left + radius * 2.0))
            .hypot(gradient_center_y - (element_top + radius * 2.0)),
    ]
    .into_iter()
    .fold(0.0f32, f32::max)
    .max(1.0);

    let pixels = (0..size)
        .flat_map(|y| {
            (0..size).map(move |x| {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;
                let dx = px - center;
                let dy = py - center;
                let dist = (dx * dx + dy * dy).sqrt();

                let shadow_1 =
                    css_like_shadow_alpha(dist, radius, ORB_SHADOW_1_SPREAD, ORB_SHADOW_1_BLUR)
                        * ORB_SHADOW_1_ALPHA;
                let shadow_2 =
                    css_like_shadow_alpha(dist, radius, ORB_SHADOW_2_SPREAD, ORB_SHADOW_2_BLUR)
                        * ORB_SHADOW_2_ALPHA;

                let fill_alpha = if dist <= radius {
                    let gradient_dist = ((px - gradient_center_x).powi(2)
                        + (py - gradient_center_y).powi(2))
                    .sqrt();
                    let t = gradient_dist / gradient_radius;
                    if t <= 0.55 {
                        0.35 + (0.12 - 0.35) * (t / 0.55)
                    } else if t <= 0.72 {
                        0.12 * (1.0 - (t - 0.55) / 0.17)
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };

                let alpha = (shadow_1 + shadow_2 + fill_alpha).clamp(0.0, 1.0);
                Color32::from_rgba_unmultiplied(
                    rgb[0],
                    rgb[1],
                    rgb[2],
                    (alpha * 255.0).round() as u8,
                )
            })
        })
        .collect();

    ColorImage::new([size, size], pixels)
}

fn css_like_shadow_alpha(dist: f32, radius: f32, spread: f32, blur: f32) -> f32 {
    let solid_radius = radius + spread;
    if dist <= solid_radius {
        return 1.0;
    }

    let delta = dist - solid_radius;
    if delta >= blur {
        return 0.0;
    }

    (-4.5 * (delta / blur.max(1.0)).powi(2)).exp()
}

fn mix_u8(a: u8, b: u8, t: f32) -> u8 {
    ((a as f32) + ((b as f32) - (a as f32)) * t)
        .round()
        .clamp(0.0, 255.0) as u8
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

fn fit_headline(
    engine: &PretextEngine,
    max_width: f32,
    max_height: f32,
    max_size: i32,
) -> HeadlineFit {
    let mut low = HEADLINE_MIN_SIZE;
    let mut high = max_size.max(HEADLINE_MIN_SIZE);
    let mut best_size = HEADLINE_MIN_SIZE as f32;
    let mut best_line_height = (HEADLINE_MIN_SIZE as f32 * 0.93).round();
    let mut best_lines = Vec::new();

    while low <= high {
        let size = (low + high) / 2;
        let line_height = (size as f32 * 0.93).round();
        let prepared =
            engine.prepare_with_segments(HEADLINE, &headline_style(size as f32), &normal_options());
        let mut line_count = 0usize;
        let breaks_word = headline_breaks_inside_word(engine, &prepared, max_width);
        engine.walk_line_ranges(&prepared, max_width, |_| {
            line_count += 1;
        });

        let total_height = line_count as f32 * line_height;
        if !breaks_word && total_height <= max_height {
            best_size = size as f32;
            best_line_height = line_height;
            let layout = engine.layout_with_lines(&prepared, max_width, line_height);
            best_lines = layout
                .lines
                .into_iter()
                .enumerate()
                .map(|(index, line)| PositionedLine {
                    x: 0.0,
                    y: index as f32 * line_height,
                    width: line.width,
                    text: line.text.clone(),
                    visual_runs: engine.line_visual_runs(&prepared, &line),
                    glyph_runs: engine.line_glyph_runs(&prepared, &line),
                })
                .collect();
            low = size + 1;
        } else {
            high = size - 1;
        }
    }

    HeadlineFit {
        font_size: best_size,
        line_height: best_line_height,
        lines: best_lines,
    }
}

fn orb_definitions() -> [OrbDefinition; 5] {
    [
        OrbDefinition {
            fx: 0.52,
            fy: 0.22,
            radius: 110.0,
            vx: 24.0,
            vy: 16.0,
            rgb: [196, 163, 90],
        },
        OrbDefinition {
            fx: 0.18,
            fy: 0.48,
            radius: 85.0,
            vx: -19.0,
            vy: 26.0,
            rgb: [100, 140, 255],
        },
        OrbDefinition {
            fx: 0.74,
            fy: 0.58,
            radius: 95.0,
            vx: 16.0,
            vy: -21.0,
            rgb: [232, 100, 130],
        },
        OrbDefinition {
            fx: 0.38,
            fy: 0.72,
            radius: 75.0,
            vx: -26.0,
            vy: -14.0,
            rgb: [80, 200, 140],
        },
        OrbDefinition {
            fx: 0.86,
            fy: 0.18,
            radius: 65.0,
            vx: -13.0,
            vy: 19.0,
            rgb: [150, 100, 220],
        },
    ]
}

fn pull_quote_placements() -> [PullQuotePlacement; 2] {
    [
        PullQuotePlacement {
            col_idx: 0,
            y_frac: 0.48,
            w_frac: 0.52,
            side: PullQuoteSide::Right,
        },
        PullQuotePlacement {
            col_idx: 1,
            y_frac: 0.32,
            w_frac: 0.5,
            side: PullQuoteSide::Left,
        },
    ]
}

fn build_editorial_layout(
    page: GeoRect,
    engine: &PretextEngine,
    drop_cap_total_width: f32,
) -> EditorialLayout {
    let is_narrow = page.width < NARROW_BREAKPOINT;
    let gutter = if is_narrow { NARROW_GUTTER } else { GUTTER };
    let col_gap = if is_narrow { NARROW_COL_GAP } else { COL_GAP };
    let bottom_gap = if is_narrow {
        NARROW_BOTTOM_GAP
    } else {
        BOTTOM_GAP
    };
    let orb_radius_scale = if is_narrow { NARROW_ORB_SCALE } else { 1.0 };
    let active_orb_count = if is_narrow {
        NARROW_ACTIVE_ORBS
    } else {
        orb_definitions().len()
    };

    let headline_origin = Point {
        x: page.x + gutter,
        y: page.y + gutter,
    };
    let headline_width =
        (page.width - gutter * 2.0 - if is_narrow { 12.0 } else { 0.0 }).min(HEADLINE_MAX_WIDTH);
    let max_headline_height = (page.height * if is_narrow { 0.2 } else { 0.24 }).floor();
    let headline_fit = fit_headline(
        engine,
        headline_width,
        max_headline_height,
        if is_narrow {
            HEADLINE_NARROW_MAX_SIZE
        } else {
            HEADLINE_MAX_SIZE
        },
    );
    let headline_height = headline_fit.lines.len() as f32 * headline_fit.line_height;
    let body_top = page.y + gutter + headline_height + if is_narrow { 14.0 } else { 20.0 };
    let body_height = (page.height - (body_top - page.y) - bottom_gap).max(BODY_LINE_HEIGHT);
    let column_count = if page.width > 1000.0 {
        3
    } else if page.width > 640.0 {
        2
    } else {
        1
    };
    let total_gutter = gutter * 2.0 + col_gap * (column_count - 1) as f32;
    let max_content_width = page.width.min(1500.0);
    let column_width = ((max_content_width - total_gutter) / column_count as f32).floor();
    let content_width = column_width * column_count as f32 + col_gap * (column_count - 1) as f32;
    let content_left = page.x + ((page.width - content_width) * 0.5).round();

    let body_columns = (0..column_count)
        .map(|index| GeoRect {
            x: content_left + index as f32 * (column_width + col_gap),
            y: body_top,
            width: column_width,
            height: body_height,
        })
        .collect::<Vec<_>>();
    let drop_cap_rect = GeoRect {
        x: content_left - 2.0,
        y: body_top - 2.0,
        width: drop_cap_total_width,
        height: DROP_CAP_LINES as f32 * BODY_LINE_HEIGHT + 2.0,
    };

    EditorialLayout {
        page,
        is_narrow,
        gutter,
        col_gap,
        bottom_gap,
        orb_radius_scale,
        active_orb_count,
        column_count,
        content_left,
        column_width,
        body_top,
        body_height,
        headline_origin,
        headline_fit,
        body_columns,
        drop_cap_rect,
    }
}

fn editorial_reflow_signature(layout: &EditorialLayout, orbs: &[Orb]) -> u64 {
    use std::hash::{Hash, Hasher};

    let circle_obstacles = orbs
        .iter()
        .take(layout.active_orb_count)
        .map(|orb| CircleObstacle {
            center: Point { x: orb.x, y: orb.y },
            radius: orb.radius * layout.orb_radius_scale,
            horizontal_padding: if layout.is_narrow { 10.0 } else { 14.0 },
            vertical_padding: if layout.is_narrow { 2.0 } else { 4.0 },
        })
        .collect::<Vec<_>>();
    let mut state = std::collections::hash_map::DefaultHasher::new();
    layout.active_orb_count.hash(&mut state);
    layout.column_count.hash(&mut state);

    for (column_index, column) in layout.body_columns.iter().copied().enumerate() {
        column_index.hash(&mut state);
        let mut line_top = column.y;
        while line_top + BODY_LINE_HEIGHT <= column.bottom() {
            let band_top = line_top;
            let band_bottom = line_top + BODY_LINE_HEIGHT;
            let mut intervals = circle_obstacles
                .iter()
                .filter_map(|obstacle| {
                    circle_interval_for_band(
                        obstacle.center.x,
                        obstacle.center.y,
                        obstacle.radius,
                        band_top,
                        band_bottom,
                        obstacle.horizontal_padding,
                        obstacle.vertical_padding,
                    )
                })
                .collect::<Vec<_>>();
            intervals.sort_by(|left, right| {
                left.left
                    .total_cmp(&right.left)
                    .then(left.right.total_cmp(&right.right))
            });
            intervals.len().hash(&mut state);
            for interval in intervals {
                quantize_reflow_bucket(interval.left).hash(&mut state);
                quantize_reflow_bucket(interval.right).hash(&mut state);
            }
            line_top += BODY_LINE_HEIGHT;
        }
    }

    state.finish()
}

fn compute_editorial_projection(
    engine: &PretextEngine,
    body_prepared: &PreparedTextWithSegments,
    pull_quote_prepared: &[PreparedTextWithSegments],
    drop_cap_prepared: &PreparedTextWithSegments,
    layout: &EditorialLayout,
    orbs: &[Orb],
) -> EditorialProjection {
    let headline_lines = layout
        .headline_fit
        .lines
        .iter()
        .cloned()
        .map(|mut line| {
            line.x += layout.headline_origin.x;
            line.y += layout.headline_origin.y;
            line
        })
        .collect();

    let circle_obstacles = orbs
        .iter()
        .take(layout.active_orb_count)
        .map(|orb| CircleObstacle {
            center: Point { x: orb.x, y: orb.y },
            radius: orb.radius * layout.orb_radius_scale,
            horizontal_padding: if layout.is_narrow { 10.0 } else { 14.0 },
            vertical_padding: if layout.is_narrow { 2.0 } else { 4.0 },
        })
        .collect::<Vec<_>>();

    let placements = pull_quote_placements();
    let mut pull_quotes = Vec::new();
    if !layout.is_narrow {
        for (index, placement) in placements.iter().enumerate() {
            if placement.col_idx >= layout.column_count {
                continue;
            }
            let prepared = &pull_quote_prepared[index];
            let quote_width = (layout.column_width * placement.w_frac).round();
            let quote_layout = engine.layout_with_lines(
                prepared,
                (quote_width - 20.0).max(1.0),
                QUOTE_LINE_HEIGHT,
            );
            let quote_height = quote_layout.lines.len() as f32 * QUOTE_LINE_HEIGHT + 16.0;
            let column_x = layout.content_left
                + placement.col_idx as f32 * (layout.column_width + layout.col_gap);
            let quote_x = match placement.side {
                PullQuoteSide::Right => column_x + layout.column_width - quote_width,
                PullQuoteSide::Left => column_x,
            };
            let quote_y = (layout.body_top + layout.body_height * placement.y_frac).round();
            let lines = quote_layout
                .lines
                .into_iter()
                .enumerate()
                .map(|(line_index, line)| PositionedLine {
                    x: quote_x + 20.0,
                    y: quote_y + 8.0 + line_index as f32 * QUOTE_LINE_HEIGHT,
                    width: line.width,
                    text: line.text.clone(),
                    visual_runs: engine.line_visual_runs(prepared, &line),
                    glyph_runs: engine.line_glyph_runs(prepared, &line),
                })
                .collect();

            pull_quotes.push(PullQuoteBox {
                rect: GeoRect {
                    x: quote_x,
                    y: quote_y,
                    width: quote_width,
                    height: quote_height,
                },
                lines,
                col_idx: placement.col_idx,
            });
        }
    }

    let mut body_lines = Vec::new();
    let mut cursor = BODY_START_CURSOR;
    for (column_index, column) in layout.body_columns.iter().copied().enumerate() {
        let mut rect_obstacles = Vec::new();
        if column_index == 0 {
            rect_obstacles.push(RectObstacle {
                rect: layout.drop_cap_rect,
            });
        }
        for pull_quote in &pull_quotes {
            if pull_quote.col_idx != column_index {
                continue;
            }
            rect_obstacles.push(RectObstacle {
                rect: pull_quote.rect,
            });
        }

        let (mut lines, next_cursor) = layout_column(
            engine,
            body_prepared,
            cursor,
            column,
            BODY_LINE_HEIGHT,
            &circle_obstacles,
            &rect_obstacles,
            layout.is_narrow,
        );
        body_lines.append(&mut lines);
        cursor = next_cursor;
    }

    let mut drop_cap_cursor = LayoutCursor::default();
    let drop_cap_line = engine
        .layout_next_line(drop_cap_prepared, &mut drop_cap_cursor, UNBOUNDED_WIDTH)
        .map(|line| PositionedLine {
            x: layout.content_left,
            y: layout.body_top,
            width: line.width,
            text: line.text.clone(),
            visual_runs: engine.line_visual_runs(drop_cap_prepared, &line),
            glyph_runs: engine.line_glyph_runs(drop_cap_prepared, &line),
        })
        .expect("drop cap line should fit");

    EditorialProjection {
        headline_lines,
        body_lines,
        pull_quotes,
        drop_cap_line,
    }
}

fn layout_column(
    engine: &PretextEngine,
    prepared: &PreparedTextWithSegments,
    start_cursor: LayoutCursor,
    region: GeoRect,
    line_height: f32,
    circle_obstacles: &[CircleObstacle],
    rect_obstacles: &[RectObstacle],
    single_slot_only: bool,
) -> (Vec<PositionedLine>, LayoutCursor) {
    let mut cursor = start_cursor;
    let mut line_top = region.y;
    let mut lines = Vec::new();
    let mut text_exhausted = false;

    while line_top + line_height <= region.bottom() && !text_exhausted {
        let band_top = line_top;
        let band_bottom = line_top + line_height;
        let mut blocked = Vec::new();

        for obstacle in circle_obstacles {
            if let Some(interval) = circle_interval_for_band(
                obstacle.center.x,
                obstacle.center.y,
                obstacle.radius,
                band_top,
                band_bottom,
                obstacle.horizontal_padding,
                obstacle.vertical_padding,
            ) {
                blocked.push(interval);
            }
        }

        for obstacle in rect_obstacles {
            if band_bottom <= obstacle.rect.y || band_top >= obstacle.rect.bottom() {
                continue;
            }
            blocked.push(Interval {
                left: obstacle.rect.x,
                right: obstacle.rect.right(),
            });
        }

        let slots = carve_editorial_slots(
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

        let ordered_slots = if single_slot_only {
            vec![slots
                .into_iter()
                .reduce(|best, slot| {
                    let best_width = best.right - best.left;
                    let slot_width = slot.right - slot.left;
                    if slot_width > best_width {
                        slot
                    } else if slot_width < best_width {
                        best
                    } else if slot.left < best.left {
                        slot
                    } else {
                        best
                    }
                })
                .expect("single slot should exist")]
        } else {
            let mut ordered = slots;
            ordered.sort_by(|a, b| a.left.total_cmp(&b.left));
            ordered
        };

        for slot in ordered_slots {
            let mut next_cursor = cursor;
            let Some(line) = engine.layout_next_line(
                prepared,
                &mut next_cursor,
                (slot.right - slot.left).max(1.0),
            ) else {
                text_exhausted = true;
                break;
            };
            if next_cursor == cursor {
                text_exhausted = true;
                break;
            }

            lines.push(PositionedLine {
                x: slot.left.round(),
                y: line_top.round(),
                width: line.width,
                text: line.text.clone(),
                visual_runs: engine.line_visual_runs(prepared, &line),
                glyph_runs: engine.line_glyph_runs(prepared, &line),
            });
            cursor = next_cursor;
        }

        line_top += line_height;
    }

    (lines, cursor)
}

fn carve_editorial_slots(base: Interval, blocked: &[Interval]) -> Vec<Interval> {
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
        .filter(|slot| slot.right - slot.left >= MIN_SLOT_WIDTH)
        .collect()
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

fn build_positioned_single_line(
    engine: &PretextEngine,
    text: &str,
    style: &pretext::TextStyleSpec,
    x: f32,
    y: f32,
) -> Option<PositionedLine> {
    let prepared = engine.prepare_with_segments(text, style, &normal_options());
    let mut cursor = LayoutCursor::default();
    let line = engine.layout_next_line(&prepared, &mut cursor, UNBOUNDED_WIDTH)?;
    Some(PositionedLine {
        x,
        y,
        width: line.width,
        text: line.text.clone(),
        visual_runs: engine.line_visual_runs(&prepared, &line),
        glyph_runs: engine.line_glyph_runs(&prepared, &line),
    })
}

fn orb_hit_test(
    orbs: &[Orb],
    active_count: usize,
    radius_scale: f32,
    pointer: egui::Pos2,
) -> Option<usize> {
    for index in (0..active_count).rev() {
        let orb = orbs[index];
        let radius = orb.radius * radius_scale;
        let dx = pointer.x - orb.x;
        let dy = pointer.y - orb.y;
        if dx * dx + dy * dy <= radius * radius {
            return Some(index);
        }
    }
    None
}

fn handle_orb_interaction(
    ui: &mut egui::Ui,
    page_rect: Rect,
    layout: &EditorialLayout,
    orbs: &mut [Orb],
    drag: &mut Option<DragState>,
) -> Option<usize> {
    let pointer_pos = ui.ctx().input(|i| i.pointer.interact_pos());
    let hovered_orb_index = pointer_pos
        .filter(|pos| page_rect.contains(*pos))
        .and_then(|pos| orb_hit_test(orbs, layout.active_orb_count, layout.orb_radius_scale, pos));

    if drag.is_none() && ui.ctx().input(|i| i.pointer.primary_pressed()) {
        if let (Some(index), Some(pointer_pos)) = (hovered_orb_index, pointer_pos) {
            let orb = orbs[index];
            *drag = Some(DragState {
                orb_index: index,
                start_pointer: Point {
                    x: pointer_pos.x,
                    y: pointer_pos.y,
                },
                last_pointer: Point {
                    x: pointer_pos.x,
                    y: pointer_pos.y,
                },
                start_orb: Point { x: orb.x, y: orb.y },
            });
        }
    }

    if let Some(state) = drag.as_mut() {
        let orb = &mut orbs[state.orb_index];
        if ui.ctx().input(|i| i.pointer.primary_down()) {
            if let Some(pointer_pos) = pointer_pos {
                state.last_pointer = Point {
                    x: pointer_pos.x,
                    y: pointer_pos.y,
                };
                orb.x = state.start_orb.x + (pointer_pos.x - state.start_pointer.x);
                orb.y = state.start_orb.y + (pointer_pos.y - state.start_pointer.y);
                clamp_orb_to_bounds(orb, layout);
            }
        } else if ui.ctx().input(|i| i.pointer.primary_released()) {
            let dx = state.last_pointer.x - state.start_pointer.x;
            let dy = state.last_pointer.y - state.start_pointer.y;
            if dx * dx + dy * dy < 16.0 {
                orb.paused = !orb.paused;
            }
            *drag = None;
        }
    }

    if drag.is_some() {
        ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
    } else if hovered_orb_index.is_some() {
        ui.ctx().set_cursor_icon(CursorIcon::Grab);
    }

    drag.as_ref().map(|state| state.orb_index)
}

fn clamp_orb_to_bounds(orb: &mut Orb, layout: &EditorialLayout) {
    let radius = orb.radius * layout.orb_radius_scale;
    orb.x = orb
        .x
        .clamp(layout.page.x + radius, layout.page.right() - radius);
    orb.y = orb.y.clamp(
        layout.page.y + layout.gutter * 0.5 + radius,
        layout.page.bottom() - layout.bottom_gap - radius,
    );
}

fn update_orbs(
    now: f64,
    layout: &EditorialLayout,
    orbs: &mut [Orb],
    last_time: &mut Option<f64>,
    dragged_orb_index: Option<usize>,
) -> bool {
    let last = last_time.unwrap_or(now);
    let dt = ((now - last) as f32).clamp(0.0, 0.05);
    let mut animating = false;

    for (index, orb) in orbs.iter_mut().enumerate().take(layout.active_orb_count) {
        let radius = orb.radius * layout.orb_radius_scale;
        if orb.paused || Some(index) == dragged_orb_index {
            continue;
        }
        animating = true;
        orb.x += orb.vx * dt;
        orb.y += orb.vy * dt;

        if orb.x - radius < layout.page.x {
            orb.x = layout.page.x + radius;
            orb.vx = orb.vx.abs();
        }
        if orb.x + radius > layout.page.right() {
            orb.x = layout.page.right() - radius;
            orb.vx = -orb.vx.abs();
        }
        if orb.y - radius < layout.page.y + layout.gutter * 0.5 {
            orb.y = layout.page.y + layout.gutter * 0.5 + radius;
            orb.vy = orb.vy.abs();
        }
        if orb.y + radius > layout.page.bottom() - layout.bottom_gap {
            orb.y = layout.page.bottom() - layout.bottom_gap - radius;
            orb.vy = -orb.vy.abs();
        }
    }

    for index in 0..layout.active_orb_count {
        let (left, tail) = orbs.split_at_mut(index + 1);
        let a = &mut left[index];
        let a_radius = a.radius * layout.orb_radius_scale;
        for (other_index, b) in tail.iter_mut().enumerate() {
            let b_index = index + 1 + other_index;
            if b_index >= layout.active_orb_count {
                break;
            }
            let b_radius = b.radius * layout.orb_radius_scale;
            let dx = b.x - a.x;
            let dy = b.y - a.y;
            let dist = (dx * dx + dy * dy).sqrt();
            let min_dist = a_radius + b_radius + if layout.is_narrow { 12.0 } else { 20.0 };
            if dist >= min_dist || dist <= 0.1 {
                continue;
            }

            let force = (min_dist - dist) * 0.8;
            let nx = dx / dist;
            let ny = dy / dist;

            if !a.paused && Some(index) != dragged_orb_index {
                a.vx -= nx * force * dt;
                a.vy -= ny * force * dt;
            }
            if !b.paused && Some(b_index) != dragged_orb_index {
                b.vx += nx * force * dt;
                b.vy += ny * force * dt;
            }
        }
    }

    *last_time = if animating { Some(now) } else { None };
    animating
}

fn paint_editorial_background(painter: &egui::Painter, rect: Rect, texture: &TextureHandle) {
    painter.image(
        texture.id(),
        rect,
        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );
}

fn paint_projection(
    painter: &egui::Painter,
    projection: &EditorialProjection,
    layout: &EditorialLayout,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut AssetRegistry,
) {
    paint_positioned_lines(
        painter,
        &projection.headline_lines,
        &headline_style(layout.headline_fit.font_size),
        layout.headline_fit.line_height,
        headline_render_slack_y(
            layout.headline_fit.font_size,
            layout.headline_fit.line_height,
        ),
        Color32::WHITE,
        ctx,
        engine,
        assets,
    );

    paint_positioned_lines(
        painter,
        std::slice::from_ref(&projection.drop_cap_line),
        &drop_cap_style(),
        BODY_LINE_HEIGHT * DROP_CAP_LINES as f32 - 4.0,
        6.0,
        Color32::from_rgb(196, 163, 90),
        ctx,
        engine,
        assets,
    );

    for pull_quote in &projection.pull_quotes {
        painter.line_segment(
            [
                egui::pos2(pull_quote.rect.x, pull_quote.rect.y),
                egui::pos2(pull_quote.rect.x, pull_quote.rect.bottom()),
            ],
            Stroke::new(3.0, Color32::from_rgb(107, 90, 61)),
        );
        paint_positioned_lines(
            painter,
            &pull_quote.lines,
            &quote_style(),
            QUOTE_LINE_HEIGHT,
            2.0,
            Color32::from_rgb(184, 160, 112),
            ctx,
            engine,
            assets,
        );
    }

    paint_positioned_lines(
        painter,
        &projection.body_lines,
        &body_style(),
        BODY_LINE_HEIGHT,
        2.0,
        Color32::from_rgb(232, 228, 220),
        ctx,
        engine,
        assets,
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

fn paint_orbs(
    painter: &egui::Painter,
    layout: &EditorialLayout,
    orbs: &[Orb],
    textures: &[SizedTexture],
) {
    for (orb, texture) in orbs
        .iter()
        .take(layout.active_orb_count)
        .zip(textures.iter())
    {
        let texture_rect = Rect::from_center_size(
            egui::pos2(orb.x, orb.y),
            egui::vec2(texture.size[0] as f32, texture.size[1] as f32),
        );
        let alpha = if orb.paused { 115 } else { 255 };
        painter.image(
            texture.texture.id(),
            texture_rect,
            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::from_white_alpha(alpha),
        );
    }
}

fn paint_editorial_chrome(
    painter: &egui::Painter,
    rect: Rect,
    is_narrow: bool,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut AssetRegistry,
) {
    if !is_narrow {
        let hint_style = chrome_style(13.0);
        let mut hint_line = build_positioned_single_line(engine, HINT_TEXT, &hint_style, 0.0, 0.0)
            .unwrap_or(PositionedLine {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                text: HINT_TEXT.to_owned(),
                visual_runs: Vec::new(),
                glyph_runs: Vec::new(),
            });
        let hint_rect = Rect::from_min_size(
            egui::pos2(
                rect.center().x - (hint_line.width + 36.0) * 0.5,
                rect.top() + 16.0,
            ),
            egui::vec2(hint_line.width + 36.0, 29.0),
        );
        hint_line.x = hint_rect.left() + 18.0;
        hint_line.y = hint_rect.top() + 8.0;
        painter.rect_filled(
            hint_rect,
            CornerRadius::same(255),
            Color32::from_rgba_premultiplied(0, 0, 0, 115),
        );
        paint_positioned_lines(
            painter,
            std::slice::from_ref(&hint_line),
            &hint_style,
            13.0,
            2.0,
            Color32::from_rgba_premultiplied(255, 255, 255, 56),
            ctx,
            engine,
            assets,
        );

        let credit_style = chrome_style(11.0);
        let mut credit_line =
            build_positioned_single_line(engine, CREDIT_TEXT, &credit_style, 0.0, 0.0).unwrap_or(
                PositionedLine {
                    x: 0.0,
                    y: 0.0,
                    width: 0.0,
                    text: CREDIT_TEXT.to_owned(),
                    visual_runs: Vec::new(),
                    glyph_runs: Vec::new(),
                },
            );
        credit_line.x = rect.right() - 16.0 - credit_line.width;
        credit_line.y = rect.bottom() - 12.0 - 11.0;
        paint_positioned_lines(
            painter,
            std::slice::from_ref(&credit_line),
            &credit_style,
            11.0,
            2.0,
            Color32::from_rgba_premultiplied(255, 255, 255, 72),
            ctx,
            engine,
            assets,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const JS_EDITORIAL_ENGINE_TS: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../pretext_js/pages/demos/editorial-engine.ts"
    ));

    #[test]
    fn orb_obstacles_change_editorial_projection() {
        let engine = PretextEngine::with_font_data_and_system_fonts(
            AssetRegistry::bundled_font_data(),
            false,
        );
        let body = engine.prepare_with_segments(BODY_TEXT, &body_style(), &normal_options());
        let pull_quotes = PULL_QUOTE_TEXTS
            .iter()
            .map(|text| engine.prepare_with_segments(text, &quote_style(), &normal_options()))
            .collect::<Vec<_>>();
        let drop_cap = engine.prepare_with_segments("T", &drop_cap_style(), &normal_options());
        let page = GeoRect {
            x: 0.0,
            y: 0.0,
            width: 1024.0,
            height: 720.0,
        };
        let layout = build_editorial_layout(
            page,
            &engine,
            measure_single_line_width(&engine, &drop_cap).ceil() + 10.0,
        );
        let no_orbs =
            compute_editorial_projection(&engine, &body, &pull_quotes, &drop_cap, &layout, &[]);
        let with_orbs = compute_editorial_projection(
            &engine,
            &body,
            &pull_quotes,
            &drop_cap,
            &layout,
            &orb_definitions()
                .iter()
                .map(|definition| Orb {
                    x: page.width * definition.fx,
                    y: page.height * definition.fy,
                    radius: definition.radius,
                    vx: definition.vx,
                    vy: definition.vy,
                    rgb: definition.rgb,
                    paused: false,
                })
                .collect::<Vec<_>>(),
        );
        assert_ne!(with_orbs.body_lines, no_orbs.body_lines);
    }

    #[test]
    fn editorial_copy_matches_checked_in_js_source() {
        assert_eq!(
            HEADLINE,
            extract_js_source_between(
                JS_EDITORIAL_ENGINE_TS,
                "const HEADLINE_TEXT = '",
                "'\nconst GUTTER = "
            )
        );
        assert_eq!(
            BODY_TEXT,
            extract_js_source_between(
                JS_EDITORIAL_ENGINE_TS,
                "const BODY_TEXT = `",
                "`\n\nconst PULLQUOTE_TEXTS = ["
            )
        );
        assert_eq!(
            PULL_QUOTE_TEXTS[0],
            extract_js_source_between(
                JS_EDITORIAL_ENGINE_TS,
                "const PULLQUOTE_TEXTS = [\n  '",
                "',\n  '"
            )
        );
        assert_eq!(
            PULL_QUOTE_TEXTS[1],
            extract_js_source_between(JS_EDITORIAL_ENGINE_TS, "',\n  '", "',\n]\n\nconst stage = ")
        );
    }

    #[test]
    fn pull_quotes_stay_fixed_when_orbs_move() {
        let engine = PretextEngine::with_font_data_and_system_fonts(
            AssetRegistry::bundled_font_data(),
            false,
        );
        let body = engine.prepare_with_segments(BODY_TEXT, &body_style(), &normal_options());
        let pull_quotes = PULL_QUOTE_TEXTS
            .iter()
            .map(|text| engine.prepare_with_segments(text, &quote_style(), &normal_options()))
            .collect::<Vec<_>>();
        let drop_cap = engine.prepare_with_segments("T", &drop_cap_style(), &normal_options());
        let page = GeoRect {
            x: 0.0,
            y: 0.0,
            width: 1200.0,
            height: 760.0,
        };
        let layout = build_editorial_layout(
            page,
            &engine,
            measure_single_line_width(&engine, &drop_cap).ceil() + 10.0,
        );
        let without_orbs =
            compute_editorial_projection(&engine, &body, &pull_quotes, &drop_cap, &layout, &[]);
        let with_orbs = compute_editorial_projection(
            &engine,
            &body,
            &pull_quotes,
            &drop_cap,
            &layout,
            &orb_definitions()
                .iter()
                .map(|definition| Orb {
                    x: page.width * definition.fx,
                    y: page.height * definition.fy,
                    radius: definition.radius,
                    vx: definition.vx,
                    vy: definition.vy,
                    rgb: definition.rgb,
                    paused: false,
                })
                .collect::<Vec<_>>(),
        );

        assert_eq!(without_orbs.pull_quotes.len(), with_orbs.pull_quotes.len());
        for (left, right) in without_orbs
            .pull_quotes
            .iter()
            .zip(with_orbs.pull_quotes.iter())
        {
            assert_eq!(left.rect, right.rect);
            assert_eq!(left.lines, right.lines);
        }
    }

    #[test]
    fn editorial_projection_is_deterministic() {
        let engine = PretextEngine::with_font_data_and_system_fonts(
            AssetRegistry::bundled_font_data(),
            false,
        );
        let body = engine.prepare_with_segments(BODY_TEXT, &body_style(), &normal_options());
        let pull_quotes = PULL_QUOTE_TEXTS
            .iter()
            .map(|text| engine.prepare_with_segments(text, &quote_style(), &normal_options()))
            .collect::<Vec<_>>();
        let drop_cap = engine.prepare_with_segments("T", &drop_cap_style(), &normal_options());
        let page = GeoRect {
            x: 0.0,
            y: 0.0,
            width: 980.0,
            height: 700.0,
        };
        let layout = build_editorial_layout(
            page,
            &engine,
            measure_single_line_width(&engine, &drop_cap).ceil() + 10.0,
        );
        let orbs = orb_definitions()
            .iter()
            .map(|definition| Orb {
                x: page.width * definition.fx,
                y: page.height * definition.fy,
                radius: definition.radius,
                vx: definition.vx,
                vy: definition.vy,
                rgb: definition.rgb,
                paused: false,
            })
            .collect::<Vec<_>>();

        let first =
            compute_editorial_projection(&engine, &body, &pull_quotes, &drop_cap, &layout, &orbs);
        let second =
            compute_editorial_projection(&engine, &body, &pull_quotes, &drop_cap, &layout, &orbs);
        assert_eq!(first.body_lines, second.body_lines);
        assert_eq!(first.pull_quotes.len(), second.pull_quotes.len());
    }

    #[test]
    fn editorial_layout_keeps_visual_runs_for_mixed_direction_text() {
        let engine = PretextEngine::with_font_data_and_system_fonts(
            AssetRegistry::bundled_font_data(),
            false,
        );
        let prepared = engine.prepare_with_segments(
            "English قبل العربية and then back again",
            &body_style(),
            &normal_options(),
        );
        let (lines, _) = layout_column(
            &engine,
            &prepared,
            LayoutCursor::default(),
            GeoRect {
                x: 26.0,
                y: 136.0,
                width: 320.0,
                height: 112.0,
            },
            BODY_LINE_HEIGHT,
            &[],
            &[],
            false,
        );
        let line = lines
            .iter()
            .find(|line| {
                line.visual_runs
                    .iter()
                    .any(|run| run.direction == BidiDirection::Rtl)
            })
            .expect("editorial layout should produce a line with an RTL run");

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
    fn headline_fit_avoids_mid_word_breaks() {
        let engine = PretextEngine::with_font_data_and_system_fonts(
            AssetRegistry::bundled_font_data(),
            false,
        );
        let fit = fit_headline(&engine, 920.0, 180.0, HEADLINE_MAX_SIZE);
        let prepared = engine.prepare_with_segments(
            HEADLINE,
            &headline_style(fit.font_size),
            &normal_options(),
        );

        assert!(fit.font_size >= HEADLINE_MIN_SIZE as f32);
        assert!(!headline_breaks_inside_word(&engine, &prepared, 920.0));
    }

    #[test]
    fn wide_layout_places_two_pull_quotes_and_narrow_layout_hides_them() {
        let engine = PretextEngine::with_font_data_and_system_fonts(
            AssetRegistry::bundled_font_data(),
            false,
        );
        let body = engine.prepare_with_segments(BODY_TEXT, &body_style(), &normal_options());
        let pull_quotes = PULL_QUOTE_TEXTS
            .iter()
            .map(|text| engine.prepare_with_segments(text, &quote_style(), &normal_options()))
            .collect::<Vec<_>>();
        let drop_cap = engine.prepare_with_segments("T", &drop_cap_style(), &normal_options());

        let wide_page = GeoRect {
            x: 0.0,
            y: 0.0,
            width: 1200.0,
            height: 760.0,
        };
        let wide_layout = build_editorial_layout(
            wide_page,
            &engine,
            measure_single_line_width(&engine, &drop_cap).ceil() + 10.0,
        );
        let wide_projection = compute_editorial_projection(
            &engine,
            &body,
            &pull_quotes,
            &drop_cap,
            &wide_layout,
            &[],
        );

        let narrow_page = GeoRect {
            x: 0.0,
            y: 0.0,
            width: 720.0,
            height: 760.0,
        };
        let narrow_layout = build_editorial_layout(
            narrow_page,
            &engine,
            measure_single_line_width(&engine, &drop_cap).ceil() + 10.0,
        );
        let narrow_projection = compute_editorial_projection(
            &engine,
            &body,
            &pull_quotes,
            &drop_cap,
            &narrow_layout,
            &[],
        );

        assert_eq!(wide_layout.column_count, 3);
        assert_eq!(wide_projection.pull_quotes.len(), 2);
        assert!(narrow_layout.is_narrow);
        assert!(narrow_projection.pull_quotes.is_empty());
    }

    fn extract_js_source_between<'a>(source: &'a str, start: &str, end: &str) -> &'a str {
        let start_index = source
            .find(start)
            .map(|index| index + start.len())
            .expect("JS source start marker should exist");
        let end_index = source[start_index..]
            .find(end)
            .map(|index| start_index + index)
            .expect("JS source end marker should exist");
        &source[start_index..end_index]
    }
}
