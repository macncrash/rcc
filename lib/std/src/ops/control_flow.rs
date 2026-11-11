pub enum ControlFlow<B, C = ()> {
    Continue(C),
    Break(B),
}

impl<B, C> ControlFlow<B, C> {
    pub fn is_break(&self) -> bool {
        match self {
            ControlFlow::Break(_) => true,
            ControlFlow::Continue(_) => false,
        }
    }
    pub fn is_continue(&self) -> bool {
        !self.is_break()
    }
    pub fn break_value(self) -> Option<B> {
        match self {
            ControlFlow::Break(b) => Some(b),
            ControlFlow::Continue(_) => None,
        }
    }
}

/// What `Iterator::try_fold` and friends are generic over (`Option`,
/// `Result`, `ControlFlow`). The `?` operator itself is built into the
/// compiler for `Option` and `Result` and does not use this trait.
pub trait Try {
    type Output;
    type Residual;
    fn from_output(output: Self::Output) -> Self;
    fn from_residual(residual: Self::Residual) -> Self;
    fn branch(self) -> ControlFlow<Self::Residual, Self::Output>;
}

impl<T> Try for Option<T> {
    type Output = T;
    type Residual = Option<crate::convert::Infallible>;
    fn from_output(output: T) -> Option<T> {
        Some(output)
    }
    fn from_residual(_residual: Option<crate::convert::Infallible>) -> Option<T> {
        None
    }
    fn branch(self) -> ControlFlow<Option<crate::convert::Infallible>, T> {
        match self {
            Some(v) => ControlFlow::Continue(v),
            None => ControlFlow::Break(None),
        }
    }
}

impl<T, E> Try for Result<T, E> {
    type Output = T;
    type Residual = Result<crate::convert::Infallible, E>;
    fn from_output(output: T) -> Result<T, E> {
        Ok(output)
    }
    fn from_residual(residual: Result<crate::convert::Infallible, E>) -> Result<T, E> {
        match residual {
            Err(e) => Err(e),
            Ok(never) => match never {},
        }
    }
    fn branch(self) -> ControlFlow<Result<crate::convert::Infallible, E>, T> {
        match self {
            Ok(v) => ControlFlow::Continue(v),
            Err(e) => ControlFlow::Break(Err(e)),
        }
    }
}

impl<B, C> Try for ControlFlow<B, C> {
    type Output = C;
    type Residual = ControlFlow<B, crate::convert::Infallible>;
    fn from_output(output: C) -> ControlFlow<B, C> {
        ControlFlow::Continue(output)
    }
    fn from_residual(residual: ControlFlow<B, crate::convert::Infallible>) -> ControlFlow<B, C> {
        match residual {
            ControlFlow::Break(b) => ControlFlow::Break(b),
            ControlFlow::Continue(never) => match never {},
        }
    }
    fn branch(self) -> ControlFlow<ControlFlow<B, crate::convert::Infallible>, C> {
        match self {
            ControlFlow::Continue(c) => ControlFlow::Continue(c),
            ControlFlow::Break(b) => ControlFlow::Break(ControlFlow::Break(b)),
        }
    }
}
