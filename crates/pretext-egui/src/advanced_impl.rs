use egui::TextureOptions;
use pretext::{
    BidiDirection, PretextEngine, PretextGlyphRun, PretextParagraphLayout, PretextStyle,
    PretextVisualRun,
};
use unicode_segmentation::UnicodeSegmentation;

use crate::advanced_types::{
    EmojiOverlay, EmojiOverlayOptions, EmojiOverlayRun, PendingEmojiPaint, PendingFallbackText,
    PositionedTextRunRef, PretextFragmentPainter, StyledPositionedTextRunRef,
};
use crate::{
    BaselineMetrics, BaselineMode, EguiPretextPaintOptions, EguiPretextRenderer, GlyphSceneBuilder,
    PretextTextureRasterRequest,
};

#[doc(hidden)]
pub fn shaped_text_baseline_metrics(
    engine: &PretextEngine,
    request: PretextTextureRasterRequest<'_>,
) -> BaselineMetrics {
    pretext_render::text_baseline_metrics(engine, super::text_raster_request(request))
}

#[doc(hidden)]
pub fn new_glyph_scene(assets: &EguiPretextRenderer) -> GlyphSceneBuilder {
    assets.begin_glyph_scene()
}

#[allow(clippy::too_many_arguments)]
#[doc(hidden)]
pub fn append_glyph_runs(
    scene: &mut GlyphSceneBuilder,
    x: f32,
    y: f32,
    glyph_runs: &[PretextGlyphRun],
    style: &PretextStyle,
    line_height: f32,
    color: egui::Color32,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut EguiPretextRenderer,
) -> bool {
    assets.append_line_glyph_runs_to_scene(
        scene,
        x,
        y,
        glyph_runs,
        style,
        line_height,
        color,
        ctx,
        engine,
    )
}

#[doc(hidden)]
pub fn flush_glyph_scene(
    painter: &egui::Painter,
    scene: &mut GlyphSceneBuilder,
    assets: &mut EguiPretextRenderer,
) -> bool {
    assets.flush_glyph_scene(painter, scene)
}

#[doc(hidden)]
pub fn split_builtin_emoji_glyphs(
    visual_runs: &[PretextVisualRun],
    glyph_runs: &[PretextGlyphRun],
    options: EmojiOverlayOptions<'_>,
    engine: &PretextEngine,
) -> (Vec<PretextGlyphRun>, Vec<EmojiOverlayRun>) {
    let mut run_left = 0.0f32;
    let mut output = Vec::with_capacity(glyph_runs.len());
    let mut overlays = Vec::new();

    for (visual_run, glyph_run) in visual_runs.iter().zip(glyph_runs.iter()) {
        if !contains_builtin_emoji(&visual_run.text) {
            output.push(glyph_run.clone());
            run_left += visual_run.width;
            continue;
        }

        let prefix_widths = engine.prefix_widths(&visual_run.text, options.style);
        let emoji_ranges = visual_run
            .text
            .grapheme_indices(true)
            .enumerate()
            .filter_map(|(index, (_, grapheme))| {
                EguiPretextRenderer::builtin_emoji_for_grapheme(grapheme).map(|emoji_id| {
                    EmojiOverlay {
                        start: prefix_widths[index],
                        end: prefix_widths[index + 1],
                        emoji_id,
                    }
                })
            })
            .collect::<Vec<_>>();
        if emoji_ranges.is_empty() {
            output.push(glyph_run.clone());
            run_left += visual_run.width;
            continue;
        }

        let mut filtered = glyph_run.clone();
        filtered.glyphs.retain(|glyph| {
            let center_x = glyph.x + glyph.x_offset + glyph.advance * 0.5;
            !emoji_ranges.iter().any(|overlay| {
                let start = run_left + overlay.start;
                let end = run_left + overlay.end;
                center_x >= start && center_x <= end
            })
        });
        output.push(filtered);
        overlays.push(EmojiOverlayRun {
            line_offset: run_left,
            width: visual_run.width,
            direction: visual_run.direction,
            baseline_metrics: shaped_text_baseline_metrics(
                engine,
                PretextTextureRasterRequest {
                    text: &visual_run.text,
                    style: options.style,
                    direction: visual_run.direction,
                    slot_height: options.slot_height,
                    padding_x: options.padding_x,
                    padding_y: options.padding_y,
                    slack_x: options.slack_x,
                    slack_y: options.slack_y,
                    baseline_mode: options.baseline_mode,
                    texture_options: TextureOptions::NEAREST,
                },
            ),
            emojis: emoji_ranges,
        });
        run_left += visual_run.width;
    }

    if glyph_runs.len() > visual_runs.len() {
        output.extend(glyph_runs[visual_runs.len()..].iter().cloned());
    }

    (output, overlays)
}

