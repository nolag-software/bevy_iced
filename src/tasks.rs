//! Async Task integration for bevy_iced
//!
//! This module provides integration between iced's Task system and Bevy's ECS,
//! enabling async operations with automatic UI updates.
//!
//! # Usage Patterns
//!
//! ```ignore
//! use bevy::prelude::*;
//! use bevy_iced::{IcedPlugin, IcedTaskPlugin};
//!
//! App::new()
//!     .add_plugins((
//!         IcedPlugin::<Message>::default(),
//!         IcedTaskPlugin::<Message>::default(), // Uses Bevy task pools
//!     ));
//!
//! // Or with Tokio support:
//! App::new()
//!     .add_plugins((
//!         IcedPlugin::<Message>::default(),
//!         IcedTaskPlugin::<Message>::with_tokio(), // Uses Tokio runtime
//!     ));
//! ```
//!
//! ## Implement IcedProgram for the UI application
//! ```ignore
//! #[derive(Component)]
//! struct MyProgram { /* state */ }
//!
//! impl IcedProgram<Message> for MyProgram {
//!     fn init() -> (Self, IcedTask<Message>) { /* ... */ }
//!     fn update(&mut self, msg: Message) -> IcedTask<Message> { /* ... */ }
//!     fn view(&self) -> Element<Message> { /* ... */ }
//! }
//! ```

#![cfg(feature = "iced_tasks")]

use bevy_ecs::prelude::*;
use crossbeam_channel::{Receiver, Sender, unbounded};
use std::future::Future;

pub use iced_runtime::{Action, Task};

use crate::redraw_requestor::RedrawRequestVariant;
use crate::redraw_requestor::RedrawRequestor;

// -------- Core Task Management --------

/// Manages iced Tasks and routes results to Bevy's message system
#[derive(Resource)]
pub struct TaskManager<M: Send + 'static> {
    queue: AsyncTaskQueue<M>,
    #[cfg(feature = "tokio")]
    tokio_runtime: Option<std::sync::Arc<tokio::runtime::Runtime>>,
}

impl<M: Send + 'static> TaskManager<M> {
    /// Create a new task manager
    pub fn new(
        #[cfg(feature = "tokio")] tokio_runtime: Option<std::sync::Arc<tokio::runtime::Runtime>>,
    ) -> Self {
        Self {
            queue: AsyncTaskQueue::new(),
            #[cfg(feature = "tokio")]
            tokio_runtime,
        }
    }

    /// Spawn an iced Task - results will be routed to Bevy's message system
    pub fn spawn(&mut self, task: iced_runtime::Task<M>) {
        let sender = self.queue.sender();

        // Extract the task's stream and execute it
        if let Some(stream) = iced_runtime::task::into_stream(task) {
            self.spawn_future(Self::execute_task_stream(stream, sender));
        }
    }

    /// Spawn a raw future on the appropriate executor
    pub fn spawn_future<F>(&self, fut: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        #[cfg(feature = "tokio")]
        if let Some(rt) = &self.tokio_runtime {
            rt.spawn(fut);
            return;
        }

        // Fallback to Bevy's task pool
        bevy_tasks::IoTaskPool::get().spawn(fut).detach();
    }

    /// Execute an iced Task stream, routing Actions appropriately
    async fn execute_task_stream(
        mut stream: iced_runtime::futures::BoxStream<iced_runtime::Action<M>>,
        sender: Sender<M>,
    ) {
        use iced_runtime::futures::futures::stream::StreamExt;

        while let Some(action) = stream.next().await {
            match action {
                // Route output messages to Bevy's message system
                iced_runtime::Action::Output(message) => {
                    let _ = sender.try_send(message);
                }

                // For now, log other actions - future versions can handle them
                iced_runtime::Action::LoadFont { .. } => {
                    bevy_log::warn!("Task LoadFont action not yet supported in bevy_iced");
                }
                iced_runtime::Action::Widget(_) => {
                    bevy_log::warn!("Task Widget action not yet supported in bevy_iced");
                }
                iced_runtime::Action::Clipboard(_) => {
                    bevy_log::warn!("Task Clipboard action not yet supported in bevy_iced");
                }
                iced_runtime::Action::Window(_) => {
                    bevy_log::warn!("Task Window action not yet supported in bevy_iced");
                }
                iced_runtime::Action::System(_) => {
                    bevy_log::warn!("Task System action not yet supported in bevy_iced");
                }
                iced_runtime::Action::Reload => {
                    bevy_log::warn!("Task Reload action not yet supported in bevy_iced");
                }
                iced_runtime::Action::Exit => {
                    bevy_log::warn!("Task Exit action not yet supported in bevy_iced");
                }
            }
        }
    }

    /// Get messages from completed tasks
    pub fn drain_messages(&self) -> impl Iterator<Item = M> + '_ {
        self.queue.try_drain()
    }
}

