use crate::{
    is_obj, obj_val,
    object::{
        Obj, ObjBoundMethod, ObjClass, ObjClosure, ObjFunction, ObjInstance, ObjNative, ObjString,
        ObjType, ObjUpvalue, Object,
    },
    table::Table,
    value::{as_obj, Value, ValueArray},
    vm::vm,
};
use std::{alloc::Layout, ptr::null_mut};

static GC_HEAP_GROW_FACTOR: usize = 2;

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
    let add_size = size_of * size;
    vm().bytes_allocated += add_size;

    #[cfg(feature = "debug_stress_gc")]
    collect_garbage();

    if vm().bytes_allocated > vm().next_gc {
        collect_garbage();
    }
    unsafe {
        let layout = Layout::from_size_align(add_size, std::mem::align_of::<T>()).unwrap();
        std::alloc::alloc(layout) as *mut T
    }
}

pub fn dealloc<T>(ptr: *mut T, size: usize) {
    let size_of = std::mem::size_of::<T>();
    let layout = Layout::from_size_align(size_of * size, std::mem::align_of::<T>()).unwrap();
    unsafe { std::alloc::dealloc(ptr as *mut u8, layout) };
}

fn collect_garbage() {
    let before: i32;
    #[cfg(feature = "debug_log_gc")]
    {
        println!("-- gc begin");
        before = vm().bytes_allocated;
    }

    mark_roots();
    trace_references();
    table_remove_white(&mut vm().strings);
    sweep();

    vm().next_gc = vm().bytes_allocated * GC_HEAP_GROW_FACTOR;

    #[cfg(feature = "debug_log_gc")]
    {
        println!("-- gc end");
        println!(
            "   collected {} bytes (from {} to {}) next at {}",
            before - vm().bytes_allocated,
            before,
            vm().bytes_allocated,
            vm().next_gc,
        );
    }
}

// 清扫
fn sweep() {
    let mut previous: *mut Obj = null_mut();
    let mut object = vm().objects;
    while !object.is_null() {
        let object_ref = unsafe { object.as_mut().unwrap() };
        if object_ref.is_marked {
            object_ref.is_marked = false;
            previous = object;
            object = object_ref.next;
        } else {
            let unreached = object;
            object = object_ref.next;
            if !previous.is_null() {
                unsafe {
                    (*previous).next = object;
                }
            } else {
                vm().objects = object;
            }

            free_object(unreached);
        }
    }
}

// 释放对象
fn free_object(object: *mut Obj) {
    #[cfg(feature = "debug_log_gc")]
    unsafe {
        println!("{:p} free type {}", object, (*object).type_ as i32);
    }
    let object_ref = unsafe { object.as_mut().unwrap() };

    match object_ref.type_ {
        ObjType::BoundMethod => dealloc::<ObjBoundMethod>(object as *mut ObjBoundMethod, 1),
        ObjType::Class => {
            let class: *mut ObjClass = object as *mut ObjClass;
            unsafe {
                dealloc::<Table>((*class).methods, 1);
            }
            dealloc::<ObjClass>(object as *mut ObjClass, 1);
        }
        ObjType::Closure => {
            let closure = object as *mut ObjClosure;
            unsafe {
                dealloc::<ObjUpvalue>(*(*closure).upvalues, (*closure).upvalue_count);
            }
            dealloc::<ObjClosure>(object as *mut ObjClosure, 1);
        }
        ObjType::Function => {
            dealloc::<ObjFunction>(object as *mut ObjFunction, 1);
        }
        ObjType::Instance => {
            let instance = object as *mut ObjInstance;
            dealloc::<Table>(unsafe { instance.as_ref().unwrap().fields }, 1);
            dealloc::<ObjInstance>(object as *mut ObjInstance, 1);
        }
        ObjType::Native => dealloc::<ObjNative>(object as *mut ObjNative, 1),
        ObjType::String => {
            dealloc::<ObjString>(object as *mut ObjString, 1);
        }
        ObjType::Upvalue => dealloc::<ObjUpvalue>(object as *mut ObjUpvalue, 1),
    }
}

