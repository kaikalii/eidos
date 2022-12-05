use std::f32::consts::PI;

use eframe::egui::*;

use crate::plot::MapPlot;

pub struct World {
    pub static_objects: Vec<Object>,
}

pub struct Object {
    pub shape: Vec<Vec2>,
}

impl Object {
    pub fn new(shape: impl IntoIterator<Item = Vec2>) -> Self {
        Object {
            shape: shape.into_iter().collect(),
        }
    }
}

fn rect_poly(min: Vec2, max: Vec2) -> Vec<Vec2> {
    vec![min, vec2(max.x, min.y), max, vec2(min.x, max.y)]
}

fn regular_poly(center: Vec2, radius: f32, sides: usize, rotation: f32) -> Vec<Vec2> {
    (0..sides)
        .map(|i| {
            let angle = i as f32 / sides as f32 * 2.0 * PI + rotation;
            center + radius * vec2(angle.sin(), angle.cos())
        })
        .collect()
}

impl Default for World {
    fn default() -> Self {
        World {
            static_objects: vec![
                Object::new(rect_poly(vec2(2.0, 3.0), vec2(6.0, 6.0))),
                Object::new(regular_poly(vec2(-3.0, -3.0), 1.5, 5, 0.0)),
            ],
        }
    }
}

impl World {
    pub fn ui(&self, ui: &mut Ui) {
        self.test_plot(ui);
    }
    fn test_plot(&self, ui: &mut Ui) {
        MapPlot::new("test", Vec2::ZERO, 10.0, |x, y| {
            let obstructed = self
                .static_objects
                .iter()
                .any(|obj| polygon_contains(&obj.shape, vec2(x, y) + Vec2::splat(1e-5)));
            obstructed as u8 as f32
        })
        .ui(ui);
    }
}

#[derive(PartialEq, Eq)]
enum TriOrientation {
    Cw,
    Ccw,
    Collinear,
}

fn polygon_contains(vertices: &[Vec2], point: Vec2) -> bool {
    let mut intersections = 0;
    let tester = vec2(1e6, point.y);
    for i in 0..vertices.len() {
        let a1 = vertices[i];
        let a2 = vertices[(i + 1) % vertices.len()];
        if segments_intersect(a1, a2, point, tester) {
            intersections += 1;
        }
    }
    intersections % 2 == 1
}

fn segments_intersect(p1: Vec2, q1: Vec2, p2: Vec2, q2: Vec2) -> bool {
    let o1 = orientation(p1, q1, p2);
    let o2 = orientation(p1, q1, q2);
    let o3 = orientation(p2, q2, p1);
    let o4 = orientation(p2, q2, q1);
    // General case
    if o1 != o2 && o3 != o4 {
        return true;
    }
    // Special Cases
    o1 == TriOrientation::Collinear && on_segment(p1, p2, q1)
        || o2 == TriOrientation::Collinear && on_segment(p1, q2, q1)
        || o3 == TriOrientation::Collinear && on_segment(p2, p1, q2)
        || o4 == TriOrientation::Collinear && on_segment(p2, q1, q2)
}

fn orientation(p: Vec2, q: Vec2, r: Vec2) -> TriOrientation {
    let val = (q.y - p.y) * (r.x - q.x) - (q.x - p.x) * (r.y - q.y);
    if val.abs() < f32::EPSILON * 7.0 {
        TriOrientation::Collinear
    } else if val > 0.0 {
        TriOrientation::Cw
    } else {
        TriOrientation::Ccw
    }
}
fn on_segment(p: Vec2, q: Vec2, r: Vec2) -> bool {
    q.x < p.x.max(r.x) && q.x > p.x.min(r.x) && q.y < p.y.max(r.y) && q.y > p.y.min(r.y)
}

#[test]
fn seg_test() {
    /*
      b
    c e d
      a
    */
    let a = vec2(0.0, 0.0);
    let b = vec2(0.0, 2.0);
    let c = vec2(-1.0, 1.0);
    let d = vec2(1.0, 1.0);
    let e = vec2(0.0, 1.0);
    assert!(segments_intersect(a, b, c, d));
    assert!(segments_intersect(a, e, c, d));
    assert!(segments_intersect(a, b, d, c));
    assert!(segments_intersect(b, a, c, d));
    assert!(!segments_intersect(a, d, c, e));
    assert!(!segments_intersect(a, d, c, b));
}

#[test]
fn polygon_contains_test() {
    let square = rect_poly(vec2(-1.0, -1.0), vec2(1.0, 1.0));
    assert!(polygon_contains(&square, vec2(0.0, 0.0)));
    assert!(polygon_contains(&square, vec2(0.0, 0.5)));
    assert!(polygon_contains(&square, vec2(0.6, 0.5)));

    let rectangle = rect_poly(vec2(1.0, 1.0), vec2(2.0, 3.0));
    assert!(!polygon_contains(&rectangle, vec2(0.0, 0.0)));
    assert!(polygon_contains(&rectangle, vec2(1.5, 2.0)));
    assert!(!polygon_contains(&rectangle, vec2(-1.0, 1.0)));
    assert!(!polygon_contains(&rectangle, vec2(-1.5, 2.0)));
}
