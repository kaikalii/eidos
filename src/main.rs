use eidos::*;

fn main() {
    let fibs = Field1::list([1.0, 1.0, 2.0, 3.0, 5.0, 8.0, 13.0, 21.0]);
    let fibs_times_2 = fibs * 2.0;
    for s in fibs_times_2.sample_range(0.0..8.0, 1.0) {
        print!("{s:<4}");
    }
    println!();
}
