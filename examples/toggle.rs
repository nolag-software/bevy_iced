use bevy::prelude::*;
use bevy_iced::iced::widget::text;
use bevy_iced::{IcedContext, IcedPlugin, IcedProgramSet, iced};

const NOTOSANS_REGULAR: iced::Font = iced::Font::with_name("Noto Sans");
const NOTOSANS_REGULAR_BYTES: &[u8] = include_bytes!("../assets/fonts/NotoSans-Regular.ttf");

#[derive(bevy_ecs::message::Message)]
pub enum UiMessage {}

#[derive(Resource, PartialEq, Eq)]
pub struct UiActive(bool);

pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(
            IcedPlugin::<UiMessage>::default()
                .fonts(vec![NOTOSANS_REGULAR_BYTES])
                .settings(iced::Settings {
                    default_font: NOTOSANS_REGULAR,
                    ..Default::default()
                }),
        )
        .add_message::<UiMessage>()
        .insert_resource(UiActive(true))
        .add_systems(
            Update,
            (
                toggle_system,
                ui_system
                    .in_set(IcedProgramSet::View)
                    .run_if(resource_equals(UiActive(true))),
            ),
        )
        .run();
}

fn toggle_system(keys: Res<ButtonInput<KeyCode>>, mut active: ResMut<UiActive>) {
    if keys.just_pressed(KeyCode::Space) {
        active.0 = !active.0;
        println!("Toggled UI to {}", active.0);
    }
}
fn ui_system(mut ctx: IcedContext<UiMessage>) {
    ctx.display(text("Press space to toggle GUI."));
}
