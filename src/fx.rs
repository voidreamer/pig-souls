use bevy::prelude::*;
use bevy_hanabi::prelude::*;
use crate::game_states::AppState;

// Components to mark entities with specific effects
#[derive(Component)]
pub struct SparkEffect;

#[derive(Component)]
pub struct FireEffect;

#[derive(Component)]
pub struct FireStepEffect;


// Create a component to store effect handles for later spawning on demand
#[derive(Resource)]
pub struct EffectHandles {
    pub spark: Handle<EffectAsset>,
    pub fire: Handle<EffectAsset>,
    pub fire_step: Handle<EffectAsset>,
}

fn create_fire_effect(effects: &mut Assets<EffectAsset>, position: Vec3) -> Handle<EffectAsset> {
    let mut color_gradient_fire = Gradient::new();
    color_gradient_fire.add_key(0.0, Vec4::new(10.0, 0.9, 0.4, 0.0));     // Start transparent
    color_gradient_fire.add_key(0.05, Vec4::new(10.8, 1.5, 0.5, 0.9));    // Bright yellow core
    color_gradient_fire.add_key(0.2, Vec4::new(10.8, 0.8, 0.2, 0.9));     // Intense orange
    color_gradient_fire.add_key(0.4, Vec4::new(10.5, 0.5, 0.1, 0.8));     // Dark orange
    color_gradient_fire.add_key(0.7, Vec4::new(10.0, 0.2, 0.05, 0.6));    // Deep red
    color_gradient_fire.add_key(0.9, Vec4::new(10.5, 0.1, 0.05, 0.3));    // Dark smoke-like
    color_gradient_fire.add_key(1.0, Vec4::new(10.2, 0.1, 0.05, 0.0));    // Fade out

    // Varied sizes for a more dynamic fire
    let mut size_gradient_fire = Gradient::new();
    size_gradient_fire.add_key(0.0, Vec3::splat(0.02));         // Start small
    size_gradient_fire.add_key(0.1, Vec3::splat(0.08));         // Grow quickly
    size_gradient_fire.add_key(0.3, Vec3::splat(0.15));         // Peak size
    size_gradient_fire.add_key(0.7, Vec3::splat(0.18));         // Expand as it rises
    size_gradient_fire.add_key(1.0, Vec3::splat(0.05));         // Shrink at end but not to zero

    let writer = ExprWriter::new();
    let effect_scale = 1.2;

    // Using sphere for fire base
    let fire_pos = SetPositionSphereModifier {
        center: writer.lit(position).expr(),
        radius: writer.lit(effect_scale).expr(),
        dimension: ShapeDimension::Volume,
    };

    // Initial velocity with upward bias
    let init_vel = SetVelocitySphereModifier {
        center: writer.lit(Vec3::new(0.0, 0.4, 0.0)).expr(), // Upward bias
        speed: writer.lit(0.3).uniform(writer.lit(0.7)).expr(),
    };

    let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.0).expr());

    // Varied lifetime for realistic flicker
    let init_lifetime = SetAttributeModifier::new(
        Attribute::LIFETIME,
        writer.lit(1.0).uniform(writer.lit(1.8)).expr(),
    );

    // Stronger upward acceleration for realistic fire behavior
    let accel = writer.lit(Vec3::new(0.0, 1.0, 0.0)).expr();
    let update_accel = AccelModifier::new(accel);

    // Add some drag to slow particles as they rise
    let drag_val = writer.lit(0.3).expr();

    let module = writer.finish();
    let drag = LinearDragModifier::new(drag_val);

    let effect = effects.add(
        EffectAsset::new(15000, SpawnerSettings::rate(12000.0.into()), module)
            .with_name("fire")
            .init(fire_pos)
            .init(init_vel)
            .init(init_age)
            .init(init_lifetime)
            .update(update_accel)
            .update(drag)
            .render(ColorOverLifetimeModifier {
                gradient: color_gradient_fire,
            })
            .render(SizeOverLifetimeModifier {
                gradient: size_gradient_fire,
                screen_space_size: false,
            }),
    );
    effect
}

