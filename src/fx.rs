//! Thick Smoke Cloud Explosion Effect
//!
//! Creates a slow-moving, volumetric smoke explosion with
//! billowing clouds, drifting particles, and subtle sparkles.

use std::f32::consts::PI;
use bevy::{
    prelude::*,
};
use bevy::asset::RenderAssetUsages;
use bevy_hanabi::prelude::*;
use crate::game_states::AppState;

// Constants for the explosion
const EXPLOSION_DURATION: f32 = 8.0; // Total duration of the explosion effect

#[derive(Component)]
struct ExplosionTimer {
    timer: Timer,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // commands.init_resource::<EffectAsset>();
    // Add a small platform to show where the explosion will occur
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(0.5))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::BLACK,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.01, 0.0).with_rotation(Quat::from_rotation_x(-PI/2.0)),
    ));

    // Add a button to trigger the explosion
    commands.spawn((
        Button,
        Interaction::default(),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(20.0),
            bottom: Val::Px(20.0),
            width: Val::Px(150.0),
            height: Val::Px(40.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
    ));
}

fn handle_button(
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<Button>)>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut effects: ResMut<Assets<EffectAsset>>,
) {
    for interaction in &interaction_query {
        if matches!(interaction, Interaction::Pressed) {
            spawn_explosion(&mut commands, &asset_server, &mut effects);
        }
    }

    spawn_explosion(&mut commands, &asset_server, &mut effects);
}

pub fn spawn_explosion(
    commands: &mut Commands,
    asset_server: &Res<AssetServer>,
    effects: &mut ResMut<Assets<EffectAsset>>,
) {
    // Load the cloud texture from file
    let cloud_texture = asset_server.load("textures/fx/cloud.png");

    // Create the various explosion effects
    let core_effect = create_core_explosion_effect(effects);
    let dense_smoke_effect = create_dense_smoke_effect(effects);
    let detail_particles_effect = create_detail_particles_effect(effects);
    let sparkle_effect = create_sparkle_effect(effects);

    // Spawn an entity to hold all explosion effects
    let explosion_id = commands.spawn((
        Transform::from_xyz(0.0, 0.0, 0.0),
        ExplosionTimer {
            timer: Timer::from_seconds(EXPLOSION_DURATION, TimerMode::Once)
        },
        Name::new("explosion"),
    )).id();

    // Add all explosion components
    commands.entity(explosion_id).with_children(|parent| {
        // Core initial burst
        parent.spawn((
            ParticleEffect::new(core_effect),
            EffectMaterial {
                images: vec![cloud_texture.clone()],
            },
            Transform::from_xyz(0.0, 0.2, 0.0),
            Name::new("core_explosion"),
        ));

        // Dense billowing smoke clouds
        parent.spawn((
            ParticleEffect::new(dense_smoke_effect),
            EffectMaterial {
                images: vec![cloud_texture.clone()],
            },
            Transform::from_xyz(0.0, 0.1, 0.0),
            Name::new("dense_smoke"),
        ));

        // Smaller detail particles
        parent.spawn((
            ParticleEffect::new(detail_particles_effect),
            EffectMaterial {
                images: vec![cloud_texture.clone()],
            },
            Transform::from_xyz(0.0, 0.0, 0.0),
            Name::new("detail_particles"),
        ));

        // Sparkles/embers
        parent.spawn((
            ParticleEffect::new(sparkle_effect),
            EffectMaterial {
                images: vec![cloud_texture.clone()],
            },
            Transform::from_xyz(0.0, 0.0, 0.0),
            Name::new("sparkles"),
        ));
    });
}

