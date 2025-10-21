use bevy::math::primitives::{Cuboid, Plane3d};
use bevy::prelude::*;
use bevy::window::WindowResolution;
use bevy_iced::{iced, IcedContext, IcedPlugin, IcedProgramSet};

use bevy_panorbit_camera::{PanOrbitCamera, PanOrbitCameraPlugin};
use iced::widget::{
    button, column, container, row, text, tooltip, Space,
};
use iced::{Alignment, Length};

// ---------- UI model ----------
#[derive(Clone, Debug, bevy_ecs::message::Message)]
enum UiMessage {
    Select(TabId),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum TabId {
    Dashboard,
    Metrics,
    Settings,
}

impl TabId {
    fn label(self) -> &'static str {
        match self {
            TabId::Dashboard => "Dashboard",
            TabId::Metrics => "Metrics",
            TabId::Settings => "Settings",
        }
    }
    fn icon(self) -> &'static str {
        match self {
            TabId::Dashboard => "🔷",
            TabId::Metrics => "🔶",
            TabId::Settings => "⚙️",
        }
    }
    fn all() -> [TabId; 3] {
        [TabId::Dashboard, TabId::Metrics, TabId::Settings]
    }
}

#[derive(Resource, Debug)]
struct UiState {
    active: TabId,
}

fn ui_system(mut ctx: IcedContext<UiMessage>, state: Res<UiState>) {
    // tiny knobs
    let rail_w = 72.0;
    let chip_d = 32.0;

    // --- left rail (icons with tooltips) ---
    let tip_pos = tooltip::Position::Right;

    let mut rail_col = column![
        container(text("🧊")).padding([8, 0]).align_x(Alignment::Center),
        iced::widget::rule::horizontal(1),
    ]
    .spacing(10)
    .align_x(Alignment::Center);

    for tab in TabId::all() {
        let is_active = state.active == tab;

        let chip = container(text(tab.icon()).size(16))
            .width(Length::Fixed(chip_d))
            .height(Length::Fixed(chip_d))
            .align_x(Alignment::Center)
            .align_y(Alignment::Center);

        let btn_content: iced::Element<_> = if is_active {
            row![chip]
                .align_y(Alignment::Center)
                .into()
        } else {
            chip.into()
        };

        let btn = button(btn_content)
            .on_press(UiMessage::Select(tab))
            .padding(4)
            .width(Length::Fill);

        let content: iced::Element<_> = if is_active {
            btn.into()
        } else {
            tooltip(btn, text(tab.label()), tip_pos).gap(6).padding(6).into()
        };
        rail_col = rail_col.push(content);
    }

    rail_col = rail_col
        .push(Space::new().height(Length::Fill))
        .push(iced::widget::rule::horizontal(1))
        .push(
            container(text("online").size(12))
                .padding([6, 10])
                .style(|_| {
                    container::Style::default()
                        .background(iced::Color::from_rgba(0.20, 0.80, 0.45, 0.25))
                        .border(iced::Border {
                            radius: 999.0.into(),
                            width: 0.0,
                            color: iced::Color::TRANSPARENT,
                        })
                        .color(iced::Color::from_rgba(0.95, 1.0, 0.98, 0.96))
                }),
        );

    let rail = container(rail_col)
        .padding(8)
        .width(Length::Fixed(rail_w))
        .height(Length::Fill)
        .style(|_| {
            container::Style::default()
                .background(iced::Color::from_rgba(0.10, 0.12, 0.16, 0.65))
                .border(iced::Border {
                    radius: 12.0.into(),
                    width: 1.0,
                    color: iced::Color::from_rgba(1.0, 1.0, 1.0, 0.06),
                })
        });

    // simple title + content card
    let title = match state.active {
        TabId::Dashboard => "Dashboard",
        TabId::Metrics   => "Metrics",
        TabId::Settings  => "Settings",
    };

    let content_inner: iced::Element<_> = match state.active {
        TabId::Dashboard => text("Welcome to the dashboard.").into(),
        TabId::Metrics   => text("Metrics view coming soon...").into(),
        TabId::Settings  => text("Adjust your settings here.").into(),
    };

    let content_card = container(
        column![
            text(title).size(18),
            iced::widget::rule::horizontal(1),
            content_inner
        ]
        .spacing(10),
    )
    .padding(16)
    .width(Length::Fill)
    .style(|_| {
        container::Style::default()
            .background(iced::Color::from_rgba(0.16, 0.18, 0.22, 0.90))
            .border(iced::Border {
                radius: 12.0.into(),
                width: 1.0,
                color: iced::Color::from_rgba(1.0, 1.0, 1.0, 0.06),
            })
            .color(iced::Color::from_rgba(0.95, 0.98, 1.0, 0.98))
    });

    // thin divider keeps things tidy
    let layout = row![rail, content_card]
        .spacing(12)
        .height(Length::Fill)
        .align_y(Alignment::Start)
        .padding(10);

    ctx.display(layout);
}


fn update_system(
    mut state: ResMut<UiState>,
    mut messages: bevy_ecs::message::MessageReader<UiMessage>,
) {
    for m in messages.read() {
        if let UiMessage::Select(tab) = m {
            state.active = *tab;
        }
    }
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Ground
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(5.0, 5.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
    ));
    // Cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
    // Light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
    // Camera
    commands.spawn((
        Transform::from_xyz(0.0, 1.5, 5.0),
        PanOrbitCamera::default(),
    ));
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy + Iced HUD Overlay".into(),
                resolution: WindowResolution::new(1600, 1000),
                resizable: true,
                ..Default::default()
            }),
            ..Default::default()
        }))
		.add_plugins(PanOrbitCameraPlugin)
        .add_plugins(IcedPlugin::<UiMessage>::default())
        .add_message::<UiMessage>()
        .insert_resource(UiState { active: TabId::Dashboard })
        .add_systems(Startup, setup_scene)
        .add_systems(
            Update,
            (
                update_system.in_set(IcedProgramSet::Update),
                ui_system.in_set(IcedProgramSet::View),
            ),
        )
        .run();
}

