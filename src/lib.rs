#![allow(dead_code)]
#![allow(incomplete_features)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![cfg_attr(feature = "allocator_api", feature(allocator_api))]
#![cfg_attr(feature = "allocator_api", feature(slice_ptr_len))]
#![cfg_attr(feature = "unstable", feature(const_intrinsic_copy))]
#![cfg_attr(feature = "unstable", feature(const_mut_refs))]
#![cfg_attr(feature = "unstable", feature(const_type_name))]
#![cfg_attr(feature = "unstable", feature(generic_const_exprs))]

use std::{error::Error, ffi::CString};

pub mod alloc;
pub mod color;
pub mod frame;
#[cfg(feature = "futures")]
pub mod future;
pub mod plot;
pub mod zone;

#[cfg(feature = "enable")]
#[ctor::ctor]
unsafe fn startup_tracy() { sys::___tracy_startup_profiler(); }

#[cfg(feature = "enable")]
#[ctor::dtor]
unsafe fn shutdown_tracy() { sys::___tracy_shutdown_profiler(); }

/// Set the current thread's name. Panics if the name contains interior nulls.
pub fn set_thread_name<T>(name: T)
where
	T: TryInto<CString>,
	T::Error: Error,
{
	#[cfg(feature = "enable")]
	unsafe {
		let cstr = name.try_into().expect("name is not a valid string");
		sys::___tracy_set_thread_name(cstr.as_ptr());
	}
}

pub const fn clamp_callstack_depth(depth: u32) -> u32 {
	if depth < 62 {
		depth
	} else {
		62
	}
}

/// Create a `&'static CStr` from a string literal.
#[macro_export]
macro_rules! c_str {
	($str:literal) => {
		unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(concat!($str, "\0").as_bytes()) }
	};
}
