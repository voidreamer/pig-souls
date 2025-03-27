use bevy::{
    core_pipeline::{bloom::Bloom, experimental::taa::{TemporalAntiAliasPlugin, TemporalAntiAliasing}, motion_blur::MotionBlur, tonemapping::Tonemapping, Skybox},
    input::{
        keyboard::KeyCode, mouse::{MouseMotion, MouseWheel}
    },
    pbr::{ScreenSpaceAmbientOcclusion, ScreenSpaceAmbientOcclusionQualityLevel, VolumetricFog},
    prelude::*,
    window::PrimaryWindow,
    math::StableInterpolate
};
use avian3d::prelude::*;
use crate::game_states::AppState;
use crate::player::Player;

#[derive(Component)]
pub struct ThirdPersonCamera {
    pub pitch: f32,
    pub yaw: f32,
    pub distance: f32,
    pub height_offset: f32,
    // Target offset for camera focus
    pub rotation_speed: f32,
    pub zoom_speed: f32,
    pub smoothness: f32, // Camera lag factor (0 = instant, 1 = no movement)
    // Camera controls inversion flags
    pub invert_x: bool,
    pub invert_y: bool,
}

impl Default for ThirdPersonCamera {
    fn default() -> Self {
        Self {
            pitch: 0.4,          // Initial pitch angle in radians
            yaw: 0.0,            // Initial yaw angle in radians
            distance: 5.0,       // Distance behind player
            height_offset: 1.5,  // Camera height above player
            rotation_speed: 0.004, // Mouse sensitivity
            zoom_speed: 0.5,     // Scroll zoom sensitivity
            smoothness: 5.0,    // Camera lag (the lower the lazier)
            invert_x: false,     // Don't invert horizontal mouse
            invert_y: false,     // Don't invert vertical mouse
        }
    }
}

// Spawn camera system
pub fn spawn_camera(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    commands.spawn((
        Camera3d::default(),
        Camera {
            hdr: true,
            ..default()
        },
        DistanceFog{
            color: Color::srgb_u8(43, 44, 100),
            falloff: FogFalloff::Exponential{
                density: 15e-3,
            },
            ..default()
        },
        Bloom {
            intensity: 0.03,
            ..default()
        },
        Tonemapping::TonyMcMapface,
        // Msaa is off to let ssao work.
        Msaa::Off,
        ScreenSpaceAmbientOcclusion::default(),
        TemporalAntiAliasing::default(),

        // Add depth prepass for post-processing
        MotionBlur{
            samples: 8,
            shutter_angle: 1.5,
            ..default()
        },
        VolumetricFog {
            ambient_intensity: 0.5,
            ..default()
        },

        EnvironmentMapLight{
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 2000.0,
            ..default()
        },

        // Add third-person camera controller
        ThirdPersonCamera::default(),

    ))
    .insert(ScreenSpaceAmbientOcclusion{
        quality_level: ScreenSpaceAmbientOcclusionQualityLevel::High,
        constant_object_thickness: 4.0,
    })
    .insert(Skybox{
            image: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            brightness: 1000.0,
            ..default()
    });
}