fn create_core_explosion_effect(effects: &mut ResMut<Assets<EffectAsset>>) -> Handle<EffectAsset> {
    // Initial burst effect - forms the core of the explosion
    let mut color_gradient = Gradient::new();
    color_gradient.add_key(0.0, Vec4::new(1.0, 1.0, 1.0, 1.0));    // Bright white
    color_gradient.add_key(0.2, Vec4::new(0.9, 0.9, 1.0, 0.9));    // Slightly blue-white
    color_gradient.add_key(0.5, Vec4::new(0.5, 0.6, 0.9, 0.7));    // Medium blue
    color_gradient.add_key(0.8, Vec4::new(0.2, 0.3, 0.5, 0.4));    // Dark blue/gray
    color_gradient.add_key(1.0, Vec4::new(0.1, 0.1, 0.2, 0.0));    // Near black, transparent

    // Size gradient - rapid expansion then slow growth
    let mut size_gradient = Gradient::new();
    size_gradient.add_key(0.0, Vec3::new(0.2, 0.2, 0.2));          // Start medium
    size_gradient.add_key(0.1, Vec3::new(0.8, 0.8, 0.8));          // Expand quickly
    size_gradient.add_key(0.5, Vec3::new(1.2, 1.2, 1.2));          // Continue growing
    size_gradient.add_key(0.8, Vec3::new(1.0, 1.0, 1.0));          // Slight contraction
    size_gradient.add_key(1.0, Vec3::new(0.8, 0.8, 0.8));          // Final size

    let writer = ExprWriter::new();

    // Initialize age to 0
    let age = writer.lit(0.0).expr();
    let init_age = SetAttributeModifier::new(Attribute::AGE, age);

    // Longer lifetime for core
    let lifetime = writer.lit(2.0).uniform(writer.lit(3.0)).expr();
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    // Spawn within a small sphere at the core
    let init_pos = SetPositionSphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        radius: writer.lit(0.3).expr(),
        dimension: ShapeDimension::Volume,
    };

    // Outward velocity from center
    let init_vel = SetAttributeModifier::new(
        Attribute::VELOCITY,
        (writer.attr(Attribute::POSITION).normalized() *
            (writer.lit(1.5) + writer.rand(ScalarType::Float).mul(writer.lit(1.0)))).expr(),
    );

    // Add random rotation
    let rotation = (writer.rand(ScalarType::Float) * writer.lit(2.0 * PI)).expr();
    let init_rotation = SetAttributeModifier::new(Attribute::F32_0, rotation);

    // Add upward acceleration (buoyancy)
    let accel = writer.lit(Vec3::new(0.0, 0.7, 0.0)).expr();
    let update_accel = AccelModifier::new(accel);

    // Texture reference
    let texture_slot = writer.lit(0u32).expr();

    let mut a = Some(writer.attr(Attribute::F32_0).expr());
    let mut module = writer.finish();
    module.add_texture_slot("cloud_texture");

    // Create once burst with many particles
    let effect = effects.add(
        EffectAsset::new(100, SpawnerSettings::once(CpuValue::Single(50.0)), module)
            .with_name("core_explosion_effect")
            .with_alpha_mode(bevy_hanabi::AlphaMode::Add)
            .init(init_pos)
            .init(init_vel)
            .init(init_age)
            .init(init_lifetime)
            .init(init_rotation)
            .update(update_accel)
            .render(ParticleTextureModifier {
                texture_slot,
                sample_mapping: ImageSampleMapping::ModulateOpacityFromR,
            })
            .render(ColorOverLifetimeModifier {
                gradient: color_gradient,
            })
            .render(SizeOverLifetimeModifier {
                gradient: size_gradient,
                screen_space_size: false,
            })
            .render(OrientModifier {
                mode: OrientMode::FaceCameraPosition,
                rotation: a,
            })
    );

    effect
}