fn table_remove_white(table: *mut Table) {
    unsafe {
        for (key, value) in &table.as_ref().unwrap().map {
            if !key.is_null() && !key.as_ref().unwrap().obj.is_marked {
                table.as_mut().unwrap().remove(key.clone());
            }
            mark_object(key.clone() as *mut Obj);
            mark_value(value.clone());
        }
    }
}

// 跟踪对象
fn trace_references() {
    while vm().gray_stack.len() > 0 {
        let object = vm().gray_stack[vm().gray_stack.len() as usize];
        vm().gray_stack.pop();
        blacken_object(object);
    }
}

// 置黑对象
fn blacken_object(object: *mut Obj) {
    #[cfg(feature = "debug_log_gc")]
    {
        print!("{:p} blacken ", object);
        obj_val!(object).print();
        println!();
    }

    match unsafe { (*object).type_ } {
        ObjType::BoundMethod => {
            let bound = object as *mut ObjBoundMethod;
            let bound = unsafe { bound.as_ref().unwrap() };
            mark_value(bound.receiver);
            mark_object(bound.method as *mut Obj);
        }
        ObjType::Class => {
            let class = object as *mut ObjClass;
            let class = unsafe { class.as_ref().unwrap() };
            mark_object(class.name as *mut Obj);
            mark_table(class.methods);
        }
        ObjType::Closure => {
            let closure = object as *mut ObjClosure;
            let closure = unsafe { closure.as_ref().unwrap() };
            mark_object(closure.function as *mut Obj);
            for i in 0..closure.upvalue_count {
                mark_object(unsafe { closure.upvalues.add(i) } as *mut Obj);
            }
        }
        ObjType::Function => {
            let function = object as *mut ObjFunction;
            let function = unsafe { function.as_ref().unwrap() };
            mark_object(function.name as *mut Obj);
            mark_array(&function.chunk.constants);
        }
        ObjType::Instance => {
            let instance = object as *mut ObjInstance;
            let instance = unsafe { instance.as_ref().unwrap() };
            mark_object(instance.class as *mut Obj);
            mark_table(instance.fields);
        }
        ObjType::Upvalue => unsafe { mark_value((*(object as *mut ObjUpvalue)).closed) },
        ObjType::Native | ObjType::String => {}
    }
}

// 标记数组
fn mark_array(array: &ValueArray) {
    for i in 0..array.count() {
        mark_value(array.values[i]);
    }
}

// 标记根对象
fn mark_roots() {
    // 标记虚拟机栈
    let mut slot = &mut vm().stack as *mut Value;
    while slot < vm().stack_top {
        unsafe {
            mark_value(*slot);
            slot = slot.add(1);
        }
    }

    // 闭包
    for i in 0..vm().frame_count {
        mark_object(vm().frames[i].closure as *mut Obj);
    }

    // 提升值
    let mut upvalue = vm().open_upvalues;
    while !upvalue.is_null() {
        mark_object(upvalue as *mut Obj);
        unsafe {
            upvalue = (*upvalue).next;
        }
    }

    // 全局变量
    mark_table(&mut vm().globals);
    mark_compiler_roots();
    mark_object(vm().init_string as *mut Obj);
}

fn mark_compiler_roots() {
    let mut compiler = vm().current_compiler;
    while !compiler.is_null() {
        mark_object(unsafe { compiler.as_ref().unwrap().function } as *mut Obj);
        compiler = unsafe { compiler.as_ref().unwrap().enclosing };
    }
}

fn mark_value(value: Value) {
    if is_obj!(value) {
        mark_object(as_obj(value));
    }
}

fn mark_object(object: *mut Obj) {
    if object.is_null() {
        return;
    }
    if unsafe { (*object).is_marked } {
        return;
    }

    #[cfg(feature = "debug_log_gc")]
    {
        print!("{:p} mark ", object);
        obj_val!(object).print();
        println!("");
    }

    unsafe {
        (*object).is_marked = true;
    }

    vm().gray_stack.push(object);
}

fn mark_table(table: *mut Table) {
    for (key, value) in unsafe { &table.as_ref().unwrap().map } {
        mark_object(key.clone() as *mut Obj);
        mark_value(value.clone());
    }
}
