//! Hash / Hasher traits. The hashers and the collections live in
//! `collections` (not part of the compiler core).

pub trait Hasher {
    fn finish(&self) -> u64;
    fn write(&mut self, bytes: &[u8]);
    fn write_u8(&mut self, i: u8) {
        self.write(&[i])
    }
    fn write_u16(&mut self, i: u16) {
        self.write(&i.to_ne_bytes())
    }
    fn write_u32(&mut self, i: u32) {
        self.write(&i.to_ne_bytes())
    }
    fn write_u64(&mut self, i: u64) {
        self.write(&i.to_ne_bytes())
    }
    fn write_u128(&mut self, i: u128) {
        self.write(&i.to_ne_bytes())
    }
    fn write_usize(&mut self, i: usize) {
        self.write(&i.to_ne_bytes())
    }
    fn write_i8(&mut self, i: i8) {
        self.write_u8(i as u8)
    }
    fn write_i16(&mut self, i: i16) {
        self.write_u16(i as u16)
    }
    fn write_i32(&mut self, i: i32) {
        self.write_u32(i as u32)
    }
    fn write_i64(&mut self, i: i64) {
        self.write_u64(i as u64)
    }
    fn write_i128(&mut self, i: i128) {
        self.write_u128(i as u128)
    }
    fn write_isize(&mut self, i: isize) {
        self.write_usize(i as usize)
    }
    fn write_length_prefix(&mut self, len: usize) {
        self.write_usize(len);
    }
    fn write_str(&mut self, s: &str) {
        self.write(s.as_bytes());
        self.write_u8(0xff);
    }
}

impl<H: Hasher + ?Sized> Hasher for &mut H {
    fn finish(&self) -> u64 {
        (**self).finish()
    }
    fn write(&mut self, bytes: &[u8]) {
        (**self).write(bytes)
    }
    fn write_u8(&mut self, i: u8) {
        (**self).write_u8(i)
    }
    fn write_u16(&mut self, i: u16) {
        (**self).write_u16(i)
    }
    fn write_u32(&mut self, i: u32) {
        (**self).write_u32(i)
    }
    fn write_u64(&mut self, i: u64) {
        (**self).write_u64(i)
    }
    fn write_usize(&mut self, i: usize) {
        (**self).write_usize(i)
    }
}

pub trait Hash {
    fn hash<H: Hasher>(&self, state: &mut H);
    fn hash_slice<H: Hasher>(data: &[Self], state: &mut H)
    where
        Self: Sized,
    {
        let mut i = 0;
        while i < data.len() {
            data[i].hash(state);
            i += 1;
        }
    }
}

pub trait BuildHasher {
    type Hasher: Hasher;
    fn build_hasher(&self) -> Self::Hasher;
    fn hash_one<T: Hash>(&self, x: T) -> u64
    where
        Self: Sized,
    {
        let mut h = self.build_hasher();
        x.hash(&mut h);
        h.finish()
    }
}

pub struct BuildHasherDefault<H> {
    marker: crate::marker::PhantomData<H>,
}
impl<H> BuildHasherDefault<H> {
    pub const fn new() -> BuildHasherDefault<H> {
        BuildHasherDefault { marker: crate::marker::PhantomData }
    }
}
impl<H: Default + Hasher> BuildHasher for BuildHasherDefault<H> {
    type Hasher = H;
    fn build_hasher(&self) -> H {
        H::default()
    }
}
impl<H> Clone for BuildHasherDefault<H> {
    fn clone(&self) -> BuildHasherDefault<H> {
        BuildHasherDefault { marker: crate::marker::PhantomData }
    }
}
impl<H> Default for BuildHasherDefault<H> {
    fn default() -> BuildHasherDefault<H> {
        BuildHasherDefault { marker: crate::marker::PhantomData }
    }
}

macro_rules! impl_hash_int {
    ($($t:ty, $m:ident;)*) => { $(
        impl Hash for $t {
            fn hash<H: Hasher>(&self, state: &mut H) { state.$m(*self) }
            fn hash_slice<H: Hasher>(data: &[$t], state: &mut H) {
                let bytes = unsafe {
                    crate::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * crate::mem::size_of::<$t>())
                };
                state.write(bytes)
            }
        }
    )* };
}
impl_hash_int! {
    u8, write_u8; u16, write_u16; u32, write_u32; u64, write_u64; u128, write_u128; usize, write_usize;
    i8, write_i8; i16, write_i16; i32, write_i32; i64, write_i64; i128, write_i128; isize, write_isize;
}

