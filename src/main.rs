mod animation;
mod menu;
mod game_states;
mod fx;

use bevy::prelude::*;
use bevy_inspector_egui::quick::WorldInspectorPlugin;


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
        .add_plugins(menu::MenuPlugin)
        .add_plugins(animation::AnimationTestPlugin)
        .add_plugins(fx::SmokeExplosionPlugin)
        .run();
}
