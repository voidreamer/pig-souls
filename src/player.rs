use std::f32::consts::PI;
use avian3d::{prelude::*};
use bevy::prelude::*;
use crate::game_states::AppState;
use crate::character_controller::*;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(AppState::InGame), setup);
    }
}

const CHARACTER_PATH: &str = "models/animated/Fox.glb";

#[derive(Component)]
pub struct Player {
    pub is_moving: bool,

    pub movement_direction: Vec3,

    // Movement speeds
    pub walk_speed: f32,
    pub run_speed: f32,
    pub current_speed: f32,
    pub is_sprinting: bool,

    // Roll mechanics
    pub is_rolling: bool,
    pub roll_speed: f32,
    pub roll_duration: f32,
    pub roll_cooldown: f32,
    pub roll_timer: f32,
    pub roll_cooldown_timer: f32,
    pub roll_direction: Vec3,
    pub can_roll: bool,

    // Jump improvements
    pub fall_multiplier: f32, // Increases gravity during falling
    pub coyote_time: f32, // Time player can jump after leaving a platform
    pub coyote_timer: f32,

    // Block mechanics
    pub is_blocking: bool,
    pub can_move_while_blocking: bool,
    pub block_movement_penalty: f32, // Speed reduction while blocking

    // Added for UI
    pub stamina: f32,
    pub max_stamina: f32,
    pub stamina_regen_rate: f32,
    pub stamina_use_rate: f32,
    pub exhausted: bool,
    pub exhaustion_timer: f32,

    // Stamina costs
    pub roll_stamina_cost: f32,
    pub block_stamina_cost_per_sec: f32,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            is_moving: false,
            movement_direction: Vec3::new(0.0, 0.0, 0.0),

            // Default movement speeds
            walk_speed: 200.0,       // Normal walking speed (increased as requested)
            run_speed: 350.0,        // Sprint speed when holding Shift
            current_speed: 200.0,    // Start at walking speed
            is_sprinting: false,     // Not sprinting initially

            // Roll settings
            is_rolling: false,
            roll_speed: 1000.0,       // Fast roll speed
            roll_duration: 0.1,      // How long the roll lasts in seconds
            roll_cooldown: 0.5,      // Time before player can roll again
            roll_timer: 0.0,         // Current active roll time
            roll_cooldown_timer: 0.0, // Current cooldown timer
            roll_direction: Vec3::ZERO,
            can_roll: true,          // Can player roll right now

            // Jump improvements
            fall_multiplier: 2.5,    // Makes falling faster than rising
            coyote_time: 0.1,        // Short grace period for jumps
            coyote_timer: 0.0,       // Current coyote time

            // Block settings
            is_blocking: false,
            can_move_while_blocking: true,
            block_movement_penalty: 0.5, // Move at 50% speed while blocking

            // Stats
            stamina: 100.0,
            max_stamina: 100.0,
            stamina_regen_rate: 30.0,
            stamina_use_rate: 15.0,
            exhausted: false,
            exhaustion_timer: 0.0,

            // Stamina costs
            roll_stamina_cost: 20.0,       // Cost per roll
            block_stamina_cost_per_sec: 5.0, // Cost per second while blocking

        }
    }
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let body_collider = Collider::capsule(0.5, 1.0);

    commands.spawn((
        SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(CHARACTER_PATH))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
        //Transform::from_xyz(0.0, 1.5, 0.0),
        Transform::from_xyz(20.0, 1.0, 20.0).with_scale(Vec3::new(0.3, 0.3, 0.3)).with_rotation(Quat::from_rotation_y(-PI * 0.25)),
        Player::default(),
        CharacterController::new(body_collider), // This should add GroundNormal via required components
        Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
        Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
        GravityScale(2.0),
        Mass(2.0),
        ExternalImpulse::new(Vec3::new(-1.0, 0.5, 0.0)),
    ));
}