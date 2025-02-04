use std::{any::TypeId, borrow::Cow, cell::UnsafeCell, num::NonZeroU64};

use tracing::{span::Attributes, Id, Subscriber};
use tracing_subscriber::{
	fmt::{format::DefaultFields, FormatFields, FormattedFields},
	layer::Context,
	registry::LookupSpan,
	Layer,
};

thread_local! {
	static STACK: UnsafeCell<Vec<u32>> = UnsafeCell::new(Vec::new());
}

/// A tracing layer that tracks spans.
pub struct TracyLayer;

impl<S> Layer<S> for TracyLayer
where
	S: Subscriber,
	S: for<'a> LookupSpan<'a>,
{
	fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
		#[cfg(feature = "enable")]
		{
			if let Some(span) = ctx.span(id) {
				let mut extensions = span.extensions_mut();

				if extensions.get_mut::<FormattedFields<DefaultFields>>().is_none() {
					let mut fields = FormattedFields::<DefaultFields>::new(String::with_capacity(64));

					if DefaultFields::default()
						.format_fields(fields.as_writer(), attrs)
						.is_ok()
					{
						extensions.insert(fields);
					}
				}
			}
		}
	}

	fn on_enter(&self, id: &Id, ctx: Context<'_, S>) {
		#[cfg(feature = "enable")]
		{
			let Some(span) = ctx.span(id) else {
				return;
			};
			let meta = span.metadata();
			let file = meta.file().unwrap_or("");
			let module = meta.module_path().unwrap_or("");
			let name: Cow<str> = if let Some(fields) = span.extensions().get::<FormattedFields<DefaultFields>>() {
				if fields.fields.as_str().is_empty() {
					meta.name().into()
				} else {
					format!("{}{{{}}}", meta.name(), fields.fields.as_str()).into()
				}
			} else {
				meta.name().into()
			};

			unsafe {
				let srcloc = sys::___tracy_alloc_srcloc_name(
					meta.line().unwrap_or(0),
					file.as_ptr() as _,
					file.len(),
					module.as_ptr() as _,
					module.len(),
					name.as_ptr() as _,
					name.len(),
					0,
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
