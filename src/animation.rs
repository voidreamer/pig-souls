use std::{f32::consts::PI, time::Duration};

use bevy::{
    animation::{AnimationTargetId, RepeatAnimation},
    color::palettes::css::WHITE,
    pbr::CascadeShadowConfigBuilder,
    prelude::*,
};
use bevy::animation::AnimationTarget;
use bevy::color::palettes::css::LIGHT_GRAY;
use bevy::utils::HashSet;
use bevy_hanabi::EffectAsset;
use rand::{thread_rng, Rng};
use crate::game_states::AppState;

const FOX_PATH: &str = "models/animated/Fox.glb";

pub struct AnimationTestPlugin;

impl Plugin for AnimationTestPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<ParticleAssets>()
            .init_resource::<FoxFeetTargets>()
            .init_resource::<FoxAppState>()
            .insert_resource(AmbientLight {
                color: Color::WHITE,
                brightness: 2000.,
            })
                .add_systems(OnEnter(AppState::InGame), (setup, setup_ui))
                //.add_systems(Update, setup_scene_once_loaded.run_if(in_state(AppState::InGame)))
                .add_systems(Update, (handle_button_toggles, update_ui).run_if(in_state(AppState::InGame)))
                .add_systems(Update, setup_animation_graph_once_loaded.run_if(in_state(AppState::InGame)))
                .add_systems(Update, simulate_particles.run_if(in_state(AppState::InGame)))
                .add_systems(Update, keyboard_animation_control.run_if(in_state(AppState::InGame)));
    }
}
// IDs of the mask groups we define for the running fox model.
//
// Each mask group defines a set of bones for which animations can be toggled on
// and off.
const MASK_GROUP_HEAD: u32 = 0;
const MASK_GROUP_LEFT_FRONT_LEG: u32 = 1;
const MASK_GROUP_RIGHT_FRONT_LEG: u32 = 2;
const MASK_GROUP_LEFT_HIND_LEG: u32 = 3;
const MASK_GROUP_RIGHT_HIND_LEG: u32 = 4;
const MASK_GROUP_TAIL: u32 = 5;

// The width in pixels of the small buttons that allow the user to toggle a mask
// group on or off.
const MASK_GROUP_BUTTON_WIDTH: f32 = 250.0;

// The names of the bones that each mask group consists of. Each mask group is
// defined as a (prefix, suffix) tuple. The mask group consists of a single
// bone chain rooted at the prefix. For example, if the chain's prefix is
// "A/B/C" and the suffix is "D/E", then the bones that will be included in the
// mask group are "A/B/C", "A/B/C/D", and "A/B/C/D/E".
//
// The fact that our mask groups are single chains of bones isn't an engine
// requirement; it just so happens to be the case for the model we're using. A
// mask group can consist of any set of animation targets, regardless of whether
// they form a single chain.
const MASK_GROUP_PATHS: [(&str, &str); 6] = [
    // Head
    (
        "root/_rootJoint/b_Root_00/b_Hip_01/b_Spine01_02/b_Spine02_03",
        "b_Neck_04/b_Head_05",
    ),
    // Left front leg
    (
        "root/_rootJoint/b_Root_00/b_Hip_01/b_Spine01_02/b_Spine02_03/b_LeftUpperArm_09",
        "b_LeftForeArm_010/b_LeftHand_011",
    ),
    // Right front leg
    (
        "root/_rootJoint/b_Root_00/b_Hip_01/b_Spine01_02/b_Spine02_03/b_RightUpperArm_06",
        "b_RightForeArm_07/b_RightHand_08",
    ),
    // Left hind leg
    (
        "root/_rootJoint/b_Root_00/b_Hip_01/b_LeftLeg01_015",
        "b_LeftLeg02_016/b_LeftFoot01_017/b_LeftFoot02_018",
    ),
    // Right hind leg
    (
        "root/_rootJoint/b_Root_00/b_Hip_01/b_RightLeg01_019",
        "b_RightLeg02_020/b_RightFoot01_021/b_RightFoot02_022",
    ),
    // Tail
    (
        "root/_rootJoint/b_Root_00/b_Hip_01/b_Tail01_012",
        "b_Tail02_013/b_Tail03_014",
    ),
];

#[derive(Clone, Copy, Component)]
struct AnimationControl {
    // The ID of the mask group that this button controls.
    group_id: u32,
    label: AnimationLabel,
}

