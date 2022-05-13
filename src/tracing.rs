use std::{any::TypeId, cell::UnsafeCell, num::NonZeroU64};

use tracing::{Id, Subscriber};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

thread_local! {
	static STACK: UnsafeCell<Vec<u32>> = UnsafeCell::new(Vec::new());
}

pub struct TracyLayer;

impl<S> Layer<S> for TracyLayer
where
	S: Subscriber,
	S: for<'a> LookupSpan<'a>,
{
	fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
		#[cfg(feature = "enable")]
		{
			let meta = ctx.metadata(id).unwrap();
			let file = meta.file().unwrap_or("");
			let module = meta.module_path().unwrap_or("");
			unsafe {
				let srcloc = sys::___tracy_alloc_srcloc_name(
					meta.line().unwrap_or(0),
					file.as_ptr() as _,
					file.len(),
					module.as_ptr() as _,
					module.len(),
					meta.name().as_ptr() as _,
					meta.name().len(),
				);

				let ctx = sys::___tracy_emit_zone_begin_alloc(srcloc, 1);

				STACK.with(|stack| {
					let stack = &mut *stack.get();
					stack.push(ctx.id);
				})
			}
		}
	}

	fn on_exit(&self, id: &Id, ctx: Context<'_, S>) {
		#[cfg(feature = "enable")]
		{
			STACK.with(|stack| unsafe {
				let stack = &mut *stack.get();
				sys::___tracy_emit_zone_end(sys::___tracy_c_zone_context {
					id: stack.pop().unwrap(),
					active: 1,
				})
			});
		}
	}
}
