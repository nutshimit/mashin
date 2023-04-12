use crate::NativeType;
use libffi::middle::Arg;
use std::ffi::c_void;

/// Intermediate format for easy translation from NativeType + V8 value
/// to libffi argument types.
#[repr(C)]
pub union NativeValue {
    pub void_value: (),
    pub bool_value: bool,
    pub u8_value: u8,
    pub i8_value: i8,
    pub u16_value: u16,
    pub i16_value: i16,
    pub u32_value: u32,
    pub i32_value: i32,
    pub u64_value: u64,
    pub i64_value: i64,
    pub usize_value: usize,
    pub isize_value: isize,
    pub f32_value: f32,
    pub f64_value: f64,
    pub pointer: *mut c_void,
}

impl NativeValue {
    pub unsafe fn as_arg(&self, native_type: &NativeType) -> Arg {
        match native_type {
            NativeType::Void => unreachable!(),
            NativeType::Bool => Arg::new(&self.bool_value),
            NativeType::U8 => Arg::new(&self.u8_value),
            NativeType::I8 => Arg::new(&self.i8_value),
            NativeType::U16 => Arg::new(&self.u16_value),
            NativeType::I16 => Arg::new(&self.i16_value),
            NativeType::U32 => Arg::new(&self.u32_value),
            NativeType::I32 => Arg::new(&self.i32_value),
            NativeType::U64 => Arg::new(&self.u64_value),
            NativeType::I64 => Arg::new(&self.i64_value),
            NativeType::USize => Arg::new(&self.usize_value),
            NativeType::ISize => Arg::new(&self.isize_value),
            NativeType::F32 => Arg::new(&self.f32_value),
            NativeType::F64 => Arg::new(&self.f64_value),
            NativeType::Pointer | NativeType::Buffer | NativeType::Function => {
                Arg::new(&self.pointer)
            }
            NativeType::Struct(_) => Arg::new(&*self.pointer),
        }
    }
}

// SAFETY: unsafe trait must have unsafe implementation
unsafe impl Send for NativeValue {}
