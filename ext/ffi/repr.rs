// Copyright 2018-2023 the Deno authors. All rights reserved. MIT license.

use crate::check_unstable;
use crate::FfiPermissions;
use deno_core::error::range_error;
use deno_core::error::type_error;
use deno_core::error::AnyError;
use deno_core::op;
use deno_core::serde_v8;
use deno_core::v8;
use deno_core::OpState;
use std::ffi::c_char;
use std::ffi::c_void;
use std::ffi::CStr;
use std::ptr;

#[op(fast)]
fn op_ffi_ptr_create<FP>(state: &mut OpState, ptr_number: usize) -> Result<*mut c_void, AnyError>
where
    FP: FfiPermissions + 'static,
{
    check_unstable(state, "Deno.UnsafePointer#create");
    let permissions = state.borrow_mut::<FP>();
    permissions.check(None)?;

    Ok(ptr_number as *mut c_void)
}

#[op(fast)]
pub fn op_ffi_ptr_equals<FP>(
    state: &mut OpState,
    a: *const c_void,
    b: *const c_void,
) -> Result<bool, AnyError>
where
    FP: FfiPermissions + 'static,
{
    check_unstable(state, "Deno.UnsafePointer#equals");
    let permissions = state.borrow_mut::<FP>();
    permissions.check(None)?;

    Ok(a == b)
}

#[op(fast)]
pub fn op_ffi_ptr_of<FP>(state: &mut OpState, buf: *const u8) -> Result<*mut c_void, AnyError>
where
    FP: FfiPermissions + 'static,
{
    check_unstable(state, "Deno.UnsafePointer#of");
    let permissions = state.borrow_mut::<FP>();
    permissions.check(None)?;

    Ok(buf as *mut c_void)
}

#[op(fast)]
fn op_ffi_ptr_offset<FP>(
    state: &mut OpState,
    ptr: *mut c_void,
    offset: isize,
) -> Result<*mut c_void, AnyError>
where
    FP: FfiPermissions + 'static,
{
    check_unstable(state, "Deno.UnsafePointer#offset");
    let permissions = state.borrow_mut::<FP>();
    permissions.check(None)?;

    if ptr.is_null() {
        return Err(type_error("Invalid pointer to offset, pointer is null"));
    }

    // SAFETY: Pointer and offset are user provided.
    Ok(unsafe { ptr.offset(offset) })
}

unsafe extern "C" fn noop_deleter_callback(
    _data: *mut c_void,
    _byte_length: usize,
    _deleter_data: *mut c_void,
) {
}

#[op(fast)]
fn op_ffi_ptr_value<FP>(
    state: &mut OpState,
    ptr: *mut c_void,
    out: &mut [u32],
) -> Result<(), AnyError>
where
    FP: FfiPermissions + 'static,
{
    check_unstable(state, "Deno.UnsafePointer#value");
    let permissions = state.borrow_mut::<FP>();
    permissions.check(None)?;

    let outptr = out.as_ptr() as *mut usize;
    let length = out.len();
    assert!(length >= (std::mem::size_of::<usize>() / std::mem::size_of::<u32>()));
    assert_eq!(outptr as usize % std::mem::size_of::<usize>(), 0);

    // SAFETY: Out buffer was asserted to be at least large enough to hold a usize, and properly aligned.
    let out = unsafe { &mut *outptr };
    *out = ptr as usize;

    Ok(())
}

