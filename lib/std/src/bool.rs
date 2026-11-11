//! Inherent methods of `bool`.

impl bool {
    pub fn then_some<T>(self, t: T) -> Option<T> {
        if self { Some(t) } else { None }
    }
    pub fn then<T, F: FnOnce() -> T>(self, f: F) -> Option<T> {
        if self { Some(f()) } else { None }
    }
}
