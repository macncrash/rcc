pub fn black_box<T>(x: T) -> T {
    x
}
pub fn spin_loop() {}
pub unsafe fn unreachable_unchecked() -> ! {
    crate::intrinsics::unreachable()
}
pub fn must_use<T>(x: T) -> T {
    x
}
