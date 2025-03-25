use avian3d::PhysicsPlugins;
use avian3d::prelude::PhysicsInterpolationPlugin;
use bevy::prelude::*;

pub(crate) struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(
                PhysicsPlugins::default().set(PhysicsInterpolationPlugin::extrapolate_all()));
    }
}

