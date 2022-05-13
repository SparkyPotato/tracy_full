use std::{
	future::Future,
	marker::PhantomData,
	mem::ManuallyDrop,
	ops::{Deref, DerefMut},
	pin::Pin,
	sync::atomic::{AtomicU8, Ordering},
};

use futures_lite::{
	future::{block_on, poll_once},
	pin,
	FutureExt,
};
use wgpu::{
	Buffer,
	BufferAsyncError,
	BufferDescriptor,
	BufferUsages,
	CommandBuffer,
	CommandEncoder,
	CommandEncoderDescriptor,
	ComputePass,
	ComputePassDescriptor,
	Device,
	Maintain,
	MapMode,
	QuerySet,
	QuerySetDescriptor,
	QueryType,
	Queue,
	RenderPass,
	RenderPassDescriptor,
	QUERY_SET_MAX_QUERIES,
};
use wgpu_core::api::{Dx12, Vulkan};

#[macro_export]
macro_rules! wgpu_command_encoder {
	($device:expr, $profiler:expr, $desc:expr $(,)?) => {{
		struct S;
		let s = std::any::type_name::<S>();
		$profiler.create_command_encoder(&$device, &$desc, line!(), file!(), &s[..s.len() - 3])
	}};
}

#[macro_export]
macro_rules! wgpu_render_pass {
	($encoder:expr, $desc:expr) => {{
		struct S;
		let s = std::any::type_name::<S>();
		$encoder.begin_render_pass(&$desc, line!(), file!(), &s[..s.len() - 3])
	}};
}

#[cfg(feature = "enable")]
static CONTEXTS: AtomicU8 = AtomicU8::new(0);

#[cfg(feature = "enable")]
fn get_next_context() -> u8 {
	let next = CONTEXTS.fetch_add(1, Ordering::Relaxed);
	if next == 255 {
		panic!("Too many contexts");
	}

	next
}

#[cfg(feature = "enable")]
struct QueryPool {
	readback: Buffer,
	mapping: Option<Pin<Box<dyn Future<Output = Result<(), BufferAsyncError>> + Send>>>,
	query: QuerySet,
	used_queries: u16,
	base_query_id: u16,
}

#[cfg(feature = "enable")]
impl QueryPool {
	const QUERY_POOL_SIZE: u16 = 128;

	pub fn new(device: &Device, base_query_id: u16) -> Self {
		Self {
			readback: device.create_buffer(&BufferDescriptor {
				label: Some("Tracy Readback Buffer"),
				size: 8 * Self::QUERY_POOL_SIZE as u64,
				usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
				mapped_at_creation: false,
			}),
			mapping: None,
			query: device.create_query_set(&QuerySetDescriptor {
				label: Some("Tracy Query Set"),
				ty: QueryType::Timestamp,
				count: Self::QUERY_POOL_SIZE as _,
			}),
			used_queries: 0,
			base_query_id,
		}
	}

	pub fn write_query<T: Pass>(&mut self, pass: &mut T) -> (u16, bool) {
		let id = self.base_query_id + self.used_queries;
		pass.write_timestamp(&self.query, self.used_queries as _);
		self.used_queries += 1;
		(id, self.used_queries == Self::QUERY_POOL_SIZE)
	}

	pub fn reset(&mut self) {
		self.used_queries = 0;
		self.mapping = None;
		self.readback.unmap();
	}
}

#[cfg(feature = "enable")]
struct FrameInFlight {
	pools: Vec<QueryPool>,
	curr_pool: usize,
}

#[cfg(feature = "enable")]
impl FrameInFlight {
	fn new() -> Self {
		Self {
			pools: Vec::new(),
			curr_pool: 0,
		}
	}

	fn get_pool(&mut self, device: &Device, used_query_ids: &mut u16) -> &mut QueryPool {
		let idx = self
			.pools
			.iter()
			.enumerate()
			.nth(self.curr_pool)
			.map(|(i, _)| i)
			.unwrap_or_else(|| {
				let pool = QueryPool::new(device, *used_query_ids);
				self.pools.push(pool);
				*used_query_ids += QueryPool::QUERY_POOL_SIZE;
				self.pools.len() - 1
			});

		&mut self.pools[idx]
	}
}

pub struct ProfileContext {
	#[cfg(feature = "enable")]
	context: u8,
	#[cfg(feature = "enable")]
	frames: Vec<FrameInFlight>,
	#[cfg(feature = "enable")]
	curr_frame: usize,
	#[cfg(feature = "enable")]
	used_query_ids: u16,

	#[cfg(not(feature = "enable"))]
	_context: (),
}

