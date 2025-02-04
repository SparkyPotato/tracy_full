//! Bevy related profiling.

use std::{any::TypeId, borrow::Cow, ffi::CString};

use bevy_ecs::{
	archetype::ArchetypeComponentId,
	component::{ComponentId, Tick},
	prelude::World,
	query::Access,
	system::{IntoSystem, System, SystemInput},
	world::{unsafe_world_cell::UnsafeWorldCell, DeferredWorld},
};

/// Create a system that appears as a separate fiber in the profiler.
#[inline(always)]
pub fn timeline<In: SystemInput, Out, Params, T: IntoSystem<In, Out, Params>>(sys: T) -> SystemWrapper<T::System> {
	let sys = T::into_system(sys);
	SystemWrapper {
		name: CString::new::<Vec<u8>>(match sys.name() {
			Cow::Borrowed(b) => b.into(),
			Cow::Owned(o) => o.into(),
		})
		.expect("System name must not have null bytes"),
		inner: sys,
	}
}

/// A wrapper around a system that appears as a separate fiber in the profiler.
pub struct SystemWrapper<T> {
	inner: T,
	name: CString,
}

impl<T, In: SystemInput, Out> System for SystemWrapper<T>
where
	T: System<In = In, Out = Out>,
{
	type In = T::In;
	type Out = T::Out;

	#[inline(always)]
	fn name(&self) -> Cow<'static, str> { self.inner.name() }

	#[inline(always)]
	fn component_access(&self) -> &Access<ComponentId> { self.inner.component_access() }

	#[inline(always)]
	fn archetype_component_access(&self) -> &Access<ArchetypeComponentId> { self.inner.archetype_component_access() }

	#[inline(always)]
	fn is_send(&self) -> bool { self.inner.is_send() }

	#[inline(always)]
	unsafe fn run_unsafe(&mut self, input: <Self::In as SystemInput>::Inner<'_>, world: UnsafeWorldCell) -> Self::Out {
		#[cfg(feature = "enable")]
		sys::___tracy_fiber_enter(self.name.as_ptr());
		let out = self.inner.run_unsafe(input, world);
		#[cfg(feature = "enable")]
		sys::___tracy_fiber_leave();
		out
	}

	#[inline(always)]
	fn run(&mut self, input: <Self::In as SystemInput>::Inner<'_>, world: &mut World) -> Self::Out {
		self.inner.run(input, world)
	}

	#[inline(always)]
	fn initialize(&mut self, _world: &mut World) { self.inner.initialize(_world) }

	#[inline(always)]
	fn update_archetype_component_access(&mut self, world: UnsafeWorldCell) {
		self.inner.update_archetype_component_access(world)
	}

	#[inline(always)]
	fn check_change_tick(&mut self, change_tick: Tick) { self.inner.check_change_tick(change_tick) }

	#[inline(always)]
	fn is_exclusive(&self) -> bool { self.inner.is_exclusive() }

	fn type_id(&self) -> TypeId { self.inner.type_id() }

	fn has_deferred(&self) -> bool { self.inner.has_deferred() }

	fn apply_deferred(&mut self, world: &mut World) { self.inner.apply_deferred(world) }

	fn get_last_run(&self) -> Tick { self.inner.get_last_run() }

	fn set_last_run(&mut self, last_run: Tick) { self.inner.set_last_run(last_run) }

	fn queue_deferred(&mut self, world: DeferredWorld) { self.inner.queue_deferred(world) }

	unsafe fn validate_param_unsafe(&mut self, world: UnsafeWorldCell) -> bool {
		self.inner.validate_param_unsafe(world)
	}
}