#[op(v8)]
pub fn op_ffi_get_buf<FP, 'scope>(
    scope: &mut v8::HandleScope<'scope>,
    state: &mut OpState,
    ptr: *mut c_void,
    offset: isize,
    len: usize,
) -> Result<serde_v8::Value<'scope>, AnyError>
where
    FP: FfiPermissions + 'static,
{
    check_unstable(state, "Deno.UnsafePointerView#getArrayBuffer");

    let permissions = state.borrow_mut::<FP>();
    permissions.check(None)?;

    if ptr.is_null() {
        return Err(type_error("Invalid ArrayBuffer pointer, pointer is null"));
    }

    // SAFETY: Trust the user to have provided a real pointer, offset, and a valid matching size to it. Since this is a foreign pointer, we should not do any deletion.
    let backing_store = unsafe {
        v8::ArrayBuffer::new_backing_store_from_ptr(
            ptr.offset(offset),
            len,
            noop_deleter_callback,
            std::ptr::null_mut(),
        )
    }
    .make_shared();
    let array_buffer: v8::Local<v8::Value> =
        v8::ArrayBuffer::with_backing_store(scope, &backing_store).into();
    Ok(array_buffer.into())
}

#[op(fast)]
pub fn op_ffi_buf_copy_into<FP>(
    state: &mut OpState,
    src: *mut c_void,
    offset: isize,
    dst: &mut [u8],
    len: usize,
) -> Result<(), AnyError>
where
    FP: FfiPermissions + 'static,
{
    check_unstable(state, "Deno.UnsafePointerView#copyInto");

    let permissions = state.borrow_mut::<FP>();
    permissions.check(None)?;

    if src.is_null() {
        Err(type_error("Invalid ArrayBuffer pointer, pointer is null"))
    } else if dst.len() < len {
        Err(range_error(
            "Destination length is smaller than source length",
        ))
    } else {
        let src = src as *const c_void;

        // SAFETY: src and offset are user defined.
        // dest is properly aligned and is valid for writes of len * size_of::<T>() bytes.
        unsafe { ptr::copy::<u8>(src.offset(offset) as *const u8, dst.as_mut_ptr(), len) };
        Ok(())
    }
}

#[op(v8)]
pub fn op_ffi_cstr_read<FP, 'scope>(
    scope: &mut v8::HandleScope<'scope>,
    state: &mut OpState,
    ptr: *mut c_void,
    offset: isize,
) -> Result<serde_v8::Value<'scope>, AnyError>
where
    FP: FfiPermissions + 'static,
{
    check_unstable(state, "Deno.UnsafePointerView#getCString");

    let permissions = state.borrow_mut::<FP>();
    permissions.check(None)?;

    if ptr.is_null() {
        return Err(type_error("Invalid CString pointer, pointer is null"));
    }

    let cstr =
  // SAFETY: Pointer and offset are user provided.
    unsafe { CStr::from_ptr(ptr.offset(offset) as *const c_char) }.to_bytes();
    let value: v8::Local<v8::Value> =
        v8::String::new_from_utf8(scope, cstr, v8::NewStringType::Normal)
            .ok_or_else(|| type_error("Invalid CString pointer, string exceeds max length"))?
            .into();
    Ok(value.into())
}

#[op(fast)]
pub fn op_ffi_read_bool<FP>(
    state: &mut OpState,
    ptr: *mut c_void,
    offset: isize,
) -> Result<bool, AnyError>
where
    FP: FfiPermissions + 'static,
{
    check_unstable(state, "Deno.UnsafePointerView#getBool");

    let permissions = state.borrow_mut::<FP>();
    permissions.check(None)?;

    if ptr.is_null() {
        return Err(type_error("Invalid bool pointer, pointer is null"));
    }

    // SAFETY: ptr and offset are user provided.
    Ok(unsafe { ptr::read_unaligned::<bool>(ptr.offset(offset) as *const bool) })
}

#[op(fast)]
pub fn op_ffi_read_u8<FP>(
    state: &mut OpState,
    ptr: *mut c_void,
    offset: isize,
) -> Result<u32, AnyError>
where
    FP: FfiPermissions + 'static,
{
    check_unstable(state, "Deno.UnsafePointerView#getUint8");

    let permissions = state.borrow_mut::<FP>();
    permissions.check(None)?;

    if ptr.is_null() {
        return Err(type_error("Invalid u8 pointer, pointer is null"));
    }

    // SAFETY: ptr and offset are user provided.
    Ok(unsafe { ptr::read_unaligned::<u8>(ptr.offset(offset) as *const u8) as u32 })
}