#[derive(Clone, Copy, Component, PartialEq, Debug)]
enum AnimationLabel {
    Idle = 0,
    Walk = 1,
    Run = 2,
    Off = 3,
}

#[derive(Clone, Debug, Resource)]
struct AnimationNodes([AnimationNodeIndex; 3]);

#[derive(Clone, Copy, Debug, Resource,Default)]
struct FoxAppState([MaskGroupState; 6]);

#[derive(Clone, Copy, Debug, Default)]
struct MaskGroupState {
    clip: u8,
}


#[derive(Resource, Default)]
struct Animations {
    animations: Vec<AnimationNodeIndex>,
    graph: Handle<AnimationGraph>,
}

#[derive(Event, Reflect, Clone)]
struct OnStep;

fn observe_on_step(
    trigger: Trigger<OnStep>,
    particle: Res<ParticleAssets>,
    mut commands: Commands,
    transforms: Query<&GlobalTransform>,
    mut effects: ResMut<Assets<EffectAsset>>,
    asset_server: Res<AssetServer>,
) {
    crate::fx::spawn_explosion(&mut commands, &asset_server, &mut effects);
    let translation = transforms.get(trigger.entity()).unwrap().translation();
    let mut rng = thread_rng();
    // Spawn a bunch of particles.
    for _ in 0..14 {
        let horizontal= rng.gen_range(0.0..4.0);
        let vertical = rng.gen_range(0.0..4.0);
        let size = rng.gen_range(0.2..1.0);
        commands.queue(spawn_particle(
            particle.mesh.clone(),
            particle.material.clone(),
            translation.reject_from_normalized(Vec3::Y),
            rng.gen_range(0.2..0.6),
            size,
            Vec3::new(horizontal, vertical, horizontal) * 10.0,
        ));
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    commands.init_resource::<Animations>();
    commands.init_resource::<FoxAppState>();
    commands.add_observer(observe_on_step);

    // Build the animation graph
    let (graph, node_indices) = AnimationGraph::from_clips([
        asset_server.load(GltfAssetLabel::Animation(2).from_asset(FOX_PATH)),
        asset_server.load(GltfAssetLabel::Animation(1).from_asset(FOX_PATH)),
        asset_server.load(GltfAssetLabel::Animation(0).from_asset(FOX_PATH)),
    ]);

    // Insert a resource with the current scene information
    let graph_handle = graphs.add(graph);
    commands.insert_resource(Animations {
        animations: node_indices,
        graph: graph_handle,
    });

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(100.0, 100.0, 150.0).looking_at(Vec3::new(0.0, 20.0, 0.0), Vec3::Y),
    ));

    // Plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(500000.0, 500000.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
    ));

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

    // Fox
    commands.spawn(SceneRoot(
        asset_server.load(GltfAssetLabel::Scene(0).from_asset(FOX_PATH)),
    ));

    println!("Animation controls:");
    println!("  - spacebar: play / pause");
    println!("  - arrow up / down: speed up / slow down animation playback");
    println!("  - arrow left / right: seek backward / forward");
    println!("  - digit 1 / 3 / 5: play the animation <digit> times");
    println!("  - L: loop the animation forever");
    println!("  - return: change animation");
}

fn get_clip<'a>(
    node: AnimationNodeIndex,
    graph: &AnimationGraph,
    clips: &'a mut Assets<AnimationClip>,
) -> &'a mut AnimationClip {
    let node = graph.get(node).unwrap();
    let clip = match &node.node_type {
        AnimationNodeType::Clip(handle) => clips.get_mut(handle),
        _ => unreachable!(),
    };
    clip.unwrap()
}

