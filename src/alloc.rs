//! Allocation profiling.

#[cfg(feature = "allocator_api")]
use std::alloc::{AllocError, Allocator};
use std::{
	alloc::{GlobalAlloc, Layout, System},
	ffi::CStr,
	ptr::NonNull,
};

use crate::clamp_callstack_depth;

/// Create an allocator that is tracked by tracy.
#[cfg(feature = "allocator_api")]
#[macro_export]
macro_rules! tracked_allocator {
	($name:literal, $alloc:expr) => {
		$crate::alloc::TrackedAllocator::new($alloc, $crate::c_str!($name))
	};

	($name:literal, $alloc:expr, $depth:expr) => {
		$crate::alloc::TrackedAllocatorSampled::new($alloc, $crate::c_str!($name), $depth)
	};
}

/// A wrapper around an allocator that tracy tracks as a memory pool.
#[cfg(feature = "allocator_api")]
pub struct TrackedAllocator<'a, T> {
	inner: T,
	#[cfg(feature = "enable")]
	name: &'a CStr,
}

#[cfg(feature = "allocator_api")]
impl<'a, T: Allocator> TrackedAllocator<'a, T> {
	#[inline(always)]
	pub const fn new(inner: T, name: &'a CStr) -> Self {
		Self {
			inner,
			#[cfg(feature = "enable")]
			name,
		}
	}
}

#[cfg(feature = "allocator_api")]
unsafe impl<T: Allocator> Allocator for TrackedAllocator<'_, T> {
	fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		#[cfg(feature = "enable")]
		{
			self.inner.allocate(layout).map(|value| unsafe {
				sys::___tracy_emit_memory_alloc_named(value.as_ptr() as _, value.len(), 0, self.name.as_ptr());
				value
			})
		}

		#[cfg(not(feature = "enable"))]
		self.inner.allocate(layout)
	}

	fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		#[cfg(feature = "enable")]
		{
			self.inner.allocate_zeroed(layout).map(|value| unsafe {
				sys::___tracy_emit_memory_alloc_named(value.as_ptr() as _, value.len(), 0, self.name.as_ptr());
				value
			})
		}

		#[cfg(not(feature = "enable"))]
		self.inner.allocate_zeroed(layout)
	}

	unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
		#[cfg(feature = "enable")]
		sys::___tracy_emit_memory_free_named(ptr.as_ptr() as _, 0, self.name.as_ptr());
		self.inner.deallocate(ptr, layout);
	}

	unsafe fn grow(
		&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout,
	) -> Result<NonNull<[u8]>, AllocError> {
		#[cfg(feature = "enable")]
		{
			sys::___tracy_emit_memory_free_named(ptr.as_ptr() as _, 0, self.name.as_ptr());
			self.inner.grow(ptr, old_layout, new_layout).map(|value| {
				sys::___tracy_emit_memory_alloc_named(value.as_ptr() as _, value.len(), 0, self.name.as_ptr());
				value
			})
		}

		#[cfg(not(feature = "enable"))]
		self.inner.grow(ptr, old_layout, new_layout)
	}

	unsafe fn grow_zeroed(
		&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout,
	) -> Result<NonNull<[u8]>, AllocError> {
		#[cfg(feature = "enable")]
		{
			sys::___tracy_emit_memory_free_named(ptr.as_ptr() as _, 0, self.name.as_ptr());
			self.inner.grow_zeroed(ptr, old_layout, new_layout).map(|value| {
				sys::___tracy_emit_memory_alloc_named(value.as_ptr() as _, value.len(), 0, self.name.as_ptr());
				value
			})
		}

		#[cfg(not(feature = "enable"))]
		self.inner.grow_zeroed(ptr, old_layout, new_layout)
	}

	unsafe fn shrink(
		&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout,
	) -> Result<NonNull<[u8]>, AllocError> {
		#[cfg(feature = "enable")]
		{
			sys::___tracy_emit_memory_free_named(ptr.as_ptr() as _, 0, self.name.as_ptr());
			self.inner.shrink(ptr, old_layout, new_layout).map(|value| {
				sys::___tracy_emit_memory_alloc_named(value.as_ptr() as _, value.len(), 0, self.name.as_ptr());
				value
			})
		}

		#[cfg(not(feature = "enable"))]
		self.inner.shrink(ptr, old_layout, new_layout)
	}
}