pub fn create_fire_step_effect(
    effects: &mut Assets<EffectAsset>,
    position: Vec3,
    scale_factor: f32
) -> Handle<EffectAsset> {
    let mut color_gradient_fire = Gradient::new();
    color_gradient_fire.add_key(0.0, Vec4::new(10.0, 0.9, 0.4, 0.0));     // Start transparent
    color_gradient_fire.add_key(0.05, Vec4::new(10.8, 1.5, 0.5, 0.9));    // Bright yellow core
    color_gradient_fire.add_key(0.2, Vec4::new(10.8, 0.8, 0.2, 0.9));     // Intense orange
    color_gradient_fire.add_key(0.4, Vec4::new(10.5, 0.5, 0.1, 0.8));     // Dark orange
    color_gradient_fire.add_key(0.7, Vec4::new(10.0, 0.2, 0.05, 0.6));    // Deep red
    color_gradient_fire.add_key(0.9, Vec4::new(10.5, 0.1, 0.05, 0.3));    // Dark smoke-like
    color_gradient_fire.add_key(1.0, Vec4::new(10.2, 0.1, 0.05, 0.0));    // Fade out

    // Scale particle sizes based on scale_factor
    let mut size_gradient_fire = Gradient::new();
    size_gradient_fire.add_key(0.0, Vec3::splat(0.1 * scale_factor));       // Start small
    size_gradient_fire.add_key(0.1, Vec3::splat(0.3 * scale_factor));       // Grow quickly
    size_gradient_fire.add_key(0.3, Vec3::splat(0.5 * scale_factor));       // Peak size
    size_gradient_fire.add_key(0.7, Vec3::splat(0.4 * scale_factor));       // Maintain as it rises
    size_gradient_fire.add_key(1.0, Vec3::splat(0.1 * scale_factor));       // Shrink at end

    let writer = ExprWriter::new();
    // Use the scale_factor for the overall effect size
    let effect_radius = scale_factor;

    // Using sphere for fire base with larger radius for fox scale
    let fire_pos = SetPositionSphereModifier {
        center: writer.lit(position).expr(),
        radius: writer.lit(effect_radius).expr(),
        dimension: ShapeDimension::Volume,
    };

    // Much higher velocity for dramatic effect, scaled with fox size
    let init_vel = SetVelocitySphereModifier {
        center: writer.lit(Vec3::new(0.0, 2.0 * scale_factor, 0.0)).expr(), // Strong upward bias
        speed: writer.lit(2.0 * scale_factor).uniform(writer.lit(5.0 * scale_factor)).expr(),
    };

    let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.0).expr());

    // Shorter lifetime for a quick burst effect
    let init_lifetime = SetAttributeModifier::new(
        Attribute::LIFETIME,
        writer.lit(0.3).uniform(writer.lit(0.7)).expr(),
    );

    // Strong upward acceleration for dramatic effect
    let accel = writer.lit(Vec3::new(0.0, 10.0 * scale_factor, 0.0)).expr();
    let update_accel = AccelModifier::new(accel);

    // Add some drag to control the upward motion
    let drag_val = writer.lit(0.4).expr();
    let drag = LinearDragModifier::new(drag_val);

    // Add a rotation to the particles for more dynamic effect
    let rotation = (writer.rand(ScalarType::Float) * writer.lit(std::f32::consts::TAU)).expr();
    let init_rotation = SetAttributeModifier::new(Attribute::F32_0, rotation);

    let module = writer.finish();

    // Use burst spawner for immediate impact rather than continuous rate
    let particle_count = (500.0 * scale_factor) as f32;

    let effect = effects.add(
        EffectAsset::new(
            15000,
            SpawnerSettings::burst(particle_count.into(), 1.0.into()),
            module
        )
            .with_name("footstep_fire")
            .init(fire_pos)
            .init(init_vel)
            .init(init_age)
            .init(init_lifetime)
            .init(init_rotation)
            .update(update_accel)
            .update(drag)
            .render(ColorOverLifetimeModifier {
                gradient: color_gradient_fire,
            })
            .render(SizeOverLifetimeModifier {
                gradient: size_gradient_fire,
                screen_space_size: false,
            })
            .render(OrientModifier::new(OrientMode::FaceCameraPosition)),
    );

    effect
}

