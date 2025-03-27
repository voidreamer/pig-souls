use std::f32::consts::PI;
use avian3d::collision::{Collider, ColliderConstructor, ColliderConstructorHierarchy};
use avian3d::prelude::{RigidBody};
use bevy::pbr::CascadeShadowConfigBuilder;
use bevy::prelude::*;
use crate::game_states::AppState;

pub(crate) struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(AppState::InGame), setup);
    }
}


fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Environment (see the `collider_constructors` example for creating colliders from scenes)
    commands.spawn((
        Transform::from_xyz(0.0, 0.0, 0.0),
        Visibility::default(),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(10.0, 10.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.5, 0.5, 1.0))),
        Collider::cuboid(10.0, 0.1, 10.0),
        RigidBody::Static,
    ));
    /*
    commands.spawn((
        SceneRoot(asset_server.load("character_controller_demo.glb#Scene0")),
        Transform::from_rotation(Quat::from_rotation_y(-core::f32::consts::PI * 0.5)),
        ColliderConstructorHierarchy::new(ColliderConstructor::ConvexHullFromMesh),
        RigidBody::Static,
    ));

    // Environment (see the `collider_constructors` example for creating colliders from scenes)
    commands.spawn((
        SceneRoot(asset_server.load("area_0005.glb#Scene0")),
        // Transform::from_rotation(Quat::from_rotation_y(-core::f32::consts::PI * 0.5)),
        Transform::from_xyz(0.0, -15.0, 0.0),
        ColliderConstructorHierarchy::new(ColliderConstructor::TrimeshFromMesh),
        RigidBody::Static,
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(100.0, 0.1, 100.0))),
        Transform::from_rotation(Quat::from_rotation_y(-core::f32::consts::PI * 0.5)),
        RigidBody::Static,
        Collider::cuboid(100.0, 0.1, 100.0)
    ));
     */

    // Light
    commands.spawn((
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, 1.0, -PI / 4.)),
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        CascadeShadowConfigBuilder {
            first_cascade_far_bound: 200.0,
            maximum_distance: 400.0,
            ..default()
        }
            .build(),
    ));
}
