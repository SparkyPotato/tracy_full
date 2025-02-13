use std::{
	future::Future,
	marker::PhantomData,
	mem::{ManuallyDrop, MaybeUninit},
	ops::{Deref, DerefMut},
	pin::Pin,
	sync::atomic::{AtomicU8, Ordering},
	time::{Duration, Instant},
};

use futures_lite::{
	future::{block_on, poll_once},
	FutureExt,
};
use wgpu::{
	Adapter,
	Backend,
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
	SubmissionIndex,
	QUERY_SET_MAX_QUERIES,
};

/// Create a profiled command encoder.
#[macro_export]
macro_rules! wgpu_command_encoder {
	($device:expr, $profiler:expr, $desc:expr $(,)?) => {{
		struct S;
		let s = ::std::any::type_name::<S>();
		$profiler.create_command_encoder(&$device, &$desc, line!(), file!(), &s[..s.len() - 3])
	}};
}

/// Create a profiled render pass from a profiled command encoder.
#[macro_export]
macro_rules! wgpu_render_pass {
	($encoder:expr, $desc:expr) => {{
		struct S;
		let s = ::std::any::type_name::<S>();
		$encoder.begin_render_pass(&$desc, line!(), file!(), &s[..s.len() - 3])
	}};
}

/// Create a profiled compute pass from a profiled command encoder.
#[macro_export]
macro_rules! wgpu_compute_pass {
	($encoder:expr, $desc:expr) => {{
		struct S;
		let s = ::std::any::type_name::<S>();
		$encoder.begin_compute_pass(&$desc, line!(), file!(), &s[..s.len() - 3])
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
	resolve: Buffer,
	readback: Buffer,
	query: QuerySet,
	used_queries: u16,
	base_query_id: u16,
}

#[cfg(feature = "enable")]
impl QueryPool {
	const QUERY_POOL_SIZE: u16 = 128;

	pub fn new(device: &Device, allocated_query_ids: &mut u16) -> Self {
		let ret = Self {
			resolve: device.create_buffer(&BufferDescriptor {
				label: Some("Tracy Resolve Buffer"),
				size: 8 * Self::QUERY_POOL_SIZE as u64,
				usage: BufferUsages::COPY_SRC | BufferUsages::QUERY_RESOLVE,
				mapped_at_creation: false,
			}),
			readback: device.create_buffer(&BufferDescriptor {
				label: Some("Tracy Readback Buffer"),
				size: 8 * Self::QUERY_POOL_SIZE as u64,
				usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
				mapped_at_creation: false,
			}),
			query: device.create_query_set(&QuerySetDescriptor {
				label: Some("Tracy Query Set"),
				ty: QueryType::Timestamp,
				count: Self::QUERY_POOL_SIZE as _,
			}),
			used_queries: 0,
			base_query_id: *allocated_query_ids,
		};
		*allocated_query_ids += Self::QUERY_POOL_SIZE;
		ret
	}

	pub fn write_query<T: Pass>(&mut self, pass: &mut T) -> (u16, bool) {
		let id = self.base_query_id + self.used_queries;
		pass.write_timestamp(&self.query, self.used_queries as _);
		self.used_queries += 1;
		(id, self.used_queries == Self::QUERY_POOL_SIZE)
	}

	pub fn reset(&mut self) {
		self.used_queries = 0;
		self.readback.unmap();
	}
}

#[cfg(feature = "enable")]
struct FrameInFlight {
	pools: Vec<QueryPool>,
	curr_pool: usize,
	map_submission: Option<SubmissionIndex>,
}

#[cfg(feature = "enable")]
impl FrameInFlight {
	fn new() -> Self {
		Self {
			pools: Vec::new(),
			curr_pool: 0,
			map_submission: None,
		}
	}

	fn get_pool(&mut self, device: &Device, allocated_query_ids: &mut u16) -> &mut QueryPool {
		if self.pools.len() == self.curr_pool {
			self.pools.push(QueryPool::new(device, allocated_query_ids));
		}

		&mut self.pools[self.curr_pool]
	}
}

/// A context for profiling the GPU.
pub struct ProfileContext {
	#[cfg(feature = "enable")]
	context: u8,
	#[cfg(feature = "enable")]
	frames: Vec<FrameInFlight>,
	#[cfg(feature = "enable")]
	curr_frame: usize,
	#[cfg(feature = "enable")]
	allocated_query_ids: u16,
	#[cfg(feature = "enable")]
	enabled: bool,
	#[cfg(feature = "enable")]
	last_sync: Instant,
	#[cfg(feature = "enable")]
	resync_interval: Duration,

	#[cfg(not(feature = "enable"))]
	_context: (),
}

impl ProfileContext {
	/// Device needs `Features::TIMESTAMP_QUERY` enabled.
	/// `buffered_frames` is the number of frames to buffer before uploading the data to Tracy.
	/// `resync_interval` is the interval at which to force a full CPU-GPU sync to prevent drift.
	pub fn new(
		adapter: &Adapter, device: &Device, queue: &Queue, buffered_frames: u32, resync_interval: Duration,
	) -> Self {
		Self::with_enabled(adapter, device, queue, buffered_frames, resync_interval, true)
	}

	pub fn with_name(
		name: &str, adapter: &Adapter, device: &Device, queue: &Queue, buffered_frames: u32, resync_interval: Duration,
	) -> Self {
		Self::with_enabled_and_name(name, adapter, device, queue, buffered_frames, resync_interval, true)
	}

	pub fn with_enabled(
		adapter: &Adapter, device: &Device, queue: &Queue, buffered_frames: u32, resync_interval: Duration,
		enabled: bool,
	) -> Self {
		#[cfg(feature = "enable")]
		{
			let context = get_next_context();
			let mut allocated_query_ids = 0;

			let frames = if enabled {
				let mut frames: Vec<_> = std::iter::repeat_with(|| FrameInFlight::new())
					.take(buffered_frames as _)
					.collect();

				let period = queue.get_timestamp_period();
				let gpu_time = Self::sync_frame(&mut allocated_query_ids, &mut frames[0], device, queue);

				let mut type_ = match adapter.get_info().backend {
					Backend::Empty => 0,
					Backend::Gl => 1,
					Backend::Vulkan => 2,
					Backend::Dx12 => 4,
					Backend::Metal => 5,
					Backend::BrowserWebGpu => 6,
				};

				unsafe {
					sys::___tracy_emit_gpu_new_context_serial(sys::___tracy_gpu_new_context_data {
						gpuTime: gpu_time,
						period,
						context,
						flags: 0,
						type_,
					});
				}

				frames
			} else {
				Vec::new()
			};

			Self {
				context,
				frames,
				curr_frame: 0,
				enabled,
				allocated_query_ids,
				last_sync: Instant::now() - resync_interval,
				resync_interval,
			}
		}

		#[cfg(not(feature = "enable"))]
		{
			Self { _context: () }
		}
	}

	pub fn with_enabled_and_name(
		name: &str, adapter: &Adapter, device: &Device, queue: &Queue, buffered_frames: u32, resync_interval: Duration,
		enabled: bool,
	) -> Self {
		let this = Self::with_enabled(adapter, device, queue, buffered_frames, resync_interval, true);

		#[cfg(feature = "enable")]
		unsafe {
			sys::___tracy_emit_gpu_context_name_serial(sys::___tracy_gpu_context_name_data {
				context: this.context,
				name: name.as_ptr() as _,
				len: name.len() as _,
			});
		}

		this
	}

	/// Create a profiled command encoder.
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

	/// End a frame, uploading the data to Tracy, while also synchronizing for `buffered_frames` frames.
	pub fn end_frame(&mut self, device: &Device, queue: &Queue) {
		#[cfg(feature = "enable")]
		if self.enabled {
			Self::resolve_frame(&mut self.frames[self.curr_frame], device, queue);
			if self.last_sync.elapsed() >= self.resync_interval {
				// Force a full sync to prevent drift.
				for _ in 0..self.frames.len() {
					self.curr_frame = (self.curr_frame + 1) % self.frames.len();
					Self::readback_frame(self.context, &mut self.frames[self.curr_frame], device);
				}

				let gpu_time = Self::sync_frame(&mut self.allocated_query_ids, &mut self.frames[0], device, queue);
				unsafe {
					sys::___tracy_emit_gpu_time_sync_serial(sys::___tracy_gpu_time_sync_data {
						gpuTime: gpu_time,
						context: self.context,
					});
				}

				self.curr_frame = 0;
				self.last_sync = Instant::now();
			} else {
				self.curr_frame = (self.curr_frame + 1) % self.frames.len();
				Self::readback_frame(self.context, &mut self.frames[self.curr_frame], device);
			}
		}
	}

	#[cfg(feature = "enable")]
	fn sync_frame(allocated_query_ids: &mut u16, frame: &mut FrameInFlight, device: &Device, queue: &Queue) -> i64 {
		let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
			label: Some("Initialize Profiler"),
		});
		let pool = frame.get_pool(device, allocated_query_ids);
		encoder.write_timestamp(&pool.query, 0);
		encoder.resolve_query_set(&pool.query, 0..1, &pool.resolve, 0);
		encoder.copy_buffer_to_buffer(&pool.resolve, 0, &pool.readback, 0, 8);
		queue.submit([encoder.finish()]);
		let slice = pool.readback.slice(0..8);
		slice.map_async(MapMode::Read, |_| {});
		device.poll(Maintain::Wait);

		let gpu_time = i64::from_le_bytes(slice.get_mapped_range()[0..8].try_into().unwrap());
		pool.reset();
		gpu_time
	}

	#[cfg(feature = "enable")]
	fn resolve_frame(frame: &mut FrameInFlight, device: &Device, queue: &Queue) {
		let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
			label: Some("Tracy Query Resolve"),
		});
		for pool in &mut frame.pools {
			if pool.used_queries == 0 {
				break;
			}

			encoder.resolve_query_set(&pool.query, 0..(pool.used_queries as u32), &pool.resolve, 0);
			encoder.copy_buffer_to_buffer(&pool.resolve, 0, &pool.readback, 0, pool.used_queries as u64 * 8);
		}
		frame.map_submission = Some(queue.submit([encoder.finish()]));
		for pool in &mut frame.pools {
			if pool.used_queries == 0 {
				break;
			}

			let slice = pool.readback.slice(..(pool.used_queries as u64 * 8));
			slice.map_async(MapMode::Read, |_| {});
		}
	}

	#[cfg(feature = "enable")]
	fn readback_frame(context: u8, frame: &mut FrameInFlight, device: &Device) {
		if let Some(map_submission) = &frame.map_submission {
			device.poll(Maintain::WaitForSubmissionIndex(map_submission.to_owned()));
		}

		for pool in &mut frame.pools {
			if pool.used_queries == 0 {
				break;
			}

			let slice = pool.readback.slice(..(pool.used_queries as u64 * 8));
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
							context,
						});
					}
				}
			}

			pool.reset();
		}
	}

	#[cfg(feature = "enable")]
	fn begin_zone<T: Pass>(
		&mut self, device: &Device, pass: &mut T, name: Option<&str>, line: u32, file: &str, function: &str,
	) {
		if self.enabled {
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
						0,
					),
					None => sys::___tracy_alloc_srcloc(
						line,
						file.as_ptr() as _,
						file.len(),
						function.as_ptr() as _,
						function.len(),
						0,
					),
				};

				let frame = &mut self.frames[self.curr_frame];
				let pool = frame.get_pool(device, &mut self.allocated_query_ids);
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
	}

	#[cfg(feature = "enable")]
	fn end_zone<T: Pass>(&mut self, device: &Device, pass: &mut T) {
		if self.enabled {
			let frame = &mut self.frames[self.curr_frame];
			let pool = frame.get_pool(device, &mut self.allocated_query_ids);
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
}

pub trait Pass {
	type This<'a>: Pass;

	fn write_timestamp(&mut self, set: &QuerySet, index: u32);

	fn forget_lifetime(self) -> Self::This<'static>;
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

impl<'a, T: Pass> PassProfiler<'a, T> {
	pub fn forget_lifetime(self) -> PassProfiler<'a, T::This<'static>> {
		let mut m = MaybeUninit::new(self);
		let m = m.as_mut_ptr();

		unsafe {
			PassProfiler {
				inner: (&raw mut (*m).inner).read().forget_lifetime(),
				#[cfg(feature = "enable")]
				context: (&raw mut (*m).context).read(),
				#[cfg(feature = "enable")]
				device: (&raw mut (*m).device).read(),
				#[cfg(not(feature = "enable"))]
				context: PhantomData,
			}
		}
	}
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
	/// Begin a profiled render pass.
	pub fn begin_render_pass<'a>(
		&'a mut self, desc: &RenderPassDescriptor<'_>, line: u32, file: &str, function: &str,
	) -> PassProfiler<'a, RenderPass<'a>> {
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

	/// Begin a profiled compute pass.
	pub fn begin_compute_pass<'a>(
		&'a mut self, desc: &ComputePassDescriptor<'_>, line: u32, file: &str, function: &str,
	) -> PassProfiler<'a, ComputePass<'a>> {
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

	/// Finish the profiled encoder.
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
	type This<'a> = CommandEncoder;

	fn write_timestamp(&mut self, set: &QuerySet, index: u32) { self.write_timestamp(set, index); }

	fn forget_lifetime(self) -> Self::This<'static> { self }
}

impl Pass for RenderPass<'_> {
	type This<'a> = RenderPass<'a>;

	fn write_timestamp(&mut self, set: &QuerySet, index: u32) { self.write_timestamp(set, index); }

	fn forget_lifetime(self) -> Self::This<'static> { self.forget_lifetime() }
}

impl Pass for ComputePass<'_> {
	type This<'a> = ComputePass<'a>;

	fn write_timestamp(&mut self, set: &QuerySet, index: u32) { self.write_timestamp(set, index); }

	fn forget_lifetime(self) -> Self::This<'static> { self.forget_lifetime() }
}