fn create_spark_effect(effects: &mut Assets<EffectAsset>, position: Vec3) -> Handle<EffectAsset> {
    let mut color_gradient_spark = Gradient::new();
    color_gradient_spark.add_key(0.0, Vec4::new(2.5, 2.0, 0.8, 1.0));   // Brilliant white-yellow center
    color_gradient_spark.add_key(0.1, Vec4::new(2.2, 1.6, 0.4, 1.0));   // Bright yellow
    color_gradient_spark.add_key(0.3, Vec4::new(2.0, 0.8, 0.1, 0.9));   // Orange
    color_gradient_spark.add_key(0.6, Vec4::new(1.5, 0.4, 0.0, 0.7));   // Deep orange
    color_gradient_spark.add_key(0.8, Vec4::new(1.0, 0.2, 0.0, 0.4));   // Dark red
    color_gradient_spark.add_key(1.0, Vec4::new(0.5, 0.1, 0.0, 0.0));   // Fade out

    // Longer, thinner sparks that taper
    let mut size_gradient_spark = Gradient::new();
    size_gradient_spark.add_key(0.0, Vec3::new(0.005, 0.02, 0.005));  // Thin streaks
    size_gradient_spark.add_key(0.2, Vec3::new(0.003, 0.015, 0.003)); // Maintain thinness
    size_gradient_spark.add_key(0.5, Vec3::new(0.002, 0.01, 0.002));  // Taper
    size_gradient_spark.add_key(1.0, Vec3::new(0.001, 0.001, 0.001)); // Tiny point

    let writer = ExprWriter::new();

    // Tighter initial position for focus
    let init_pos = SetPositionSphereModifier {
        center: writer.lit(position).expr(),
        radius: writer.lit(0.01).expr(),
        dimension: ShapeDimension::Volume,
    };

    // Higher-velocity, directionally varied sparks
    let init_vel = SetVelocitySphereModifier {
        center: writer.lit(Vec3::ZERO).expr(),
        speed: writer.lit(1.5).uniform(writer.lit(3.0)).expr(), // Faster sparks
    };

    // Initialize age
    let age = writer.lit(0.0).expr();
    let init_age = SetAttributeModifier::new(Attribute::AGE, age);

    // Slightly longer lifetimes for better trails
    let lifetime = writer.lit(0.3).uniform(writer.lit(0.6)).expr();
    let init_lifetime = SetAttributeModifier::new(Attribute::LIFETIME, lifetime);

    // Stronger gravity affects sparks
    let gravity = writer.lit(Vec3::new(0.0, -2.0, 0.0)).expr(); // Stronger gravity
    let update_accel = AccelModifier::new(gravity);

    // Add drag to slow down sparks over time
    let drag_val = writer.lit(0.5).expr();
    let update_drag = LinearDragModifier::new(drag_val);

    let module = writer.finish();

    effects.add(
        EffectAsset::new(256, SpawnerSettings::burst(80.0.into(), 1.0.into()), module)
            .with_name("spark")
            .init(init_pos)
            .init(init_vel)
            .init(init_age)
            .init(init_lifetime)
            .update(update_accel)
            .update(update_drag)
            .render(ColorOverLifetimeModifier {
                gradient: color_gradient_spark,
            })
            .render(SizeOverLifetimeModifier {
                gradient: size_gradient_spark,
                screen_space_size: false,
            })
            .render(OrientModifier::new(OrientMode::AlongVelocity)),
    )
}

fn start_fx_resources(
    mut commands: Commands,
    mut effects: ResMut<Assets<EffectAsset>>,
){
    let spark_effect = create_spark_effect(&mut effects, Vec3::ZERO);
    let fire_effect= create_fire_effect(&mut effects, Vec3::ZERO);
    let fire_step_effect= create_fire_step_effect(
        &mut effects,
        Vec3::ZERO,
        0.1
    );
    commands.insert_resource(EffectHandles {
        spark: spark_effect.clone(),
        fire: fire_effect.clone(),
        fire_step: fire_step_effect.clone(),
    });
}

// Add this component to handle one-shot effects
#[derive(Component)]
pub struct OneShotParticleEffect {
    effect_handle: Handle<EffectAsset>,
    position: Vec3,
    timer: Timer,
    spawned: bool,
}

impl OneShotParticleEffect {
    pub fn new(effect_handle: Handle<EffectAsset>, position: Vec3, duration: f32) -> Self {
        Self {
            effect_handle,
            position,
            timer: Timer::from_seconds(duration, TimerMode::Once),
            spawned: false,
        }
    }
}

pub fn handle_one_shot_effects(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut OneShotParticleEffect)>,
    asset_server: Res<AssetServer>,
) {
    let cloud_texture = asset_server.load("textures/fx/cloud.png");
    for (entity, mut effect) in &mut query {
        // On the first frame, spawn the actual particle effect
        if !effect.spawned {
            commands.entity(entity).insert((
                ParticleEffect::new(effect.effect_handle.clone()),
                Transform::from_translation(effect.position),
                EffectMaterial {
                    images: vec![cloud_texture.clone()],
                },
            ));
            effect.spawned = true;
        }

        // After the timer expires, despawn the entity
        if effect.timer.tick(time.delta()).just_finished() {
            commands.entity(entity).despawn_recursive();
        }
    }
}


pub struct FXPlugin;

impl Plugin for FXPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(AppState::InGame), start_fx_resources)
            .add_plugins(HanabiPlugin);
    }
}