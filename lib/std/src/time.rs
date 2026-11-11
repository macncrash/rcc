//! `std::time`: `Duration` (Debug output identical to real std),
//! `Instant` (monotonic clock), `SystemTime`/`UNIX_EPOCH`.

use crate::fmt;
use crate::iter::Sum;
use crate::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};
use crate::os::sys;

const NANOS_PER_SEC: u32 = 1_000_000_000;
const NANOS_PER_MILLI: u32 = 1_000_000;
const NANOS_PER_MICRO: u32 = 1_000;
const MILLIS_PER_SEC: u64 = 1_000;
const MICROS_PER_SEC: u64 = 1_000_000;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Duration {
    secs: u64,
    nanos: u32,
}

impl Duration {
    pub const SECOND: Duration = Duration { secs: 1, nanos: 0 };
    pub const MILLISECOND: Duration = Duration { secs: 0, nanos: 1_000_000 };
    pub const MICROSECOND: Duration = Duration { secs: 0, nanos: 1_000 };
    pub const NANOSECOND: Duration = Duration { secs: 0, nanos: 1 };
    pub const ZERO: Duration = Duration { secs: 0, nanos: 0 };
    pub const MAX: Duration = Duration { secs: u64::MAX, nanos: 999_999_999 };

    pub fn new(secs: u64, nanos: u32) -> Duration {
        if nanos < NANOS_PER_SEC {
            return Duration { secs, nanos };
        }
        let extra = (nanos / NANOS_PER_SEC) as u64;
        let secs = match secs.checked_add(extra) {
            Some(s) => s,
            None => panic!("overflow in Duration::new"),
        };
        Duration { secs, nanos: nanos % NANOS_PER_SEC }
    }
    pub const fn from_secs(secs: u64) -> Duration {
        Duration { secs, nanos: 0 }
    }
    pub const fn from_millis(millis: u64) -> Duration {
        Duration { secs: millis / MILLIS_PER_SEC, nanos: ((millis % MILLIS_PER_SEC) as u32) * NANOS_PER_MILLI }
    }
    pub const fn from_micros(micros: u64) -> Duration {
        Duration { secs: micros / MICROS_PER_SEC, nanos: ((micros % MICROS_PER_SEC) as u32) * NANOS_PER_MICRO }
    }
    pub const fn from_nanos(nanos: u64) -> Duration {
        Duration { secs: nanos / (NANOS_PER_SEC as u64), nanos: (nanos % (NANOS_PER_SEC as u64)) as u32 }
    }
    pub fn from_mins(mins: u64) -> Duration {
        Duration::from_secs(mins * 60)
    }
    pub fn from_hours(hours: u64) -> Duration {
        Duration::from_secs(hours * 3600)
    }
    pub const fn is_zero(&self) -> bool {
        self.secs == 0 && self.nanos == 0
    }
    pub const fn as_secs(&self) -> u64 {
        self.secs
    }
    pub const fn subsec_millis(&self) -> u32 {
        self.nanos / NANOS_PER_MILLI
    }
    pub const fn subsec_micros(&self) -> u32 {
        self.nanos / NANOS_PER_MICRO
    }
    pub const fn subsec_nanos(&self) -> u32 {
        self.nanos
    }
    pub const fn as_millis(&self) -> u128 {
        self.secs as u128 * MILLIS_PER_SEC as u128 + (self.nanos / NANOS_PER_MILLI) as u128
    }
    pub const fn as_micros(&self) -> u128 {
        self.secs as u128 * MICROS_PER_SEC as u128 + (self.nanos / NANOS_PER_MICRO) as u128
    }
    pub const fn as_nanos(&self) -> u128 {
        self.secs as u128 * NANOS_PER_SEC as u128 + self.nanos as u128
    }
    pub fn as_secs_f64(&self) -> f64 {
        (self.secs as f64) + (self.nanos as f64) / (NANOS_PER_SEC as f64)
    }
    pub fn as_secs_f32(&self) -> f32 {
        (self.secs as f32) + (self.nanos as f32) / (NANOS_PER_SEC as f32)
    }
    pub fn as_millis_f64(&self) -> f64 {
        (self.secs as f64) * 1000.0 + (self.nanos as f64) / (NANOS_PER_MILLI as f64)
    }
    pub fn abs_diff(self, other: Duration) -> Duration {
        if self > other { self - other } else { other - self }
    }
    pub fn checked_add(self, rhs: Duration) -> Option<Duration> {
        match self.secs.checked_add(rhs.secs) {
            Some(mut secs) => {
                let mut nanos = self.nanos + rhs.nanos;
                if nanos >= NANOS_PER_SEC {
                    nanos -= NANOS_PER_SEC;
                    match secs.checked_add(1) {
                        Some(s) => secs = s,
                        None => return None,
                    }
                }
                Some(Duration { secs, nanos })
            }
            None => None,
        }
    }
    pub fn checked_sub(self, rhs: Duration) -> Option<Duration> {
        match self.secs.checked_sub(rhs.secs) {
            Some(mut secs) => {
                let nanos = if self.nanos >= rhs.nanos {
                    self.nanos - rhs.nanos
                } else {
                    match secs.checked_sub(1) {
                        Some(s) => {
                            secs = s;
                            self.nanos + NANOS_PER_SEC - rhs.nanos
                        }
                        None => return None,
                    }
                };
                Some(Duration { secs, nanos })
            }
            None => None,
        }
    }
    pub fn checked_mul(self, rhs: u32) -> Option<Duration> {
        let total_nanos = self.nanos as u64 * rhs as u64;
        let extra_secs = total_nanos / (NANOS_PER_SEC as u64);
        let nanos = (total_nanos % (NANOS_PER_SEC as u64)) as u32;
        match self.secs.checked_mul(rhs as u64) {
            Some(s) => match s.checked_add(extra_secs) {
                Some(secs) => Some(Duration { secs, nanos }),
                None => None,
            },
            None => None,
        }
    }
    pub fn checked_div(self, rhs: u32) -> Option<Duration> {
        if rhs == 0 {
            return None;
        }
        let secs = self.secs / (rhs as u64);
        let extra_secs = self.secs % (rhs as u64);
        let mut nanos = self.nanos / rhs;
        let extra_nanos = self.nanos % rhs;
        nanos += ((extra_secs * (NANOS_PER_SEC as u64) + extra_nanos as u64) / (rhs as u64)) as u32;
        Some(Duration::new(secs, nanos))
    }
    pub fn saturating_add(self, rhs: Duration) -> Duration {
        match self.checked_add(rhs) {
            Some(d) => d,
            None => Duration::MAX,
        }
    }
    pub fn saturating_sub(self, rhs: Duration) -> Duration {
        match self.checked_sub(rhs) {
            Some(d) => d,
            None => Duration::ZERO,
        }
    }
    pub fn saturating_mul(self, rhs: u32) -> Duration {
        match self.checked_mul(rhs) {
            Some(d) => d,
            None => Duration::MAX,
        }
    }
    pub fn from_secs_f64(secs: f64) -> Duration {
        match Duration::try_from_secs_f64(secs) {
            Ok(d) => d,
            Err(e) => panic!("{}", e.description()),
        }
    }
    pub fn from_secs_f32(secs: f32) -> Duration {
        Duration::from_secs_f64(secs as f64)
    }
    pub fn try_from_secs_f64(secs: f64) -> Result<Duration, TryFromFloatSecsError> {
        if secs < 0.0 {
            return Err(TryFromFloatSecsError { negative: true });
        }
        if !(secs < 18446744073709551616.0) {
            return Err(TryFromFloatSecsError { negative: false });
        }
        let whole = secs as u64;
        let mut nanos = ((secs - whole as f64) * 1e9 + 0.5) as u64;
        let mut s = whole;
        if nanos >= NANOS_PER_SEC as u64 {
            nanos -= NANOS_PER_SEC as u64;
            s += 1;
        }
        Ok(Duration { secs: s, nanos: nanos as u32 })
    }
    pub fn try_from_secs_f32(secs: f32) -> Result<Duration, TryFromFloatSecsError> {
        Duration::try_from_secs_f64(secs as f64)
    }
    pub fn mul_f64(self, rhs: f64) -> Duration {
        Duration::from_secs_f64(rhs * self.as_secs_f64())
    }
    pub fn mul_f32(self, rhs: f32) -> Duration {
        Duration::from_secs_f64(rhs as f64 * self.as_secs_f64())
    }
    pub fn div_f64(self, rhs: f64) -> Duration {
        Duration::from_secs_f64(self.as_secs_f64() / rhs)
    }
    pub fn div_f32(self, rhs: f32) -> Duration {
        Duration::from_secs_f64(self.as_secs_f64() / rhs as f64)
    }
    pub fn div_duration_f64(self, rhs: Duration) -> f64 {
        self.as_secs_f64() / rhs.as_secs_f64()
    }
    pub fn div_duration_f32(self, rhs: Duration) -> f32 {
        (self.as_secs_f64() / rhs.as_secs_f64()) as f32
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TryFromFloatSecsError {
    negative: bool,
}
impl TryFromFloatSecsError {
    fn description(&self) -> &'static str {
        if self.negative {
            "cannot convert float seconds to Duration: value is negative"
        } else {
            "cannot convert float seconds to Duration: value is either too big or NaN"
        }
    }
}
impl fmt::Display for TryFromFloatSecsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.description())
    }
}
impl crate::error::Error for TryFromFloatSecsError {}

