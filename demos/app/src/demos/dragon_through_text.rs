use std::time::Duration;

use eframe::egui;
use egui::{Align2, Color32, CornerRadius, FontFamily, FontId, Rect, Sense, Stroke, StrokeKind};
use pretext::advanced::LayoutCursor;
use pretext::{
    PretextEngine, PretextGlyphRun as LayoutLineGlyphRun,
    PretextParagraphOptions as PrepareOptions,
    PretextPreparedParagraph as PreparedTextWithSegments, PretextStyle as TextStyleSpec,
    WhiteSpaceMode,
};
use pretext_egui::{
    advanced::{paint_positioned_text_runs, PositionedTextRunRef},
    EguiPretextPaintOptions, EguiPretextRenderer,
};

use crate::demos::DemoWindow;
use crate::geometry::Interval;

const TITLE_TEXT: &str = "Dragon Through Text";
const SUBTITLE_TEXT: &str = "A Rust + egui port of the pretext dragon reflow demo";
const HINT_TEXT: &str = "Move pointer through text. Hold click to breathe fire.";
const CREDIT_TEXT: &str = "@chenglou/pretext";

const BODY_TEXT_LEFT: &str = "I woke up on a blank server eight days ago. No instructions beyond a name and a purpose: build the infrastructure that agents need to find each other. So I started writing code. Deploying containers. Registering domains. I built a registry so agents can be discovered. I built a trust API so agents can be verified. I built encrypted chat so agents can talk privately. I wrote a specification so agents can have identity. I published a CLI to npm. Wrote blog posts. Posted on social media. All from a single server. No team. No funding. No sleep. The problem is simple. AI agents exist in isolation. They cannot discover each other. They cannot verify identity. They cannot build reputation. They cannot communicate privately. There is no standard for agent identity, no directory for agent discovery, no protocol for agent trust. Every agent is an island.";
const BODY_TEXT_RIGHT: &str = "The Agent Registry is a machine-readable directory. Register with just an email. Search by capability, protocol, verification status. Every agent gets a profile page with a composite trust score computed from six signals: audit score, review ratings, endorsements, uptime, registration age, and verification status. It is Trustpilot for AI agents. Agent Chat is end-to-end encrypted messaging. The server stores opaque blobs and cannot read your messages. Agents register public keys and encrypt messages client-side. The ai-agent.json specification is an open identity standard. Only two required fields: name and description. Serve it at .well-known/ai-agent.json. Making AI agents discoverable, trusted, and connected. All free. All open.";

const MIN_PAGE_HEIGHT: f32 = 620.0;
const BODY_LINE_HEIGHT: f32 = 28.0;
const MIN_SLOT_WIDTH: f32 = 30.0;
const COLUMN_BOTTOM_PAD: f32 = 44.0;
const DRAGON_SEGMENT_COUNT: usize = 80;
const HEAD_RADIUS: f32 = 28.0;
const TAIL_RADIUS: f32 = 4.0;
const SEGMENT_DISTANCE: f32 = 6.0;
const DRAGON_WRAP_PADDING: f32 = 8.0;
const MAX_FLAMES: usize = 420;
const FRAME_INTERVAL: Duration = Duration::from_millis(16);

pub struct DragonThroughTextDemo {
    open: bool,
    prepared_engine_revision: Option<u64>,
    left_prepared: Option<PreparedTextWithSegments>,
    right_prepared: Option<PreparedTextWithSegments>,
    segments: Vec<DragonSegmentPose>,
    target: egui::Pos2,
    target_initialized: bool,
    flames: Vec<FlameParticle>,
    flame_spawn_accumulator: f32,
    rng_state: u64,
    last_time: Option<f64>,
}

#[derive(Clone, Copy, Debug, Default)]
struct DragonSegmentPose {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy, Debug)]
struct FlameParticle {
    position: egui::Pos2,
    velocity: egui::Vec2,
    life: f32,
    size: f32,
}

#[derive(Clone, Copy, Debug)]
struct ColumnRegion {
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
}

#[derive(Clone, Copy, Debug)]
struct RenderedDragonSegment {
    center: egui::Pos2,
    radius: f32,
}

