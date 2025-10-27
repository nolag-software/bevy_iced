//! Example: IcedTaskPlugin with IcedProgram
//!
//! This demonstrates the API for async task support using IcedTaskPlugin
//! with the IcedProgram trait, showing different task patterns and methods:
//! - Simple IcedTask::perform
//! - Bevy AsyncComputeTaskPool integration  
//! - Tokio runtime tasks (when tokio feature enabled)
//! - Task chaining with .then()
//! - Batch tasks running in parallel
//!
//! Run with: cargo run --example iced_task_plugin --features iced_tasks
//! For Tokio support: cargo run --example iced_task_plugin --features "iced_tasks,tokio"

use bevy::prelude::*;
use bevy_iced::iced::widget::{button, column, container, text};
use bevy_iced::tasks::{
    IcedProgram, IcedProgramAppExt, IcedTask, IcedTaskPlugin, TaskManager, spawn_iced_program,
};
use bevy_iced::{IcedPlugin, Renderer};
use std::time::Duration;

#[derive(bevy_ecs::message::Message, Debug, Clone)]
pub enum Message {
    // Different task patterns
    StartSimpleTask,
    StartBevyTask,
    #[cfg(feature = "tokio")]
    StartTokioTask,
    StartChainedTasks,
    StartBatchTasks,

    // Task results
    SimpleTaskResult(String),
    BevyTaskResult(String),
    #[cfg(feature = "tokio")]
    TokioTaskResult(String),
    ChainStep(u32),
    BatchTaskResult(u32, String),

    Reset,
}

#[derive(Component)]
struct TaskDemoProgram {
    simple_status: String,
    bevy_status: String,
    #[cfg(feature = "tokio")]
    tokio_status: String,
    chain_status: String,
    batch_status: String,
}

impl IcedProgram<Message> for TaskDemoProgram {
    fn init() -> (Self, IcedTask<Message>) {
        (
            Self {
                simple_status: "Ready".to_string(),
                bevy_status: "Ready".to_string(),
                #[cfg(feature = "tokio")]
                tokio_status: "Ready".to_string(),
                chain_status: "Ready".to_string(),
                batch_status: "Ready".to_string(),
            },
            IcedTask::none(),
        )
    }

    fn update(&mut self, message: Message) -> IcedTask<Message> {
        match message {
            // Simple IcedTask::perform pattern
            Message::StartSimpleTask => {
                self.simple_status = "Running simple task...".to_string();
                IcedTask::perform(
                    async {
                        #[cfg(not(target_arch = "wasm32"))]
                        std::thread::sleep(Duration::from_millis(800));
                        #[cfg(target_arch = "wasm32")]
                        gloo_timers::future::TimeoutFuture::new(800).await;

                        "Simple task completed!".to_string()
                    },
                    Message::SimpleTaskResult,
                )
            }

            Message::SimpleTaskResult(result) => {
                self.simple_status = result;
                IcedTask::none()
            }

            // Bevy task pool integration
            Message::StartBevyTask => {
                self.bevy_status = "Running Bevy task...".to_string();
                IcedTask::perform(
                    bevy_tasks::AsyncComputeTaskPool::get().spawn(async {
                        #[cfg(not(target_arch = "wasm32"))]
                        std::thread::sleep(Duration::from_millis(1000));
                        #[cfg(target_arch = "wasm32")]
                        gloo_timers::future::TimeoutFuture::new(1000).await;

                        "Bevy task completed!".to_string()
                    }),
                    Message::BevyTaskResult,
                )
            }

            Message::BevyTaskResult(result) => {
                self.bevy_status = result;
                IcedTask::none()
            }

            // Tokio task (if feature enabled)
            #[cfg(feature = "tokio")]
            Message::StartTokioTask => {
                self.tokio_status = "Running Tokio task...".to_string();
                IcedTask::perform(
                    async {
                        tokio::time::sleep(Duration::from_millis(600)).await;
                        "Tokio task completed!".to_string()
                    },
                    Message::TokioTaskResult,
                )
            }

            #[cfg(feature = "tokio")]
            Message::TokioTaskResult(result) => {
                self.tokio_status = result;
                IcedTask::none()
            }

            // Task chaining example
            Message::StartChainedTasks => {
                self.chain_status = "Starting chain...".to_string();
                IcedTask::perform(async { 1u32 }, Message::ChainStep)
                    .then(|_| {
                        IcedTask::perform(
                            async {
                                #[cfg(not(target_arch = "wasm32"))]
                                std::thread::sleep(Duration::from_millis(300));
                                #[cfg(target_arch = "wasm32")]
                                gloo_timers::future::TimeoutFuture::new(300).await;
                                2u32
                            },
                            Message::ChainStep,
                        )
                    })
                    .then(|_| {
                        IcedTask::perform(
                            async {
                                #[cfg(not(target_arch = "wasm32"))]
                                std::thread::sleep(Duration::from_millis(300));
                                #[cfg(target_arch = "wasm32")]
                                gloo_timers::future::TimeoutFuture::new(300).await;
                                3u32
                            },
                            Message::ChainStep,
                        )
                    })
            }

            Message::ChainStep(step) => {
                match step {
                    1 => self.chain_status = "Step 1/3 completed...".to_string(),
                    2 => self.chain_status = "Step 2/3 completed...".to_string(),
                    3 => self.chain_status = "All steps completed!".to_string(),
                    _ => {}
                }
                IcedTask::none()
            }

            // Batch tasks example - run multiple tasks in parallel
            Message::StartBatchTasks => {
                self.batch_status = "Running 3 tasks in parallel...".to_string();

                // Create multiple tasks that run in parallel
                let tasks = vec![
                    IcedTask::perform(
                        async {
                            #[cfg(not(target_arch = "wasm32"))]
                            std::thread::sleep(Duration::from_millis(400));
                            #[cfg(target_arch = "wasm32")]
                            gloo_timers::future::TimeoutFuture::new(400).await;
                            "Task A completed".to_string()
                        },
                        |result| Message::BatchTaskResult(1, result),
                    ),
                    IcedTask::perform(
                        async {
                            #[cfg(not(target_arch = "wasm32"))]
                            std::thread::sleep(Duration::from_millis(600));
                            #[cfg(target_arch = "wasm32")]
                            gloo_timers::future::TimeoutFuture::new(600).await;
                            "Task B completed".to_string()
                        },
                        |result| Message::BatchTaskResult(2, result),
                    ),
                    IcedTask::perform(
                        async {
                            #[cfg(not(target_arch = "wasm32"))]
                            std::thread::sleep(Duration::from_millis(300));
                            #[cfg(target_arch = "wasm32")]
                            gloo_timers::future::TimeoutFuture::new(300).await;
                            "Task C completed".to_string()
                        },
                        |result| Message::BatchTaskResult(3, result),
                    ),
                ];

                // Run all tasks in parallel using IcedTask::batch
                IcedTask::batch(tasks)
            }

            Message::BatchTaskResult(task_id, result) => {
                // Update status with individual task completions
                let current = &self.batch_status;
                if current.contains("parallel") {
                    self.batch_status = format!("Task {} done: {}", task_id, result);
                } else {
                    self.batch_status = format!("{}\nTask {} done: {}", current, task_id, result);
                }
                IcedTask::none()
            }

            Message::Reset => {
                self.simple_status = "Ready".to_string();
                self.bevy_status = "Ready".to_string();
                #[cfg(feature = "tokio")]
                {
                    self.tokio_status = "Ready".to_string();
                }
                self.chain_status = "Ready".to_string();
                self.batch_status = "Ready".to_string();
                IcedTask::none()
            }
        }
    }