/// A wrapper around an allocator that tracy tracks as a memory pool, that also samples the callstack on every
/// allocation.
#[cfg(feature = "allocator_api")]
pub struct TrackedAllocatorSampled<T> {
	inner: T,
	#[cfg(feature = "enable")]
	name: &'static CStr,
	#[cfg(feature = "enable")]
	depth: i32,
}

#[cfg(feature = "allocator_api")]
impl<T: Allocator> TrackedAllocatorSampled<T> {
	#[inline(always)]
	pub const fn new(inner: T, name: &'static CStr, depth: u32) -> Self {
		Self {
			inner,
			#[cfg(feature = "enable")]
			name,
			#[cfg(feature = "enable")]
			depth: clamp_callstack_depth(depth) as _,
		}
	}
}

#[cfg(feature = "allocator_api")]
unsafe impl<T: Allocator> Allocator for TrackedAllocatorSampled<T> {
	fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		#[cfg(feature = "enable")]
		{
			self.inner.allocate(layout).map(|value| unsafe {
				sys::___tracy_emit_memory_alloc_callstack_named(
					value.as_ptr() as _,
					value.len(),
					self.depth,
					0,
					self.name.as_ptr(),
				);
				value
			})
		}

		#[cfg(not(feature = "enable"))]
		self.inner.allocate(layout)
	}

	fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		#[cfg(feature = "enable")]
		{
			self.inner.allocate_zeroed(layout).map(|value| unsafe {
				sys::___tracy_emit_memory_alloc_callstack_named(
					value.as_ptr() as _,
					value.len(),
					self.depth,
					0,
					self.name.as_ptr(),
				);
				value
			})
		}

		#[cfg(not(feature = "enable"))]
		self.inner.allocate_zeroed(layout)
	}

	unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
		#[cfg(feature = "enable")]
		sys::___tracy_emit_memory_free_callstack_named(ptr.as_ptr() as _, self.depth, 0, self.name.as_ptr());
		self.inner.deallocate(ptr, layout);
	}

	unsafe fn grow(
		&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout,
	) -> Result<NonNull<[u8]>, AllocError> {
		#[cfg(feature = "enable")]
		{
			sys::___tracy_emit_memory_free_callstack_named(ptr.as_ptr() as _, self.depth, 0, self.name.as_ptr());
			self.inner.grow(ptr, old_layout, new_layout).map(|value| {
				sys::___tracy_emit_memory_alloc_callstack_named(
					value.as_ptr() as _,
					value.len(),
					self.depth,
					0,
					self.name.as_ptr(),
				);
				value
			})
		}

		#[cfg(not(feature = "enable"))]
		self.inner.grow(ptr, old_layout, new_layout)
	}

	unsafe fn grow_zeroed(
		&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout,
	) -> Result<NonNull<[u8]>, AllocError> {
		#[cfg(feature = "enable")]
		{
			sys::___tracy_emit_memory_free_callstack_named(ptr.as_ptr() as _, self.depth, 0, self.name.as_ptr());
			self.inner.grow_zeroed(ptr, old_layout, new_layout).map(|value| {
				sys::___tracy_emit_memory_alloc_callstack_named(
					value.as_ptr() as _,
					value.len(),
					self.depth,
					0,
					self.name.as_ptr(),
				);
				value
			})
		}

		#[cfg(not(feature = "enable"))]
		self.inner.grow_zeroed(ptr, old_layout, new_layout)
	}

	unsafe fn shrink(
		&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout,
	) -> Result<NonNull<[u8]>, AllocError> {
		#[cfg(feature = "enable")]
		{
			sys::___tracy_emit_memory_free_callstack_named(ptr.as_ptr() as _, self.depth, 0, self.name.as_ptr());
			self.inner.shrink(ptr, old_layout, new_layout).map(|value| {
				sys::___tracy_emit_memory_alloc_callstack_named(
					value.as_ptr() as _,
					value.len(),
					self.depth,
					0,
					self.name.as_ptr(),
				);
				value
			})
		}

		#[cfg(not(feature = "enable"))]
		self.inner.shrink(ptr, old_layout, new_layout)
	}
}

