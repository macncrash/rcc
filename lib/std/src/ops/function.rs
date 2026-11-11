// `F: Fn(A, B) -> R` is sugar for `F: Fn<(A, B)>` + `<F as FnOnce<(A, B)>>::Output == R`.
// Closures, fn items and fn pointers implement these traits inside the
// compiler (typeck_traits.c); calls are resolved by mono (CR_CLOSURE /
// CR_FN_UNTUPLED).

#[lang = "fn_once"]
pub trait FnOnce<Args> {
    type Output;
    fn call_once(self, args: Args) -> Self::Output;
}

#[lang = "fn_mut"]
pub trait FnMut<Args>: FnOnce<Args> {
    fn call_mut(&mut self, args: Args) -> Self::Output;
}

#[lang = "fn"]
pub trait Fn<Args>: FnMut<Args> {
    fn call(&self, args: Args) -> Self::Output;
}

impl<A, F: ?Sized + Fn<A>> Fn<A> for &F {
    fn call(&self, args: A) -> F::Output {
        (**self).call(args)
    }
}
impl<A, F: ?Sized + Fn<A>> FnMut<A> for &F {
    fn call_mut(&mut self, args: A) -> F::Output {
        (**self).call(args)
    }
}
impl<A, F: ?Sized + Fn<A>> FnOnce<A> for &F {
    type Output = F::Output;
    fn call_once(self, args: A) -> F::Output {
        (*self).call(args)
    }
}
impl<A, F: ?Sized + FnMut<A>> FnMut<A> for &mut F {
    fn call_mut(&mut self, args: A) -> F::Output {
        (**self).call_mut(args)
    }
}
impl<A, F: ?Sized + FnMut<A>> FnOnce<A> for &mut F {
    type Output = F::Output;
    fn call_once(self, args: A) -> F::Output {
        (*self).call_mut(args)
    }
}
