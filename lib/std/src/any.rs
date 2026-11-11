//! Minimal `Any`: type identity through the `type_id` intrinsic.

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct TypeId {
    t: u64,
}

impl TypeId {
    pub fn of<T: ?Sized + 'static>() -> TypeId {
        TypeId { t: unsafe { crate::intrinsics::type_id::<T>() } }
    }
}

pub trait Any {
    fn type_id(&self) -> TypeId;
}

impl<T: 'static + ?Sized> Any for T {
    fn type_id(&self) -> TypeId {
        TypeId::of::<T>()
    }
}

pub fn type_name<T: ?Sized>() -> &'static str {
    unsafe { crate::intrinsics::type_name::<T>() }
}

impl dyn Any {
    pub fn is<T: Any>(&self) -> bool {
        self.type_id() == TypeId::of::<T>()
    }
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        if self.is::<T>() { unsafe { Some(&*(self as *const dyn Any as *const T)) } } else { None }
    }
    pub fn downcast_mut<T: Any>(&mut self) -> Option<&mut T> {
        if self.is::<T>() { unsafe { Some(&mut *(self as *mut dyn Any as *mut T)) } } else { None }
    }
}

impl crate::boxed::Box<dyn Any> {
    pub fn downcast<T: Any>(self) -> Result<crate::boxed::Box<T>, crate::boxed::Box<dyn Any>> {
        if self.is::<T>() {
            let raw = crate::boxed::Box::into_raw(self);
            Ok(unsafe { crate::boxed::Box::from_raw(raw as *mut T) })
        } else {
            Err(self)
        }
    }
}

impl crate::fmt::Debug for dyn Any {
    fn fmt(&self, f: &mut crate::fmt::Formatter<'_>) -> crate::fmt::Result {
        f.write_str("Any { .. }")
    }
}