impl Add for Duration {
    type Output = Duration;
    fn add(self, rhs: Duration) -> Duration {
        match self.checked_add(rhs) {
            Some(d) => d,
            None => panic!("overflow when adding durations"),
        }
    }
}
impl AddAssign for Duration {
    fn add_assign(&mut self, rhs: Duration) {
        *self = *self + rhs;
    }
}
impl Sub for Duration {
    type Output = Duration;
    fn sub(self, rhs: Duration) -> Duration {
        match self.checked_sub(rhs) {
            Some(d) => d,
            None => panic!("overflow when subtracting durations"),
        }
    }
}
impl SubAssign for Duration {
    fn sub_assign(&mut self, rhs: Duration) {
        *self = *self - rhs;
    }
}
impl Mul<u32> for Duration {
    type Output = Duration;
    fn mul(self, rhs: u32) -> Duration {
        match self.checked_mul(rhs) {
            Some(d) => d,
            None => panic!("overflow when multiplying duration by scalar"),
        }
    }
}
impl Mul<Duration> for u32 {
    type Output = Duration;
    fn mul(self, rhs: Duration) -> Duration {
        rhs * self
    }
}
impl MulAssign<u32> for Duration {
    fn mul_assign(&mut self, rhs: u32) {
        *self = *self * rhs;
    }
}
impl Div<u32> for Duration {
    type Output = Duration;
    fn div(self, rhs: u32) -> Duration {
        match self.checked_div(rhs) {
            Some(d) => d,
            None => panic!("divide by zero error when dividing duration by scalar"),
        }
    }
}
impl DivAssign<u32> for Duration {
    fn div_assign(&mut self, rhs: u32) {
        *self = *self / rhs;
    }
}
impl Sum for Duration {
    fn sum<I: Iterator<Item = Duration>>(iter: I) -> Duration {
        let mut total = Duration::ZERO;
        for d in iter {
            total = total + d;
        }
        total
    }
}
impl<'a> Sum<&'a Duration> for Duration {
    fn sum<I: Iterator<Item = &'a Duration>>(iter: I) -> Duration {
        let mut total = Duration::ZERO;
        for d in iter {
            total = total + *d;
        }
        total
    }
}

