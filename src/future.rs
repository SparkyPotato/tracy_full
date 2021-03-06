use std::{
	ffi::CStr,
	future::Future,
	marker::PhantomData,
	pin::Pin,
	task::{Context, Poll},
};

/// Create a profiled future that appears as a fiber.
#[macro_export]
macro_rules! trace_future {
	($name:literal, $future:expr) => {
		$crate::future::FutureWrapper::new($crate::c_str!($name), $future)
	};
}

/// A wrapper for a future that appears a separate fiber.
pub struct FutureWrapper<'a, T> {
	#[cfg(feature = "enable")]
	name: &'a CStr,
	#[cfg(not(feature = "enable"))]
	phantom: PhantomData<&'a ()>,
	inner: T,
}

impl<'a, T> FutureWrapper<'a, T> {
	#[inline(always)]
	pub const fn new(name: &'a CStr, inner: T) -> Self {
		Self {
			#[cfg(feature = "enable")]
			name,
			#[cfg(not(feature = "enable"))]
			phantom: PhantomData,
			inner,
		}
	}
}

impl<T: Future> Future for FutureWrapper<'_, T> {
	type Output = T::Output;

	#[inline(always)]
	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		#[cfg(feature = "enable")]
		unsafe {
			sys::___tracy_fiber_enter(self.name.as_ptr());

			let this = self.get_unchecked_mut();
			let inner = Pin::new_unchecked(&mut this.inner);
			let val = inner.poll(cx);

			sys::___tracy_fiber_enter(this.name.as_ptr());
			sys::___tracy_fiber_leave();
			val
		}

		#[cfg(not(feature = "enable"))]
		unsafe {
			self.map_unchecked_mut(|this| &mut this.inner).poll(cx)
		}
	}
}