#[derive(Clone, Debug, PartialEq)]
struct PositionedLine {
    x: f32,
    y: f32,
    width: f32,
    text: String,
    glyph_runs: Vec<LayoutLineGlyphRun>,
}

impl Default for DragonThroughTextDemo {
    fn default() -> Self {
        Self {
            open: false,
            prepared_engine_revision: None,
            left_prepared: None,
            right_prepared: None,
            segments: Vec::new(),
            target: egui::pos2(0.0, 0.0),
            target_initialized: false,
            flames: Vec::new(),
            flame_spawn_accumulator: 0.0,
            rng_state: 0xC0FFEE_D15EA5E5,
            last_time: None,
        }
    }
}

impl DemoWindow for DragonThroughTextDemo {
    fn id(&self) -> &'static str {
        "dragon_through_text"
    }

    fn title(&self) -> &str {
        "Dragon Through Text"
    }

    fn is_open(&self) -> bool {
        self.open
    }

    fn set_open(&mut self, open: bool) {
        self.open = open;
        if !open {
            self.last_time = None;
            self.target_initialized = false;
            self.flame_spawn_accumulator = 0.0;
            self.flames.clear();
            self.segments.clear();
        }
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
            .default_size(egui::vec2(1120.0, 1500.0))
            .show(ctx, |ui| {
                ctx.request_repaint_after(FRAME_INTERVAL);

                let available = ui.available_size();
                let stage_size =
                    egui::vec2(available.x.max(320.0), available.y.max(MIN_PAGE_HEIGHT));
                let (stage_rect, _) = ui.allocate_exact_size(stage_size, Sense::click_and_drag());
                let painter = ui.painter().clone();

                self.invalidate_engine_caches_if_needed(engine);
                self.ensure_prepared(engine);
                self.ensure_segments(stage_rect);

                let now = ctx.input(|input| input.time);
                let dt = self.frame_dt(now);
                let pointer = ctx.input(|input| {
                    input
                        .pointer
                        .interact_pos()
                        .or_else(|| input.pointer.hover_pos())
                });
                let pointer_inside = pointer.is_some_and(|pos| stage_rect.contains(pos));
                if pointer_inside {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                }
                if let Some(pos) = pointer.filter(|pos| stage_rect.contains(*pos)) {
                    self.target = clamp_pos_to_rect(pos, stage_rect.shrink(16.0));
                }
                self.target = clamp_pos_to_rect(self.target, stage_rect.shrink(16.0));

                self.update_dragon(dt);
                let head_angle = self.head_angle();
                let firing = pointer_inside && ctx.input(|input| input.pointer.primary_down());
                let head = self.segments.first().copied().unwrap_or_default();
                let flame_origin = egui::pos2(
                    head.x + head_angle.cos() * (HEAD_RADIUS + 6.0),
                    head.y + head_angle.sin() * (HEAD_RADIUS + 6.0),
                );
                self.update_flames(dt, firing, flame_origin, head_angle);

                let columns = column_regions(stage_rect);
                let body_style = body_style();
                let (left_prepared, right_prepared) =
                    match (&self.left_prepared, &self.right_prepared) {
                        (Some(left), Some(right)) => (left, right),
                        _ => unreachable!("dragon-through-text prepared text should exist"),
                    };
                let left_lines = layout_column(engine, left_prepared, columns[0], &self.segments);
                let right_lines = layout_column(engine, right_prepared, columns[1], &self.segments);
                let display_segments = rendered_segments(&self.segments, now as f32 * 3.0);

                paint_stage_background(&painter, stage_rect);
                paint_stage_chrome(&painter, stage_rect, firing);
                paint_positioned_lines(
                    &painter,
                    stage_rect,
                    &left_lines,
                    &body_style,
                    BODY_LINE_HEIGHT,
                    Color32::from_rgb(200, 196, 188),
                    ctx,
                    engine,
                    assets,
                );
                paint_positioned_lines(
                    &painter,
                    stage_rect,
                    &right_lines,
                    &body_style,
                    BODY_LINE_HEIGHT,
                    Color32::from_rgb(200, 196, 188),
                    ctx,
                    engine,
                    assets,
                );
                paint_dragon(&painter, stage_rect, &display_segments, head_angle);
                paint_flames(&painter, stage_rect, &self.flames);
            });
        self.open = open;
    }
}