fn create_dense_smoke_effect(effects: &mut ResMut<Assets<EffectAsset>>) -> Handle<EffectAsset> {
    // Dense billowing smoke clouds - forms the bulk of the visible smoke
    let mut color_gradient = Gradient::new();
    color_gradient.add_key(0.0, Vec4::new(0.9, 0.9, 1.0, 0.0));    // Transparent at start
    color_gradient.add_key(0.05, Vec4::new(0.9, 0.9, 1.0, 0.9));   // Fade in quickly to white
    color_gradient.add_key(0.3, Vec4::new(0.7, 0.7, 0.8, 0.8));    // Light gray
    color_gradient.add_key(0.6, Vec4::new(0.4, 0.4, 0.6, 0.6));    // Medium blue-gray
    color_gradient.add_key(0.9, Vec4::new(0.1, 0.1, 0.3, 0.3));    // Dark blue-gray
    color_gradient.add_key(1.0, Vec4::new(0.05, 0.05, 0.1, 0.0));  // Nearly black, transparent

    // Size gradient - large billowing clouds
    let mut size_gradient = Gradient::new();
    size_gradient.add_key(0.0, Vec3::new(0.3, 0.3, 0.3));          // Start with decent size
    size_gradient.add_key(0.2, Vec3::new(1.0, 1.0, 1.0));          // Grow to full size
    size_gradient.add_key(0.7, Vec3::new(1.5, 1.5, 1.5));          // Continue expanding
    size_gradient.add_key(1.0, Vec3::new(1.8, 1.8, 1.8));          // Maximum size at end

    let writer = ExprWriter::new();

    // Initialize age to 0
    let age = writer.lit(0.0).expr();
    let init_age = SetAttributeModifier::new(Attribute::AGE, age);

    // Long lifetime for persistent smoke
    let lifetime = writer.lit(4.0).uniform(writer.lit(7.0)).expr();
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    // Random spawn within larger volume
    let init_pos = SetPositionSphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        radius: writer.lit(0.5).expr(),
        dimension: ShapeDimension::Volume,
    };

    // Slow outward and upward velocity
    let init_vel = SetAttributeModifier::new(
        Attribute::VELOCITY,
        (writer.attr(Attribute::POSITION).normalized().mul(writer.lit(0.5)) +
            writer.lit(Vec3::new(0.0, 0.3, 0.0)) +
            (writer.rand(VectorType::VEC3F) - writer.lit(Vec3::new(0.5, 0.0, 0.5))).mul(writer.lit(0.2))).expr(),
    );

    // Add random rotation
    let rotation = (writer.rand(ScalarType::Float) * writer.lit(2.0 * PI)).expr();
    let init_rotation = SetAttributeModifier::new(Attribute::F32_0, rotation);

    // Add upward acceleration (buoyancy)
    let accel = writer.lit(Vec3::new(0.0, 0.2, 0.0)).expr();
    let update_accel = AccelModifier::new(accel);

    // Add drag to slow particles over time
    let drag = writer.lit(0.3).expr();
    let update_drag = LinearDragModifier { drag };

    // Texture reference
    let texture_slot = writer.lit(0u32).expr();

    let mut a = Some(writer.attr(Attribute::F32_0).expr());
    let mut module = writer.finish();
    module.add_texture_slot("cloud_texture");

    // Create continuous smoke emission
    let effect = effects.add(
        EffectAsset::new(300, SpawnerSettings::rate(40.0.into()), module)
            .with_name("dense_smoke_effect")
            .with_alpha_mode(bevy_hanabi::AlphaMode::Add)
            .init(init_pos)
            .init(init_vel)
            .init(init_age)
            .init(init_lifetime)
            .init(init_rotation)
            .update(update_accel)
            .update(update_drag)
            .render(ParticleTextureModifier {
                texture_slot,
                sample_mapping: ImageSampleMapping::ModulateOpacityFromR,
            })
            .render(ColorOverLifetimeModifier {
                gradient: color_gradient,
            })
            .render(SizeOverLifetimeModifier {
                gradient: size_gradient,
                screen_space_size: false,
            })
            .render(OrientModifier {
                mode: OrientMode::FaceCameraPosition,
                rotation: a,
            })
    );

    effect
}

