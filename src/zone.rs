use std::{ffi::CStr, marker::PhantomData};

use crate::color::Color;

/// Mark a zone in the current scope.
#[macro_export]
macro_rules! zone {
	() => {
		let loc = $crate::get_location!();
		let _zone = $crate::zone::zone(loc, true);
	};

	($name:literal $(,)?) => {
		let loc = $crate::get_location!($name);
		let _zone = $crate::zone::zone(loc, true);
	};

	($color:expr $(,)?) => {
		let loc = $crate::get_location!($color);
		let _zone = $crate::zone::zone(loc, true);
	};

	($name:literal, $enabled:expr $(,)?) => {
		let loc = $crate::get_location!($name);
		let _zone = $crate::zone::zone(loc, $enabled);
	};

	($color:expr, $enabled:expr $(,)?) => {
		let loc = $crate::get_location!($color);
		let _zone = $crate::zone::zone(loc, $enabled);
	};

	($name:literal, $color:expr, $enabled:expr $(,)?) => {
		let loc = $crate::get_location!($name, $color);
		let _zone = $crate::zone::zone(loc, $enabled);
	};
}

/// Mark a zone in the current scope, sampling the callstack.
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

/// Create a zone.
#[inline(always)]
#[cfg(all(feature = "unstable", feature = "enable"))]
pub fn zone(loc: &'static ZoneLocation<'static>, active: bool) -> Zone {
	unsafe {
		Zone {
			unsend: PhantomData,
			ctx: sys::___tracy_emit_zone_begin(&loc.loc, active as _),
		}
	}
}

/// Create a callstack sampled zone.
#[inline(always)]
#[cfg(all(feature = "unstable", feature = "enable"))]
pub fn zone_sample(loc: &'static ZoneLocation<'static>, depth: u32, active: bool) -> Zone {
	unsafe {
		Zone {
			unsend: PhantomData,
			ctx: sys::___tracy_emit_zone_begin_callstack(&loc.loc, depth as _, active as _),
		}
	}
}

