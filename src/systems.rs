use bevy_camera::{Camera2d, ClearColorConfig};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use bevy_input::prelude::*;
use bevy_input::touch::TouchInput;
use bevy_input::{
    ButtonState,
    keyboard::KeyboardInput,
    mouse::{MouseButtonInput, MouseWheel},
};
use bevy_camera::Camera;
use bevy_render::extract_component::ExtractComponent;
use bevy_window::prelude::*;
use bevy_window::{PrimaryWindow, WindowFocused};
use iced_core::window::Event as IcedWindowEvent;
use iced_core::{
    Event as IcedEvent, Point, keyboard,
    mouse::{self, Cursor},
};
use iced_runtime::user_interface;

use crate::redraw_requestor::IcedRedrawRequest;
use crate::{
    IcedSettings, conversions, iced_resource::IcedResource,
    render::IcedViewport, utils,
};

#[derive(Resource, Deref, DerefMut, Default)]
pub struct IcedEventQueue(Vec<iced_core::Event>);
impl IcedEventQueue {
    /// Take the inner Vec of events, leaving an empty queue.
    pub fn take(&mut self) -> Vec<iced_core::Event> {
        std::mem::take(&mut self.0)
    }
}
#[derive(SystemParam)]
pub struct InputEvents<'w, 's> {
    cursor_entered: MessageReader<'w, 's, CursorEntered>,
    cursor_left: MessageReader<'w, 's, CursorLeft>,
    cursor: MessageReader<'w, 's, CursorMoved>,
    mouse_button: MessageReader<'w, 's, MouseButtonInput>,
    mouse_wheel: MessageReader<'w, 's, MouseWheel>,
    keyboard_input: MessageReader<'w, 's, KeyboardInput>,
    touch_input: MessageReader<'w, 's, TouchInput>,
    window_focused: MessageReader<'w, 's, WindowFocused>,
}

fn compute_modifiers(input_map: &ButtonInput<KeyCode>) -> keyboard::Modifiers {
    let mut modifiers = keyboard::Modifiers::default();
    if input_map.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
        modifiers |= keyboard::Modifiers::CTRL;
    }
    if input_map.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
        modifiers |= keyboard::Modifiers::SHIFT;
    }
    if input_map.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]) {
        modifiers |= keyboard::Modifiers::ALT;
    }
    if input_map.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight]) {
        modifiers |= keyboard::Modifiers::LOGO;
    }
    modifiers
}

pub fn process_input(
    mut events: InputEvents,
    mut event_queue: ResMut<IcedEventQueue>,
    input_map: Res<ButtonInput<KeyCode>>,
) {
    event_queue.clear();

    for ev in events.cursor.read() {
        event_queue.push(IcedEvent::Mouse(mouse::Event::CursorMoved {
            position: Point::new(ev.position.x, ev.position.y),
        }));
    }

    for ev in events.mouse_button.read() {
        let button = conversions::mouse_button(ev.button);
        event_queue.push(IcedEvent::Mouse(match ev.state {
            ButtonState::Pressed => iced_core::mouse::Event::ButtonPressed(button),
            ButtonState::Released => iced_core::mouse::Event::ButtonReleased(button),
        }));
    }

    for _ev in events.cursor_entered.read() {
        event_queue.push(IcedEvent::Mouse(iced_core::mouse::Event::CursorEntered));
    }

    for _ev in events.cursor_left.read() {
        event_queue.push(IcedEvent::Mouse(iced_core::mouse::Event::CursorLeft));
    }

    for ev in events.mouse_wheel.read() {
        event_queue.push(IcedEvent::Mouse(iced_core::mouse::Event::WheelScrolled {
            delta: mouse::ScrollDelta::Pixels { x: ev.x, y: ev.y },
        }));
    }

    let modifiers = compute_modifiers(&input_map);

    for ev in events.keyboard_input.read() {
        use keyboard::Event::*;
        let event = match ev.key_code {
            KeyCode::ControlLeft
            | KeyCode::ControlRight
            | KeyCode::ShiftLeft
            | KeyCode::ShiftRight
            | KeyCode::AltLeft
            | KeyCode::AltRight
            | KeyCode::SuperLeft
            | KeyCode::SuperRight => ModifiersChanged(modifiers),
            _ => {
                let key = conversions::key(&ev.logical_key);
                let physical_key = conversions::key_code(ev.key_code);
                if ev.state.is_pressed() {
                    KeyPressed {
                        // NOTE: This is supposed to be the "unmodified" key, but we don't get it from bevy events
                        key: key.clone(),
                        text: conversions::key_text(&key),
                        physical_key,
                        modified_key: key,
                        modifiers,
                        // NOTE: This is a winit thing we don't get from bevy events
                        location: keyboard::Location::Standard,
                    }
                } else {
                    KeyReleased {
                        key: key.clone(),
                        modified_key: key,
                        physical_key,
                        modifiers,
                        // NOTE: This is a winit thing we don't get from bevy events
                        location: keyboard::Location::Standard,
                    }
                }
            }
        };

        event_queue.push(IcedEvent::Keyboard(event));
    }

    for ev in events.touch_input.read() {
        event_queue.push(IcedEvent::Touch(conversions::touch_event(ev)));
    }

    for ev in events.window_focused.read() {
        event_queue.push(IcedEvent::Window(if ev.focused {
            IcedWindowEvent::Focused
        } else {
            IcedWindowEvent::Unfocused
        }));
    }
}

#[derive(Resource, Deref, DerefMut, Default)]
pub struct IcedCursor(Cursor);

pub fn iced_update<M: bevy_ecs::message::Message>(
    (viewport, mut windows): (Res<IcedViewport>, Query<&mut Window, With<PrimaryWindow>>),
    #[cfg(target_arch = "wasm32")] _props: NonSend<IcedResource>,
    #[cfg(not(target_arch = "wasm32"))] _props: Res<IcedResource>,
    (mut events, touches): (ResMut<IcedEventQueue>, Res<Touches>),
    ui_cache: NonSendMut<Option<user_interface::Cache>>,
    mut _message_writer: bevy_ecs::message::MessageWriter<M>,
    mut cursor: ResMut<IcedCursor>,
    mut _iced_redraw_request: ResMut<IcedRedrawRequest>,
) {
    // Update cursor position (based on window or touch).
    let bounds = viewport.logical_size();
    *cursor = IcedCursor({
        let window = windows.single_mut().unwrap();
        match window.cursor_position() {
            Some(position) => {
                Cursor::Available(utils::process_cursor_position(position, bounds, &window))
            }
            None => utils::process_touch_input(&touches, &events)
                .map(Cursor::Available)
                .unwrap_or(Cursor::Unavailable),
        }
    });

    // If there's no cached UI, clear pending events so they don't accumulate.
    // If we do have a cache, keep events around so display() (which has the
    // root Element) can rebuild the UserInterface from the cache + Element and
    // perform the full update/draw/message flow.
    if ui_cache.is_none() {
        events.clear();
    }
}
/// Marker component to differentiate between normal 2D cameras and the iced camera.
#[derive(Default, Component, ExtractComponent, Copy, Clone)]
pub struct IcedCamera;

/// Spawns a 2D camera, which serves as the render target for iced.
pub fn setup_iced_camera(mut commands: Commands, settings: Res<IcedSettings>) {
    commands.spawn((
        Camera {
            order: settings.camera_order,
            clear_color: ClearColorConfig::None,
            ..Default::default()
        },
        Camera2d,
        IcedCamera,
    ));
}