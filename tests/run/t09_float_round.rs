// floor, ceil, round, trunc, fract, round_ties_even
fn main() {
    let vals: [f64; 14] = [0.0, -0.0, 0.4, 0.5, 0.6, 1.5, 2.5, -0.5, -1.5, -2.5, -2.7, 123456.789, 1e20, -1e-20];
    for &v in vals.iter() {
        println!(
            "{}: floor {} ceil {} round {} trunc {} fract {} even {}",
            v,
            v.floor(),
            v.ceil(),
            v.round(),
            v.trunc(),
            v.fract(),
            v.round_ties_even()
        );
    }
    let fv: [f32; 5] = [0.5, -0.5, 2.5, -3.75, 1e10];
    for &v in fv.iter() {
        println!("f32 {}: floor {} ceil {} round {} trunc {}", v, v.floor(), v.ceil(), v.round(), v.trunc());
    }
    println!("inf floor {} nan round nan {}", f64::INFINITY.floor(), f64::NAN.round().is_nan());
    // rounding to n decimal places
    let x = 2.718281828459045f64;
    for places in 0..6 {
        let m = 10f64.powi(places);
        println!("round {} places = {}", places, (x * m).round() / m);
    }
    // floor-div style conversion
    for v in [-3.5f64, -0.1, 0.1, 3.5] {
        println!("{} floor as i32 {} trunc as i32 {}", v, v.floor() as i32, v as i32);
    }
}