#[doc(hidden)]
pub fn strip_builtin_emoji_glyphs(
    visual_runs: &[PretextVisualRun],
    glyph_runs: &[PretextGlyphRun],
    style: &PretextStyle,
    engine: &PretextEngine,
) -> Vec<PretextGlyphRun> {
    split_builtin_emoji_glyphs(
        visual_runs,
        glyph_runs,
        EmojiOverlayOptions {
            style,
            slot_height: style.size_px,
            padding_x: 0.0,
            padding_y: 0.0,
            slack_x: 0.0,
            slack_y: 0.0,
            baseline_mode: BaselineMode::AutoFontMetrics,
        },
        engine,
    )
    .0
}

#[allow(clippy::too_many_arguments)]
#[doc(hidden)]
pub fn paint_emoji_overlays(
    painter: &egui::Painter,
    line_left: f32,
    slot_top: f32,
    overlay_runs: &[EmojiOverlayRun],
    emoji_size: f32,
    slot_height: f32,
    ctx: &egui::Context,
    assets: &mut EguiPretextRenderer,
) {
    for overlay_run in overlay_runs {
        let run_left = line_left + overlay_run.line_offset;
        for emoji in &overlay_run.emojis {
            let slot_width = (emoji.end - emoji.start).max(1.0);
            let slot_left = match overlay_run.direction {
                BidiDirection::Ltr => run_left + emoji.start,
                BidiDirection::Rtl => run_left + overlay_run.width - emoji.start - slot_width,
            };
            let size = emoji_size.min(slot_height).min(slot_width).max(1.0);
            let rect = egui::Rect::from_min_size(
                egui::pos2(
                    slot_left + (slot_width - size).max(0.0) * 0.5,
                    slot_top + overlay_run.baseline_metrics.square_top(size),
                ),
                egui::vec2(size, size),
            );
            let texture = assets.emoji_texture(emoji.emoji_id, [96, 96], ctx);
            painter.image(
                texture.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }
    }
}

#[doc(hidden)]
pub fn paint_pretext_paragraph(
    painter: &egui::Painter,
    origin: egui::Pos2,
    layout: &PretextParagraphLayout,
    options: &EguiPretextPaintOptions<'_>,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut EguiPretextRenderer,
) {
    let emoji_options = paragraph_emoji_overlay_options(options);
    let mut y = origin.y;
    let mut fragment_painter = PretextFragmentPainter::new(assets);
    for paragraph_line in &layout.lines {
        let visual_runs = &paragraph_line.runs.visual_runs;
        if line_has_builtin_emoji(visual_runs) {
            let (glyph_runs, emoji_overlays) = split_builtin_emoji_glyphs(
                visual_runs,
                &paragraph_line.runs.glyph_runs,
                emoji_options,
                engine,
            );
            fragment_painter.push_fragment(
                origin.x,
                y,
                &paragraph_line.line.text,
                &glyph_runs,
                &emoji_overlays,
                options,
                ctx,
                engine,
                assets,
            );
        } else {
            fragment_painter.push_fragment(
                origin.x,
                y,
                &paragraph_line.line.text,
                &paragraph_line.runs.glyph_runs,
                &[],
                options,
                ctx,
                engine,
                assets,
            );
        }
        y += options.line_height;
    }
    let _ = fragment_painter.finish(painter, ctx, assets);
}

#[doc(hidden)]
pub fn paint_positioned_text_runs<'a>(
    painter: &egui::Painter,
    lines: impl IntoIterator<Item = PositionedTextRunRef<'a>>,
    options: &EguiPretextPaintOptions<'_>,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut EguiPretextRenderer,
) -> bool {
    let mut fragment_painter = PretextFragmentPainter::new(assets);
    for line in lines {
        fragment_painter.push_fragment(
            line.x,
            line.y,
            line.text,
            line.glyph_runs,
            line.emoji_overlays,
            options,
            ctx,
            engine,
            assets,
        );
    }
    fragment_painter.finish(painter, ctx, assets)
}

