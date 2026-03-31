use eframe::egui;
use egui::epaint::{Mesh, Vertex};
use egui::{
    Align2, Color32, CornerRadius, FontFamily, FontId, Rect, Sense, Shape, Stroke, StrokeKind,
};
#[cfg(test)]
use pretext::BidiDirection;
use pretext::{
    LayoutCursor, LayoutLineVisualRun, PrepareOptions, PreparedTextWithSegments, PretextEngine,
    WhiteSpaceMode,
};

use crate::assets::AssetRegistry;
use crate::demos::text_runs::paint_visual_runs;
use crate::demos::DemoWindow;
use crate::geometry::{
    carve_text_line_slots, hull_bounds, is_point_in_polygon, svg_alpha_hull, transform_points,
    Interval, Point, Rect as GeoRect,
};

const HEADLINE: &str = "SITUATIONAL AWARENESS: THE DECADE AHEAD";
const BYLINE: &str = "Interactive wrap geometry powered by PretextEngine";
const BODY_COPY: &str = "You can often see the future first in the places that are forced to finance it. \
The language around AI infrastructure has shifted from ambitious product launches to plans for power, land, and silicon at industrial scale. \
Every planning cycle adds another zero to the budget, another substation to the map, another reason for executives to talk about electricity before they talk about models. \
\n\n\
What matters for this demo is not the policy claim but the geometry: editorial layouts look convincing only when the text stream is continuous and the obstacles are real. \
The left column should spend text first, the right column should resume from the same cursor, and both columns should reroute around shapes instead of pretending the page is made of empty rectangles. \
\n\n\
That is why the body copy here is laid out one line at a time with `layout_next_line()`. \
The OpenAI mark intrudes from the lower left, the Claude mark leans into the upper right, and the line bands are carved into usable slots before the engine is asked to fit text. \
Stable geometry matters more than CSS mimicry: if the obstacles move, the text has to move with them. \
\n\n\
Click either logo and the hull rotates. The GPU texture rotates with it, but so does the exclusion geometry, so the paragraph reflows live instead of clipping through the artwork. \
That keeps the demo honest and gives the later editorial engine a real geometry layer to build on.";
const BODY_LINE_HEIGHT: f32 = 28.0;
const MIN_PAGE_HEIGHT: f32 = 440.0;
const LOGO_RASTER_SIZE: [usize; 2] = [320, 320];