#[op(fast)]
pub fn op_ffi_read_i8<FP>(
    state: &mut OpState,
    ptr: *mut c_void,
    offset: isize,
) -> Result<i32, AnyError>
where
    FP: FfiPermissions + 'static,
{
    check_unstable(state, "Deno.UnsafePointerView#getInt8");

    let permissions = state.borrow_mut::<FP>();
    permissions.check(None)?;

    if ptr.is_null() {
        return Err(type_error("Invalid i8 pointer, pointer is null"));
    }

    // SAFETY: ptr and offset are user provided.
    Ok(unsafe { ptr::read_unaligned::<i8>(ptr.offset(offset) as *const i8) as i32 })
}

#[op(fast)]
pub fn op_ffi_read_u16<FP>(
    state: &mut OpState,
    ptr: *mut c_void,
    offset: isize,
) -> Result<u32, AnyError>
where
    FP: FfiPermissions + 'static,
{
    check_unstable(state, "Deno.UnsafePointerView#getUint16");

    let permissions = state.borrow_mut::<FP>();
    permissions.check(None)?;

    if ptr.is_null() {
        return Err(type_error("Invalid u16 pointer, pointer is null"));
    }

    // SAFETY: ptr and offset are user provided.
    Ok(unsafe { ptr::read_unaligned::<u16>(ptr.offset(offset) as *const u16) as u32 })
}

#[op(fast)]
pub fn op_ffi_read_i16<FP>(
    state: &mut OpState,
    ptr: *mut c_void,
    offset: isize,
) -> Result<i32, AnyError>
where
    FP: FfiPermissions + 'static,
{
    check_unstable(state, "Deno.UnsafePointerView#getInt16");

    let permissions = state.borrow_mut::<FP>();
    permissions.check(None)?;

    if ptr.is_null() {
        return Err(type_error("Invalid i16 pointer, pointer is null"));
    }

    // SAFETY: ptr and offset are user provided.
    Ok(unsafe { ptr::read_unaligned::<i16>(ptr.offset(offset) as *const i16) as i32 })
}

#[op(fast)]
pub fn op_ffi_read_u32<FP>(
    state: &mut OpState,
    ptr: *mut c_void,
    offset: isize,
) -> Result<u32, AnyError>
where
    FP: FfiPermissions + 'static,
{
    check_unstable(state, "Deno.UnsafePointerView#getUint32");

    let permissions = state.borrow_mut::<FP>();
    permissions.check(None)?;

    if ptr.is_null() {
        return Err(type_error("Invalid u32 pointer, pointer is null"));
    }

    // SAFETY: ptr and offset are user provided.
    Ok(unsafe { ptr::read_unaligned::<u32>(ptr.offset(offset) as *const u32) })
}

#[op(fast)]
pub fn op_ffi_read_i32<FP>(
    state: &mut OpState,
    ptr: *mut c_void,
    offset: isize,
) -> Result<i32, AnyError>
where
    FP: FfiPermissions + 'static,
{
    check_unstable(state, "Deno.UnsafePointerView#getInt32");

    let permissions = state.borrow_mut::<FP>();
    permissions.check(None)?;

    if ptr.is_null() {
        return Err(type_error("Invalid i32 pointer, pointer is null"));
    }

    // SAFETY: ptr and offset are user provided.
    Ok(unsafe { ptr::read_unaligned::<i32>(ptr.offset(offset) as *const i32) })
}

