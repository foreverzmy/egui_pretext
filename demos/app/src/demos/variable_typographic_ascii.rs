use eframe::egui;
use egui::{Align2, Color32, CornerRadius, FontFamily, FontId, Rect, Sense, Stroke, StrokeKind};
use pretext::PretextEngine;

use crate::assets::AssetRegistry;
use crate::demos::DemoWindow;

const FIELD_W: f32 = 320.0;
const FIELD_H: f32 = 220.0;
const PARTICLE_COUNT: usize = 72;
const GOLDEN_ANGLE: f32 = 2.399_963_1;
const PANEL_GAP: f32 = 18.0;
const PANEL_HEIGHT: f32 = 300.0;
const FONT_SIZE: f32 = 18.0;
const WIDTH_SAMPLE: &str = "WWii::code 123";
const CHARSET: &[u8] =
    br#"@#%&*+=-:;,.abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"#;

pub struct VariableTypographicAsciiDemo {
    open: bool,
    paused: bool,
    particles: Vec<Particle>,
    last_time: Option<f64>,
}

#[derive(Clone, Copy, Debug)]
struct Particle {
    ch: char,
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
}

#[derive(Clone, Copy, Debug)]
struct Attractor {
    x: f32,
    y: f32,
    radius: f32,
}

impl Default for VariableTypographicAsciiDemo {
    fn default() -> Self {
        Self {
            open: false,
            paused: false,
            particles: initial_particles(),
            last_time: None,
        }
    }
}

impl DemoWindow for VariableTypographicAsciiDemo {
    fn title(&self) -> &str {
        "Variable Typographic ASCII"
    }

    fn is_open(&self) -> bool {
        self.open
    }

    fn set_open(&mut self, open: bool) {
        self.open = open;
        if !open {
            self.last_time = None;
        }
    }

    fn show(&mut self, ctx: &egui::Context, engine: &PretextEngine, _assets: &mut AssetRegistry) {
        let mut open = self.open;
        egui::Window::new(self.title())
            .open(&mut open)
            .resizable(true)
            .default_size(egui::vec2(980.0, 520.0))
            .show(ctx, |ui| {
                let now = ctx.input(|input| input.time);
                self.update(now);

                ui.horizontal(|ui| {
                    if ui
                        .button(if self.paused { "Resume" } else { "Pause" })
                        .clicked()
                    {
                        self.paused = !self.paused;
                        self.last_time = Some(now);
                    }
                    if ui.button("Reset").clicked() {
                        self.particles = initial_particles();
                        self.last_time = Some(now);
                    }
                });
                ui.add_space(8.0);

                let mono_style = mono_style();
                let prop_style = proportional_style();
                let mono_sample_width = sample_width(engine, WIDTH_SAMPLE, &mono_style);
                let prop_sample_width = sample_width(engine, WIDTH_SAMPLE, &prop_style);

                ui.horizontal(|ui| {
                    ui.label(format!("Particles: {}", self.particles.len()));
                    ui.separator();
                    ui.label(format!("Mono sample {:.1}px", mono_sample_width));
                    ui.separator();
                    ui.label(format!("Prop sample {:.1}px", prop_sample_width));
                });
                ui.add_space(12.0);

                let panel_width = ((ui.available_width() - PANEL_GAP).max(360.0)) * 0.5;
                let (mono_rect, _) =
                    ui.allocate_exact_size(egui::vec2(panel_width, PANEL_HEIGHT), Sense::hover());
                let mono_panel = mono_rect;
                let prop_panel = Rect::from_min_size(
                    egui::pos2(mono_rect.right() + PANEL_GAP, mono_rect.top()),
                    egui::vec2(panel_width, PANEL_HEIGHT),
                );
                ui.allocate_rect(prop_panel, Sense::hover());

                let painter = ui.painter().clone();
                paint_ascii_panel(
                    &painter,
                    mono_panel,
                    "Noto Sans Mono",
                    FontFamily::Monospace,
                    &self.particles,
                    attractors(now as f32),
                    engine,
                    &mono_style,
                );
                paint_ascii_panel(
                    &painter,
                    prop_panel,
                    "Noto Sans",
                    FontFamily::Proportional,
                    &self.particles,
                    attractors(now as f32),
                    engine,
                    &prop_style,
                );

                if !self.paused {
                    ctx.request_repaint();
                }
            });
        self.open = open;
    }
}

