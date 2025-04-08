use std::sync::Arc;

use bevy::{
    app::App,
    asset::{AssetServer, Handle},
    color::{
        palettes::css::{GRAY},
        Color,
    },
    core::Name,
    math::Vec3,
    prelude::{Commands, PluginGroup, Res, Transform},
    scene::Scene,
    utils::default,
};
use bevy::app::Plugin;
use bevy::prelude::OnEnter;
use plugin::ProcGenExamplesPlugin;
use utils::load_assets;

use bevy_ghx_proc_gen::{
    assets::ModelsAssets,
    bevy_ghx_grid::debug_plugin::{view::DebugGridView, DebugGridView3dBundle},
    debug_plugin::generation::GenerationViewMode,
    proc_gen::{
        generator::{builder::GeneratorBuilder, rules::RulesBuilder},
        ghx_grid::cartesian::{coordinates::Cartesian3D, grid::CartesianGrid},
    },
    spawner_plugin::NodesSpawner,
};

use rules::rules_and_assets;
use crate::game_states::AppState;

mod rules;
mod plugin;
mod utils;
mod anim;

pub struct ProceduralPlugin;

impl Plugin for ProceduralPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ProcGenExamplesPlugin::<Cartesian3D, Handle<Scene>>::new(
                GENERATION_VIEW_MODE,
                ASSETS_SCALE,
            ),
        ));
        app.add_systems(OnEnter(AppState::InGame), setup_generator);

    }
}

// -----------------  Configurable values ---------------------------
/// Modify this value to control the way the generation is visualized
const GENERATION_VIEW_MODE: GenerationViewMode = GenerationViewMode::Final;

/// Modify these values to control the map size.
const GRID_HEIGHT: u32 = 7;
const GRID_X: u32 = 30;
const GRID_Z: u32 = 70;
// ------------------------------------------------------------------

/// Size of a block in world units
const BLOCK_SIZE: f32 = 1.;
const NODE_SIZE: Vec3 = Vec3::splat(BLOCK_SIZE);

const ASSETS_SCALE_FACTOR: f32 = BLOCK_SIZE / 4.; // Models are 4 units wide
const ASSETS_SCALE: Vec3 = Vec3::splat(ASSETS_SCALE_FACTOR);

fn setup_generator(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Get rules from rules.rs
    let (models_asset_paths, models, socket_collection) = rules_and_assets();

    let rules = Arc::new(
        RulesBuilder::new_cartesian_3d(models, socket_collection)
            .build()
            .unwrap(),
    );
    let grid = CartesianGrid::new_cartesian_3d(GRID_X, GRID_HEIGHT, GRID_Z, false, false, false);
    let gen_builder = GeneratorBuilder::new()
        // We share the Rules between all the generators
        .with_shared_rules(rules.clone())
        .with_grid(grid.clone());

    let models_assets: ModelsAssets<Handle<Scene>> =
        load_assets(&asset_server, models_asset_paths, "pillars", "glb#Scene0");
    let node_spawner = NodesSpawner::new(
        models_assets,
        NODE_SIZE,
        // We spawn assets with a scale of 0 since we animate their scale in the examples
        Vec3::ZERO,
    );

    for i in 0..=1 {
        let mut gen_builder = gen_builder.clone();
        let observer = gen_builder.add_queued_observer();
        let generator = gen_builder.build().unwrap();

        commands.spawn((
            Name::new(format!("Grid n°{}", i)),
            Transform::from_translation(Vec3 {
                x: (grid.size_x() as f32) * (i as f32 - 1.),
                y: 0.,
                z: -(grid.size_z() as f32) * 0.5,
            }),
            grid.clone(),
            generator,
            observer,
            // We also share the ModelsAssets between all the generators
            node_spawner.clone(),
            DebugGridView3dBundle {
                view: DebugGridView::new(false, true, Color::Srgba(GRAY), NODE_SIZE),
                ..default()
            },
        ));
    }
}