#[op]
pub fn op_ffi_read_u64<FP>(
    state: &mut OpState,
    ptr: *mut c_void,
    offset: isize,
    out: &mut [u32],
) -> Result<(), AnyError>
where
    FP: FfiPermissions + 'static,
{
    check_unstable(state, "Deno.UnsafePointerView#getBigUint64");

    let permissions = state.borrow_mut::<FP>();
    permissions.check(None)?;

    let outptr = out.as_mut_ptr() as *mut u64;

    assert!(out.len() >= (std::mem::size_of::<u64>() / std::mem::size_of::<u32>()));
    assert_eq!((outptr as usize % std::mem::size_of::<u64>()), 0);

    if ptr.is_null() {
        return Err(type_error("Invalid u64 pointer, pointer is null"));
    }

    let value =
  // SAFETY: ptr and offset are user provided.
    unsafe { ptr::read_unaligned::<u64>(ptr.offset(offset) as *const u64) };

    // SAFETY: Length and alignment of out slice were asserted to be correct.
    unsafe { *outptr = value };
    Ok(())
}

#[op(fast)]
pub fn op_ffi_read_i64<FP>(
    state: &mut OpState,
    ptr: *mut c_void,
    offset: isize,
    out: &mut [u32],
) -> Result<(), AnyError>
where
    FP: FfiPermissions + 'static,
{
    check_unstable(state, "Deno.UnsafePointerView#getBigUint64");

    let permissions = state.borrow_mut::<FP>();
    permissions.check(None)?;

    let outptr = out.as_mut_ptr() as *mut i64;

    assert!(out.len() >= (std::mem::size_of::<i64>() / std::mem::size_of::<u32>()));
    assert_eq!((outptr as usize % std::mem::size_of::<i64>()), 0);

    if ptr.is_null() {
        return Err(type_error("Invalid i64 pointer, pointer is null"));
    }

    let value =
  // SAFETY: ptr and offset are user provided.
    unsafe { ptr::read_unaligned::<i64>(ptr.offset(offset) as *const i64) };
    // SAFETY: Length and alignment of out slice were asserted to be correct.
    unsafe { *outptr = value };
    Ok(())
}

#[op(fast)]
pub fn op_ffi_read_f32<FP>(
    state: &mut OpState,
    ptr: *mut c_void,
    offset: isize,
) -> Result<f32, AnyError>
where
    FP: FfiPermissions + 'static,
{
    check_unstable(state, "Deno.UnsafePointerView#getFloat32");

    let permissions = state.borrow_mut::<FP>();
    permissions.check(None)?;

    if ptr.is_null() {
        return Err(type_error("Invalid f32 pointer, pointer is null"));
    }

    // SAFETY: ptr and offset are user provided.
    Ok(unsafe { ptr::read_unaligned::<f32>(ptr.offset(offset) as *const f32) })
}

#[op(fast)]
pub fn op_ffi_read_f64<FP>(
    state: &mut OpState,
    ptr: *mut c_void,
    offset: isize,
) -> Result<f64, AnyError>
where
    FP: FfiPermissions + 'static,
{
    check_unstable(state, "Deno.UnsafePointerView#getFloat64");

    let permissions = state.borrow_mut::<FP>();
    permissions.check(None)?;

    if ptr.is_null() {
        return Err(type_error("Invalid f64 pointer, pointer is null"));
    }

    // SAFETY: ptr and offset are user provided.
    Ok(unsafe { ptr::read_unaligned::<f64>(ptr.offset(offset) as *const f64) })
}

#[op(fast)]
pub fn op_ffi_read_ptr<FP>(
    state: &mut OpState,
    ptr: *mut c_void,
    offset: isize,
) -> Result<*mut c_void, AnyError>
where
    FP: FfiPermissions + 'static,
{
    check_unstable(state, "Deno.UnsafePointerView#getPointer");

    let permissions = state.borrow_mut::<FP>();
    permissions.check(None)?;

    if ptr.is_null() {
        return Err(type_error("Invalid pointer pointer, pointer is null"));
    }

    // SAFETY: ptr and offset are user provided.
    Ok(unsafe { ptr::read_unaligned::<*mut c_void>(ptr.offset(offset) as *const *mut c_void) })
}
