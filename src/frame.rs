use std::{ffi::CStr, marker::PhantomData};

/// Macro to make working with frame marks easier.
///
/// # Example
/// ```
/// # use tracy_full::frame;
///
/// // End of main continuous frame.
/// frame!();
///
/// // End of secondary continuous frame.
/// frame!("Secondary Frame");
///
/// // Discontinuous frame in the current scope.
/// frame!(discontinuous "Discontinuous Frame");
/// ```
#[macro_export]
macro_rules! frame {
	() => {
		$crate::frame::frame();
	};

	($name:literal $(,)?) => {
		$crate::frame::named_frame($crate::c_str!($name));
	};

	(discontinuous $name:literal $(,)?) => {
		let _frame = $crate::frame::discontinuous_frame($crate::c_str!($name));
	};
}

/// The processing of the main continuous frame has ended.
///
/// A 'continuous frame' is some work that repeats continuously for the duration of the program.
#[inline(always)]
pub fn frame() {
	#[cfg(feature = "enable")]
	unsafe {
		sys::___tracy_emit_frame_mark(std::ptr::null());
	}
}

/// The processing of a secondary continuous frame has ended.
///
/// A 'continuous frame' is some work that repeats continuously for the duration of the program.
#[inline(always)]
pub fn named_frame(name: &'static CStr) {
	#[cfg(feature = "enable")]
	unsafe {
		sys::___tracy_emit_frame_mark(name.as_ptr());
	}
}

/// Start a discontinuous frame. The frame ends when the returned object is dropped.
///
/// A 'discontinuous frame' is some work that runs periodically, with gaps between executions.
#[inline(always)]
pub fn discontinuous_frame(name: &'static CStr) -> DiscontinuousFrame {
	#[cfg(feature = "enable")]
	unsafe {
		sys::___tracy_emit_frame_mark_start(name.as_ptr());
		DiscontinuousFrame {
			unsend: PhantomData,
			name,
		}
	}
	#[cfg(not(feature = "enable"))]
	DiscontinuousFrame {
		unsend: PhantomData,
		name: (),
	}
}

/// A discontinuous frame.
pub struct DiscontinuousFrame {
	unsend: PhantomData<*mut ()>,
	#[cfg(feature = "enable")]
	name: &'static CStr,
	#[cfg(not(feature = "enable"))]
	name: (),
}

impl Drop for DiscontinuousFrame {
	#[inline(always)]
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
#[inline(always)]
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