/// Internal message queue for task results
#[derive(Resource)]
struct AsyncTaskQueue<M: Send + 'static> {
    rx: Receiver<M>,
    tx: Sender<M>,
}

impl<M: Send + 'static> AsyncTaskQueue<M> {
    fn new() -> Self {
        let (tx, rx) = unbounded::<M>();
        Self { rx, tx }
    }

    fn sender(&self) -> Sender<M> {
        self.tx.clone()
    }

    fn try_drain(&self) -> impl Iterator<Item = M> + '_ {
        self.rx.try_iter()
    }
}

// -------- System Integration --------

/// System: drain async task messages into Bevy's message bus and request redraw
pub fn pump_async_messages<M, U>(
    task_manager: Res<TaskManager<M>>,
    mut out: bevy_ecs::message::MessageWriter<M>,
    redraw: RedrawRequestor<'_, '_, U>,
) where
    M: bevy_ecs::message::Message + Send + 'static,
    U: RedrawRequestVariant + 'static,
{
    let mut emitted = false;
    for msg in task_manager.drain_messages() {
        emitted = true;
        out.write(msg);
    }
    if emitted {
        // Request redraw so UI updates immediately when async tasks complete
        let _ = redraw.event_loop_proxy.send_event(U::REDRAW_REQUEST);
    }
}

// -------- IcedTaskPlugin --------

/// Plugin for enabling async task support in bevy_iced.
///
/// This plugin handles the integration between iced's Task system and Bevy's ECS,
/// enabling async operations with automatic UI updates.
///
/// # Examples
///
/// Basic usage with Bevy task pools:
/// ```ignore
/// app.add_plugins((
///     IcedPlugin::<Message>::default(),
///     IcedTaskPlugin::<Message>::default(),
/// ));
/// ```
///
/// With custom Tokio runtime:
/// ```ignore
/// let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
/// app.add_plugins((
///     IcedPlugin::<Message>::default(),
///     IcedTaskPlugin::<Message>::with_tokio_runtime(Some(Arc::new(runtime))),
/// ));
/// ```
pub struct IcedTaskPlugin<Message, WinitUserEvent = bevy_winit::WakeUp> {
    #[cfg(feature = "tokio")]
    tokio_runtime: Option<std::sync::Arc<tokio::runtime::Runtime>>,
    _marker: std::marker::PhantomData<(Message, WinitUserEvent)>,
}