/// A tracked global allocator.
pub struct GlobalAllocator<T = System> {
	inner: T,
}

impl GlobalAllocator {
	#[inline(always)]
	pub const fn new() -> Self { Self::new_with(System) }
}

impl<T: GlobalAlloc> GlobalAllocator<T> {
	#[inline(always)]
	pub const fn new_with(inner: T) -> Self { Self { inner } }
}

impl Default for GlobalAllocator {
	#[inline(always)]
	fn default() -> Self { Self::new() }
}

unsafe impl<T: GlobalAlloc> GlobalAlloc for GlobalAllocator<T> {
	#[inline(always)]
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		let value = self.inner.alloc(layout);
		#[cfg(feature = "enable")]
		sys::___tracy_emit_memory_alloc(value as _, layout.size(), 0);
		value
	}

	#[inline(always)]
	unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
		#[cfg(feature = "enable")]
		sys::___tracy_emit_memory_free(ptr as _, 0);
		self.inner.dealloc(ptr, layout);
	}

	#[inline(always)]
	unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
		let value = self.inner.alloc_zeroed(layout);
		#[cfg(feature = "enable")]
		sys::___tracy_emit_memory_alloc(value as _, layout.size(), 0);
		value
	}

	#[inline(always)]
	unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
		#[cfg(feature = "enable")]
		sys::___tracy_emit_memory_free(ptr as _, 0);
		let value = self.inner.realloc(ptr, layout, new_size);
		#[cfg(feature = "enable")]
		sys::___tracy_emit_memory_alloc(value as _, new_size, 0);
		value
	}
}

/// A tracked global allocator that samples the callstack on every allocation.
pub struct GlobalAllocatorSampled<T = System> {
	inner: T,
	#[cfg(feature = "enable")]
	depth: i32,
}

impl GlobalAllocatorSampled {
	#[inline(always)]
	pub const fn new(depth: u32) -> Self { Self::new_with(System, depth) }
}

impl<T: GlobalAlloc> GlobalAllocatorSampled<T> {
	#[inline(always)]
	pub const fn new_with(inner: T, depth: u32) -> Self {
		Self {
			inner,
			#[cfg(feature = "enable")]
			depth: clamp_callstack_depth(depth) as _,
		}
	}
}

unsafe impl<T: GlobalAlloc> GlobalAlloc for GlobalAllocatorSampled<T> {
	#[inline(always)]
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		let value = self.inner.alloc(layout);
		#[cfg(feature = "enable")]
		sys::___tracy_emit_memory_alloc_callstack(value as _, layout.size(), self.depth, 0);
		value
	}

	#[inline(always)]
	unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
		#[cfg(feature = "enable")]
		sys::___tracy_emit_memory_free_callstack(ptr as _, self.depth, 0);
		self.inner.dealloc(ptr, layout);
	}

	#[inline(always)]
	unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
		let value = self.inner.alloc_zeroed(layout);
		#[cfg(feature = "enable")]
		sys::___tracy_emit_memory_alloc_callstack(value as _, layout.size(), self.depth, 0);
		value
	}

	#[inline(always)]
	unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
		#[cfg(feature = "enable")]
		sys::___tracy_emit_memory_free_callstack(ptr as _, self.depth, 0);
		let value = self.inner.realloc(ptr, layout, new_size);
		#[cfg(feature = "enable")]
		sys::___tracy_emit_memory_alloc_callstack(value as _, new_size, self.depth, 0);
		value
	}
}
