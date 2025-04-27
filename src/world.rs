use std::f32::consts::PI;
use avian3d::prelude::{ColliderConstructor, ColliderConstructorHierarchy};
use avian3d::prelude::{RigidBody};
use bevy::pbr::CascadeShadowConfigBuilder;
use bevy::pbr::light_consts::lux;
use bevy::prelude::*;
use crate::game_states::AppState;

pub(crate) struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(AppState::InGame), setup)
            .add_systems(Update, dynamic_scene.run_if(in_state(AppState::InGame)))
        ;
    }
}


fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    // Environment (see the `collider_constructors` example for creating colliders from scenes)
    commands.spawn((
        SceneRoot(asset_server.load("character_controller_demo.glb#Scene0")),
        Transform::from_rotation(Quat::from_rotation_y(-PI * 0.5)),
        ColliderConstructorHierarchy::new(ColliderConstructor::ConvexHullFromMesh),
        RigidBody::Static,
    ));

    commands.spawn((
        SceneRoot(asset_server.load("models/piggy.glb#Scene0")),
        Transform::from_xyz(20.0, -0.0, 20.0).with_scale(Vec3::new(0.3, 0.3, 0.3)).with_rotation(Quat::from_rotation_y(-PI * 0.25)),
    ));


    // Environment (see the `collider_constructors` example for creating colliders from scenes)
    commands.spawn((
        SceneRoot(asset_server.load("area_0001.glb#Scene0")),
        Transform::from_xyz(0.0, 0.0, 0.0),
        ColliderConstructorHierarchy::new(ColliderConstructor::TrimeshFromMesh),
        RigidBody::Static,
    ));


    // Light
    commands.spawn((
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, 1.0, -PI / 4.)),
        DirectionalLight {
            shadows_enabled: true,
            illuminance: lux::RAW_SUNLIGHT,
            ..default()
        },
        CascadeShadowConfigBuilder {
            first_cascade_far_bound: 0.3,
            maximum_distance: 10.0,
            ..default()
        }
            .build(),
    ));
}
fn dynamic_scene(mut suns: Query<&mut Transform, With<DirectionalLight>>, time: Res<Time>) {
    suns.iter_mut()
        .for_each(|mut tf| tf.rotate_x(-time.delta_secs() * PI / 10.0));
}