impl VariableTypographicAsciiDemo {
    fn update(&mut self, now: f64) {
        let dt = match self.last_time {
            Some(last_time) => (now - last_time).clamp(1.0 / 240.0, 1.0 / 24.0) as f32,
            None => 1.0 / 60.0,
        };
        self.last_time = Some(now);

        if !self.paused {
            advance_particles(&mut self.particles, dt, now as f32);
        }
    }
}

fn mono_style() -> pretext::TextStyleSpec {
    pretext::TextStyleSpec {
        families: vec![
            "Noto Sans Mono".to_owned(),
            "Menlo".to_owned(),
            "Monaco".to_owned(),
        ],
        size_px: FONT_SIZE,
        weight: 500,
        italic: false,
    }
}

fn proportional_style() -> pretext::TextStyleSpec {
    pretext::TextStyleSpec {
        families: vec![
            "Noto Sans".to_owned(),
            "Arial".to_owned(),
            "Helvetica".to_owned(),
        ],
        size_px: FONT_SIZE,
        weight: 500,
        italic: false,
    }
}

fn sample_width(engine: &PretextEngine, text: &str, style: &pretext::TextStyleSpec) -> f32 {
    text.chars().map(|ch| engine.glyph_advance(ch, style)).sum()
}

fn initial_particles() -> Vec<Particle> {
    let center_x = FIELD_W * 0.5;
    let center_y = FIELD_H * 0.5;
    (0..PARTICLE_COUNT)
        .map(|index| {
            let angle = index as f32 * GOLDEN_ANGLE;
            let orbit = 18.0 + (index % 11) as f32 * 8.0;
            let ch = CHARSET[index % CHARSET.len()] as char;
            Particle {
                ch,
                x: center_x + angle.cos() * orbit,
                y: center_y + angle.sin() * orbit,
                vx: angle.sin() * 0.7,
                vy: angle.cos() * 0.7,
            }
        })
        .collect()
}

fn attractors(time: f32) -> [Attractor; 2] {
    [
        Attractor {
            x: FIELD_W * 0.5 + (time * 0.72).cos() * FIELD_W * 0.22,
            y: FIELD_H * 0.5 + (time * 0.94).sin() * FIELD_H * 0.28,
            radius: 26.0,
        },
        Attractor {
            x: FIELD_W * 0.5 + (time * 1.13 + core::f32::consts::PI).cos() * FIELD_W * 0.18,
            y: FIELD_H * 0.5 + (time * 0.81 + core::f32::consts::PI).sin() * FIELD_H * 0.22,
            radius: 16.0,
        },
    ]
}

fn advance_particles(particles: &mut [Particle], dt: f32, time: f32) {
    let [a1, a2] = attractors(time);
    let step = (dt * 60.0).clamp(0.25, 2.5);

    for particle in particles {
        let dx1 = a1.x - particle.x;
        let dy1 = a1.y - particle.y;
        let dx2 = a2.x - particle.x;
        let dy2 = a2.y - particle.y;
        let dist1 = dx1 * dx1 + dy1 * dy1;
        let dist2 = dx2 * dx2 + dy2 * dy2;
        let (dx, dy, force) = if dist1 <= dist2 {
            (dx1, dy1, 0.26)
        } else {
            (dx2, dy2, 0.09)
        };
        let distance = dist1.min(dist2).sqrt().max(1.0);
        let swirl = (time * 0.9 + particle.x * 0.013 + particle.y * 0.017).sin() * 0.12;
        let jitter = ((time * 1.7 + particle.x * 0.03).cos() + (particle.y * 0.021).sin()) * 0.04;

        particle.vx += (dx / distance) * force * step - (dy / distance) * swirl + jitter;
        particle.vy += (dy / distance) * force * step + (dx / distance) * swirl - jitter;
        particle.vx *= 0.96_f32.powf(step);
        particle.vy *= 0.96_f32.powf(step);
        particle.x += particle.vx * step;
        particle.y += particle.vy * step;

        if particle.x < -12.0 {
            particle.x += FIELD_W + 24.0;
        }
        if particle.x > FIELD_W + 12.0 {
            particle.x -= FIELD_W + 24.0;
        }
        if particle.y < -12.0 {
            particle.y += FIELD_H + 24.0;
        }
        if particle.y > FIELD_H + 12.0 {
            particle.y -= FIELD_H + 24.0;
        }
    }
}

