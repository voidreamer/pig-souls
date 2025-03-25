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

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(OnEnter(AppState::InGame), spawn_camera)
            .add_systems(Update, third_person_camera.run_if(in_state(AppState::InGame)))
            .add_plugins(TemporalAntiAliasPlugin);
    }
}