impl Hash for bool {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u8(*self as u8)
    }
}
impl Hash for char {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u32(*self as u32)
    }
}
impl Hash for str {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_str(self)
    }
}
impl Hash for () {
    fn hash<H: Hasher>(&self, _state: &mut H) {}
}
impl<T: ?Sized + Hash> Hash for &T {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state)
    }
}
impl<T: ?Sized + Hash> Hash for &mut T {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state)
    }
}
impl<T: Hash> Hash for [T] {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_length_prefix(self.len());
        Hash::hash_slice(self, state)
    }
}
impl<T: Hash, const N: usize> Hash for [T; N] {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let s: &[T] = self;
        s.hash(state)
    }
}
impl<T: Hash> Hash for Option<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            None => state.write_isize(0),
            Some(x) => {
                state.write_isize(1);
                x.hash(state)
            }
        }
    }
}
impl<T: Hash, E: Hash> Hash for Result<T, E> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Ok(x) => {
                state.write_isize(0);
                x.hash(state)
            }
            Err(e) => {
                state.write_isize(1);
                e.hash(state)
            }
        }
    }
}

macro_rules! tuple_hash {
    ($( ($($name:ident $idx:tt)+) )*) => { $(
        impl<$($name: Hash),+> Hash for ($($name,)+) {
            fn hash<S: Hasher>(&self, state: &mut S) { $(self.$idx.hash(state);)+ }
        }
    )* };
}
tuple_hash! {
    (A 0) (A 0 B 1) (A 0 B 1 C 2) (A 0 B 1 C 2 D 3) (A 0 B 1 C 2 D 3 E 4)
    (A 0 B 1 C 2 D 3 E 4 F 5) (A 0 B 1 C 2 D 3 E 4 F 5 G 6) (A 0 B 1 C 2 D 3 E 4 F 5 G 6 H 7)
}

// ---- SipHash-1-3, DefaultHasher, RandomState (owned by std::collections) ----
//
// `DefaultHasher::new()` is SipHash-1-3 with zero keys, exactly as real std,
// so printed hash values match rustc's. `RandomState` is deterministic: its
// keys are fixed (zero), so `RandomState::new().build_hasher()` hashes like
// `DefaultHasher::new()`.

#[derive(Clone, Copy, Debug)]
struct SipState {
    v0: u64,
    v1: u64,
    v2: u64,
    v3: u64,
}

impl SipState {
    #[inline]
    fn round(&mut self) {
        self.v0 = self.v0.wrapping_add(self.v1);
        self.v1 = self.v1.rotate_left(13);
        self.v1 ^= self.v0;
        self.v0 = self.v0.rotate_left(32);
        self.v2 = self.v2.wrapping_add(self.v3);
        self.v3 = self.v3.rotate_left(16);
        self.v3 ^= self.v2;
        self.v0 = self.v0.wrapping_add(self.v3);
        self.v3 = self.v3.rotate_left(21);
        self.v3 ^= self.v0;
        self.v2 = self.v2.wrapping_add(self.v1);
        self.v1 = self.v1.rotate_left(17);
        self.v1 ^= self.v2;
        self.v2 = self.v2.rotate_left(32);
    }
}

/// Little-endian load of `len` (< 8) bytes of `buf` starting at `start`.
#[inline]
fn sip_load_partial(buf: &[u8], start: usize, len: usize) -> u64 {
    let mut out = 0u64;
    let mut i = 0;
    while i < len {
        out |= (buf[start + i] as u64) << (8 * i);
        i += 1;
    }
    out
}

#[inline]
fn sip_load_u64(buf: &[u8], i: usize) -> u64 {
    (buf[i] as u64)
        | ((buf[i + 1] as u64) << 8)
        | ((buf[i + 2] as u64) << 16)
        | ((buf[i + 3] as u64) << 24)
        | ((buf[i + 4] as u64) << 32)
        | ((buf[i + 5] as u64) << 40)
        | ((buf[i + 6] as u64) << 48)
        | ((buf[i + 7] as u64) << 56)
}

