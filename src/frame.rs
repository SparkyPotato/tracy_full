use std::ffi::CStr;

/// Macro to make working with frame marks easier.
///
/// # Example
/// ```
/// # use tracy_full::frame;
///
/// // Main continuous frame.
/// frame!();
///
/// // Secondary continuous frame.
/// frame!("Secondary Frame");
///
/// // Discontinuous frame.
/// frame!(discontinuous "Discontinuous Frame");
/// ```
#[macro_export]
macro_rules! frame {
	() => {
		$crate::frame::frame();
	};

	($name:literal) => {
		$crate::frame::named_frame($crate::c_str!($name));
	};

	(discontinuous $name:literal) => {
		let _frame = $crate::frame::discontinuous_frame($crate::c_str!($name));
	};
}

#[inline]
/// The processing of the main continuous frame has ended.
///
/// A 'continuous frame' is some work that repeats continuously for the duration of the program.
pub fn frame() {
	#[cfg(feature = "enable")]
	unsafe {
		sys::___tracy_emit_frame_mark(std::ptr::null());
	}
}

#[inline]
/// The processing of a secondary continuous frame has ended.
///
/// A 'continuous frame' is some work that repeats continuously for the duration of the program.
pub fn named_frame(name: &'static CStr) {
	#[cfg(feature = "enable")]
	unsafe {
		sys::___tracy_emit_frame_mark(name.as_ptr());
	}
}

#[inline]
/// Start a discontinuous frame. The frame ends when the returned object is dropped.
///
/// A 'discontinuous frame' is some work that runs periodically, with gaps between executions.
pub fn discontinuous_frame(name: &'static CStr) -> DiscontinuousFrame {
	#[cfg(feature = "enable")]
	unsafe {
		sys::___tracy_emit_frame_mark_start(name.as_ptr());
		DiscontinuousFrame { name }
	}
	#[cfg(not(feature = "enable"))]
	DiscontinuousFrame { name: (), }
}

pub struct DiscontinuousFrame {
	#[cfg(feature = "enable")]
	name: &'static CStr,
	#[cfg(not(feature = "enable"))]
	name: (),
}

impl Drop for DiscontinuousFrame {
	#[inline]
	fn drop(&mut self) {
		#[cfg(feature = "enable")]
		unsafe {
			sys::___tracy_emit_frame_mark_end(self.name.as_ptr());
		}
	}
}

#[repr(C)]
pub struct Pixel {
	pub r: u8,
	pub g: u8,
	pub b: u8,
	pub a: u8,
}

/// An image sent to the profiler.
pub struct Image<'a> {
	/// The pixels of the image.
	pub data: &'a [Pixel],
	/// The width of the image.
	pub width: u16,
	/// The height of the image.
	pub height: u16,
	/// The number of frames that passed between being rendered and sent to the profiler.
	pub lag: u8,
	/// If the image is flipped.
	pub flip: bool,
}

/// Send an image to the profiler.
///
/// The image is attached to the frame that is currently being processed: before a continuous frame mark, or inside a
/// discontinuous frame.
pub fn frame_image(image: Image) {
	#[cfg(feature = "enable")]
	unsafe {
		sys::___tracy_emit_frame_image(
			image.data.as_ptr() as *const _,
			image.width,
			image.height,
			image.lag,
			image.flip as _,
		);
	}
}
