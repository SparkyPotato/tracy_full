use std::{ffi::CStr, marker::PhantomData};

#[macro_export]
macro_rules! plotter {
	($name:literal) => {
		$crate::plot::Plotter::new($crate::c_str!($name))
	};
}

pub struct Plotter<'a> {
	#[cfg(feature = "enable")]
	name: &'a CStr,
	#[cfg(not(feature = "enable"))]
	name: PhantomData<&'a ()>,
}

impl<'a> Plotter<'a> {
	#[inline(always)]
	pub const fn new(name: &'a CStr) -> Self {
		Self {
			#[cfg(feature = "enable")]
			name,
			#[cfg(not(feature = "enable"))]
			name: PhantomData,
		}
	}

	#[inline(always)]
	pub fn value(&self, value: f64) {
		#[cfg(feature = "enable")]
		unsafe {
			sys::___tracy_emit_plot(self.name.as_ptr(), value);
		}
	}
}