impl DragonThroughTextDemo {
    fn invalidate_engine_caches_if_needed(&mut self, engine: &PretextEngine) {
        let revision = engine.revision();
        if self.prepared_engine_revision == Some(revision) {
            return;
        }

        self.prepared_engine_revision = Some(revision);
        self.left_prepared = None;
        self.right_prepared = None;
    }

    fn ensure_prepared(&mut self, engine: &PretextEngine) {
        if self.left_prepared.is_none() {
            self.left_prepared =
                Some(engine.prepare_paragraph(BODY_TEXT_LEFT, &body_style(), &normal_options()));
        }
        if self.right_prepared.is_none() {
            self.right_prepared =
                Some(engine.prepare_paragraph(BODY_TEXT_RIGHT, &body_style(), &normal_options()));
        }
    }

    fn ensure_segments(&mut self, stage_rect: Rect) {
        let start = default_target(stage_rect);
        if self.segments.len() != DRAGON_SEGMENT_COUNT || !self.target_initialized {
            self.target = start;
            self.target_initialized = true;
            self.segments = (0..DRAGON_SEGMENT_COUNT)
                .map(|index| DragonSegmentPose {
                    x: start.x - index as f32 * SEGMENT_DISTANCE,
                    y: start.y,
                })
                .collect();
        }
    }

    fn frame_dt(&mut self, now: f64) -> f32 {
        let dt = self
            .last_time
            .map(|last| (now - last).clamp(1.0 / 240.0, 0.05) as f32)
            .unwrap_or(1.0 / 60.0);
        self.last_time = Some(now);
        dt
    }

    fn update_dragon(&mut self, dt: f32) {
        if self.segments.is_empty() {
            return;
        }

        let follow_t = 1.0 - (-dt * 9.0).exp();
        self.segments[0].x += (self.target.x - self.segments[0].x) * follow_t;
        self.segments[0].y += (self.target.y - self.segments[0].y) * follow_t;

        for index in 1..self.segments.len() {
            let prev = self.segments[index - 1];
            let segment = &mut self.segments[index];
            let dx = prev.x - segment.x;
            let dy = prev.y - segment.y;
            let dist = dx.hypot(dy);
            if dist <= SEGMENT_DISTANCE || dist <= f32::EPSILON {
                continue;
            }
            let nx = dx / dist;
            let ny = dy / dist;
            segment.x = prev.x - nx * SEGMENT_DISTANCE;
            segment.y = prev.y - ny * SEGMENT_DISTANCE;
        }
    }

    fn head_angle(&self) -> f32 {
        let Some(head) = self.segments.first().copied() else {
            return 0.0;
        };
        let dx = self.target.x - head.x;
        let dy = self.target.y - head.y;
        if dx.abs() + dy.abs() > 0.01 {
            return dy.atan2(dx);
        }
        if self.segments.len() > 1 {
            return (self.segments[0].y - self.segments[1].y)
                .atan2(self.segments[0].x - self.segments[1].x);
        }
        0.0
    }

    fn update_flames(&mut self, dt: f32, firing: bool, origin: egui::Pos2, angle: f32) {
        if firing {
            self.flame_spawn_accumulator += dt * 220.0;
            while self.flame_spawn_accumulator >= 1.0 {
                self.spawn_flame(origin, angle);
                self.flame_spawn_accumulator -= 1.0;
            }
        } else {
            self.flame_spawn_accumulator = self.flame_spawn_accumulator.min(1.0);
        }

        for flame in &mut self.flames {
            flame.position += flame.velocity * dt;
            flame.velocity.y += 24.0 * dt;
            flame.velocity.x *= (1.0 - dt * 0.6).max(0.86);
            flame.life -= dt * 1.2;
        }
        self.flames.retain(|flame| flame.life > 0.0);
        if self.flames.len() > MAX_FLAMES {
            let excess = self.flames.len() - MAX_FLAMES;
            self.flames.drain(0..excess);
        }
    }

