use std::{borrow::Cow, ffi::CString};

use bevy_ecs::{
	archetype::ArchetypeComponentId,
	component::ComponentId,
	prelude::{SystemLabel, World},
	query::Access,
	schedule::SystemDescriptor,
	system::{IntoSystem, System},
};

#[inline(always)]
pub fn timeline<In, Out, Params, T: IntoSystem<In, Out, Params>>(sys: T) -> SystemWrapper<T::System> {
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

pub struct SystemWrapper<T> {
	inner: T,
	name: CString,
}

impl<T, In, Out> System for SystemWrapper<T>
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
	unsafe fn run_unsafe(&mut self, input: Self::In, world: &World) -> Self::Out {
		#[cfg(feature = "enable")]
		sys::___tracy_fiber_enter(self.name.as_ptr());
		let out = self.inner.run_unsafe(input, world);
		#[cfg(feature = "enable")]
		sys::___tracy_fiber_leave();
		out
	}

	#[inline(always)]
	fn run(&mut self, input: Self::In, world: &mut World) -> Self::Out { self.inner.run(input, world) }

	#[inline(always)]
	fn apply_buffers(&mut self, world: &mut World) { self.inner.apply_buffers(world) }

	#[inline(always)]
	fn initialize(&mut self, _world: &mut World) { self.inner.initialize(_world) }

	#[inline(always)]
	fn update_archetype_component_access(&mut self, world: &World) {
		self.inner.update_archetype_component_access(world)
	}

	#[inline(always)]
	fn check_change_tick(&mut self, change_tick: u32) { self.inner.check_change_tick(change_tick) }

	#[inline(always)]
	fn default_labels(&self) -> Vec<Box<dyn SystemLabel>> { self.inner.default_labels() }
}
