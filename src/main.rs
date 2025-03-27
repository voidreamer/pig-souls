mod animation;
mod menu;
mod game_states;
mod camera;
mod fx;
mod player;
mod character_controller;
mod physics;
mod world;

use bevy::prelude::*;
use bevy::window::{WindowResolution};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_skein::SkeinPlugin;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    //mode: bevy::window::WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
                    present_mode: bevy::window::PresentMode::AutoVsync,
                    resolution: WindowResolution::new(1920., 1080.).with_scale_factor_override(1.0),
                    /*
                    cursor_options: CursorOptions {
                        grab_mode: CursorGrabMode::Confined,
                        visible: false,
                        ..default()
                    },
                     */
                    ..default()
                }),
                ..default()
            })
            .set(ImagePlugin::default_nearest()))
        .add_plugins(WorldInspectorPlugin::new())
        .add_plugins(SkeinPlugin::default())
        .add_plugins(menu::MenuPlugin)
        .add_plugins(animation::AnimationTestPlugin)
        .add_plugins(fx::FXPlugin)
        .add_plugins(physics::PhysicsPlugin)
        .add_plugins(player::PlayerPlugin)
        .add_plugins(camera::CameraPlugin)
        .add_plugins(world::WorldPlugin)
        .add_plugins(character_controller::CharacterControllerPlugin)
        .run();
}
