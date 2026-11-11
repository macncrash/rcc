//! Stable merge sort (used for both the stable and "unstable" sorts).

use crate::cmp::Ordering;
use crate::intrinsics;

fn insertion_sort<T, F: FnMut(&T, &T) -> Ordering>(v: &mut [T], compare: &mut F) {
    let n = v.len();
    let p = v.as_mut_ptr();
    let mut i = 1;
    while i < n {
        unsafe {
            let mut j = i;
            if let Ordering::Greater = compare(&*p.add(j - 1), &*p.add(j)) {
                let tmp = intrinsics::read(p.add(j) as *const T);
                loop {
                    intrinsics::copy(p.add(j - 1) as *const T, p.add(j), 1);
                    j -= 1;
                    if j == 0 {
                        break;
                    }
                    if let Ordering::Greater = compare(&*p.add(j - 1), &tmp) {
                    } else {
                        break;
                    }
                }
                intrinsics::write(p.add(j), tmp);
            }
        }
        i += 1;
    }
}

/// Merge the sorted runs v[..mid] and v[mid..] using `buf` (capacity >= mid).
unsafe fn merge<T, F: FnMut(&T, &T) -> Ordering>(v: &mut [T], mid: usize, buf: *mut T, compare: &mut F) {
    let n = v.len();
    let p = v.as_mut_ptr();
    intrinsics::copy_nonoverlapping(p as *const T, buf, mid);
    let mut i = 0; // in buf
    let mut j = mid; // in v
    let mut k = 0; // output
    while i < mid && j < n {
        if let Ordering::Greater = compare(&*buf.add(i), &*p.add(j)) {
            intrinsics::copy(p.add(j) as *const T, p.add(k), 1);
            j += 1;
        } else {
            intrinsics::copy_nonoverlapping(buf.add(i) as *const T, p.add(k), 1);
            i += 1;
        }
        k += 1;
    }
    if i < mid {
        intrinsics::copy_nonoverlapping(buf.add(i) as *const T, p.add(k), mid - i);
    }
}

fn sort_rec<T, F: FnMut(&T, &T) -> Ordering>(v: &mut [T], buf: *mut T, compare: &mut F) {
    let n = v.len();
    if n <= 16 {
        insertion_sort(v, compare);
        return;
    }
    let mid = n / 2;
    {
        let (a, b) = v.split_at_mut(mid);
        sort_rec(a, buf, compare);
        sort_rec(b, buf, compare);
    }
    unsafe {
        // already ordered?
        if let Ordering::Greater = compare(&*v.as_ptr().add(mid - 1), &*v.as_ptr().add(mid)) {
            merge(v, mid, buf, compare);
        }
    }
}

pub(crate) fn merge_sort<T, F: FnMut(&T, &T) -> Ordering>(v: &mut [T], compare: &mut F) {
    let n = v.len();
    if n <= 16 {
        insertion_sort(v, compare);
        return;
    }
    unsafe {
        let buf = crate::alloc::alloc_array::<T>(n / 2 + 1);
        sort_rec(v, buf, compare);
        crate::alloc::dealloc_array(buf);
    }
}
