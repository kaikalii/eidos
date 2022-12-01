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
}
