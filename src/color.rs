use std::ops::*;

use eframe::epaint::{Color32, Hsva};
use image::Rgba;

#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }
    pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
    pub fn with_a(self, a: f32) -> Self {
        Self { a, ..self }
    }
    pub fn mul_a(self, a: f32) -> Self {
        Self {
            a: self.a * a,
            ..self
        }
    }
}

impl MulAssign<f32> for Color {
    fn mul_assign(&mut self, rhs: f32) {
        self.r *= rhs;
        self.g *= rhs;
        self.b *= rhs;
    }
}

impl Mul<f32> for Color {
    type Output = Self;
    fn mul(mut self, rhs: f32) -> Self::Output {
        self *= rhs;
        self
    }
}

impl From<Color> for Color32 {
    fn from(color: Color) -> Self {
        Self::from_rgba_unmultiplied(
            (color.r * 255.0).clamp(0.0, 255.0) as u8,
            (color.g * 255.0).clamp(0.0, 255.0) as u8,
            (color.b * 255.0).clamp(0.0, 255.0) as u8,
            (color.a * 255.0).clamp(1.0, 255.0) as u8,
        )
    }
}

impl From<Rgba<u8>> for Color {
    fn from(color: Rgba<u8>) -> Self {
        Color::rgba(
            color[0] as f32 / 255.0,
            color[1] as f32 / 255.0,
            color[2] as f32 / 255.0,
            color[3] as f32 / 255.0,
        )
    }
}

impl From<Hsva> for Color {
    fn from(color: Hsva) -> Self {
        let Hsva { h, s, v, a } = color;
        let h = h * 6.0;
        let i = h as i32;
        let f = h - i as f32;
        let p = v * (1.0 - s);
        let q = v * (1.0 - s * f);
        let t = v * (1.0 - s * (1.0 - f));
        match i {
            0 => Color::rgba(v, t, p, a),
            1 => Color::rgba(q, v, p, a),
            2 => Color::rgba(p, v, t, a),
            3 => Color::rgba(p, q, v, a),
            4 => Color::rgba(t, p, v, a),
            5 => Color::rgba(v, p, q, a),
            _ => Color::rgba(0.0, 0.0, 0.0, a),
        }
    }
}