#[doc(hidden)]
pub fn paint_styled_positioned_text_runs<'a, 'b>(
    painter: &egui::Painter,
    lines: impl IntoIterator<Item = StyledPositionedTextRunRef<'a, 'b>>,
    ctx: &egui::Context,
    engine: &PretextEngine,
    assets: &mut EguiPretextRenderer,
) -> bool {
    let mut fragment_painter = PretextFragmentPainter::new(assets);
    for line in lines {
        fragment_painter.push_fragment(
            line.x,
            line.y,
            line.text,
            line.glyph_runs,
            line.emoji_overlays,
            &line.options,
            ctx,
            engine,
            assets,
        );
    }
    fragment_painter.finish(painter, ctx, assets)
}

impl PretextFragmentPainter {
    pub fn new(assets: &EguiPretextRenderer) -> Self {
        Self {
            scene: new_glyph_scene(assets),
            pending_fallbacks: Vec::new(),
            pending_emoji: Vec::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn push_fragment(
        &mut self,
        x: f32,
        slot_top: f32,
        text: &str,
        glyph_runs: &[PretextGlyphRun],
        emoji_overlays: &[EmojiOverlayRun],
        options: &EguiPretextPaintOptions<'_>,
        ctx: &egui::Context,
        engine: &PretextEngine,
        assets: &mut EguiPretextRenderer,
    ) {
        let painted_glyphs = append_glyph_runs(
            &mut self.scene,
            x,
            slot_top,
            glyph_runs,
            options.style,
            options.line_height,
            options.color,
            ctx,
            engine,
            assets,
        );
        let emoji_only_fragment = glyph_runs.is_empty() && !emoji_overlays.is_empty();
        let needs_text_fallback = !painted_glyphs && !emoji_only_fragment;
        if needs_text_fallback {
            self.pending_fallbacks.push(PendingFallbackText {
                origin: egui::pos2(x, slot_top),
                text: text.to_owned(),
                fallback_align: options.fallback_align,
                fallback_font: options.fallback_font.clone(),
                color: options.color,
            });
        }

        if !emoji_overlays.is_empty() && !needs_text_fallback {
            self.pending_emoji.push(PendingEmojiPaint {
                line_left: x,
                slot_top,
                overlay_runs: emoji_overlays.to_vec(),
                emoji_size: options.emoji_size,
                slot_height: options.emoji_slot_height,
            });
        }
    }

    pub fn finish(
        mut self,
        painter: &egui::Painter,
        ctx: &egui::Context,
        assets: &mut EguiPretextRenderer,
    ) -> bool {
        let painted = flush_glyph_scene(painter, &mut self.scene, assets);
        let had_fallbacks = !self.pending_fallbacks.is_empty();
        let had_emoji = !self.pending_emoji.is_empty();
        for fallback in self.pending_fallbacks {
            painter.text(
                fallback.origin,
                fallback.fallback_align,
                fallback.text,
                fallback.fallback_font,
                fallback.color,
            );
        }
        for emoji in self.pending_emoji {
            paint_emoji_overlays(
                painter,
                emoji.line_left,
                emoji.slot_top,
                &emoji.overlay_runs,
                emoji.emoji_size,
                emoji.slot_height,
                ctx,
                assets,
            );
        }

        painted || had_fallbacks || had_emoji
    }
}

fn contains_builtin_emoji(text: &str) -> bool {
    text.graphemes(true)
        .any(|grapheme| EguiPretextRenderer::builtin_emoji_for_grapheme(grapheme).is_some())
}

fn line_has_builtin_emoji(visual_runs: &[PretextVisualRun]) -> bool {
    visual_runs
        .iter()
        .any(|visual_run| contains_builtin_emoji(&visual_run.text))
}

fn paragraph_emoji_overlay_options<'a>(
    options: &EguiPretextPaintOptions<'a>,
) -> EmojiOverlayOptions<'a> {
    EmojiOverlayOptions {
        style: options.style,
        slot_height: options.emoji_slot_height,
        padding_x: 0.0,
        padding_y: 0.0,
        slack_x: 0.0,
        slack_y: 0.0,
        baseline_mode: BaselineMode::AutoFontMetrics,
    }
}
