use std::{ffi::CStr, marker::PhantomData};

use crate::color::Color;

#[macro_export]
macro_rules! zone {
	() => {
		let _zone = $crate::zone::zone($crate::get_location!(), true);
	};

	($name:literal, $enabled:expr $(,)?) => {
		let _zone = $crate::zone::zone($crate::get_location!($name), $enabled);
	};

	($color:expr, $enabled:expr $(,)?) => {
		let _zone = $crate::zone::zone($crate::get_location!($color), $enabled);
	};

	($name:literal, $color:expr, $enabled:expr $(,)?) => {
		let _zone = $crate::zone::zone($crate::get_location!($name, $color), $enabled);
	};
}

#[macro_export]
macro_rules! zone_sample {
	($depth:literal) => {
		let _zone = $crate::zone::zone_sample($crate::get_location!(), $crate::clamp_callstack_depth($depth), true);
	};

	($name:literal, $depth:literal, $enabled:expr $(,)?) => {
		let _zone = $crate::zone::zone_sample(
			$crate::get_location!($name),
			$crate::clamp_callstack_depth($depth),
			$enabled,
		);
	};

	($color:expr, $depth:literal, $enabled:expr $(,)?) => {
		let _zone = $crate::zone::zone_sample(
			$crate::get_location!($color),
			$crate::clamp_callstack_depth($depth),
			$enabled,
		);
	};

	($name:literal, $color:expr, $depth:literal, $enabled:expr $(,)?) => {
		let _zone = $crate::zone::zone_sample(
			$crate::get_location!($name, $color),
			$crate::clamp_callstack_depth($depth),
			$enabled,
		);
	};
}

pub fn zone(loc: &'static ZoneLocation, active: bool) -> Zone {
	#[cfg(feature = "enable")]
	unsafe {
		Zone {
			unsend: PhantomData,
			ctx: sys::___tracy_emit_zone_begin(&loc.loc, active as _),
		}
	}
	#[cfg(not(feature = "enable"))]
	Zone {
		unsend: PhantomData,
		ctx: (),
	}
}

pub fn zone_sample(loc: &'static ZoneLocation, depth: u32, active: bool) -> Zone {
	#[cfg(feature = "enable")]
	unsafe {
		Zone {
			unsend: PhantomData,
			ctx: sys::___tracy_emit_zone_begin_callstack(&loc.loc, depth as _, active as _),
		}
	}
	#[cfg(not(feature = "enable"))]
	Zone {
		unsend: PhantomData,
		ctx: (),
	}
}

pub struct Zone {
	unsend: PhantomData<*mut ()>,
	#[cfg(feature = "enable")]
	ctx: sys::___tracy_c_zone_context,
	#[cfg(not(feature = "enable"))]
	ctx: (),
}

impl Drop for Zone {
	fn drop(&mut self) {
		#[cfg(feature = "enable")]
		unsafe {
			sys::___tracy_emit_zone_end(self.ctx);
		}
	}
}

#[doc(hidden)]
#[cfg(feature = "unstable")]
pub const fn get_function_name_from_local_type<T, const TY: usize>() -> [u8; std::any::type_name::<T>().len() - (TY + 1)]
where
	[(); std::any::type_name::<T>().len() - (TY + 1)]:,
{
	let mut name = [0; std::any::type_name::<T>().len() - (TY + 1)];
	unsafe {
		std::ptr::copy_nonoverlapping(std::any::type_name::<T>().as_ptr(), name.as_mut_ptr(), name.len() - 1);
		name
	}
}

pub struct ZoneLocation {
	#[cfg(feature = "enable")]
	loc: sys::___tracy_source_location_data,
	#[cfg(not(feature = "enable"))]
	loc: (),
}

unsafe impl Send for ZoneLocation {}
unsafe impl Sync for ZoneLocation {}

impl ZoneLocation {
	pub const fn from_function_file_line(function: &CStr, file: &CStr, line: u32) -> Self {
		#[cfg(feature = "enable")]
		{
			Self {
				loc: sys::___tracy_source_location_data {
					name: std::ptr::null(),
					function: function.as_ptr(),
					file: file.as_ptr(),
					line,
					color: Color::none().to_u32(),
				},
			}
		}

		#[cfg(not(feature = "enable"))]
		Self { loc: () }
	}

