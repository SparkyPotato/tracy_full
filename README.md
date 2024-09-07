Complete Rust bindings for the [Tracy](https://github.com/wolfpld/tracy) profiler.

## Getting Started

Just add the following to your `Cargo.toml`:
```toml
[dependencies.tracy]
package = "tracy_full"
version = "1.10.0"
```

To enable profiling for a build, add the `enable` feature:
```toml
[dependencies.tracy]
...
features = ["enable"]
```

## Features

### Allocation Tracking
```rust
#[global_allocator]
static ALLOC: tracy::GlobalAllocator = tracy::GlobalAllocator::new();
```
This tracks all allocations, using the default `System` allocator for allocations.

For a custom allocator:
```rust
#[global_allocator]
static ALLOC: tracy::GlobalAllocator<MyAlloc> = tracy::GlobalAllocator::new_with(MyAlloc::new());
```

Tracy also supports tracking custom allocators using the `allocator_api` feature:
```toml
[dependencies.tracy]
...
features = ["allocator_api"]
```
```rust
let alloc = TrackedAllocator::new(alloc, tracy::c_str!("TrackedAllocator"));
```
This creates a memory pool named `TrackedAllocator` in Tracy.

All the allocators have a `*Sampled` variant that samples the callstack on each allocation.

### Frame Marks
Mark the end of the main frame using:
```rust
use tracy::frame;

frame!();
```

Mark the end of a sub-frame using:
```rust
frame!("Name");
```

Mark the scope of a discontinuous frame using:
```rust
frame!(discontinuous "Name");
```

#### The difference between frame types
The main frame what is usually thought of as a 'frame'. 
It is usually placed after the swapchain present call on the main thread.

Sub-frames are parts of the main frame, for example, input gathering, physics, and rendering:
```rust
loop {
    // Gather input
    frame!("Input");
    // Process input
    frame!("Processing");
    // Render
    frame!("Render");

    swapchain.present();
    frame!();
}
```

Discontinuous frames are frames that are not in sync with the frame on the main thread.
This can be things like async asset loading on different threads.

### Plotting
You can plot graphs in Tracy:
```rust
use tracy::plotter;

let plotter = plotter!("MyGraph");
plotter.value(1.0);
plotter.value(2.0);
```

### Zones
```rust
use tracy::zone;

zone!(); // Zone with no name
zone!("MyZone"); // Zone with name "MyZone"
zone!(tracy::color::RED); // Zone with color red
zone!("MyZone", true); // Zone with name "MyZone", and enabled with a runtime expression.
zone!(tracy::color::RED, true); // Zone with color red, and enabled with a runtime expression.
zone!("MyZone", tracy::color::RED, true); // Zone with name "MyZone", color red, and enabled with a runtime expression.
```
All zones profile from creation to the end of the enclosed scope.

## Extra features

### Future support
Futures can be represented as fibers in Tracy. The `futures` feature must be enabled.

```toml
[dependencies.tracy]
...
features = ["enable", "futures"]
```
```rust
use tracy::future;

trace_future!(async_function(), "Async Function").await;
```

### Unstable
The `unstable` feature allows for optimizations that require a nightly compiler.

```toml
[dependencies.tracy]
...
features = ["enable", "unstable"]
```

## External Library Integration

### `bevy`
Enable the `bevy` feature to be able to profile Bevy systems.
```toml
[dependencies.tracy]
...
features = ["enable", "bevy"]
```

```rust
use tracy::bevy::timeline;

App::new().add_system(timeline(my_system)).run();
```

This creates a separate fiber for the system in the tracy timeline.

### `tracing`
Enable the `tracing` feature to be able to profile tracing spans.
```toml
[dependencies.tracy]
...
features = ["enable", "tracing"]
```

```rust
use tracy::tracing::TracyLayer;

tracing::subscriber::set_global_default(
    tracing_subscriber::registry().with(TracyLayer)
);
```

### `wgpu`
Enable the `wgpu` feature to be able to profile wgpu command encoders and render/compute passes.
```toml
[dependencies.tracy]
...
features = ["enable", "wgpu"]
```

```rust
use tracy::wgpu::ProfileContext;

let mut profile_context = ProfileContext::with_name("Name", &adapter, &device, &queue, buffered_frames);
```
`buffered_frames`: the number of frames of profiling data you want the profiler to buffer. 
Note that you must synchronize the host and device accordingly, or else the call to `end_frame` will panic.

You also need to have one `ProfileContext` per host thread.

You can create a profiled command encoder:
```rust
use tracy::{wgpu_command_encoder, wgpu_render_pass, wgpu_compute_pass};

let mut command_encoder = wgpu_command_encoder!(device, profile_context, desc);
{
    let render_pass = wgpu_render_pass!(command_encoder, desc)
}

{
    let compute_pass = wgpu_compute_pass!(command_encoder, desc)
}
```

At the end of each frame, you must call `end_frame`:
```rust
profile_context.end_frame(&device, &queue);
```
This uploads the profiling data to Tracy.
