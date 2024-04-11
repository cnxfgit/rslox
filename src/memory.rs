use crate::object::{Obj, ObjType, Object};
use std::{alloc::Layout, ptr::null_mut};

pub fn allocate_obj<T: Object>(type_: ObjType) -> *mut T {
    let raw_ptr = allocate::<T>(1);
    unsafe {
        let obj_ptr = raw_ptr as *mut Obj;
        (*obj_ptr).type_ = type_;
        (*obj_ptr).is_marked = false;
        (*obj_ptr).next = null_mut();
    }

    raw_ptr
}

pub fn allocate<T>(size: usize) -> *mut T {
    let size_of = std::mem::size_of::<T>();
    unsafe {
        let layout = Layout::from_size_align(size_of * size, std::mem::align_of::<T>()).unwrap();
        std::alloc::alloc(layout) as *mut T
    }
}

pub fn dealloc<T>(ptr: *mut T, size: usize) {
    let size_of = std::mem::size_of::<T>();
    let layout = Layout::from_size_align(size_of * size, std::mem::align_of::<T>()).unwrap();
    unsafe { std::alloc::dealloc(ptr as *mut u8, layout) };
}
