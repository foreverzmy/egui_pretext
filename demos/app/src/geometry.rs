use resvg::usvg;

const WRAP_HULL_ALPHA_THRESHOLD: u8 = 12;
const WRAP_HULL_SMOOTH_RADIUS: i32 = 6;
const WRAP_HULL_MAX_ROW_SAMPLES: usize = 52;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interval {
    pub left: f32,
    pub right: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PolygonBandCache {
    start_y: i32,
    row_intervals: Vec<Option<Interval>>,
}

impl Rect {
    pub fn right(self) -> f32 {
        self.x + self.width
    }

    pub fn bottom(self) -> f32 {
        self.y + self.height
    }

    pub fn contains_point(self, point: Point) -> bool {
        point.x >= self.x
            && point.x <= self.right()
            && point.y >= self.y
            && point.y <= self.bottom()
    }
}

impl PolygonBandCache {
    pub fn new(points: &[Point]) -> Self {
        let Some(bounds) = hull_bounds(points) else {
            return Self {
                start_y: 0,
                row_intervals: Vec::new(),
            };
        };
        let start_y = bounds.y.floor() as i32;
        let end_y = bounds.bottom().ceil() as i32;
        let mut row_intervals = Vec::with_capacity((end_y - start_y + 1).max(0) as usize);
        for y in start_y..=end_y {
            row_intervals.push(scan_polygon_row_interval(points, y as f32 + 0.5));
        }
        Self {
            start_y,
            row_intervals,
        }
    }

    pub fn interval_for_band(
        &self,
        band_top: f32,
        band_bottom: f32,
        horizontal_padding: f32,
        vertical_padding: f32,
    ) -> Option<Interval> {
        let sample_top = band_top - vertical_padding;
        let sample_bottom = band_bottom + vertical_padding;
        let start_y = sample_top.floor() as i32;
        let end_y = sample_bottom.ceil() as i32;

        let mut left = f32::INFINITY;
        let mut right = f32::NEG_INFINITY;

        for y in start_y..=end_y {
            let Some(row_interval) = self.row_interval(y) else {
                continue;
            };
            left = left.min(row_interval.left);
            right = right.max(row_interval.right);
        }

        if !left.is_finite() || !right.is_finite() {
            return None;
        }

        Some(Interval {
            left: left - horizontal_padding,
            right: right + horizontal_padding,
        })
    }

    pub fn vertical_span(&self) -> Option<(f32, f32)> {
        if self.row_intervals.is_empty() {
            return None;
        }
        Some((
            self.start_y as f32,
            (self.start_y + self.row_intervals.len() as i32) as f32,
        ))
    }

    fn row_interval(&self, y: i32) -> Option<Interval> {
        let offset = y - self.start_y;
        if offset < 0 {
            return None;
        }
        self.row_intervals.get(offset as usize).copied().flatten()
    }
}

pub fn alpha_hull(points: &[Point]) -> Vec<Point> {
    if points.len() <= 3 {
        return points.to_vec();
    }

    let mut sorted = points.to_vec();
    sorted.sort_by(|left, right| {
        left.x
            .total_cmp(&right.x)
            .then_with(|| left.y.total_cmp(&right.y))
    });

    let mut lower = Vec::new();
    for point in &sorted {
        while lower.len() >= 2
            && cross(lower[lower.len() - 2], lower[lower.len() - 1], *point) <= 0.0
        {
            lower.pop();
        }
        lower.push(*point);
    }

    let mut upper = Vec::new();
    for point in sorted.iter().rev() {
        while upper.len() >= 2
            && cross(upper[upper.len() - 2], upper[upper.len() - 1], *point) <= 0.0
        {
            upper.pop();
        }
        upper.push(*point);
    }

    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

pub fn hull_bounds(points: &[Point]) -> Option<Rect> {
    let first = points.first().copied()?;
    let mut min_x = first.x;
    let mut min_y = first.y;
    let mut max_x = first.x;
    let mut max_y = first.y;

    for point in points.iter().copied() {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }

    Some(Rect {
        x: min_x,
        y: min_y,
        width: (max_x - min_x).max(0.0),
        height: (max_y - min_y).max(0.0),
    })
}

pub fn transform_points(points: &[Point], rect: Rect, angle: f32) -> Vec<Point> {
    if angle == 0.0 {
        return points
            .iter()
            .map(|point| Point {
                x: rect.x + point.x * rect.width,
                y: rect.y + point.y * rect.height,
            })
            .collect();
    }

    let center_x = rect.x + rect.width * 0.5;
    let center_y = rect.y + rect.height * 0.5;
    let cos = angle.cos();
    let sin = angle.sin();

    points
        .iter()
        .map(|point| {
            let local_x = (point.x - 0.5) * rect.width;
            let local_y = (point.y - 0.5) * rect.height;
            Point {
                x: center_x + local_x * cos - local_y * sin,
                y: center_y + local_x * sin + local_y * cos,
            }
        })
        .collect()
}

pub fn is_point_in_polygon(points: &[Point], point: Point) -> bool {
    if points.is_empty() {
        return false;
    }

    let mut inside = false;
    let mut prev = points[points.len() - 1];
    for &next in points {
        let intersects = ((next.y > point.y) != (prev.y > point.y))
            && (point.x < ((prev.x - next.x) * (point.y - next.y)) / (prev.y - next.y) + next.x);
        if intersects {
            inside = !inside;
        }
        prev = next;
    }
    inside
}

pub fn get_polygon_interval_for_band(
    points: &[Point],
    band_top: f32,
    band_bottom: f32,
    horizontal_padding: f32,
    vertical_padding: f32,
) -> Option<Interval> {
    scan_polygon_interval_for_band(
        points,
        band_top,
        band_bottom,
        horizontal_padding,
        vertical_padding,
    )
}

fn scan_polygon_interval_for_band(
    points: &[Point],
    band_top: f32,
    band_bottom: f32,
    horizontal_padding: f32,
    vertical_padding: f32,
) -> Option<Interval> {
    let sample_top = band_top - vertical_padding;
    let sample_bottom = band_bottom + vertical_padding;
    let start_y = sample_top.floor() as i32;
    let end_y = sample_bottom.ceil() as i32;

    let mut left = f32::INFINITY;
    let mut right = f32::NEG_INFINITY;

    for y in start_y..=end_y {
        let Some(row_interval) = scan_polygon_row_interval(points, y as f32 + 0.5) else {
            continue;
        };
        left = left.min(row_interval.left);
        right = right.max(row_interval.right);
    }

    if !left.is_finite() || !right.is_finite() {
        return None;
    }

    Some(Interval {
        left: left - horizontal_padding,
        right: right + horizontal_padding,
    })
}

fn scan_polygon_row_interval(points: &[Point], y: f32) -> Option<Interval> {
    let xs = polygon_xs_at_y(points, y);
    let mut left = f32::INFINITY;
    let mut right = f32::NEG_INFINITY;

    for pair in xs.chunks_exact(2) {
        left = left.min(pair[0]);
        right = right.max(pair[1]);
    }

    if !left.is_finite() || !right.is_finite() {
        return None;
    }

    Some(Interval { left, right })
}

pub fn get_rect_intervals_for_band(
    rects: &[Rect],
    band_top: f32,
    band_bottom: f32,
    horizontal_padding: f32,
    vertical_padding: f32,
) -> Vec<Interval> {
    let mut intervals = Vec::new();
    append_rect_intervals_for_band(
        &mut intervals,
        rects,
        band_top,
        band_bottom,
        horizontal_padding,
        vertical_padding,
    );
    intervals
}

pub fn append_rect_intervals_for_band(
    out: &mut Vec<Interval>,
    rects: &[Rect],
    band_top: f32,
    band_bottom: f32,
    horizontal_padding: f32,
    vertical_padding: f32,
) {
    for rect in rects.iter().copied() {
        if band_bottom <= rect.y - vertical_padding || band_top >= rect.bottom() + vertical_padding
        {
            continue;
        }
        out.push(Interval {
            left: rect.x - horizontal_padding,
            right: rect.right() + horizontal_padding,
        });
    }
}

pub fn carve_text_line_slots_into(
    base: Interval,
    blocked: &[Interval],
    slots: &mut Vec<Interval>,
    scratch: &mut Vec<Interval>,
) {
    slots.clear();
    slots.push(base);
    scratch.clear();
    for interval in blocked {
        scratch.clear();
        for slot in slots.iter().copied() {
            if interval.right <= slot.left || interval.left >= slot.right {
                scratch.push(slot);
                continue;
            }
            if interval.left > slot.left {
                scratch.push(Interval {
                    left: slot.left,
                    right: interval.left,
                });
            }
            if interval.right < slot.right {
                scratch.push(Interval {
                    left: interval.right,
                    right: slot.right,
                });
            }
        }
        std::mem::swap(slots, scratch);
    }

    slots.retain(|slot| slot.right - slot.left >= 24.0);
    scratch.clear();
}

pub fn carve_text_line_slots(base: Interval, blocked: &[Interval]) -> Vec<Interval> {
    let mut slots = Vec::new();
    let mut scratch = Vec::new();
    carve_text_line_slots_into(base, blocked, &mut slots, &mut scratch);
    slots
}

pub fn svg_alpha_hull(svg_bytes: &[u8], raster_size: [usize; 2]) -> Option<Vec<Point>> {
    let options = usvg::Options::default();
    let tree = usvg::Tree::from_data(svg_bytes, &options).ok()?;
    let mut pixmap = tiny_skia::Pixmap::new(raster_size[0] as u32, raster_size[1] as u32)?;
    let svg_size = tree.size();
    let scale_x = raster_size[0] as f32 / svg_size.width();
    let scale_y = raster_size[1] as f32 / svg_size.height();
    let transform = tiny_skia::Transform::from_scale(scale_x, scale_y);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let width = raster_size[0] as i32;
    let height = raster_size[1] as i32;
    let mut lefts = vec![None; height as usize];
    let mut rights = vec![None; height as usize];
    let mut valid_rows = Vec::new();

    for y in 0..height {
        let mut left = None;
        let mut right = None;
        for x in 0..width {
            let alpha = alpha_at(pixmap.data(), width, x, y);
            if alpha < WRAP_HULL_ALPHA_THRESHOLD {
                continue;
            }
            if left.is_none() {
                left = Some(x as f32);
            }
            right = Some((x + 1) as f32);
        }
        if let (Some(left), Some(right)) = (left, right) {
            lefts[y as usize] = Some(left);
            rights[y as usize] = Some(right);
            valid_rows.push(y as usize);
        }
    }

    if valid_rows.is_empty() {
        return None;
    }

    let mut smoothed_lefts = vec![0.0f32; height as usize];
    let mut smoothed_rights = vec![0.0f32; height as usize];
    for &row in &valid_rows {
        let mut left_sum = 0.0f32;
        let mut right_sum = 0.0f32;
        let mut count = 0.0f32;

        for sample in
            (row as i32 - WRAP_HULL_SMOOTH_RADIUS)..=(row as i32 + WRAP_HULL_SMOOTH_RADIUS)
        {
            if sample < 0 || sample >= height {
                continue;
            }
            let sample = sample as usize;
            let (Some(left), Some(right)) = (lefts[sample], rights[sample]) else {
                continue;
            };
            left_sum += left;
            right_sum += right;
            count += 1.0;
        }

        if count == 0.0 {
            continue;
        }

        smoothed_lefts[row] = left_sum / count;
        smoothed_rights[row] = right_sum / count;
    }

    let step = (valid_rows.len() / WRAP_HULL_MAX_ROW_SAMPLES).max(1);
    let mut sampled_rows = Vec::new();
    let mut index = 0usize;
    while index < valid_rows.len() {
        sampled_rows.push(valid_rows[index]);
        index += step;
    }
    if sampled_rows.last().copied() != valid_rows.last().copied() {
        sampled_rows.push(*valid_rows.last().expect("valid rows should be non-empty"));
    }

    let mut hull = Vec::with_capacity(sampled_rows.len() * 2);
    for &row in &sampled_rows {
        hull.push(Point {
            x: smoothed_lefts[row],
            y: row as f32 + 0.5,
        });
    }
    for &row in sampled_rows.iter().rev() {
        hull.push(Point {
            x: smoothed_rights[row],
            y: row as f32 + 0.5,
        });
    }

    let bounds = hull_bounds(&hull)?;
    let width = bounds.width.max(1.0);
    let height = bounds.height.max(1.0);
    Some(
        hull.into_iter()
            .map(|point| Point {
                x: (point.x - bounds.x) / width,
                y: (point.y - bounds.y) / height,
            })
            .collect(),
    )
}

fn cross(origin: Point, a: Point, b: Point) -> f32 {
    (a.x - origin.x) * (b.y - origin.y) - (a.y - origin.y) * (b.x - origin.x)
}

fn polygon_xs_at_y(points: &[Point], y: f32) -> Vec<f32> {
    let mut xs = Vec::new();
    let mut prev = match points.last().copied() {
        Some(point) => point,
        None => return xs,
    };

    for &next in points {
        if (prev.y <= y && y < next.y) || (next.y <= y && y < prev.y) {
            xs.push(prev.x + ((y - prev.y) * (next.x - prev.x)) / (next.y - prev.y));
        }
        prev = next;
    }
    xs.sort_by(|left, right| left.total_cmp(right));
    xs
}

fn alpha_at(data: &[u8], width: i32, x: i32, y: i32) -> u8 {
    let index = ((y * width + x) * 4 + 3) as usize;
    data.get(index).copied().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hull_of_scattered_points_is_convex_ring() {
        let points = [
            Point { x: 0.0, y: 0.0 },
            Point { x: 2.0, y: 1.0 },
            Point { x: 1.0, y: 2.0 },
            Point { x: 0.5, y: 1.0 },
            Point { x: 1.2, y: 0.4 },
        ];
        let hull = alpha_hull(&points);
        assert!(hull.len() >= 3);
        let bounds = hull_bounds(&hull).unwrap();
        assert!(bounds.width >= 2.0);
        assert!(bounds.height >= 2.0);
    }

    #[test]
    fn carve_slots_removes_blocked_ranges() {
        let slots = carve_text_line_slots(
            Interval {
                left: 80.0,
                right: 420.0,
            },
            &[Interval {
                left: 200.0,
                right: 310.0,
            }],
        );
        assert_eq!(
            slots,
            vec![
                Interval {
                    left: 80.0,
                    right: 200.0
                },
                Interval {
                    left: 310.0,
                    right: 420.0
                }
            ]
        );
    }

    #[test]
    fn carve_slots_into_matches_allocating_variant() {
        let base = Interval {
            left: 80.0,
            right: 420.0,
        };
        let blocked = [
            Interval {
                left: 110.0,
                right: 150.0,
            },
            Interval {
                left: 180.0,
                right: 260.0,
            },
            Interval {
                left: 320.0,
                right: 360.0,
            },
        ];
        let expected = carve_text_line_slots(base, &blocked);
        let mut slots = vec![Interval {
            left: 0.0,
            right: 1.0,
        }];
        let mut scratch = vec![Interval {
            left: 2.0,
            right: 3.0,
        }];

        carve_text_line_slots_into(base, &blocked, &mut slots, &mut scratch);

        assert_eq!(slots, expected);
        assert!(scratch.is_empty());
    }

    #[test]
    fn svg_alpha_hull_normalizes_into_unit_bounds() {
        let hull = svg_alpha_hull(
            include_bytes!("../assets/logos/openai-symbol.svg"),
            [320, 320],
        )
        .expect("openai logo hull");
        assert!(!hull.is_empty());
        let bounds = hull_bounds(&hull).expect("normalized hull bounds");
        assert!(bounds.x.abs() < 0.01);
        assert!(bounds.y.abs() < 0.01);
        assert!((bounds.right() - 1.0).abs() < 0.02);
        assert!((bounds.bottom() - 1.0).abs() < 0.02);
    }

    #[test]
    fn normalized_hull_bounds_cover_unit_rect_corners() {
        let hull = svg_alpha_hull(
            include_bytes!("../assets/logos/claude-symbol.svg"),
            [320, 320],
        )
        .expect("claude logo hull");
        let bounds = hull_bounds(&hull).expect("normalized hull bounds");

        assert!(bounds.contains_point(Point { x: 0.0, y: 0.0 }));
        assert!(bounds.contains_point(Point { x: 1.0, y: 0.0 }));
        assert!(bounds.contains_point(Point { x: 0.0, y: 1.0 }));
        assert!(bounds.contains_point(Point { x: 1.0, y: 1.0 }));
    }

    #[test]
    fn polygon_band_cache_matches_direct_polygon_queries() {
        let polygon = vec![
            Point { x: 40.0, y: 20.0 },
            Point { x: 140.0, y: 28.0 },
            Point { x: 168.0, y: 92.0 },
            Point { x: 104.0, y: 154.0 },
            Point { x: 28.0, y: 118.0 },
        ];
        let cache = PolygonBandCache::new(&polygon);
        let bands = [(18.0, 34.0), (44.0, 70.0), (88.0, 126.0)];

        for (band_top, band_bottom) in bands {
            assert_eq!(
                cache.interval_for_band(band_top, band_bottom, 12.0, 8.0),
                get_polygon_interval_for_band(&polygon, band_top, band_bottom, 12.0, 8.0)
            );
        }
    }
}