fn create_detail_particles_effect(effects: &mut ResMut<Assets<EffectAsset>>) -> Handle<EffectAsset> {
    // Smaller detail particles to add texture and depth to the smoke
    let mut color_gradient = Gradient::new();
    color_gradient.add_key(0.0, Vec4::new(0.8, 0.8, 0.9, 0.0));    // Transparent at start
    color_gradient.add_key(0.05, Vec4::new(0.8, 0.8, 0.9, 0.7));   // Light blue-white
    color_gradient.add_key(0.4, Vec4::new(0.5, 0.5, 0.7, 0.6));    // Medium blue-gray
    color_gradient.add_key(0.8, Vec4::new(0.2, 0.2, 0.4, 0.3));    // Darker blue-gray
    color_gradient.add_key(1.0, Vec4::new(0.1, 0.1, 0.2, 0.0));    // Nearly black, transparent

    // Size gradient - smaller particles for texture
    let mut size_gradient = Gradient::new();
    size_gradient.add_key(0.0, Vec3::new(0.1, 0.1, 0.1));          // Start small
    size_gradient.add_key(0.2, Vec3::new(0.3, 0.3, 0.3));          // Grow to medium size
    size_gradient.add_key(0.8, Vec3::new(0.4, 0.4, 0.4));          // Peak size
    size_gradient.add_key(1.0, Vec3::new(0.2, 0.2, 0.2));          // Shrink at end

    let writer = ExprWriter::new();

    // Initialize age to 0
    let age = writer.lit(0.0).expr();
    let init_age = SetAttributeModifier::new(Attribute::AGE, age);

    // Medium lifetime for detail particles
    let lifetime = writer.lit(3.0).uniform(writer.lit(5.0)).expr();
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    // Spawn throughout a wider volume
    let init_pos = SetPositionSphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        radius: writer.lit(0.7).expr(),
        dimension: ShapeDimension::Volume,
    };

    // More dynamic, swirling movement
    let init_vel = SetAttributeModifier::new(
        Attribute::VELOCITY,
        ((writer.rand(VectorType::VEC3F) - writer.lit(Vec3::new(0.5, 0.2, 0.5))).mul(writer.lit(1.5)) +
            writer.lit(Vec3::new(0.0, 0.4, 0.0))).expr(),
    );

    // Add random rotation
    let rotation = (writer.rand(ScalarType::Float) * writer.lit(2.0 * PI)).expr();
    let init_rotation = SetAttributeModifier::new(Attribute::F32_0, rotation);

    // Add upward acceleration (buoyancy)
    let accel = writer.lit(Vec3::new(0.0, 0.3, 0.0)).expr();
    let update_accel = AccelModifier::new(accel);

    // Texture reference
    let texture_slot = writer.lit(0u32).expr();

    let mut a = Some(writer.attr(Attribute::F32_0).expr());
    let mut module = writer.finish();
    module.add_texture_slot("cloud_texture");

    // Create continuous emission of detail particles
    let effect = effects.add(
        EffectAsset::new(500, SpawnerSettings::rate(70.0.into()), module)
            .with_name("detail_particles_effect")
            .with_alpha_mode(bevy_hanabi::AlphaMode::Add)
            .init(init_pos)
            .init(init_vel)
            .init(init_age)
            .init(init_lifetime)
            .init(init_rotation)
            .update(update_accel)
            .render(ParticleTextureModifier {
                texture_slot,
                sample_mapping: ImageSampleMapping::ModulateOpacityFromR,
            })
            .render(ColorOverLifetimeModifier {
                gradient: color_gradient,
            })
            .render(SizeOverLifetimeModifier {
                gradient: size_gradient,
                screen_space_size: false,
            })
            .render(OrientModifier {
                mode: OrientMode::FaceCameraPosition,
                rotation: a,
            })
    );

    effect
}