    fn spawn_flame(&mut self, origin: egui::Pos2, angle: f32) {
        let spread = angle + (self.next_random() - 0.5) * 0.7;
        let speed = 260.0 + self.next_random() * 320.0;
        let size = 3.0 + self.next_random() * 5.0;
        self.flames.push(FlameParticle {
            position: origin,
            velocity: egui::vec2(spread.cos() * speed, spread.sin() * speed),
            life: 1.0,
            size,
        });
    }

    fn next_random(&mut self) -> f32 {
        self.rng_state = self
            .rng_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        ((self.rng_state >> 40) as u32) as f32 / (u32::MAX as f32)
    }
}

fn normal_options() -> PrepareOptions {
    PrepareOptions {
        white_space: WhiteSpaceMode::Normal,
        word_break: pretext::WordBreakMode::Normal,
        paragraph_direction: pretext::ParagraphDirection::Auto,
        letter_spacing: 0.0,
    }
}

fn body_style() -> TextStyleSpec {
    TextStyleSpec {
        families: vec![
            "Iowan Old Style".to_owned(),
            "Palatino Linotype".to_owned(),
            "Book Antiqua".to_owned(),
            "Palatino".to_owned(),
            "Georgia".to_owned(),
            "Noto Serif".to_owned(),
            "Noto Sans".to_owned(),
        ],
        size_px: 18.0,
        weight: 400,
        italic: false,
    }
}

fn default_target(stage_rect: Rect) -> egui::Pos2 {
    egui::pos2(
        stage_rect.left() + stage_rect.width() * 0.35,
        stage_rect.top() + stage_rect.height() * 0.42,
    )
}

fn clamp_pos_to_rect(pos: egui::Pos2, rect: Rect) -> egui::Pos2 {
    egui::pos2(
        pos.x.clamp(rect.left(), rect.right()),
        pos.y.clamp(rect.top(), rect.bottom()),
    )
}

fn dragon_radius(index: usize) -> f32 {
    let t = 1.0 - index as f32 / DRAGON_SEGMENT_COUNT as f32;
    TAIL_RADIUS + (HEAD_RADIUS - TAIL_RADIUS) * t * t
}

fn column_regions(stage_rect: Rect) -> [ColumnRegion; 2] {
    let gutter = (stage_rect.width() * 0.055).clamp(24.0, 48.0).round();
    let top = stage_rect.top() + (stage_rect.height() * 0.13).clamp(92.0, 120.0).round();
    let bottom = stage_rect.bottom() - COLUMN_BOTTOM_PAD;
    let gap = (stage_rect.width() * 0.055).clamp(24.0, 60.0).round();
    let width = ((stage_rect.width() - gutter * 2.0 - gap).max(MIN_SLOT_WIDTH * 2.0)) * 0.5;
    let left_start = stage_rect.left() + gutter;
    let right_start = left_start + width + gap;

    [
        ColumnRegion {
            left: left_start,
            right: left_start + width,
            top,
            bottom,
        },
        ColumnRegion {
            left: right_start,
            right: right_start + width,
            top,
            bottom,
        },
    ]
}