fn setup_animation_graph_once_loaded(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut clips: ResMut<Assets<AnimationClip>>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
    mut players: Query<(Entity, &mut AnimationPlayer), Added<AnimationPlayer>>,
    targets: Query<(Entity, &AnimationTarget)>,
    feet: Res<FoxFeetTargets>,
) {

    for (entity, mut player) in &mut players {
        // Load the animation clip from the glTF file.
        let mut animation_graph = AnimationGraph::new();
        let blend_node = animation_graph.add_additive_blend(1.0, animation_graph.root);

        let animation_graph_nodes: [AnimationNodeIndex; 3] =
            std::array::from_fn(|animation_index| {
                let handle = asset_server.load(
                    GltfAssetLabel::Animation(animation_index)
                        .from_asset("models/animated/Fox.glb"),
                );
                let mask = if animation_index == 0 { 0 } else { 0x3f };
                animation_graph.add_clip_with_mask(handle, mask, 1.0, blend_node)
            });

        // Create each mask group.
        let mut all_animation_target_ids = HashSet::new();
        for (mask_group_index, (mask_group_prefix, mask_group_suffix)) in
            MASK_GROUP_PATHS.iter().enumerate()
        {
            // Split up the prefix and suffix, and convert them into `Name`s.
            let prefix: Vec<_> = mask_group_prefix.split('/').map(Name::new).collect();
            let suffix: Vec<_> = mask_group_suffix.split('/').map(Name::new).collect();

            // Add each bone in the chain to the appropriate mask group.
            for chain_length in 0..=suffix.len() {
                let animation_target_id = AnimationTargetId::from_names(
                    prefix.iter().chain(suffix[0..chain_length].iter()),
                );
                animation_graph
                    .add_target_to_mask_group(animation_target_id, mask_group_index as u32);
                all_animation_target_ids.insert(animation_target_id);
            }
        }

        // We're doing constructing the animation graph. Add it as an asset.
        let animation_graph2 = animation_graphs.add(animation_graph.clone());
        commands
            .entity(entity)
            .insert(AnimationGraphHandle(animation_graph2));

        // Remove animation targets that aren't in any of the mask groups. If we
        // don't do that, those bones will play all animations at once, which is
        // ugly.
        for (target_entity, target) in &targets {
            if !all_animation_target_ids.contains(&target.id) {
                commands.entity(target_entity).remove::<AnimationTarget>();
            }
        }

        // Play the animation.
        for animation_graph_node in animation_graph_nodes {
            player.play(animation_graph_node).repeat();

            // probably there is a better way than to do this on a loop all the time
            let anim_clip = get_clip(animation_graph_node, &animation_graph, &mut clips);
            anim_clip.add_event_to_target(feet.front_left, 0.625, OnStep);
            anim_clip.add_event_to_target(feet.front_right, 0.5, OnStep);
            anim_clip.add_event_to_target(feet.back_left, 0.0, OnStep);
            anim_clip.add_event_to_target(feet.back_right, 0.125, OnStep);
        }

        // Record the graph nodes.
        commands.insert_resource(AnimationNodes(animation_graph_nodes));
    }
}

