use std::ffi::CStr;

use sys::___tracy_source_location_data;

use crate::color::Color;

#[macro_export]
macro_rules! zone {
	() => {};
}

pub struct Zone {
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

pub struct ZoneLocation {
	#[cfg(feature = "enable")]
	loc: ___tracy_source_location_data,
	#[cfg(not(feature = "enable"))]
	loc: (),
}

#[cfg(all(feature = "enable", feature = "unstable"))]
#[macro_export]
macro_rules! get_location {
	() => {
		#![feature(inline_const)]
		{
			struct S;
			let function = const {
				let name = std::any::type_name::<S>();
			};
			$crate::zone::ZoneLocation { loc: () }
		}
	};
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
	($name:literal) => {
		ZoneLocation { loc: () }
	};
}