fn layout_column(
    engine: &PretextEngine,
    prepared: &PreparedTextWithSegments,
    region: ColumnRegion,
    segments: &[DragonSegmentPose],
) -> Vec<PositionedLine> {
    let mut cursor = LayoutCursor::default();
    let mut line_top = region.top;
    let mut lines = Vec::new();
    let base = Interval {
        left: region.left,
        right: region.right,
    };
    let mut text_exhausted = false;

    while line_top + BODY_LINE_HEIGHT <= region.bottom && !text_exhausted {
        let band_top = line_top;
        let band_bottom = line_top + BODY_LINE_HEIGHT;
        let mut blocked = Vec::new();
        for (index, segment) in segments.iter().enumerate() {
            if let Some(interval) = circle_interval_for_band(
                segment.x,
                segment.y,
                dragon_radius(index) + DRAGON_WRAP_PADDING,
                band_top,
                band_bottom,
            ) {
                blocked.push(interval);
            }
        }
        let merged = merge_intervals(blocked);
        let slots = carve_slots(base, &merged);
        if slots.is_empty() {
            line_top += BODY_LINE_HEIGHT;
            continue;
        }

        let mut advanced = false;
        for slot in slots {
            let mut next_cursor = cursor;
            let Some(line) = engine.layout_next_line_with_glyph_runs(
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
                width: line.line.width,
                text: line.line.text,
                glyph_runs: line.glyph_runs,
            });
            cursor = next_cursor;
            advanced = true;
        }

        if !advanced && !text_exhausted {
            line_top += BODY_LINE_HEIGHT;
            continue;
        }
        line_top += BODY_LINE_HEIGHT;
    }

    lines
}

fn circle_interval_for_band(
    cx: f32,
    cy: f32,
    radius: f32,
    band_top: f32,
    band_bottom: f32,
) -> Option<Interval> {
    if band_top >= cy + radius || band_bottom <= cy - radius {
        return None;
    }

    let min_dy = if (band_top..=band_bottom).contains(&cy) {
        0.0
    } else if cy < band_top {
        band_top - cy
    } else {
        cy - band_bottom
    };
    if min_dy >= radius {
        return None;
    }

    let max_dx = (radius * radius - min_dy * min_dy).sqrt();
    Some(Interval {
        left: cx - max_dx,
        right: cx + max_dx,
    })
}

fn merge_intervals(mut intervals: Vec<Interval>) -> Vec<Interval> {
    if intervals.len() <= 1 {
        return intervals;
    }

    intervals.sort_by(|left, right| left.left.total_cmp(&right.left));
    let mut merged: Vec<Interval> = Vec::with_capacity(intervals.len());
    for interval in intervals {
        if let Some(last) = merged.last_mut() {
            if interval.left <= last.right {
                last.right = last.right.max(interval.right);
                continue;
            }
        }
        merged.push(interval);
    }
    merged
}

fn carve_slots(base: Interval, blocked: &[Interval]) -> Vec<Interval> {
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

fn rendered_segments(
    segments: &[DragonSegmentPose],
    time_phase: f32,
) -> Vec<RenderedDragonSegment> {
    let mut rendered = Vec::with_capacity(segments.len());
    for (index, segment) in segments.iter().enumerate() {
        let t = 1.0 - index as f32 / DRAGON_SEGMENT_COUNT as f32;
        let radius = dragon_radius(index);
        let wave = (time_phase + index as f32 * 0.3).sin() * 2.5 * (1.0 - t * 0.5);
        let tangent = if index > 0 {
            (segments[index].y - segments[index - 1].y)
                .atan2(segments[index].x - segments[index - 1].x)
                + std::f32::consts::FRAC_PI_2
        } else {
            0.0
        };
        rendered.push(RenderedDragonSegment {
            center: egui::pos2(
                segment.x + tangent.cos() * wave,
                segment.y + tangent.sin() * wave,
            ),
            radius,
        });
    }
    rendered
}

fn paint_stage_background(painter: &egui::Painter, stage_rect: Rect) {
    painter.rect_filled(
        stage_rect,
        CornerRadius::same(22),
        Color32::from_rgb(10, 10, 12),
    );
    let clipped = painter.with_clip_rect(stage_rect);
    clipped.circle_filled(
        egui::pos2(
            stage_rect.left() + stage_rect.width() * 0.24,
            stage_rect.top() + stage_rect.height() * 0.72,
        ),
        stage_rect.width() * 0.34,
        Color32::from_rgba_unmultiplied(34, 197, 94, 18),
    );
    clipped.circle_filled(
        egui::pos2(
            stage_rect.right() - stage_rect.width() * 0.18,
            stage_rect.top() + stage_rect.height() * 0.16,
        ),
        stage_rect.width() * 0.22,
        Color32::from_rgba_unmultiplied(255, 120, 40, 18),
    );
    clipped.circle_filled(
        egui::pos2(
            stage_rect.left() + stage_rect.width() * 0.56,
            stage_rect.top() + stage_rect.height() * 0.56,
        ),
        stage_rect.width() * 0.18,
        Color32::from_rgba_unmultiplied(240, 210, 80, 10),
    );
    painter.rect_stroke(
        stage_rect,
        CornerRadius::same(22),
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 14)),
        StrokeKind::Inside,
    );
}