// An `AnimationPlayer` is automatically added to the scene when it's ready.
// When the player is added, start the animation.
fn setup_scene_once_loaded(
    mut commands: Commands,
    animations: Res<Animations>,
    feet: Res<FoxFeetTargets>,
    graphs: Res<Assets<AnimationGraph>>,
    mut clips: ResMut<Assets<AnimationClip>>,
    mut players: Query<(Entity, &mut AnimationPlayer), Added<AnimationPlayer>>,
) {

    for (entity, mut player) in &mut players {
        let graph = graphs.get(&animations.graph).unwrap();

        // Send `OnStep` events once the fox feet hits the ground in the running animation.
        let running_animation = get_clip(animations.animations[0], graph, &mut clips);
        // You can determine the time an event should trigger if you know witch frame it occurs and
        // the frame rate of the animation. Let's say we want to trigger an event at frame 15,
        // and the animation has a frame rate of 24 fps, then time = 15 / 24 = 0.625.
        running_animation.add_event_to_target(feet.front_left, 0.625, OnStep);
        running_animation.add_event_to_target(feet.front_right, 0.5, OnStep);
        running_animation.add_event_to_target(feet.back_left, 0.0, OnStep);
        running_animation.add_event_to_target(feet.back_right, 0.125, OnStep);

        let mut transitions = AnimationTransitions::new();

        // Make sure to start the animation via the `AnimationTransitions`
        // component. The `AnimationTransitions` component wants to manage all
        // the animations and will get confused if the animations are started
        // directly via the `AnimationPlayer`.
        transitions
            .play(&mut player, animations.animations[0], Duration::ZERO)
            .repeat();

        commands
            .entity(entity)
            .insert(AnimationGraphHandle(animations.graph.clone()))
            .insert(transitions);
    }
}
// Adds a button that allows the user to toggle a mask group on and off.
//
// The button will automatically become a child of the parent that owns the
// given `ChildBuilder`.
fn add_mask_group_control(parent: &mut ChildBuilder, label: &str, width: Val, mask_group_id: u32) {
    let button_text_style = (
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor::WHITE,
    );
    let selected_button_text_style = (button_text_style.0.clone(), TextColor::BLACK);
    let label_text_style = (
        button_text_style.0.clone(),
        TextColor(Color::Srgba(LIGHT_GRAY)),
    );

    parent
        .spawn((
            Node {
                border: UiRect::all(Val::Px(1.0)),
                width,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::ZERO,
                margin: UiRect::ZERO,
                ..default()
            },
            BorderColor(Color::WHITE),
            BorderRadius::all(Val::Px(3.0)),
            BackgroundColor(Color::BLACK),
        ))
        .with_children(|builder| {
            builder
                .spawn((
                    Node {
                        border: UiRect::ZERO,
                        width: Val::Percent(100.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        padding: UiRect::ZERO,
                        margin: UiRect::ZERO,
                        ..default()
                    },
                    BackgroundColor(Color::BLACK),
                ))
                .with_child((
                    Text::new(label),
                    label_text_style.clone(),
                    Node {
                        margin: UiRect::vertical(Val::Px(3.0)),
                        ..default()
                    },
                ));

            builder
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::top(Val::Px(1.0)),
                        ..default()
                    },
                    BorderColor(Color::WHITE),
                ))
                .with_children(|builder| {
                    for (index, label) in [
                        AnimationLabel::Run,
                        AnimationLabel::Walk,
                        AnimationLabel::Idle,
                        AnimationLabel::Off,
                    ]
                        .iter()
                        .enumerate()
                    {
                        builder
                            .spawn((
                                Button,
                                BackgroundColor(if index > 0 {
                                    Color::BLACK
                                } else {
                                    Color::WHITE
                                }),
                                Node {
                                    flex_grow: 1.0,
                                    border: if index > 0 {
                                        UiRect::left(Val::Px(1.0))
                                    } else {
                                        UiRect::ZERO
                                    },
                                    ..default()
                                },
                                BorderColor(Color::WHITE),
                                AnimationControl {
                                    group_id: mask_group_id,
                                    label: *label,
                                },
                            ))
                            .with_child((
                                Text(format!("{:?}", label)),
                                if index > 0 {
                                    button_text_style.clone()
                                } else {
                                    selected_button_text_style.clone()
                                },
                                TextLayout::new_with_justify(JustifyText::Center),
                                Node {
                                    flex_grow: 1.0,
                                    margin: UiRect::vertical(Val::Px(3.0)),
                                    ..default()
                                },
                            ));
                    }
                });
        });
}
fn setup_ui(mut commands: Commands) {
    // Add help text.
    commands.spawn((
        Text::new("Click on a button to toggle animations for its associated bones"),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(12.0),
            top: Val::Px(12.0),
            ..default()
        },
    ));

    // Add the buttons that allow the user to toggle mask groups on and off.
    commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            position_type: PositionType::Absolute,
            row_gap: Val::Px(6.0),
            left: Val::Px(12.0),
            bottom: Val::Px(12.0),
            ..default()
        })
        .with_children(|parent| {
            let row_node = Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(6.0),
                ..default()
            };

            add_mask_group_control(parent, "Head", Val::Auto, MASK_GROUP_HEAD);

            parent.spawn(row_node.clone()).with_children(|parent| {
                add_mask_group_control(
                    parent,
                    "Left Front Leg",
                    Val::Px(MASK_GROUP_BUTTON_WIDTH),
                    MASK_GROUP_LEFT_FRONT_LEG,
                );
                add_mask_group_control(
                    parent,
                    "Right Front Leg",
                    Val::Px(MASK_GROUP_BUTTON_WIDTH),
                    MASK_GROUP_RIGHT_FRONT_LEG,
                );
            });

            parent.spawn(row_node).with_children(|parent| {
                add_mask_group_control(
                    parent,
                    "Left Hind Leg",
                    Val::Px(MASK_GROUP_BUTTON_WIDTH),
                    MASK_GROUP_LEFT_HIND_LEG,
                );
                add_mask_group_control(
                    parent,
                    "Right Hind Leg",
                    Val::Px(MASK_GROUP_BUTTON_WIDTH),
                    MASK_GROUP_RIGHT_HIND_LEG,
                );
            });

            add_mask_group_control(parent, "Tail", Val::Auto, MASK_GROUP_TAIL);
        });
}

