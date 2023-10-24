use std::any::TypeId;

pub fn is_unit<T: 'static>() -> bool {
    TypeId::of::<T>() == TypeId::of::<()>()
}
