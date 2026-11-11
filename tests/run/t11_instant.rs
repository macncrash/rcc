// std::time::Instant / Duration: monotonic and arithmetic, without printing measured times
use std::time::{Duration, Instant};

fn main() {
    let start = Instant::now();
    let mut x: u64 = 0;
    for i in 0..1_000_000u64 {
        x = x.wrapping_add(i * i);
    }
    let mid = Instant::now();
    let el = mid.duration_since(start);
    println!("work result {}", x);
    println!("monotonic {}", mid >= start);
    println!("elapsed nonnegative {}", el >= Duration::ZERO);
    println!("elapsed() >= duration_since {}", start.elapsed() >= el);
    println!("checked_duration_since reversed is_none_or_zero {}", start.checked_duration_since(mid).map_or(true, |d| d == Duration::ZERO));
    println!("saturating reversed zero {}", start.saturating_duration_since(mid) == Duration::ZERO);
    let later = start + Duration::from_millis(5);
    println!("later - start = {:?}", later - start);
    let d = Duration::from_secs(3) + Duration::from_millis(250) + Duration::from_nanos(7);
    println!("secs {} millis {} micros {} nanos {} subsec_nanos {}", d.as_secs(), d.as_millis(), d.as_micros(), d.as_nanos(), d.subsec_nanos());
    println!("as_secs_f64 {}", Duration::from_millis(1500).as_secs_f64());
    println!("debug {:?} {:?} {:?}", Duration::from_millis(1500), Duration::from_micros(42), Duration::from_nanos(3));
    println!("mul {:?} div {:?}", Duration::from_millis(300) * 3, Duration::from_secs(1) / 4);
    println!("checked_sub {:?}", Duration::from_secs(1).checked_sub(Duration::from_secs(2)));
    println!("cmp {}", Duration::from_secs(1) > Duration::from_millis(999));
    std::thread::sleep(Duration::from_millis(2));
    println!("slept at least 2ms {}", start.elapsed() >= Duration::from_millis(2));
}