// Third-person camera controller
pub fn third_person_camera(
    primary_window: Query<&Window, With<PrimaryWindow>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut mouse_wheel: EventReader<MouseWheel>,
    keyboard: Res<ButtonInput<KeyCode>>,
    gamepads: Query<&Gamepad>,
    player_query: Query<&Transform, (With<Player>, Without<ThirdPersonCamera>)>,
    mut camera_query: Query<(&mut Transform, &mut ThirdPersonCamera)>,
    time: Res<Time>,
    mut exit: EventWriter<AppExit>,
) {
    // Handle ESC key to exit the game
    if keyboard.just_pressed(KeyCode::Escape) {
        exit.send(AppExit::default());
    }

    // Only update if we have a player and a camera
    if let (Ok(player_transform), Ok((mut camera_transform, mut camera_params))) =
        (player_query.get_single(), camera_query.get_single_mut()) {

        // Handle mouse input for camera rotation
        let window = primary_window.single();
        let window_focused = window.focused;

        if window_focused {
            // Update camera rotation based on mouse movement
            for event in mouse_motion.read() {
                // Apply inversion if configured
                let dx = if camera_params.invert_x { -event.delta.x } else { event.delta.x };
                let dy = if camera_params.invert_y { -event.delta.y } else { event.delta.y };

                // Apply rotation speed
                camera_params.yaw -= dx * camera_params.rotation_speed;
                camera_params.pitch += dy * camera_params.rotation_speed;

                // Clamp pitch to prevent flipping (limit how far up/down the camera can look)
                camera_params.pitch = camera_params.pitch.clamp(0.5, 1.4);
            }

            // Handle zoom with mouse wheel
            for event in mouse_wheel.read() {
                camera_params.distance -= event.y * camera_params.zoom_speed;
                // Clamp distance to reasonable values
                camera_params.distance = camera_params.distance.clamp(2.0, 15.0);
            }
        }

        // GAMEPAD CAMERA CONTROL
        // Check for any connected gamepads
        for gamepad in gamepads.iter() {
            // Use right stick for camera rotation
            if let (Some(right_stick_x), Some(right_stick_y)) = (
                gamepad.get(GamepadAxis::RightStickX),
                gamepad.get(GamepadAxis::RightStickY),
            ) {
                // Only apply rotation if stick is being moved (add deadzone)
                if right_stick_x.abs() > 0.1 || right_stick_y.abs() > 0.1 {
                    // Convert gamepad input to camera rotation
                    // Adjust these multipliers to get the right sensitivity
                    let gamepad_sensitivity = 0.05; // Adjust as needed

                    let inverted_stick_y = -right_stick_y;

                    // Apply inversion if configured
                    let dx = if camera_params.invert_x { -right_stick_x } else { right_stick_x };
                    let dy = if camera_params.invert_y { -inverted_stick_y } else { inverted_stick_y };

                    // Apply rotation with time-based smoothing
                    camera_params.yaw -= dx * gamepad_sensitivity * time.delta_secs() * 60.0;
                    camera_params.pitch += dy * gamepad_sensitivity * time.delta_secs() * 60.0;

                    // Clamp pitch to prevent flipping
                    camera_params.pitch = camera_params.pitch.clamp(0.5, 1.4);
                }
            }

            // Clamp distance to reasonable values
            camera_params.distance = camera_params.distance.clamp(1.0, 5.0);
        }

        // Get player position as the center point
        let player_pos = player_transform.translation;

        // Create rotation quaternions from euler angles
        let pitch_rot = Quat::from_rotation_x(camera_params.pitch);
        let yaw_rot = Quat::from_rotation_y(camera_params.yaw);
        let camera_rotation = yaw_rot * pitch_rot;

        // Calculate the orbital camera position
        let camera_offset = camera_rotation * Vec3::new(
            0.0,
            camera_params.height_offset,
            camera_params.distance // Positive distance is behind in orbital coordinates
        );

        // The camera should be positioned behind the player
        let target_position = player_pos - camera_offset;

        // Calculate the focus point (where the camera should look)
        // Look at the player position with a slight height offset but don't use target_offset
        let focus_pos = player_pos + Vec3::new(0.0, camera_params.height_offset * 0.5, 0.0);

        // Apply smoothing for camera movement (creates a more natural following effect)
        let mut position = camera_transform.translation;
        position.smooth_nudge(
            &target_position,
            camera_params.smoothness,
            time.delta_secs()
        );
        camera_transform.translation = position;

        // Make camera look at the focus point
        camera_transform.look_at(focus_pos, Vec3::Y);
    }
}

pub fn camera_collision_detection(
    player_query: Query<(Entity, &Transform), (With<Player>, Without<ThirdPersonCamera>)>,
    mut camera_query: Query<(&mut Transform, &ThirdPersonCamera), Without<Player>>,
    spatial_query: SpatialQuery,
) {
    // Get player and camera data
    let Ok((player_entity, player_transform)) = player_query.get_single() else { return };
    let Ok((mut camera_transform, camera_params)) = camera_query.get_single_mut() else { return };

    // Player position (with a slight vertical offset to match eye level)
    let player_position = player_transform.translation + Vec3::Y * camera_params.height_offset * 0.5;

    // Current camera position
    let current_camera_position = camera_transform.translation;

    // Direction from player to camera
    let direction = (current_camera_position - player_position).normalize();
    let dir3 = match Dir3::new(direction) {
        Ok(d) => d,
        Err(_) => return, // Invalid direction, skip collision check
    };

    // Create a shape for the camera collision
    let camera_shape = Collider::sphere(0.3);

    // Create a filter that excludes the player entity
    let filter = SpatialQueryFilter::default().with_excluded_entities([player_entity]);

    // Configure the shape cast
    let config = ShapeCastConfig {
        max_distance: camera_params.distance * 1.2, // Max distance slightly longer than camera distance
        target_distance: camera_params.distance,    // Preferred distance
        compute_contact_on_penetration: true,       // Compute contact info when penetrating
        ignore_origin_penetration: true,            // Ignore if already penetrating at origin
    };

    // Perform the shape cast
    if let Some(hit) = spatial_query.cast_shape(
        &camera_shape,
        player_position,
        Quat::default(),
        dir3,
        &config,
        &filter
    ) {
        // Calculate collision distance
        let hit_distance = hit.distance;

        // Adjust distance to prevent camera clipping (subtract a small buffer)
        let collision_buffer = 0.2;
        let new_distance = (hit_distance - collision_buffer).max(camera_params.distance * 0.4);

        // Calculate new camera position
        let new_camera_position = player_position - direction * new_distance;

        // Update camera position
        camera_transform.translation = new_camera_position;
    }
}

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(AppState::InGame), spawn_camera)
            .add_systems(Update, (
                third_person_camera,
                camera_collision_detection
            ).chain().run_if(in_state(AppState::InGame)))
            .add_plugins(TemporalAntiAliasPlugin);
    }
}