fn decimal_width(mut n: u64) -> usize {
    let mut w = 1;
    while n >= 10 {
        n /= 10;
        w += 1;
    }
    w
}

/// Port of real std's `fmt_decimal` (Duration's Debug).
fn fmt_decimal(
    f: &mut fmt::Formatter<'_>,
    integer_part: u64,
    mut fractional_part: u32,
    mut divisor: u32,
    prefix: &str,
    postfix: &str,
) -> fmt::Result {
    let mut buf = [b'0'; 9];
    let mut pos = 0;
    let end = match f.precision() {
        Some(p) => if p < 9 { p } else { 9 },
        None => 9,
    };
    while fractional_part > 0 && pos < end {
        buf[pos] = b'0' + (fractional_part / divisor) as u8;
        fractional_part %= divisor;
        divisor /= 10;
        pos += 1;
    }
    let mut overflow = false;
    let mut integer_part = integer_part;
    if fractional_part > 0 && fractional_part >= divisor * 5 {
        let mut rev_pos = pos;
        let mut carry = true;
        while carry && rev_pos > 0 {
            rev_pos -= 1;
            if buf[rev_pos] < b'9' {
                buf[rev_pos] += 1;
                carry = false;
            } else {
                buf[rev_pos] = b'0';
            }
        }
        if carry {
            match integer_part.checked_add(1) {
                Some(n) => integer_part = n,
                None => overflow = true,
            }
        }
    }
    let end = match f.precision() {
        Some(p) => if p < 9 { p } else { 9 },
        None => pos,
    };
    let frac_w = match f.precision() {
        Some(p) => p,
        None => pos,
    };

    let mut s = crate::string::String::new();
    s.push_str(prefix);
    if overflow {
        s.push_str("18446744073709551616");
    } else {
        s.push_str(&crate::fmt::format(format_args!("{}", integer_part)));
    }
    if end > 0 {
        s.push('.');
        let mut i = 0;
        while i < end {
            s.push(buf[i] as char);
            i += 1;
        }
        while i < frac_w {
            s.push('0');
            i += 1;
        }
    }
    s.push_str(postfix);

    match f.width() {
        None => f.write_str(&s),
        Some(requested) => {
            let mut actual = prefix.len() + postfix.chars().count();
            actual += if overflow { 20 } else { decimal_width(integer_part) };
            if end > 0 {
                actual += 1 + frac_w;
            }
            if requested <= actual {
                f.write_str(&s)
            } else {
                let pad = requested - actual;
                let fill = f.fill();
                let (pre, post) = match f.align() {
                    Some(fmt::Align::Right) => (pad, 0),
                    Some(fmt::Align::Center) => (pad / 2, (pad + 1) / 2),
                    _ => (0, pad),
                };
                for _ in 0..pre {
                    f.write_char(fill)?;
                }
                f.write_str(&s)?;
                for _ in 0..post {
                    f.write_char(fill)?;
                }
                Ok(())
            }
        }
    }
}

