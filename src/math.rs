use std::{
    f32::consts::PI,
    ops::{Add, Rem},
};

use eframe::epaint::{pos2, vec2, Pos2, Vec2};
use rapier2d::{na::Vector2, prelude::*};

pub fn round_to(x: f32, dx: f32) -> f32 {
    (x / dx).round() * dx
}

pub fn modulus<T>(x: T, m: T) -> T
where
    T: Copy + Add<Output = T> + Rem<Output = T>,
{
    (x % m + m) % m
}

pub fn rotate(v: Vec2, theta: f32) -> Vec2 {
    vec2(
        v.x * theta.cos() - v.y * theta.sin(),
        v.y * theta.cos() + v.x * theta.sin(),
    )
}

pub trait Convert<U> {
    fn convert(self) -> U;
}

impl Convert<Vector2<f32>> for Vec2 {
    fn convert(self) -> Vector2<f32> {
        vector!(self.x, self.y)
    }
}

impl Convert<Vec2> for Vector2<f32> {
    fn convert(self) -> Vec2 {
        vec2(self.x, self.y)
    }
}

impl Convert<Vector2<f32>> for Pos2 {
    fn convert(self) -> Vector2<f32> {
        vector!(self.x, self.y)
    }
}

impl Convert<Pos2> for Vector2<f32> {
    fn convert(self) -> Pos2 {
        pos2(self.x, self.y)
    }
}

impl Convert<Point<f32>> for Pos2 {
    fn convert(self) -> Point<f32> {
        [self.x, self.y].into()
    }
}

impl Convert<Pos2> for Point<f32> {
    fn convert(self) -> Pos2 {
        pos2(self.x, self.y)
    }
}

pub fn rect_poly(min: Pos2, max: Pos2) -> Vec<Pos2> {
    vec![min, pos2(max.x, min.y), max, pos2(min.x, max.y)]
}

pub fn regular_poly(center: Pos2, radius: f32, sides: usize, rotation: f32) -> Vec<Pos2> {
    (0..sides)
        .map(|i| {
            let angle = i as f32 / sides as f32 * 2.0 * PI + rotation;
            center + radius * Vec2::angled(angle)
        })
        .collect()
}

#[derive(PartialEq, Eq)]
enum TriOrientation {
    Cw,
    Ccw,
    Collinear,
}

pub fn polygon_contains(vertices: &[Pos2], point: Pos2) -> bool {
    let mut intersections = 0;
    let tester = pos2(1e6, point.y);
    for i in 0..vertices.len() {
        let a1 = vertices[i];
        let a2 = vertices[(i + 1) % vertices.len()];
        if segments_intersect(a1, a2, point, tester) {
            intersections += 1;
        }
    }
    intersections % 2 == 1
}

pub fn segments_intersect(p1: Pos2, q1: Pos2, p2: Pos2, q2: Pos2) -> bool {
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

fn orientation(p: Pos2, q: Pos2, r: Pos2) -> TriOrientation {
    let val = (q.y - p.y) * (r.x - q.x) - (q.x - p.x) * (r.y - q.y);
    if val.abs() < f32::EPSILON * 7.0 {
        TriOrientation::Collinear
    } else if val > 0.0 {
        TriOrientation::Cw
    } else {
        TriOrientation::Ccw
    }
}
fn on_segment(p: Pos2, q: Pos2, r: Pos2) -> bool {
    q.x < p.x.max(r.x) && q.x > p.x.min(r.x) && q.y < p.y.max(r.y) && q.y > p.y.min(r.y)
}

#[test]
fn seg_test() {
    /*
      b
    c e d
      a
    */
    let a = pos2(0.0, 0.0);
    let b = pos2(0.0, 2.0);
    let c = pos2(-1.0, 1.0);
    let d = pos2(1.0, 1.0);
    let e = pos2(0.0, 1.0);
    assert!(segments_intersect(a, b, c, d));
    assert!(segments_intersect(a, e, c, d));
    assert!(segments_intersect(a, b, d, c));
    assert!(segments_intersect(b, a, c, d));
    assert!(!segments_intersect(a, d, c, e));
    assert!(!segments_intersect(a, d, c, b));
}

#[test]
fn polygon_contains_test() {
    let square = rect_poly(pos2(-1.0, -1.0), pos2(1.0, 1.0));
    assert!(polygon_contains(&square, pos2(0.0, 0.0)));
    assert!(polygon_contains(&square, pos2(0.0, 0.5)));
    assert!(polygon_contains(&square, pos2(0.6, 0.5)));

    let rectangle = rect_poly(pos2(1.0, 1.0), pos2(2.0, 3.0));
    assert!(!polygon_contains(&rectangle, pos2(0.0, 0.0)));
    assert!(polygon_contains(&rectangle, pos2(1.5, 2.0)));
    assert!(!polygon_contains(&rectangle, pos2(-1.0, 1.0)));
    assert!(!polygon_contains(&rectangle, pos2(-1.5, 2.0)));
}