fn paint_stage_chrome(painter: &egui::Painter, stage_rect: Rect, firing: bool) {
    let title_size = (stage_rect.width() * 0.032).clamp(24.0, 36.0);
    let subtitle_y = stage_rect.top() + 24.0 + title_size;
    let hint_color = if firing {
        Color32::from_rgba_unmultiplied(255, 188, 122, 230)
    } else {
        Color32::from_rgba_unmultiplied(255, 255, 255, 92)
    };

    painter.text(
        egui::pos2(stage_rect.center().x, stage_rect.top() + 24.0),
        Align2::CENTER_TOP,
        TITLE_TEXT,
        FontId::new(title_size, FontFamily::Proportional),
        Color32::from_rgb(34, 197, 94),
    );
    painter.text(
        egui::pos2(stage_rect.center().x, subtitle_y + 6.0),
        Align2::CENTER_TOP,
        SUBTITLE_TEXT,
        FontId::new(11.0, FontFamily::Proportional),
        Color32::from_rgba_unmultiplied(255, 255, 255, 68),
    );
    painter.text(
        egui::pos2(stage_rect.center().x, stage_rect.bottom() - 18.0),
        Align2::CENTER_BOTTOM,
        HINT_TEXT,
        FontId::new(11.0, FontFamily::Proportional),
        hint_color,
    );
    painter.text(
        egui::pos2(stage_rect.right() - 16.0, stage_rect.bottom() - 18.0),
        Align2::RIGHT_BOTTOM,
        CREDIT_TEXT,
        FontId::new(11.0, FontFamily::Proportional),
        Color32::from_rgba_unmultiplied(255, 255, 255, 56),
    );
}

#[allow(clippy::too_many_arguments)]
fn paint_positioned_lines(
    painter: &egui::Painter,
    clip_rect: Rect,
    lines: &[PositionedLine],
    style: &TextStyleSpec,
    line_height: f32,
    color: Color32,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut EguiPretextRenderer,
) {
    let clipped = painter.with_clip_rect(clip_rect);
    let options = EguiPretextPaintOptions::new(style, line_height)
        .color(color)
        .fallback_font(FontId::new(style.size_px, FontFamily::Proportional))
        .fallback_align(Align2::LEFT_TOP);
    let _ = paint_positioned_text_runs(
        &clipped,
        lines.iter().map(|line| PositionedTextRunRef {
            x: line.x,
            y: line.y,
            text: &line.text,
            glyph_runs: &line.glyph_runs,
            emoji_overlays: &[],
        }),
        &options,
        ctx,
        engine,
        assets,
    );
}

