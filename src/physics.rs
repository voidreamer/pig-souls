use avian3d::PhysicsPlugins;
use avian3d::prelude::{PhysicsDebugPlugin, PhysicsInterpolationPlugin};
use bevy::prelude::*;
use crate::breakable::BreakablePropsPlugin;

pub(crate) struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(BreakablePropsPlugin)
            .add_plugins(PhysicsDebugPlugin::default())
            .add_plugins(PhysicsPlugins::default().set(PhysicsInterpolationPlugin::extrapolate_all()));
    }
}