impl ProfileContext {
	/// Device needs `Features::TIMESTAMP_QUERY` enabled.
	pub fn new(device: &Device, queue: &Queue, buffered_frames: u32) -> Self {
		#[cfg(feature = "enable")]
		{
			// We have to make our own struct and transmute it because the bindgened one has a private field for some
			// reason.
			struct ContextData {
				gpu_time: i64,
				period: f32,
				context: u8,
				flags: u8,
				type_: u8,
			}

			let context = get_next_context();
			let period = queue.get_timestamp_period();

			let mut frames: Vec<_> = std::iter::repeat_with(|| FrameInFlight::new())
				.take(buffered_frames as _)
				.collect();

			let frame = &mut frames[0];
			frame.pools.push(QueryPool::new(device, 0));
			let pool = &mut frame.pools[0];

			let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
				label: Some("initialize profiler"),
			});
			encoder.write_timestamp(&pool.query, 0);
			encoder.resolve_query_set(&pool.query, 0..1, &pool.readback, 0);
			queue.submit([encoder.finish()]);
			let slice = pool.readback.slice(0..8);
			let _ = slice.map_async(MapMode::Read);
			device.poll(Maintain::Wait);

			let gpu_time = i64::from_le_bytes(slice.get_mapped_range()[0..8].try_into().unwrap());
			pool.reset();

			let mut type_ = 0;
			unsafe {
				device.as_hal::<Vulkan, _, _>(|dev| {
					if dev.is_some() {
						type_ = 2;
					}
				});
				#[cfg(target_os = "linux")]
				device.as_hal::<Dx12, _, _>(|dev| {
					if dev.is_some() {
						type_ = 4;
					}
				});
			}

			unsafe {
				sys::___tracy_emit_gpu_new_context_serial(std::mem::transmute(ContextData {
					gpu_time,
					period,
					context,
					flags: 0,
					type_,
				}))
			}

			Self {
				context,
				frames,
				curr_frame: 0,
				used_query_ids: QueryPool::QUERY_POOL_SIZE,
			}
		}

		#[cfg(not(feature = "enable"))]
		{
			Self { _context: () }
		}
	}

	pub fn with_name(name: &str, device: &Device, queue: &Queue, buffered_frames: u32) -> Self {
		let this = Self::new(device, queue, buffered_frames);

		#[cfg(feature = "enable")]
		unsafe {
			sys::___tracy_emit_gpu_context_name(sys::___tracy_gpu_context_name_data {
				context: this.context,
				name: name.as_ptr() as _,
				len: name.len() as _,
			});
		}

		this
	}

	pub fn create_command_encoder<'a>(
		&'a mut self, device: &'a Device, desc: &CommandEncoderDescriptor, line: u32, file: &str, function: &str,
	) -> EncoderProfiler<'a> {
		#[cfg(feature = "enable")]
		{
			let mut inner = device.create_command_encoder(desc);
			self.begin_zone(device, &mut inner, desc.label, line, file, function);
			EncoderProfiler {
				inner,
				context: self,
				device,
			}
		}

		#[cfg(not(feature = "enable"))]
		EncoderProfiler {
			inner: device.create_command_encoder(desc),
			context: PhantomData,
		}
	}

	pub fn end_frame(&mut self, device: &Device, queue: &Queue) {
		#[cfg(feature = "enable")]
		{
			let frame = &mut self.frames[self.curr_frame];
			let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
				label: Some("Tracy Query Resolve"),
			});
			for pool in &mut frame.pools {
				encoder.resolve_query_set(&pool.query, 0..(pool.used_queries as u32), &pool.readback, 0);
			}
			queue.submit([encoder.finish()]);

			self.curr_frame = (self.curr_frame + 1) % self.frames.len();
			let frame = &mut self.frames[self.curr_frame];
			for pool in &mut frame.pools {
				let slice = pool.readback.slice(..(pool.used_queries as u64 * 8));
				let mapping = slice.map_async(MapMode::Read);
				pin!(mapping);
				device.poll(Maintain::Wait);
				let _ = block_on(mapping);

				{
					let view = slice.get_mapped_range();
					for i in 0..pool.used_queries {
						let query_id = pool.base_query_id + i;
						let view_base = i as usize * 8;
						let gpu_time = i64::from_le_bytes(view[view_base..view_base + 8].try_into().unwrap());

						unsafe {
							sys::___tracy_emit_gpu_time_serial(sys::___tracy_gpu_time_data {
								gpuTime: gpu_time,
								queryId: query_id,
								context: self.context,
							});
						}
					}
				}

				pool.used_queries = 0;
				pool.readback.unmap();
			}
		}
	}

	#[cfg(feature = "enable")]
	fn begin_zone<T: Pass>(
		&mut self, device: &Device, pass: &mut T, name: Option<&str>, line: u32, file: &str, function: &str,
	) {
		unsafe {
			let srcloc = match name {
				Some(label) => sys::___tracy_alloc_srcloc_name(
					line,
					file.as_ptr() as _,
					file.len(),
					function.as_ptr() as _,
					function.len(),
					label.as_ptr() as _,
					label.len(),
				),
				None => sys::___tracy_alloc_srcloc(
					line,
					file.as_ptr() as _,
					file.len(),
					function.as_ptr() as _,
					function.len(),
				),
			};

			let frame = &mut self.frames[self.curr_frame];
			let pool = frame.get_pool(device, &mut self.used_query_ids);
			let (query_id, need_new_pool) = pool.write_query(pass);
			if need_new_pool {
				frame.curr_pool += 1;
			}

			sys::___tracy_emit_gpu_zone_begin_alloc_serial(sys::___tracy_gpu_zone_begin_data {
				srcloc,
				queryId: query_id,
				context: self.context,
			});
		}
	}

	#[cfg(feature = "enable")]
	fn end_zone<T: Pass>(&mut self, device: &Device, pass: &mut T) {
		let frame = &mut self.frames[self.curr_frame];
		let pool = frame.get_pool(device, &mut self.used_query_ids);
		let (query_id, need_new_pool) = pool.write_query(pass);
		if need_new_pool {
			frame.curr_pool += 1;
		}

		unsafe {
			sys::___tracy_emit_gpu_zone_end_serial(sys::___tracy_gpu_zone_end_data {
				queryId: query_id,
				context: self.context,
			});
		}
	}
}

