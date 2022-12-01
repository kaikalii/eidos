use std::ops::RangeInclusive;

use eidos::*;

fn main() {
    let fibs = Field1::list([1.0, 1.0, 2.0, 3.0, 5.0, 8.0, 13.0, 21.0]);
    let fibs_times_x = fibs.square(BinOp::Add, Field::Identity);
    for s in fibs_times_x.sample_range(0.0..8.0, 1.0) {
        for s in s.sample_range(0.0..8.0, 1.0) {
            print!("{s:<4}");
        }
        println!();
    }
    println!();
    for s in Field1::Identity.flip(2.0).sample_range(0.0..=10.0, 1.0) {
        print!("{s:<4}");
    }
    println!();
    println!();
    for s in Field1::Identity
        .square(BinOp::Mul, Field1::Identity)
        .sample_range(0.0..=4.0, 0.25)
    {
        for s in s.sample_range(0.0..=4.0, 0.25) {
            print!("{}", sample_char(*s, 0.0..=16.0));
        }
        println!();
    }
}

const CHARS: &str = "⬛🟥🟧🟨🟩🟦🟪⬜";

fn sample_char(s: f32, range: RangeInclusive<f32>) -> char {
    let t = (s - *range.start()) / (*range.end() - *range.start());
    CHARS.chars().nth((t.min(0.9999) * 8.0) as usize).unwrap()
}