	pub const fn from_name_function_file_line(name: &CStr, function: &CStr, file: &CStr, line: u32) -> Self {
		#[cfg(feature = "enable")]
		{
			Self {
				loc: sys::___tracy_source_location_data {
					name: name.as_ptr(),
					function: function.as_ptr(),
					file: file.as_ptr(),
					line,
					color: Color::none().to_u32(),
				},
			}
		}

		#[cfg(not(feature = "enable"))]
		Self { loc: () }
	}

	pub const fn from_function_file_line_color(function: &CStr, file: &CStr, line: u32, color: Color) -> Self {
		#[cfg(feature = "enable")]
		{
			Self {
				loc: sys::___tracy_source_location_data {
					name: std::ptr::null(),
					function: function.as_ptr(),
					file: file.as_ptr(),
					line,
					color: color.to_u32(),
				},
			}
		}

		#[cfg(not(feature = "enable"))]
		Self { loc: () }
	}

	pub const fn from_name_function_file_line_color(
		name: &CStr, function: &CStr, file: &CStr, line: u32, color: Color,
	) -> Self {
		#[cfg(feature = "enable")]
		{
			Self {
				loc: sys::___tracy_source_location_data {
					name: name.as_ptr(),
					function: function.as_ptr(),
					file: file.as_ptr(),
					line,
					color: color.to_u32(),
				},
			}
		}

		#[cfg(not(feature = "enable"))]
		Self { loc: () }
	}
}

#[cfg(all(feature = "enable", feature = "unstable"))]
#[macro_export]
/// Get a `&'static ZoneLocation`.
macro_rules! get_location {
	() => {{
		struct S;
		static FUNCTION: &[u8] = &$crate::zone::get_function_name_from_local_type::<S, 1>();
		static LOC: $crate::zone::ZoneLocation = $crate::zone::ZoneLocation::from_function_file_line(
			unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(FUNCTION) },
			unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(concat!(file!(), "\0").as_bytes()) },
			line!(),
		);
		&LOC
	}};

	($name:literal $(,)?) => {{
		struct S;
		static FUNCTION: &[u8] = &$crate::zone::get_function_name_from_local_type::<S, 1>();
		static LOC: $crate::zone::ZoneLocation = $crate::zone::ZoneLocation::from_name_function_file_line(
			$crate::c_str!($name),
			unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(FUNCTION) },
			unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(concat!(file!(), "\0").as_bytes()) },
			line!(),
		);
		&LOC
	}};

	($color:expr $(,)?) => {{
		struct S;
		static FUNCTION: &[u8] = &$crate::zone::get_function_name_from_local_type::<S, 1>();
		static LOC: $crate::zone::ZoneLocation = $crate::zone::ZoneLocation::from_function_file_line_color(
			unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(FUNCTION) },
			unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(concat!(file!(), "\0").as_bytes()) },
			line!(),
			$color,
		);
		&LOC
	}};

	($name:literal, $color:expr $(,)?) => {{
		struct S;
		static FUNCTION: &[u8] = &$crate::zone::get_function_name_from_local_type::<S, 1>();
		static LOC: $crate::zone::ZoneLocation = $crate::zone::ZoneLocation::from_name_function_file_line_color(
			$crate::c_str!($name),
			unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(FUNCTION) },
			unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(concat!(file!(), "\0").as_bytes()) },
			line!(),
			$color,
		);
		&LOC
	}};
}

#[cfg(all(feature = "enable", not(feature = "unstable")))]
#[macro_export]
macro_rules! get_location {
	() => {
		struct S;
		let name = std::any::type_name::<S>();
		ZoneLocation { loc: () }
	};
}

#[cfg(not(feature = "enable"))]
#[macro_export]
macro_rules! get_location {
	() => {
		ZoneLocation { loc: () }
	};

	($name:literal) => {
		ZoneLocation { loc: () }
	};

	($color:expr) => {
		ZoneLocation { loc: () }
	};
}