impl<Message, WinitUserEvent> Default for IcedTaskPlugin<Message, WinitUserEvent> {
    fn default() -> Self {
        Self {
            #[cfg(feature = "tokio")]
            tokio_runtime: None,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<Message, WinitUserEvent> IcedTaskPlugin<Message, WinitUserEvent> {
    /// Create a new task plugin that uses Bevy's task pools (default behavior).
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a task plugin with a custom Tokio runtime.
    #[cfg(feature = "tokio")]
    pub fn with_tokio_runtime(runtime: Option<std::sync::Arc<tokio::runtime::Runtime>>) -> Self {
        Self {
            tokio_runtime: runtime,
            _marker: std::marker::PhantomData,
        }
    }

    /// Convenience method to create a basic current-thread Tokio runtime.
    ///
    /// This is equivalent to creating your own runtime and passing it to `with_tokio_runtime()`.
    #[cfg(feature = "tokio")]
    pub fn with_tokio() -> Self {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime for IcedTaskPlugin");
        Self::with_tokio_runtime(Some(std::sync::Arc::new(runtime)))
    }

    /// Create a multi-threaded Tokio runtime.
    #[cfg(feature = "tokio")]
    pub fn with_tokio_multi_thread() -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create multi-thread Tokio runtime for IcedTaskPlugin");
        Self::with_tokio_runtime(Some(std::sync::Arc::new(runtime)))
    }

    /// Create a Tokio runtime with custom configuration.
    ///
    /// Pass a closure that creates and configures any type of `tokio::runtime::Builder`.
    /// 
    /// Examples:
    /// ```ignore
    /// // Current-thread with custom config:
    /// IcedTaskPlugin::<Message>::with_tokio_custom(|| {
    ///     tokio::runtime::Builder::new_current_thread().enable_all()
    /// })
    /// 
    /// // Multi-threaded with custom config:
    /// IcedTaskPlugin::<Message>::with_tokio_custom(|| {
    ///     tokio::runtime::Builder::new_multi_thread()
    ///         .worker_threads(4)
    ///         .enable_all()
    /// })
    /// ```
    #[cfg(feature = "tokio")]
    pub fn with_tokio_custom<F>(configure: F) -> Self
    where
        F: FnOnce() -> tokio::runtime::Builder,
    {
        let runtime = configure()
            .build()
            .expect("Failed to create custom Tokio runtime for IcedTaskPlugin");
        Self::with_tokio_runtime(Some(std::sync::Arc::new(runtime)))
    }
}

impl<M, U> bevy_app::Plugin for IcedTaskPlugin<M, U>
where
    M: bevy_ecs::message::Message + Send + 'static,
    U: RedrawRequestVariant + 'static,
{
    fn build(&self, app: &mut bevy_app::App) {
        use crate::IcedProgramSet;
        use crate::systems::iced_update;
        use bevy_app::Update;

        // Create TaskManager with the configured runtime
        let task_manager: TaskManager<M> = {
            #[cfg(feature = "tokio")]
            {
                TaskManager::new(self.tokio_runtime.clone())
            }
            #[cfg(not(feature = "tokio"))]
            {
                TaskManager::new()
            }
        };

        app.insert_resource(task_manager);
        app.add_systems(
            Update,
            pump_async_messages::<M, U>
                .after(iced_update::<M>)
                .in_set(IcedProgramSet::Update),
        );
    }
}

// -------- Convenience API --------

/// Wrapper for iced_runtime::Task that provides convenient integration with Bevy
///
/// This provides a nicer API for working with iced Tasks in Bevy systems.
pub struct IcedTask<M>(iced_runtime::Task<M>);

impl<M> IcedTask<M>
where
    M: Send + 'static,
{
    /// Create a task from an async closure
    pub fn future<F>(fut: F) -> Self
    where
        F: Future<Output = M> + iced_runtime::futures::MaybeSend + 'static,
    {
        Self(iced_runtime::Task::future(fut))
    }

    /// Create a task that performs a future and maps the result
    pub fn perform<A, F>(
        future: F,
        f: impl FnOnce(A) -> M + iced_runtime::futures::MaybeSend + 'static,
    ) -> Self
    where
        F: Future<Output = A> + iced_runtime::futures::MaybeSend + 'static,
        A: iced_runtime::futures::MaybeSend + 'static,
        M: iced_runtime::futures::MaybeSend + 'static,
    {
        Self(iced_runtime::Task::perform(future, f))
    }

    /// Create a task that does nothing
    pub fn none() -> Self {
        Self(iced_runtime::Task::none())
    }

    /// Create a task that immediately produces a value
    pub fn done(value: M) -> Self
    where
        M: iced_runtime::futures::MaybeSend + 'static,
    {
        Self(iced_runtime::Task::done(value))
    }

    /// Map the task's output with a function
    pub fn map<O>(
        self,
        f: impl FnMut(M) -> O + iced_runtime::futures::MaybeSend + 'static,
    ) -> IcedTask<O>
    where
        M: iced_runtime::futures::MaybeSend + 'static,
        O: iced_runtime::futures::MaybeSend + 'static,
    {
        IcedTask(self.0.map(f))
    }

    /// Chain another task after this one
    pub fn then<O>(
        self,
        mut f: impl FnMut(M) -> IcedTask<O> + iced_runtime::futures::MaybeSend + 'static,
    ) -> IcedTask<O>
    where
        M: iced_runtime::futures::MaybeSend + 'static,
        O: iced_runtime::futures::MaybeSend + 'static,
    {
        IcedTask(self.0.then(move |m| f(m).0))
    }

    /// Combine multiple tasks to run in parallel
    pub fn batch(tasks: impl IntoIterator<Item = Self>) -> Self
    where
        M: 'static,
    {
        let tasks = tasks.into_iter().map(|t| t.0);
        Self(iced_runtime::Task::batch(tasks))
    }

    /// Get the underlying iced_runtime::Task
    pub fn into_inner(self) -> iced_runtime::Task<M> {
        self.0
    }
}

impl<M> From<iced_runtime::Task<M>> for IcedTask<M> {
    fn from(task: iced_runtime::Task<M>) -> Self {
        Self(task)
    }
}

impl<M> From<IcedTask<M>> for iced_runtime::Task<M> {
    fn from(task: IcedTask<M>) -> Self {
        task.0
    }
}

/// Trait for structured async UI programs
///
/// This provides a more traditional iced-style API for programs that want
/// to integrate deeply with iced's Task system. Programs implementing this
/// trait can be used as Bevy components and will automatically integrate
/// with the task system.
pub trait IcedProgram<M, T = iced_core::Theme, R = crate::Renderer>: Send + Sync + 'static {
    /// Initialize the program, returning initial state and optional init task
    fn init() -> (Self, IcedTask<M>)
    where
        Self: Sized;

    /// Update program state with a message, returning optional task
    fn update(&mut self, msg: M) -> IcedTask<M>;

    /// Build the UI view
    fn view(&self) -> iced_core::Element<'_, M, T, R>;
}

/// Component wrapper for IcedProgram implementations
///
/// This allows the IcedProgram implementations to be used as Bevy components
#[derive(Component)]
pub struct ProgramComponent<P>
where
    P: Send + Sync + 'static,
{
    program: P,
}

impl<P> ProgramComponent<P>
where
    P: Send + Sync + 'static,
{
    /// Create a new program component
    pub fn new(program: P) -> Self {
        Self { program }
    }

    /// Get a reference to the underlying program
    pub fn program(&self) -> &P {
        &self.program
    }

    /// Get a mutable reference to the underlying program
    pub fn program_mut(&mut self) -> &mut P {
        &mut self.program
    }
}

/// System to handle IcedProgram updates and task spawning
pub fn run_iced_programs<P, M, T, R>(
    mut programs: Query<&mut ProgramComponent<P>>,
    mut messages: bevy_ecs::message::MessageReader<M>,
    mut task_manager: ResMut<TaskManager<M>>,
) where
    P: IcedProgram<M, T, R> + Send + Sync + 'static,
    M: bevy_ecs::message::Message + Send + Clone + 'static,
    T: Clone + Send + Sync + 'static,
    R: 'static,
{
    for message in messages.read() {
        for mut program_component in programs.iter_mut() {
            let task = program_component.program.update(message.clone());
            // Spawn the task directly - it's already properly constructed
            task_manager.spawn(task.into_inner());
        }
    }
}

/// System to render IcedProgram views
pub fn render_iced_programs<P, M, T>(
    programs: Query<&ProgramComponent<P>>,
    mut ctx: crate::IcedContext<M, T>,
) where
    P: IcedProgram<M, T, crate::Renderer> + Send + Sync + 'static,
    M: bevy_ecs::message::Message + Send + 'static,
    T: crate::BevyIcedTheme,
{
    // For now, render the first program found
    // In the future, this could support multiple programs or layering
    if let Some(program_component) = programs.iter().next() {
        let element = program_component.program.view();
        ctx.display(element);
    }
}

/// Convenience function to spawn an IcedProgram as a Bevy entity
pub fn spawn_iced_program<P, M, T>(
    commands: &mut bevy_ecs::system::Commands,
    mut task_manager: ResMut<TaskManager<M>>,
) -> bevy_ecs::entity::Entity
where
    P: IcedProgram<M, T, crate::Renderer> + Send + Sync + 'static,
    M: bevy_ecs::message::Message + Send + 'static,
    T: Clone + Send + Sync + 'static,
{
    let (program, init_task) = P::init();

    // Spawn the initial task
    task_manager.spawn(init_task.into_inner());

    commands.spawn(ProgramComponent::new(program)).id()
}

/// Extension trait for App to easily add IcedProgram support
pub trait IcedProgramAppExt {
    /// Add systems to run IcedProgram components
    fn add_iced_program<P, M, T>(&mut self) -> &mut Self
    where
        P: IcedProgram<M, T, crate::Renderer> + Send + Sync + 'static,
        M: bevy_ecs::message::Message + Send + Clone + 'static,
        T: crate::BevyIcedTheme;
}

impl IcedProgramAppExt for bevy_app::App {
    fn add_iced_program<P, M, T>(&mut self) -> &mut Self
    where
        P: IcedProgram<M, T, crate::Renderer> + Send + Sync + 'static,
        M: bevy_ecs::message::Message + Send + Clone + 'static,
        T: crate::BevyIcedTheme,
    {
        use crate::IcedProgramSet;
        use bevy_app::Update;

        self.add_systems(
            Update,
            (
                run_iced_programs::<P, M, T, crate::Renderer>.in_set(IcedProgramSet::Update),
                render_iced_programs::<P, M, T>.in_set(IcedProgramSet::View),
            ),
        )
    }
}