// A system that handles requests from the user to toggle mask groups on and
// off.
fn handle_button_toggles(
    mut interactions: Query<(&Interaction, &mut AnimationControl), Changed<Interaction>>,
    mut animation_players: Query<&AnimationGraphHandle, With<AnimationPlayer>>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
    mut animation_nodes: Option<ResMut<AnimationNodes>>,
    mut app_state: ResMut<FoxAppState>,
) {
    let Some(ref mut animation_nodes) = animation_nodes else {
        return;
    };

    for (interaction, animation_control) in interactions.iter_mut() {
        // We only care about press events.
        if *interaction != Interaction::Pressed {
            continue;
        }

        // Toggle the state of the clip.
        app_state.0[animation_control.group_id as usize].clip = animation_control.label as u8;

        // Now grab the animation player. (There's only one in our case, but we
        // iterate just for clarity's sake.)
        for animation_graph_handle in animation_players.iter_mut() {
            // The animation graph needs to have loaded.
            let Some(animation_graph) = animation_graphs.get_mut(animation_graph_handle) else {
                continue;
            };

            for (clip_index, &animation_node_index) in animation_nodes.0.iter().enumerate() {
                let Some(animation_node) = animation_graph.get_mut(animation_node_index) else {
                    continue;
                };

                if animation_control.label as usize == clip_index {
                    animation_node.mask &= !(1 << animation_control.group_id);
                } else {
                    animation_node.mask |= 1 << animation_control.group_id;
                }
            }
        }
    }
}

// A system that updates the UI based on the current app state.
fn update_ui(
    mut animation_controls: Query<(&AnimationControl, &mut BackgroundColor, &Children)>,
    texts: Query<Entity, With<Text>>,
    mut writer: TextUiWriter,
    app_state: Res<FoxAppState>,
) {
    for (animation_control, mut background_color, kids) in animation_controls.iter_mut() {
        let enabled =
            app_state.0[animation_control.group_id as usize].clip == animation_control.label as u8;

        *background_color = if enabled {
            BackgroundColor(Color::WHITE)
        } else {
            BackgroundColor(Color::BLACK)
        };

        for &kid in kids {
            let Ok(text) = texts.get(kid) else {
                continue;
            };

            writer.for_each_color(text, |mut color| {
                color.0 = if enabled { Color::BLACK } else { Color::WHITE };
            });
        }
    }
}


fn keyboard_animation_control(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut animation_players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
    animations: Res<Animations>,
    mut current_animation: Local<usize>,
) {
    for (mut player, mut transitions) in &mut animation_players {
        let Some((&playing_animation_index, _)) = player.playing_animations().next() else {
            continue;
        };

        if keyboard_input.just_pressed(KeyCode::Space) {
            let playing_animation = player.animation_mut(playing_animation_index).unwrap();
            if playing_animation.is_paused() {
                playing_animation.resume();
            } else {
                playing_animation.pause();
            }
        }

        if keyboard_input.just_pressed(KeyCode::ArrowUp) {
            let playing_animation = player.animation_mut(playing_animation_index).unwrap();
            let speed = playing_animation.speed();
            playing_animation.set_speed(speed * 1.2);
        }

        if keyboard_input.just_pressed(KeyCode::ArrowDown) {
            let playing_animation = player.animation_mut(playing_animation_index).unwrap();
            let speed = playing_animation.speed();
            playing_animation.set_speed(speed * 0.8);
        }

        if keyboard_input.just_pressed(KeyCode::ArrowLeft) {
            let playing_animation = player.animation_mut(playing_animation_index).unwrap();
            let elapsed = playing_animation.seek_time();
            playing_animation.seek_to(elapsed - 0.1);
        }

        if keyboard_input.just_pressed(KeyCode::ArrowRight) {
            let playing_animation = player.animation_mut(playing_animation_index).unwrap();
            let elapsed = playing_animation.seek_time();
            playing_animation.seek_to(elapsed + 0.1);
        }

        if keyboard_input.just_pressed(KeyCode::Enter) {
            *current_animation = (*current_animation + 1) % animations.animations.len();

            transitions
                .play(
                    &mut player,
                    animations.animations[*current_animation],
                    Duration::from_millis(250),
                )
                .repeat();
        }

        if keyboard_input.just_pressed(KeyCode::Digit1) {
            let playing_animation = player.animation_mut(playing_animation_index).unwrap();
            playing_animation
                .set_repeat(RepeatAnimation::Count(1))
                .replay();
        }

        if keyboard_input.just_pressed(KeyCode::Digit3) {
            let playing_animation = player.animation_mut(playing_animation_index).unwrap();
            playing_animation
                .set_repeat(RepeatAnimation::Count(3))
                .replay();
        }

        if keyboard_input.just_pressed(KeyCode::Digit5) {
            let playing_animation = player.animation_mut(playing_animation_index).unwrap();
            playing_animation
                .set_repeat(RepeatAnimation::Count(5))
                .replay();
        }

        if keyboard_input.just_pressed(KeyCode::KeyL) {
            let playing_animation = player.animation_mut(playing_animation_index).unwrap();
            playing_animation.set_repeat(RepeatAnimation::Forever);
        }
    }
}

