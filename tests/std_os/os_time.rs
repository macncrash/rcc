// Duration arithmetic/conversions/Debug formatting, SystemTime arithmetic, Instant ordering (no measured times printed)
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn main() {
    let ds = [
        Duration::ZERO,
        Duration::from_nanos(1),
        Duration::from_nanos(999),
        Duration::from_nanos(1_000),
        Duration::from_nanos(1_500),
        Duration::from_micros(999_999),
        Duration::from_millis(1),
        Duration::from_millis(1_500),
        Duration::new(1, 5),
        Duration::new(59, 999_999_999),
        Duration::from_secs(3600),
        Duration::new(5, 1_000_000_123),
        Duration::MAX,
    ];
    for d in ds {
        println!("{:?} | {:.0?} | {:.1?} | {:.3?} | {:>12?} | {:<10?}| {:+?} | secs {} ms {} us {} ns {} sub {} {} {}", d, d, d, d, d, d, d, d.as_secs(), d.as_millis(), d.as_micros(), d.as_nanos(), d.subsec_millis(), d.subsec_micros(), d.subsec_nanos());
    }
    println!("{:.2?} {:.0?} {:?}", Duration::from_nanos(999_999_999), Duration::from_nanos(9_999_999_999), Duration::new(0, 999_999_999));
    println!("f64 {} {} {}", Duration::from_millis(2500).as_secs_f64(), Duration::from_micros(1).as_secs_f64(), Duration::from_secs_f64(1.25).as_nanos());
    println!("f32 {}", Duration::from_millis(250).as_secs_f32());
    let a = Duration::from_millis(1800);
    let b = Duration::from_millis(700);
    println!("add {:?} sub {:?} mul {:?} div {:?} rmul {:?}", a + b, a - b, a * 3, a / 7, 2 * b);
    println!("checked {:?} {:?} {:?} {:?}", a.checked_sub(b), b.checked_sub(a), Duration::MAX.checked_add(b), a.checked_div(0));
    println!("saturating {:?} {:?}", b.saturating_sub(a), Duration::MAX.saturating_add(a));
    let mut acc = Duration::ZERO;
    acc += a;
    acc -= b;
    acc *= 2;
    acc /= 4;
    println!("assign ops {:?}", acc);
    let total: Duration = [a, b, Duration::from_nanos(5)].iter().sum();
    println!("sum {:?}", total);
    println!("cmp {} {} {:?}", a > b, Duration::from_secs(1) == Duration::from_millis(1000), a.max(b));
    println!("is_zero {} {}", Duration::ZERO.is_zero(), b.is_zero());
    println!("mul_f64 {:?} div_f64 {:?}", a.mul_f64(1.5), a.div_f64(4.0));
    println!("abs_diff {:?}", b.abs_diff(a));

    let t = UNIX_EPOCH + Duration::from_secs(1_000_000_000) + Duration::from_nanos(42);
    println!("systime since epoch {:?}", t.duration_since(UNIX_EPOCH));
    let earlier = t - Duration::from_secs(10);
    println!("diff {:?} reversed err {:?}", t.duration_since(earlier), earlier.duration_since(t).map_err(|e| e.duration()));
    println!("err display {}", earlier.duration_since(t).unwrap_err());
    println!("checked_sub before epoch {:?}", UNIX_EPOCH.checked_sub(Duration::from_secs(1)).map(|t| t < UNIX_EPOCH));
    println!("now after 2020 {}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() > 1_577_836_800);
    println!("epoch debug {:?}", UNIX_EPOCH);

    let i0 = Instant::now();
    let i1 = i0 + Duration::from_millis(250);
    println!("instant diff {:?} rev sat {:?} checked {:?}", i1 - i0, i0.saturating_duration_since(i1), i0.checked_duration_since(i1));
    println!("instant ord {} {}", i1 > i0, (i1 - Duration::from_millis(250)) == i0);
    let mut spin = 0u64;
    while i0.elapsed() < Duration::from_millis(3) {
        spin += 1;
    }
    println!("spun {}", spin > 0);
}
