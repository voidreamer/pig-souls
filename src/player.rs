//! A basic implementation of a character controller for a dynamic rigid body.
//!
//! This showcases the following:
//!
//! - Basic directional movement and jumping
//! - Support for both keyboard and gamepad input
//! - A configurable maximum slope angle for jumping
//! - Loading a platformer environment from a glTF
//!
//! The character controller logic is contained within the `plugin` module.
//!
//! For a kinematic character controller, see the `kinematic_character_3d` example.

use crate::character_controller;
use avian3d::{math::*, prelude::*};
use avian3d::parry::shape::Capsule;
use bevy::prelude::*;
use character_controller::*;
use crate::game_states::AppState;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_plugins(CharacterControllerPlugin)
            .add_systems(OnEnter(AppState::InGame), setup);
    }
}

const CHARACTER_PATH: &str = "models/animated/Fox.glb";

// Update your Player struct in src/player.rs to include speed variables
#[derive(Component)]
pub struct Player {
    pub is_moving: bool,
    pub is_attacking: bool,

    // Movement speeds
    pub walk_speed: f32,
    pub run_speed: f32,
    pub current_speed: f32,
    pub is_sprinting: bool,

    // Added for UI
    pub health: f32,
    pub max_health: f32,
    pub stamina: f32,
    pub max_stamina: f32,
    pub stamina_regen_rate: f32,
    pub stamina_use_rate: f32,
    pub exhausted: bool,
    pub exhaustion_timer: f32,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            is_moving: false,
            is_attacking: false,

            // Default movement speeds
            walk_speed: 200.0,       // Normal walking speed
            run_speed: 400.0,        // Sprint speed when holding Shift
            current_speed: 200.0,    // Start at walking speed
            is_sprinting: false,    // Not sprinting initially

            // Stats
            health: 100.0,
            max_health: 100.0,
            stamina: 100.0,
            max_stamina: 100.0,
            stamina_regen_rate: 30.0,
            stamina_use_rate: 15.0,
            exhausted: false,
            exhaustion_timer: 0.0,
        }
    }
}

#[derive(Resource)]
pub struct PlayerGltfHandle(pub Handle<Gltf>);


fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Player
    commands.spawn((
        Mesh3d(meshes.add(Capsule3d::new(0.4, 1.0))),
        SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(CHARACTER_PATH))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
        Transform::from_xyz(0.0, 1.5, 0.0),
        Player::default(),
        CharacterControllerBundle::new(
            Collider::capsule(0.4, 1.0)).with_movement(
                30.0,
                0.92,
                7.0,
                (30.0 as Scalar).to_radians(),
        ),
        Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
        Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
        GravityScale(2.0),
    ));

    // A cube to move around
    commands.spawn((
        RigidBody::Dynamic,
        Collider::cuboid(1.0, 1.0, 1.0),
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
        Transform::from_xyz(3.0, 2.0, 3.0),
    ));

    // Environment (see the `collider_constructors` example for creating colliders from scenes)
    commands.spawn((
        SceneRoot(asset_server.load("character_controller_demo.glb#Scene0")),
        Transform::from_rotation(Quat::from_rotation_y(-core::f32::consts::PI * 0.5)),
        ColliderConstructorHierarchy::new(ColliderConstructor::ConvexHullFromMesh),
        RigidBody::Static,
    ));
}