/// SipHash-1-3 (one compression round, three finalization rounds).
#[derive(Clone, Copy, Debug)]
pub struct SipHasher13 {
    k0: u64,
    k1: u64,
    length: usize,
    state: SipState,
    tail: u64,
    ntail: usize,
}

impl SipHasher13 {
    pub fn new() -> SipHasher13 {
        SipHasher13::new_with_keys(0, 0)
    }
    pub fn new_with_keys(key0: u64, key1: u64) -> SipHasher13 {
        SipHasher13 {
            k0: key0,
            k1: key1,
            length: 0,
            state: SipState {
                v0: key0 ^ 0x736f6d6570736575,
                v1: key1 ^ 0x646f72616e646f6d,
                v2: key0 ^ 0x6c7967656e657261,
                v3: key1 ^ 0x7465646279746573,
            },
            tail: 0,
            ntail: 0,
        }
    }
    #[inline]
    fn absorb(&mut self, m: u64) {
        self.state.v3 ^= m;
        self.state.round();
        self.state.v0 ^= m;
    }
}

impl Default for SipHasher13 {
    fn default() -> SipHasher13 {
        SipHasher13::new()
    }
}

impl Hasher for SipHasher13 {
    fn write(&mut self, msg: &[u8]) {
        let length = msg.len();
        self.length += length;
        let mut needed = 0;
        if self.ntail != 0 {
            needed = 8 - self.ntail;
            let fill = if length < needed { length } else { needed };
            self.tail |= sip_load_partial(msg, 0, fill) << (8 * (self.ntail & 7));
            if length < needed {
                self.ntail += length;
                return;
            }
            let t = self.tail;
            self.absorb(t);
            self.ntail = 0;
        }
        let len = length - needed;
        let left = len & 7;
        let end = needed + (len - left);
        let mut i = needed;
        while i < end {
            let mi = sip_load_u64(msg, i);
            self.absorb(mi);
            i += 8;
        }
        self.tail = sip_load_partial(msg, i, left);
        self.ntail = left;
    }
    fn write_u8(&mut self, i: u8) {
        self.write(&[i])
    }
    fn write_str(&mut self, s: &str) {
        self.write(s.as_bytes());
        self.write_u8(0xff);
    }
    fn finish(&self) -> u64 {
        let mut state = self.state;
        let b: u64 = (((self.length as u64) & 0xff) << 56) | self.tail;
        state.v3 ^= b;
        state.round();
        state.v0 ^= b;
        state.v2 ^= 0xff;
        state.round();
        state.round();
        state.round();
        state.v0 ^ state.v1 ^ state.v2 ^ state.v3
    }
}

/// The default hasher of `HashMap`/`HashSet` (SipHash-1-3).
#[derive(Clone, Debug)]
pub struct DefaultHasher(SipHasher13);

impl DefaultHasher {
    pub fn new() -> DefaultHasher {
        DefaultHasher(SipHasher13::new_with_keys(0, 0))
    }
}
impl Default for DefaultHasher {
    fn default() -> DefaultHasher {
        DefaultHasher::new()
    }
}
impl Hasher for DefaultHasher {
    fn write(&mut self, msg: &[u8]) {
        self.0.write(msg)
    }
    fn write_str(&mut self, s: &str) {
        self.0.write_str(s)
    }
    fn finish(&self) -> u64 {
        self.0.finish()
    }
}

/// Deterministic stand-in for std's `RandomState` (fixed zero keys).
#[derive(Clone, Debug)]
pub struct RandomState {
    k0: u64,
    k1: u64,
}
impl RandomState {
    pub fn new() -> RandomState {
        RandomState { k0: 0, k1: 0 }
    }
}
impl Default for RandomState {
    fn default() -> RandomState {
        RandomState::new()
    }
}
impl BuildHasher for RandomState {
    type Hasher = DefaultHasher;
    fn build_hasher(&self) -> DefaultHasher {
        DefaultHasher(SipHasher13::new_with_keys(self.k0, self.k1))
    }
}