/// Create a zone.
#[cfg(not(feature = "unstable"))]
pub fn zone(loc: ZoneLocation<'static>, active: bool) -> Zone {
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

/// Create a zone.
#[cfg(not(feature = "unstable"))]
pub fn zone_sample(loc: ZoneLocation<'static>, depth: u32, active: bool) -> Zone {
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
	#[inline(always)]
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

pub struct ZoneLocation<'a> {
	#[cfg(feature = "enable")]
	lifetime: PhantomData<&'a ()>,
	#[cfg(feature = "enable")]
	loc: sys::___tracy_source_location_data,
	#[cfg(not(feature = "enable"))]
	pub loc: (),
	#[cfg(not(feature = "enable"))]
	pub lifetime: PhantomData<&'a ()>,
}

unsafe impl<'a> Send for ZoneLocation<'a> {}
unsafe impl<'a> Sync for ZoneLocation<'a> {}

impl<'a> ZoneLocation<'a> {
	pub const fn from_function_file_line(function: &'a CStr, file: &'a CStr, line: u32) -> Self {
		#[cfg(feature = "enable")]
		{
			Self {
				lifetime: PhantomData,
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
		Self {
			lifetime: PhantomData,
			loc: (),
		}
	}

	pub const fn from_name_function_file_line(name: &'a CStr, function: &'a CStr, file: &'a CStr, line: u32) -> Self {
		#[cfg(feature = "enable")]
		{
			Self {
				lifetime: PhantomData,
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
		Self {
			lifetime: PhantomData,
			loc: (),
		}
	}

	pub const fn from_function_file_line_color(function: &'a CStr, file: &'a CStr, line: u32, color: Color) -> Self {
		#[cfg(feature = "enable")]
		{
			Self {
				lifetime: PhantomData,
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
		Self {
			lifetime: PhantomData,
			loc: (),
		}
	}

	pub const fn from_name_function_file_line_color(
		name: &'a CStr, function: &'a CStr, file: &'a CStr, line: u32, color: Color,
	) -> Self {
		#[cfg(feature = "enable")]
		{
			Self {
				lifetime: PhantomData,
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
		Self {
			lifetime: PhantomData,
			loc: (),
		}
	}
}

/// Get a zone location.
#[cfg(all(feature = "enable", feature = "unstable"))]
#[macro_export]
macro_rules! get_location {
	() => {{
		{
			struct S;
			static FUNCTION: &[u8] = &$crate::zone::get_function_name_from_local_type::<S, 1>();
			static LOC: $crate::zone::ZoneLocation<'static> = $crate::zone::ZoneLocation::from_function_file_line(
				unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(FUNCTION) },
				unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(concat!(file!(), "\0").as_bytes()) },
				line!(),
			);
			&LOC
		}
	}};

	($name:literal $(,)?) => {{
		struct S;
		static FUNCTION: &[u8] = &$crate::zone::get_function_name_from_local_type::<S, 1>();
		static LOC: $crate::zone::ZoneLocation<'static> = $crate::zone::ZoneLocation::from_name_function_file_line(
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
		static LOC: $crate::zone::ZoneLocation<'static> = $crate::zone::ZoneLocation::from_function_file_line_color(
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
		static LOC: $crate::zone::ZoneLocation<'static> =
			$crate::zone::ZoneLocation::from_name_function_file_line_color(
				$crate::c_str!($name),
				unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(FUNCTION) },
				unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(concat!(file!(), "\0").as_bytes()) },
				line!(),
				$color,
			);
		&LOC
	}};
}

/// Get a zone location.
#[cfg(all(feature = "enable", not(feature = "unstable")))]
#[macro_export]
macro_rules! get_location {
	() => {{
		static FN_NAME: $crate::once_cell::sync::Lazy<std::ffi::CString> = $crate::once_cell::sync::Lazy::new(|| {
			struct S;
			let name = std::any::type_name::<S>();
			let len = name.len() - 2;
			let mut name = name[0..len].as_bytes().to_owned();
			name[len - 1] = 0;
			unsafe { std::ffi::CString::from_vec_unchecked(name.into()) }
		});

		$crate::zone::ZoneLocation::from_function_file_line(
			&FN_NAME,
			unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(concat!(file!(), "\0").as_bytes()) },
			line!(),
		)
	}};

	($name:literal $(,)?) => {{
		static FN_NAME: $crate::once_cell::sync::Lazy<std::ffi::CString> = $crate::once_cell::sync::Lazy::new(|| {
			struct S;
			let name = std::any::type_name::<S>();
			let len = name.len() - 2;
			let mut name = name[0..len].as_bytes().to_owned();
			name[len - 1] = 0;
			unsafe { std::ffi::CString::from_vec_unchecked(name.into()) }
		});

		$crate::zone::ZoneLocation::from_name_function_file_line(
			$crate::c_str!($name),
			&FN_NAME,
			unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(concat!(file!(), "\0").as_bytes()) },
			line!(),
		)
	}};

	($color:expr $(,)?) => {{
		static FN_NAME: $crate::once_cell::sync::Lazy<std::ffi::CString> = $crate::once_cell::sync::Lazy::new(|| {
			struct S;
			let name = std::any::type_name::<S>();
			let len = name.len() - 2;
			let mut name = name[0..len].as_bytes().to_owned();
			name[len - 1] = 0;
			unsafe { std::ffi::CString::from_vec_unchecked(name.into()) }
		});

		$crate::zone::ZoneLocation::from_function_file_line_color(
			&FN_NAME,
			unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(concat!(file!(), "\0").as_bytes()) },
			line!(),
			$color,
		)
	}};

	($name:literal, $color:expr $(,)?) => {{
		static FN_NAME: $crate::once_cell::sync::Lazy<std::ffi::CString> = $crate::once_cell::sync::Lazy::new(|| {
			struct S;
			let name = std::any::type_name::<S>();
			let len = name.len() - 2;
			let mut name = name[0..len].as_bytes().to_owned();
			name[len - 1] = 0;
			unsafe { std::ffi::CString::from_vec_unchecked(name.into()) }
		});

		$crate::zone::ZoneLocation::from_name_function_file_line_color(
			$crate::c_str!($name),
			&FN_NAME,
			unsafe { std::ffi::CStr::from_bytes_with_nul_unchecked(concat!(file!(), "\0").as_bytes()) },
			line!(),
			$color,
		)
	}};
}

/// Get a zone location.
#[cfg(not(feature = "enable"))]
#[macro_export]
macro_rules! get_location {
	() => {{
		$crate::zone::ZoneLocation {
			loc: (),
			lifetime: std::marker::PhantomData,
		}
	}};

	($name:literal) => {{
		$crate::zone::ZoneLocation {
			loc: (),
			lifetime: std::marker::PhantomData,
		}
	}};

	($color:expr) => {{
		$crate::zone::ZoneLocation {
			loc: (),
			lifetime: std::marker::PhantomData,
		}
	}};

	($name:literal, $color:expr $(,)?) => {{
		$crate::zone::ZoneLocation {
			loc: (),
			lifetime: std::marker::PhantomData,
		}
	}};
}