pub struct DynamicLayoutDemo {
    open: bool,
    openai_logo: LogoAnimationState,
    claude_logo: LogoAnimationState,
    body_prepared: Option<PreparedTextWithSegments>,
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

#[derive(Clone, Debug, PartialEq)]
struct PositionedLine {
    x: f32,
    y: f32,
    width: f32,
    text: String,
    visual_runs: Vec<LayoutLineVisualRun>,
}

#[derive(Clone, Copy)]
struct PageLayout {
    page: GeoRect,
    title_origin: Point,
    byline_origin: Point,
    column_left: GeoRect,
    column_right: Option<GeoRect>,
    openai_rect: GeoRect,
    claude_rect: GeoRect,
    title_size: f32,
    title_line_height: f32,
}

impl Default for DynamicLayoutDemo {
    fn default() -> Self {
        Self {
            open: false,
            openai_logo: LogoAnimationState::default(),
            claude_logo: LogoAnimationState::default(),
            body_prepared: None,
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
            .default_size(egui::vec2(1040.0, 720.0))
            .show(ctx, |ui| {
                let now = ctx.input(|input| input.time);
                let animating = update_spin_state(&mut self.openai_logo, now)
                    | update_spin_state(&mut self.claude_logo, now);

                let available = ui.available_size();
                let page_width = available.x.max(520.0);
                let page_height = available.y.max(MIN_PAGE_HEIGHT);
                let (page_rect, _) =
                    ui.allocate_exact_size(egui::vec2(page_width, page_height), Sense::hover());

                let body_prepared = self.ensure_body_prepared(engine).clone();
                let hulls = self.ensure_hulls().clone();
                let layout = build_page_layout(page_rect);
                let title_style = title_style(layout.title_size);
                let title_prepared =
                    engine.prepare_with_segments(HEADLINE, &title_style, &normal_options());
                let title_lines = layout_simple_block(
                    engine,
                    &title_prepared,
                    layout.title_origin,
                    layout.page.width - 64.0,
                    layout.title_line_height,
                );
                let title_bottom = title_lines
                    .last()
                    .map(|line| line.y + layout.title_line_height)
                    .unwrap_or(layout.title_origin.y);

                let openai_poly =
                    transform_points(&hulls.openai, layout.openai_rect, self.openai_logo.angle);
                let claude_poly =
                    transform_points(&hulls.claude, layout.claude_rect, self.claude_logo.angle);
                let openai_obstacle = hull_bounds(&openai_poly).unwrap_or(layout.openai_rect);
                let claude_obstacle = hull_bounds(&claude_poly).unwrap_or(layout.claude_rect);

                let body_top = (title_bottom + 34.0).max(layout.column_left.y);
                let left_region = GeoRect {
                    y: body_top,
                    height: (layout.page.bottom() - body_top - 24.0).max(0.0),
                    ..layout.column_left
                };
                let right_region = layout.column_right.map(|region| GeoRect {
                    y: body_top,
                    height: (layout.page.bottom() - body_top - 24.0).max(0.0),
                    ..region
                });

                let left_obstacles = if rects_overlap_y(openai_obstacle, left_region) {
                    vec![openai_obstacle]
                } else {
                    Vec::new()
                };
                let right_obstacles = match right_region {
                    Some(region) if rects_overlap_y(claude_obstacle, region) => {
                        vec![claude_obstacle]
                    }
                    Some(_) | None => Vec::new(),
                };

                let (left_lines, cursor) = layout_column_around_rects(
                    engine,
                    &body_prepared,
                    LayoutCursor::default(),
                    left_region,
                    BODY_LINE_HEIGHT,
                    &left_obstacles,
                );
                let right_lines = right_region.map_or_else(Vec::new, |region| {
                    let (lines, _) = layout_column_around_rects(
                        engine,
                        &body_prepared,
                        cursor,
                        region,
                        BODY_LINE_HEIGHT,
                        &right_obstacles,
                    );
                    lines
                });

                let painter = ui.painter().clone();
                paint_page_background(&painter, page_rect);
                paint_positioned_lines(
                    &painter,
                    &title_lines,
                    FontId::new(layout.title_size, FontFamily::Proportional),
                    Color32::from_rgb(40, 38, 34),
                );
                painter.text(
                    egui::pos2(layout.byline_origin.x, layout.byline_origin.y),
                    Align2::LEFT_TOP,
                    BYLINE,
                    FontId::new(13.0, FontFamily::Proportional),
                    Color32::from_rgb(104, 97, 86),
                );
                paint_positioned_lines(
                    &painter,
                    &left_lines,
                    FontId::new(18.0, FontFamily::Proportional),
                    Color32::from_rgb(56, 54, 50),
                );
                paint_positioned_lines(
                    &painter,
                    &right_lines,
                    FontId::new(18.0, FontFamily::Proportional),
                    Color32::from_rgb(56, 54, 50),
                );

                let openai_texture = assets
                    .get_or_load_svg(
                        "dynamic-layout/openai",
                        AssetRegistry::openai_logo_svg(),
                        LOGO_RASTER_SIZE,
                        ctx,
                    )
                    .clone();
                let claude_texture = assets
                    .get_or_load_svg(
                        "dynamic-layout/claude",
                        AssetRegistry::claude_logo_svg(),
                        LOGO_RASTER_SIZE,
                        ctx,
                    )
                    .clone();
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
                paint_logo_hint(&painter, page_rect);

                handle_logo_interaction(
                    ui,
                    now,
                    "openai-logo",
                    layout.openai_rect,
                    &openai_poly,
                    &mut self.openai_logo,
                    1.0,
                );
                handle_logo_interaction(
                    ui,
                    now,
                    "claude-logo",
                    layout.claude_rect,
                    &claude_poly,
                    &mut self.claude_logo,
                    -1.0,
                );

                if animating {
                    ctx.request_repaint();
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

    fn ensure_hulls(&mut self) -> &LogoHulls {
        if self.hulls.is_none() {
            let openai = svg_alpha_hull(AssetRegistry::openai_logo_svg(), LOGO_RASTER_SIZE)
                .expect("openai hull");
            let claude = svg_alpha_hull(AssetRegistry::claude_logo_svg(), LOGO_RASTER_SIZE)
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

fn body_style() -> pretext::TextStyleSpec {
    pretext::TextStyleSpec {
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

fn title_style(size_px: f32) -> pretext::TextStyleSpec {
    pretext::TextStyleSpec {
        families: vec![
            "Noto Sans".to_owned(),
            "Arial".to_owned(),
            "Helvetica".to_owned(),
        ],
        size_px,
        weight: 700,
        italic: false,
    }
}

fn build_page_layout(page_rect: Rect) -> PageLayout {
    let narrow = page_rect.width() < 860.0;
    let margin = if narrow { 24.0 } else { 34.0 };
    let title_size = if narrow {
        page_rect.width().mul_add(0.03, 20.0).clamp(28.0, 40.0)
    } else {
        page_rect.width().mul_add(0.024, 24.0).clamp(34.0, 54.0)
    };
    let title_line_height = title_size * 0.92;
    let title_origin = Point {
        x: page_rect.left() + margin,
        y: page_rect.top() + margin,
    };
    let byline_origin = Point {
        x: page_rect.left() + margin + 4.0,
        y: page_rect.top() + margin + title_line_height * 2.2,
    };
    let body_top = page_rect.top() + margin + title_line_height * 2.8 + 20.0;

    if narrow {
        let column = GeoRect {
            x: page_rect.left() + margin,
            y: body_top,
            width: page_rect.width() - margin * 2.0,
            height: page_rect.height() - (body_top - page_rect.top()) - margin,
        };
        let logo_size = (page_rect.width() * 0.24).clamp(92.0, 140.0);
        return PageLayout {
            page: GeoRect {
                x: page_rect.left(),
                y: page_rect.top(),
                width: page_rect.width(),
                height: page_rect.height(),
            },
            title_origin,
            byline_origin,
            column_left: column,
            column_right: None,
            openai_rect: GeoRect {
                x: page_rect.left() + 8.0,
                y: page_rect.bottom() - logo_size - 18.0,
                width: logo_size,
                height: logo_size,
            },
            claude_rect: GeoRect {
                x: page_rect.right() - logo_size * 0.86,
                y: body_top + 18.0,
                width: logo_size * 0.76,
                height: logo_size * 0.76,
            },
            title_size,
            title_line_height,
        };
    }

    let gutter = 28.0;
    let column_width = (page_rect.width() - margin * 2.0 - gutter) * 0.5;
    let left = GeoRect {
        x: page_rect.left() + margin,
        y: body_top,
        width: column_width,
        height: page_rect.height() - (body_top - page_rect.top()) - margin,
    };
    let right = GeoRect {
        x: left.x + column_width + gutter,
        y: body_top,
        width: column_width,
        height: left.height,
    };
    let openai_size = (page_rect.width() * 0.19).clamp(140.0, 220.0);
    let claude_size = (page_rect.width() * 0.18).clamp(124.0, 210.0);
    PageLayout {
        page: GeoRect {
            x: page_rect.left(),
            y: page_rect.top(),
            width: page_rect.width(),
            height: page_rect.height(),
        },
        title_origin,
        byline_origin,
        column_left: left,
        column_right: Some(right),
        openai_rect: GeoRect {
            x: left.x - openai_size * 0.28,
            y: page_rect.bottom() - openai_size - 18.0,
            width: openai_size,
            height: openai_size,
        },
        claude_rect: GeoRect {
            x: right.right() - claude_size * 0.55,
            y: body_top + 8.0,
            width: claude_size,
            height: claude_size,
        },
        title_size,
        title_line_height,
    }
}

fn layout_simple_block(
    engine: &PretextEngine,
    prepared: &PreparedTextWithSegments,
    origin: Point,
    max_width: f32,
    line_height: f32,
) -> Vec<PositionedLine> {
    let layout = engine.layout_with_lines(prepared, max_width.max(1.0), line_height);
    layout
        .lines
        .into_iter()
        .enumerate()
        .map(|(index, line)| {
            let visual_runs = engine.line_visual_runs(prepared, &line);
            PositionedLine {
                x: origin.x,
                y: origin.y + index as f32 * line_height,
                width: line.width,
                text: line.text,
                visual_runs,
            }
        })
        .collect()
}

fn layout_column_around_rects(
    engine: &PretextEngine,
    prepared: &PreparedTextWithSegments,
    start: LayoutCursor,
    region: GeoRect,
    line_height: f32,
    obstacles: &[GeoRect],
) -> (Vec<PositionedLine>, LayoutCursor) {
    let mut cursor = start;
    let mut line_top = region.y;
    let mut lines = Vec::new();

    while line_top + line_height <= region.bottom() {
        let slots = carve_text_line_slots(
            Interval {
                left: region.x,
                right: region.right(),
            },
            &crate::geometry::get_rect_intervals_for_band(
                obstacles,
                line_top,
                line_top + line_height,
                line_height * 0.72,
                line_height * 0.12,
            ),
        );
        if slots.is_empty() {
            line_top += line_height;
            continue;
        }

        let slot = choose_widest_slot(&slots);
        let available_width = (slot.right - slot.left).max(1.0);
        let mut next_cursor = cursor;
        let Some(line) = engine.layout_next_line(prepared, &mut next_cursor, available_width)
        else {
            break;
        };
        if next_cursor == cursor {
            break;
        }
        let visual_runs = engine.line_visual_runs(prepared, &line);

        lines.push(PositionedLine {
            x: slot.left.round(),
            y: line_top.round(),
            width: line.width,
            text: line.text,
            visual_runs,
        });
        cursor = next_cursor;
        line_top += line_height;
    }

    (lines, cursor)
}

fn choose_widest_slot(slots: &[Interval]) -> Interval {
    let mut best = slots[0];
    for &slot in &slots[1..] {
        let best_width = best.right - best.left;
        let slot_width = slot.right - slot.left;
        if slot_width > best_width
            || ((slot_width - best_width).abs() < 0.001 && slot.left < best.left)
        {
            best = slot;
        }
    }
    best
}

fn rects_overlap_y(rect: GeoRect, region: GeoRect) -> bool {
    !(rect.bottom() <= region.y || rect.y >= region.bottom())
}

fn paint_page_background(painter: &egui::Painter, page_rect: Rect) {
    painter.rect_filled(
        page_rect,
        CornerRadius::same(22),
        Color32::from_rgb(247, 242, 234),
    );
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
    font_id: FontId,
    color: Color32,
) {
    for line in lines {
        paint_visual_runs(
            painter,
            line.x,
            line.y,
            &line.text,
            &line.visual_runs,
            &font_id,
            color,
        );
    }
}

fn paint_logo_hint(painter: &egui::Painter, page_rect: Rect) {
    let hint_rect = Rect::from_min_size(
        egui::pos2(page_rect.right() - 208.0, page_rect.bottom() - 44.0),
        egui::vec2(184.0, 24.0),
    );
    painter.rect_filled(
        hint_rect,
        CornerRadius::same(12),
        Color32::from_rgba_premultiplied(255, 255, 255, 164),
    );
    painter.text(
        egui::pos2(hint_rect.left() + 12.0, hint_rect.top() + 5.0),
        Align2::LEFT_TOP,
        "click logos to rotate",
        FontId::new(12.0, FontFamily::Proportional),
        Color32::from_rgb(98, 92, 83),
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
            duration: 0.85,
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
        let openai = svg_alpha_hull(AssetRegistry::openai_logo_svg(), LOGO_RASTER_SIZE)
            .expect("openai hull");
        let claude = svg_alpha_hull(AssetRegistry::claude_logo_svg(), LOGO_RASTER_SIZE)
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
        let (lines, _) = layout_column_around_rects(
            &engine,
            &prepared,
            LayoutCursor::default(),
            region,
            BODY_LINE_HEIGHT,
            &[obstacle],
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
        let lines = layout_simple_block(
            &engine,
            &prepared,
            Point { x: 24.0, y: 32.0 },
            320.0,
            BODY_LINE_HEIGHT,
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
}