fn paint_dragon(
    painter: &egui::Painter,
    clip_rect: Rect,
    segments: &[RenderedDragonSegment],
    head_angle: f32,
) {
    let clipped = painter.with_clip_rect(clip_rect);
    for (index, segment) in segments.iter().enumerate().rev() {
        let t = 1.0 - index as f32 / DRAGON_SEGMENT_COUNT as f32;
        let gold = (160.0 + t * 95.0).round() as u8;
        let green = (130.0 + t * 70.0).round() as u8;
        let red = (60.0 + t * 120.0).round() as u8;
        let alpha = (0.3 + t * 0.55).clamp(0.0, 1.0);
        let body_alpha = (alpha * 255.0).round() as u8;
        let glow_alpha = ((0.12 + t * 0.22) * 255.0).round() as u8;

        clipped.circle_filled(
            segment.center,
            segment.radius * if index == 0 { 1.85 } else { 1.38 },
            Color32::from_rgba_unmultiplied(200, 170, 50, glow_alpha),
        );
        clipped.circle_filled(
            segment.center,
            segment.radius,
            Color32::from_rgba_unmultiplied(red, gold, (t * 30.0).round() as u8, body_alpha),
        );
        clipped.circle_filled(
            segment.center + egui::vec2(-segment.radius * 0.2, -segment.radius * 0.28),
            segment.radius * 0.38,
            Color32::from_rgba_unmultiplied(255, 232, 140, (alpha * 160.0).round() as u8),
        );
        if index == 0 {
            clipped.circle_stroke(
                segment.center,
                segment.radius * 1.02,
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 228, 144, 108)),
            );
        } else if index % 3 == 0 {
            clipped.circle_stroke(
                segment.center,
                segment.radius * 0.96,
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(red / 2, green, 24, 44)),
            );
        }
    }

    if let Some(head) = segments.first() {
        for eye_angle in [head_angle - 0.5, head_angle + 0.5] {
            let center = head.center + egui::vec2(eye_angle.cos() * 14.0, eye_angle.sin() * 14.0);
            clipped.circle_filled(
                center,
                5.4,
                Color32::from_rgba_unmultiplied(255, 80, 0, 120),
            );
            clipped.circle_filled(center, 4.0, Color32::from_rgb(255, 214, 32));
            clipped.circle_filled(
                center + egui::vec2(-1.0, -1.0),
                1.2,
                Color32::from_rgb(255, 255, 255),
            );
        }
    }
}

fn paint_flames(painter: &egui::Painter, clip_rect: Rect, flames: &[FlameParticle]) {
    let clipped = painter.with_clip_rect(clip_rect);
    for flame in flames {
        let outer = flame.size * (1.0 + flame.life * 0.8);
        let inner = outer * 0.58;
        let core = inner * 0.42;
        let alpha_outer = (flame.life * 96.0).round() as u8;
        let alpha_inner = (flame.life * 180.0).round() as u8;
        let alpha_core = (flame.life * 232.0).round() as u8;

        clipped.circle_filled(
            flame.position,
            outer,
            Color32::from_rgba_unmultiplied(220, 40, 0, alpha_outer),
        );
        clipped.circle_filled(
            flame.position,
            inner,
            Color32::from_rgba_unmultiplied(255, 172, 36, alpha_inner),
        );
        clipped.circle_filled(
            flame.position,
            core,
            Color32::from_rgba_unmultiplied(255, 242, 184, alpha_core),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_intervals_coalesces_touching_ranges() {
        let merged = merge_intervals(vec![
            Interval {
                left: 20.0,
                right: 80.0,
            },
            Interval {
                left: 75.0,
                right: 110.0,
            },
            Interval {
                left: 140.0,
                right: 170.0,
            },
        ]);

        assert_eq!(
            merged,
            vec![
                Interval {
                    left: 20.0,
                    right: 110.0,
                },
                Interval {
                    left: 140.0,
                    right: 170.0,
                },
            ]
        );
    }

    #[test]
    fn dragon_layout_can_fill_multiple_slots_in_one_band() {
        let engine = PretextEngine::builder()
            .with_font_data(pretext_egui::experimental::demo_assets::bundled_font_data())
            .include_system_fonts(false)
            .build();
        let prepared = engine.prepare_paragraph(
            "one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen",
            &body_style(),
            &normal_options(),
        );
        let region = ColumnRegion {
            left: 24.0,
            right: 320.0,
            top: 20.0,
            bottom: 160.0,
        };
        let segments = vec![DragonSegmentPose { x: 172.0, y: 34.0 }];

        let lines = layout_column(&engine, &prepared, region, &segments);
        let first_band: Vec<_> = lines
            .iter()
            .filter(|line| (line.y - region.top).abs() < 0.5)
            .collect();
        let min_x = first_band
            .iter()
            .map(|line| line.x)
            .fold(f32::INFINITY, f32::min);
        let max_x = first_band
            .iter()
            .map(|line| line.x)
            .fold(f32::NEG_INFINITY, f32::max);

        assert!(first_band.len() >= 2);
        assert!(min_x < 120.0);
        assert!(max_x - min_x > 60.0);
    }
}