impl fmt::Debug for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = if f.sign_plus() { "+" } else { "" };
        if self.secs > 0 {
            fmt_decimal(f, self.secs, self.nanos, NANOS_PER_SEC / 10, prefix, "s")
        } else if self.nanos >= NANOS_PER_MILLI {
            fmt_decimal(
                f,
                (self.nanos / NANOS_PER_MILLI) as u64,
                self.nanos % NANOS_PER_MILLI,
                NANOS_PER_MILLI / 10,
                prefix,
                "ms",
            )
        } else if self.nanos >= NANOS_PER_MICRO {
            fmt_decimal(
                f,
                (self.nanos / NANOS_PER_MICRO) as u64,
                self.nanos % NANOS_PER_MICRO,
                NANOS_PER_MICRO / 10,
                prefix,
                "µs",
            )
        } else {
            fmt_decimal(f, self.nanos as u64, 0, 1, prefix, "ns")
        }
    }
}

// ---------------------------------------------------------------------------
// Instant

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant {
    t: Duration,
}

fn clock(which: i32) -> (i64, u32) {
    let mut s = 0i64;
    let mut n = 0i64;
    unsafe { sys::rcc_os_clock(which, &mut s, &mut n) };
    (s, n as u32)
}

impl Instant {
    pub fn now() -> Instant {
        let (s, n) = clock(0);
        Instant { t: Duration { secs: s as u64, nanos: n } }
    }
    pub fn duration_since(&self, earlier: Instant) -> Duration {
        self.saturating_duration_since(earlier)
    }
    pub fn checked_duration_since(&self, earlier: Instant) -> Option<Duration> {
        self.t.checked_sub(earlier.t)
    }
    pub fn saturating_duration_since(&self, earlier: Instant) -> Duration {
        self.t.saturating_sub(earlier.t)
    }
    pub fn elapsed(&self) -> Duration {
        Instant::now().duration_since(*self)
    }
    pub fn checked_add(&self, d: Duration) -> Option<Instant> {
        match self.t.checked_add(d) {
            Some(t) => Some(Instant { t }),
            None => None,
        }
    }
    pub fn checked_sub(&self, d: Duration) -> Option<Instant> {
        match self.t.checked_sub(d) {
            Some(t) => Some(Instant { t }),
            None => None,
        }
    }
}

impl Add<Duration> for Instant {
    type Output = Instant;
    fn add(self, d: Duration) -> Instant {
        match self.checked_add(d) {
            Some(i) => i,
            None => panic!("overflow when adding duration to instant"),
        }
    }
}
impl AddAssign<Duration> for Instant {
    fn add_assign(&mut self, d: Duration) {
        *self = *self + d;
    }
}
impl Sub<Duration> for Instant {
    type Output = Instant;
    fn sub(self, d: Duration) -> Instant {
        match self.checked_sub(d) {
            Some(i) => i,
            None => panic!("overflow when subtracting duration from instant"),
        }
    }
}
impl SubAssign<Duration> for Instant {
    fn sub_assign(&mut self, d: Duration) {
        *self = *self - d;
    }
}
impl Sub<Instant> for Instant {
    type Output = Duration;
    fn sub(self, other: Instant) -> Duration {
        self.duration_since(other)
    }
}
impl fmt::Debug for Instant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Instant").field("tv_sec", &self.t.secs).field("tv_nsec", &self.t.nanos).finish()
    }
}

