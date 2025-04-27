use bevy::prelude::*;
use crate::game_states::AppState;

pub struct MenuPlugin;
impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, setup)
            // This system runs when we enter `AppState::Menu`, during the `StateTransition` schedule.
            // All systems from the exit schedule of the state we're leaving are run first,
            // and then all systems from the enter schedule of the state we're entering are run second.
            .add_systems(OnEnter(AppState::Menu), setup_menu)
            // By contrast, update systems are stored in the `Update` schedule. They simply
            // check the value of the `State<T>` resource to see if they should run each frame.
            .add_systems(Update, menu.run_if(in_state(AppState::Menu)))
            .add_systems(OnExit(AppState::Menu), cleanup_menu)
            .init_state::<AppState>(); // Alternatively we could use .insert_state(AppState::Menu)
    }
}


#[derive(Resource)]
struct MenuData {
    button_entity: Entity,
}

#[derive(Resource)]
struct MenuCamera {
    camera_entity: Entity,
}

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

pub fn setup(mut commands: Commands) {
    let camera_entity = commands.spawn(Camera2d).id();
    commands.insert_resource(MenuCamera {camera_entity});
}

fn setup_menu(mut commands: Commands) {
    let button_entity = commands
        .spawn(Node {
            // center button
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn((
                    Button,
                    Node {
                        width: Val::Px(150.),
                        height: Val::Px(65.),
                        // horizontally center child text
                        justify_content: JustifyContent::Center,
                        // vertically center child text
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(NORMAL_BUTTON),
                ))
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Play"),
                        TextFont {
                            font_size: 33.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    ));
                });
        })
        .id();
    commands.insert_resource(MenuData { button_entity });
}

fn menu(
    mut next_state: ResMut<NextState<AppState>>,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = PRESSED_BUTTON.into();
                next_state.set(AppState::InGame);
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
            }
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
            }
        }
    }
}

fn cleanup_menu(mut commands: Commands, menu_data: Res<MenuData>, camera_data: Res<MenuCamera>) {
    commands.entity(menu_data.button_entity).despawn();
    commands.entity(camera_data.camera_entity).despawn();
}