fn create_sparkle_effect(effects: &mut ResMut<Assets<EffectAsset>>) -> Handle<EffectAsset> {
    // Small sparkle particles scattered throughout the smoke
    let mut color_gradient = Gradient::new();
    color_gradient.add_key(0.0, Vec4::new(1.0, 0.9, 0.7, 1.0));    // Bright yellow-white
    color_gradient.add_key(0.3, Vec4::new(1.0, 0.7, 0.3, 0.9));    // Orange
    color_gradient.add_key(0.6, Vec4::new(0.9, 0.3, 0.1, 0.7));    // Red
    color_gradient.add_key(0.9, Vec4::new(0.5, 0.1, 0.1, 0.4));    // Dark red
    color_gradient.add_key(1.0, Vec4::new(0.3, 0.0, 0.0, 0.0));    // Black-red, transparent

    // Size gradient - tiny sparks
    let mut size_gradient = Gradient::new();
    size_gradient.add_key(0.0, Vec3::new(0.02, 0.02, 0.02));       // Very small
    size_gradient.add_key(0.2, Vec3::new(0.04, 0.04, 0.04));       // Slightly larger
    size_gradient.add_key(0.7, Vec3::new(0.03, 0.03, 0.03));       // Shrink
    size_gradient.add_key(1.0, Vec3::new(0.01, 0.01, 0.01));       // Tiny at end

    let writer = ExprWriter::new();

    // Initialize age to 0
    let age = writer.lit(0.0).expr();
    let init_age = SetAttributeModifier::new(Attribute::AGE, age);

    // Medium lifetime for sparkles
    let lifetime = writer.lit(1.5).uniform(writer.lit(3.0)).expr();
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    // Spawn throughout the entire explosion volume
    let init_pos = SetPositionSphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        radius: writer.lit(0.8).expr(),
        dimension: ShapeDimension::Volume,
    };

    // Random movement in all directions, slight upward bias
    let init_vel = SetAttributeModifier::new(
        Attribute::VELOCITY,
        ((writer.rand(VectorType::VEC3F) - writer.lit(Vec3::new(0.5, 0.3, 0.5))).mul(writer.lit(1.0))).expr(),
    );

    // Add random rotation
    let rotation = (writer.rand(ScalarType::Float) * writer.lit(2.0 * PI)).expr();
    let init_rotation = SetAttributeModifier::new(Attribute::F32_0, rotation);

    // Add gravity to sparkles
    let accel = writer.lit(Vec3::new(0.0, -0.2, 0.0)).expr();
    let update_accel = AccelModifier::new(accel);

    // Texture reference
    let texture_slot = writer.lit(0u32).expr();

    let mut a = Some(writer.attr(Attribute::F32_0).expr());
    let mut module = writer.finish();
    module.add_texture_slot("cloud_texture");

    // Create continuous emission of sparkles
    let effect = effects.add(
        EffectAsset::new(300, SpawnerSettings::rate(50.0.into()), module)
            .with_name("sparkle_effect")
            .with_alpha_mode(bevy_hanabi::AlphaMode::Add)
            .init(init_pos)
            .init(init_vel)
            .init(init_age)
            .init(init_lifetime)
            .init(init_rotation)
            .update(update_accel)
            .render(ParticleTextureModifier {
                texture_slot,
                sample_mapping: ImageSampleMapping::ModulateOpacityFromR,
            })
            .render(ColorOverLifetimeModifier {
                gradient: color_gradient,
            })
            .render(SizeOverLifetimeModifier {
                gradient: size_gradient,
                screen_space_size: false,
            })
            .render(OrientModifier {
                mode: OrientMode::FaceCameraPosition,
                rotation: a,
            })
    );

    effect
}

fn update_explosion_timer(
    mut commands: Commands,
    mut explosion_query: Query<(Entity, &mut ExplosionTimer)>,
    time: Res<Time>,
) {
    for (entity, mut timer) in explosion_query.iter_mut() {
        timer.timer.tick(time.delta());

        if timer.timer.finished() {
            // Clean up the explosion entity when finished
            commands.entity(entity).despawn_recursive();
        }
    }
}

pub struct SmokeExplosionPlugin;

impl Plugin for SmokeExplosionPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(AppState::InGame), setup)
            .add_systems(Update, (update_explosion_timer, handle_button).run_if(in_state(AppState::InGame)));
    }
}