// ---------------------------------------------------------------------------
// SystemTime

/// Seconds (may be negative) and nanoseconds since the Unix epoch.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SystemTime {
    secs: i64,
    nanos: u32,
}

pub const UNIX_EPOCH: SystemTime = SystemTime { secs: 0, nanos: 0 };

#[derive(Clone, Debug)]
pub struct SystemTimeError(Duration);

impl SystemTimeError {
    pub fn duration(&self) -> Duration {
        self.0
    }
}
impl fmt::Display for SystemTimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("second time provided was later than self")
    }
}
impl crate::error::Error for SystemTimeError {}

impl SystemTime {
    pub const UNIX_EPOCH: SystemTime = SystemTime { secs: 0, nanos: 0 };

    pub(crate) fn from_parts(secs: i64, nanos: u32) -> SystemTime {
        SystemTime { secs, nanos }
    }
    pub fn now() -> SystemTime {
        let (s, n) = clock(1);
        SystemTime { secs: s, nanos: n }
    }
    fn sub_time(&self, other: &SystemTime) -> Result<Duration, Duration> {
        if self >= other {
            let (secs, nanos) = if self.nanos >= other.nanos {
                ((self.secs - other.secs) as u64, self.nanos - other.nanos)
            } else {
                ((self.secs - other.secs - 1) as u64, self.nanos + NANOS_PER_SEC - other.nanos)
            };
            Ok(Duration::new(secs, nanos))
        } else {
            match other.sub_time(self) {
                Ok(d) => Err(d),
                Err(d) => Ok(d),
            }
        }
    }
    pub fn duration_since(&self, earlier: SystemTime) -> Result<Duration, SystemTimeError> {
        match self.sub_time(&earlier) {
            Ok(d) => Ok(d),
            Err(d) => Err(SystemTimeError(d)),
        }
    }
    pub fn elapsed(&self) -> Result<Duration, SystemTimeError> {
        SystemTime::now().duration_since(*self)
    }
    pub fn checked_add(&self, d: Duration) -> Option<SystemTime> {
        let mut secs = match (d.secs as i64).checked_add(self.secs) {
            Some(s) => s,
            None => return None,
        };
        let mut nanos = self.nanos + d.nanos;
        if nanos >= NANOS_PER_SEC {
            nanos -= NANOS_PER_SEC;
            secs = match secs.checked_add(1) {
                Some(s) => s,
                None => return None,
            };
        }
        Some(SystemTime { secs, nanos })
    }
    pub fn checked_sub(&self, d: Duration) -> Option<SystemTime> {
        let mut secs = match self.secs.checked_sub(d.secs as i64) {
            Some(s) => s,
            None => return None,
        };
        let nanos = if self.nanos >= d.nanos {
            self.nanos - d.nanos
        } else {
            secs = match secs.checked_sub(1) {
                Some(s) => s,
                None => return None,
            };
            self.nanos + NANOS_PER_SEC - d.nanos
        };
        Some(SystemTime { secs, nanos })
    }
}

impl Add<Duration> for SystemTime {
    type Output = SystemTime;
    fn add(self, d: Duration) -> SystemTime {
        match self.checked_add(d) {
            Some(t) => t,
            None => panic!("overflow when adding duration to instant"),
        }
    }
}
impl AddAssign<Duration> for SystemTime {
    fn add_assign(&mut self, d: Duration) {
        *self = *self + d;
    }
}
impl Sub<Duration> for SystemTime {
    type Output = SystemTime;
    fn sub(self, d: Duration) -> SystemTime {
        match self.checked_sub(d) {
            Some(t) => t,
            None => panic!("overflow when subtracting duration from instant"),
        }
    }
}
impl SubAssign<Duration> for SystemTime {
    fn sub_assign(&mut self, d: Duration) {
        *self = *self - d;
    }
}
impl fmt::Debug for SystemTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SystemTime").field("tv_sec", &self.secs).field("tv_nsec", &self.nanos).finish()
    }
}

/// Used by `std::thread::sleep` implementations that want it.
pub(crate) fn sleep(d: Duration) {
    unsafe { sys::rcc_os_sleep(d.secs, d.nanos) }
}