fn simulate_particles(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut Particle)>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut particle) in &mut query {
        if particle.lifeteime_timer.tick(time.delta()).just_finished() {
            commands.entity(entity).despawn();
        } else {
            transform.translation += particle.velocity * time.delta_secs();
            transform.scale =
                Vec3::splat(particle.size.lerp(0.0, particle.lifeteime_timer.fraction()));
            particle
                .velocity
                .smooth_nudge(&Vec3::ZERO, 4.0, time.delta_secs());
        }
    }
}

fn spawn_particle<M: Material>(
    mesh: Handle<Mesh>,
    material: Handle<M>,
    translation: Vec3,
    lifetime: f32,
    size: f32,
    velocity: Vec3,
) -> impl Command {
    move |world: &mut World| {
        world.spawn((
            Particle {
                lifeteime_timer: Timer::from_seconds(lifetime, TimerMode::Once),
                size,
                velocity,
            },
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform {
                translation,
                scale: Vec3::splat(size),
                ..Default::default()
            },
        ));
    }
}

#[derive(Component)]
struct Particle {
    lifeteime_timer: Timer,
    size: f32,
    velocity: Vec3,
}

#[derive(Resource)]
struct ParticleAssets {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

impl FromWorld for ParticleAssets {
    fn from_world(world: &mut World) -> Self {
        Self {
            mesh: world.resource_mut::<Assets<Mesh>>().add(Sphere::new(10.0)),
            material: world
                .resource_mut::<Assets<StandardMaterial>>()
                .add(StandardMaterial {
                    base_color: WHITE.into(),
                    ..Default::default()
                }),
        }
    }
}

#[derive(Resource)]
struct FoxFeetTargets {
    front_right: AnimationTargetId,
    front_left: AnimationTargetId,
    back_left: AnimationTargetId,
    back_right: AnimationTargetId,
}

impl Default for FoxFeetTargets {
    fn default() -> Self {
        // Get the id's of the feet and store them in a resource.
        let hip_node = ["root", "_rootJoint", "b_Root_00", "b_Hip_01"];
        let front_left_foot = hip_node.iter().chain(
            [
                "b_Spine01_02",
                "b_Spine02_03",
                "b_LeftUpperArm_09",
                "b_LeftForeArm_010",
                "b_LeftHand_011",
            ]
                .iter(),
        );
        let front_right_foot = hip_node.iter().chain(
            [
                "b_Spine01_02",
                "b_Spine02_03",
                "b_RightUpperArm_06",
                "b_RightForeArm_07",
                "b_RightHand_08",
            ]
                .iter(),
        );
        let back_left_foot = hip_node.iter().chain(
            [
                "b_LeftLeg01_015",
                "b_LeftLeg02_016",
                "b_LeftFoot01_017",
                "b_LeftFoot02_018",
            ]
                .iter(),
        );
        let back_right_foot = hip_node.iter().chain(
            [
                "b_RightLeg01_019",
                "b_RightLeg02_020",
                "b_RightFoot01_021",
                "b_RightFoot02_022",
            ]
                .iter(),
        );
        Self {
            front_left: AnimationTargetId::from_iter(front_left_foot),
            front_right: AnimationTargetId::from_iter(front_right_foot),
            back_left: AnimationTargetId::from_iter(back_left_foot),
            back_right: AnimationTargetId::from_iter(back_right_foot),
        }
    }
}