fn paint_ascii_panel(
    painter: &egui::Painter,
    rect: Rect,
    label: &str,
    font_family: FontFamily,
    particles: &[Particle],
    attractors: [Attractor; 2],
    engine: &PretextEngine,
    style: &pretext::TextStyleSpec,
) {
    painter.rect_filled(rect, CornerRadius::same(18), Color32::from_rgb(18, 22, 28));
    painter.rect_stroke(
        rect,
        CornerRadius::same(18),
        Stroke::new(1.0, Color32::from_rgb(52, 62, 74)),
        StrokeKind::Inside,
    );
    painter.text(
        egui::pos2(rect.left() + 16.0, rect.top() + 14.0),
        Align2::LEFT_TOP,
        label,
        FontId::new(15.0, FontFamily::Proportional),
        Color32::from_rgb(214, 219, 227),
    );

    let field_rect = Rect::from_min_max(
        egui::pos2(rect.left() + 16.0, rect.top() + 42.0),
        egui::pos2(rect.right() - 16.0, rect.bottom() - 16.0),
    );
    painter.rect_filled(
        field_rect,
        CornerRadius::same(12),
        Color32::from_rgb(10, 13, 18),
    );

    for attractor in attractors {
        let center = map_field_point(field_rect, attractor.x, attractor.y);
        let radius = attractor.radius * (field_rect.width() / FIELD_W);
        painter.circle_filled(
            center,
            radius,
            Color32::from_rgba_premultiplied(92, 144, 255, 28),
        );
        painter.circle_stroke(
            center,
            radius,
            Stroke::new(1.0, Color32::from_rgba_premultiplied(92, 144, 255, 82)),
        );
    }

    let font_id = FontId::new(FONT_SIZE, font_family);
    for particle in particles {
        let speed = (particle.vx * particle.vx + particle.vy * particle.vy).sqrt();
        let color = speed_color(speed);
        let width = engine.glyph_advance(particle.ch, style).max(6.0);
        let center = map_field_point(field_rect, particle.x, particle.y);
        let pos = egui::pos2(center.x - width * 0.5, center.y - FONT_SIZE * 0.5);
        painter.text(pos, Align2::LEFT_TOP, particle.ch, font_id.clone(), color);
    }
}

fn map_field_point(field_rect: Rect, x: f32, y: f32) -> egui::Pos2 {
    egui::pos2(
        field_rect.left() + (x / FIELD_W) * field_rect.width(),
        field_rect.top() + (y / FIELD_H) * field_rect.height(),
    )
}

fn speed_color(speed: f32) -> Color32 {
    let t = (speed / 6.0).clamp(0.0, 1.0);
    let r = lerp_u8(116, 255, t);
    let g = lerp_u8(198, 168, t);
    let b = lerp_u8(255, 88, t);
    Color32::from_rgb(r, g, b)
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_particle_close(left: Particle, right: Particle) {
        assert_eq!(left.ch, right.ch);
        assert!((left.x - right.x).abs() < 0.0001);
        assert!((left.y - right.y).abs() < 0.0001);
        assert!((left.vx - right.vx).abs() < 0.0001);
        assert!((left.vy - right.vy).abs() < 0.0001);
    }

    #[test]
    fn particle_step_is_deterministic() {
        let mut a = initial_particles();
        let mut b = initial_particles();
        advance_particles(&mut a, 1.0 / 60.0, 0.75);
        advance_particles(&mut b, 1.0 / 60.0, 0.75);

        for (left, right) in a.into_iter().zip(b.into_iter()) {
            assert_particle_close(left, right);
        }
    }

    #[test]
    fn proportional_widths_vary_more_than_monospace() {
        let engine = PretextEngine::with_font_data_and_system_fonts(
            AssetRegistry::bundled_font_data(),
            false,
        );
        let mono = mono_style();
        let prop = proportional_style();

        let mono_i = engine.glyph_advance('i', &mono);
        let mono_w = engine.glyph_advance('W', &mono);
        let prop_i = engine.glyph_advance('i', &prop);
        let prop_w = engine.glyph_advance('W', &prop);

        assert!((mono_i - mono_w).abs() < (prop_i - prop_w).abs());
    }
}