pub trait Pass {
	fn write_timestamp(&mut self, set: &QuerySet, index: u32);
}

pub struct PassProfiler<'a, T: Pass> {
	inner: T,
	#[cfg(feature = "enable")]
	context: &'a mut ProfileContext,
	#[cfg(feature = "enable")]
	device: &'a Device,
	#[cfg(not(feature = "enable"))]
	context: PhantomData<&'a mut ()>,
}

impl<T: Pass> Drop for PassProfiler<'_, T> {
	fn drop(&mut self) {
		#[cfg(feature = "enable")]
		self.context.end_zone(&self.device, &mut self.inner);
	}
}

impl<T: Pass> Deref for PassProfiler<'_, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target { &self.inner }
}

impl<T: Pass> DerefMut for PassProfiler<'_, T> {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.inner }
}

pub struct EncoderProfiler<'a> {
	inner: CommandEncoder,
	#[cfg(feature = "enable")]
	context: &'a mut ProfileContext,
	#[cfg(feature = "enable")]
	device: &'a Device,
	#[cfg(not(feature = "enable"))]
	context: PhantomData<&'a mut ()>,
}

impl EncoderProfiler<'_> {
	pub fn begin_render_pass<'a>(
		&'a mut self, desc: &RenderPassDescriptor<'a, '_>, line: u32, file: &str, function: &str,
	) -> PassProfiler<'a, RenderPass> {
		#[cfg(feature = "enable")]
		{
			let mut inner = self.inner.begin_render_pass(desc);
			self.context
				.begin_zone(&self.device, &mut inner, desc.label, line, file, function);
			PassProfiler {
				inner,
				context: self.context,
				device: self.device,
			}
		}

		#[cfg(not(feature = "enable"))]
		PassProfiler {
			inner: self.inner.begin_render_pass(desc),
			context: PhantomData,
		}
	}

	pub fn begin_compute_pass<'a>(
		&'a mut self, desc: &ComputePassDescriptor<'a>, line: u32, file: &str, function: &str,
	) -> PassProfiler<'a, ComputePass> {
		#[cfg(feature = "enable")]
		{
			let mut inner = self.inner.begin_compute_pass(desc);
			self.context
				.begin_zone(&self.device, &mut inner, desc.label, line, file, function);
			PassProfiler {
				inner,
				context: self.context,
				device: self.device,
			}
		}

		#[cfg(not(feature = "enable"))]
		PassProfiler {
			inner: self.inner.begin_compute_pass(desc),
			context: PhantomData,
		}
	}

	pub fn finish(mut self) -> CommandBuffer {
		#[cfg(feature = "enable")]
		self.context.end_zone(&self.device, &mut self.inner);
		self.inner.finish()
	}
}

impl Deref for EncoderProfiler<'_> {
	type Target = CommandEncoder;

	fn deref(&self) -> &Self::Target { &self.inner }
}

impl DerefMut for EncoderProfiler<'_> {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.inner }
}

impl Pass for CommandEncoder {
	fn write_timestamp(&mut self, set: &QuerySet, index: u32) { self.write_timestamp(set, index); }
}

impl Pass for RenderPass<'_> {
	fn write_timestamp(&mut self, set: &QuerySet, index: u32) { self.write_timestamp(set, index); }
}

impl Pass for ComputePass<'_> {
	fn write_timestamp(&mut self, set: &QuerySet, index: u32) { self.write_timestamp(set, index); }
}
