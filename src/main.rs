mod animation;
mod menu;
mod game_states;
mod fx;
mod player;
mod character_controller;
mod physics;

use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_skein::SkeinPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Piggy souls".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(WorldInspectorPlugin::new())
        .add_plugins(SkeinPlugin::default())
        .add_plugins(menu::MenuPlugin)
        .add_plugins(animation::AnimationTestPlugin)
        .add_plugins(fx::FXPlugin)
        .add_plugins(physics::PhysicsPlugin)
        .add_plugins(player::Player)
        .run();
}