    fn view(&self) -> iced_core::Element<'_, Message, iced_core::Theme, Renderer> {
        let mut col = column![
            text("IcedTaskPlugin + IcedProgram Demo").size(24),
            text("Clean async task integration with different patterns").size(14),
            text(""),
            // Simple task
            text("Simple IcedTask::perform:").size(16),
            button("Start Simple Task").on_press(Message::StartSimpleTask),
            text(&self.simple_status).size(14),
            text(""),
            // Bevy task
            text("Bevy AsyncComputeTaskPool:").size(16),
            button("Start Bevy Task").on_press(Message::StartBevyTask),
            text(&self.bevy_status).size(14),
            text(""),
        ];

        // Add tokio section if feature is enabled
        #[cfg(feature = "tokio")]
        {
            col = col
                .push(text("Tokio Tasks (tokio feature enabled):").size(16))
                .push(button("Start Tokio Task").on_press(Message::StartTokioTask))
                .push(text(&self.tokio_status).size(14))
                .push(text(""));
        }

        #[cfg(not(feature = "tokio"))]
        {
            col = col
                .push(text("Tokio Tasks (enable 'tokio' feature):").size(16))
                .push(
                    text("cargo run --example iced_task_plugin --features 'iced_tasks,tokio'")
                        .size(12),
                )
                .push(text(""));
        }

        // Task chaining
        col = col
            .push(text("Task Chaining:").size(16))
            .push(button("Start Chained Tasks").on_press(Message::StartChainedTasks))
            .push(text(&self.chain_status).size(14))
            .push(text(""))
            // Batch tasks
            .push(text("Batch Tasks (Parallel):").size(16))
            .push(button("Start Batch Tasks").on_press(Message::StartBatchTasks))
            .push(text(&self.batch_status).size(14))
            .push(text(""))
            .push(button("Reset All").on_press(Message::Reset));

        container(col.spacing(10))
            .center(iced_core::Length::Fill)
            .into()
    }
}

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_plugins(IcedPlugin::<Message>::default());

    // Add the task plugin with appropriate runtime
    #[cfg(feature = "tokio")]
    app.add_plugins(IcedTaskPlugin::<Message>::with_tokio_multi_thread()); // Use multi-thread for blocking operations
    #[cfg(not(feature = "tokio"))]
    app.add_plugins(IcedTaskPlugin::<Message>::default()); // Use Bevy task pools by default

    app.add_message::<Message>();

    // Add support for our IcedProgram
    app.add_iced_program::<TaskDemoProgram, Message, iced_core::Theme>();

    app.add_systems(Startup, setup).run();
}

fn setup(mut commands: Commands, task_manager: ResMut<TaskManager<Message>>) {
    commands.spawn(Camera2d);

    // Spawn our iced program as a Bevy component
    spawn_iced_program::<TaskDemoProgram, Message, iced_core::Theme>(&mut commands, task_manager);
}
