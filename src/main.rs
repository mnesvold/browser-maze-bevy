use std::f32::consts::TAU;

use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    render::camera::ScalingMode,
    window::{close_on_esc, CursorGrabMode},
};

mod maze;

use maze::{generate_walls, Sizes};

/// How many rooms per half-side of the maze?
const SIDE_HALFLENGTH: i32 = 10;

/// How big is each room?
const ROOM_SIDE_LENGTH: f32 = 2.0;

const MOUSE_SENSITIVITY: f32 = 0.5;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(close_on_esc)
        .add_system(map_user_input)
        .add_system(move_avatars)
        .add_system(switch_camera)
        .run();
}

#[derive(Copy, Clone, Component)]
pub struct Avatar {
    /// Potential speed (units/sec).
    walk_speed: f32,
    /// Current speed as a multiple of `walk_speed`.
    walking: f32,
    /// Potential turn speed (radians/sec).
    turn_speed: f32,
    /// Current turn speed as a multiple of `turn_speed`.
    turning: f32,
    /// Object-specific transform to apply before `transform`. Not modified
    /// by any `Avatar`-wide systems.
    pre_transform: Transform,
    /// `Avatar`-scoped translation from the origin.
    translation: Vec3,
    /// `Avatar`-scoped rotation (radians ccw from looking in the positive `Z` direction).
    facing: f32,
}

#[derive(Copy, Clone, Default, Component)]
pub struct AvatarPitch {
    /// `Avatar`-scoped rotation (radians below horizon).
    pitch: f32,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
enum ViewMode {
    FirstPerson,
    Map,
}

#[derive(Component)]
struct RestrictToView(ViewMode);

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Resource)]
struct CurrentView(ViewMode);

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Resource)]
struct MouseGrabbed(bool);

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Player
    let avatar = Avatar {
        walk_speed: ROOM_SIDE_LENGTH * 1.3,
        walking: 0.0,
        turn_speed: TAU / 4.0,
        turning: 0.0,
        pre_transform: Transform::IDENTITY,
        translation: Vec3::new(
            (-SIDE_HALFLENGTH as f32 + 0.5) * ROOM_SIDE_LENGTH,
            0.0,
            (-SIDE_HALFLENGTH as f32 + 0.5) * ROOM_SIDE_LENGTH,
        ),
        facing: TAU * 1. / 8.,
    };
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(shape::RegularPolygon::new(0.5, 3).into()),
            material: materials.add(Color::BLUE.into()),
            ..default()
        },
        Avatar {
            pre_transform: Transform::from_rotation(
                Quat::from_rotation_y(TAU / 6.0) * Quat::from_rotation_x(-TAU / 4.0),
            )
            .with_translation(Vec3::Y * 0.1),
            ..avatar
        },
    ));
    commands.spawn((
        PointLightBundle {
            point_light: PointLight {
                intensity: 450.0,
                ..default()
            },
            transform: Transform::from_xyz(0.0, 1.0, 0.0),
            ..default()
        },
        Avatar {
            pre_transform: Transform::from_xyz(0.0, 1.0, 0.0),
            ..avatar
        },
    ));

    // Walls
    generate_walls(
        &mut commands,
        &mut meshes,
        &mut materials,
        -SIDE_HALFLENGTH..=SIDE_HALFLENGTH,
        -SIDE_HALFLENGTH..=SIDE_HALFLENGTH,
        0xaaaaaaaa,
        &Sizes {
            room_side_length: ROOM_SIDE_LENGTH,
            wall_radius: 0.1,
        },
    );

    // Floor
    commands.spawn(PbrBundle {
        mesh: meshes
            .add(shape::Plane::from_size(SIDE_HALFLENGTH as f32 * 2.0 * ROOM_SIDE_LENGTH).into()),
        material: materials.add(Color::SILVER.into()),
        ..default()
    });

    // Cameras
    commands.spawn((
        RestrictToView(ViewMode::FirstPerson),
        Camera3dBundle::default(),
        Avatar {
            pre_transform: Transform::from_xyz(0.0, 0.5, 0.0).looking_to(Vec3::Z, Vec3::Y),
            ..avatar
        },
        AvatarPitch::default(),
    ));
    commands.spawn((
        RestrictToView(ViewMode::Map),
        Camera3dBundle {
            projection: Projection::Orthographic(OrthographicProjection {
                scaling_mode: ScalingMode::AutoMin {
                    min_width: SIDE_HALFLENGTH as f32 * 2.0 * ROOM_SIDE_LENGTH,
                    min_height: SIDE_HALFLENGTH as f32 * 2.0 * ROOM_SIDE_LENGTH,
                },
                scale: 1.0,
                ..default()
            }),
            transform: Transform::from_xyz(0.0, 10.0, 0.0).looking_at(Vec3::ZERO, Vec3::X),
            ..default()
        },
    ));

    // UI settings
    commands.insert_resource(CurrentView(ViewMode::FirstPerson));
    commands.insert_resource(MouseGrabbed(false));
}

fn map_user_input(
    keyboard: Res<Input<KeyCode>>,
    mouse: Res<Input<MouseButton>>,
    mut motion: EventReader<MouseMotion>,
    mut avatars: Query<(&mut Avatar, Option<&mut AvatarPitch>)>,
    mut windows: Query<&mut Window>,
    mut view: ResMut<CurrentView>,
    mut grabbed: ResMut<MouseGrabbed>,
) {
    view.set_if_neq(CurrentView(if keyboard.pressed(KeyCode::Tab) {
        ViewMode::Map
    } else {
        ViewMode::FirstPerson
    }));
    if mouse.just_pressed(MouseButton::Left) {
        for mut window in &mut windows {
            let grab = window.cursor.grab_mode != CursorGrabMode::Locked;
            window.cursor.grab_mode = if grab {
                CursorGrabMode::Locked
            } else {
                CursorGrabMode::None
            };
            window.cursor.visible = !grab;
            grabbed.set_if_neq(MouseGrabbed(grab));
        }
    }

    const WALK_FORWARD: [KeyCode; 3] = [KeyCode::W, KeyCode::Up, KeyCode::Comma];
    const WALK_BACKWARD: [KeyCode; 3] = [KeyCode::S, KeyCode::Down, KeyCode::O];
    const TURN_LEFT: [KeyCode; 2] = [KeyCode::A, KeyCode::Left];
    const TURN_RIGHT: [KeyCode; 3] = [KeyCode::D, KeyCode::Right, KeyCode::E];
    let walking = if keyboard.any_pressed(WALK_FORWARD) {
        1.0
    } else {
        0.0
    } + if keyboard.any_pressed(WALK_BACKWARD) {
        -1.0
    } else {
        0.0
    };
    let turning = if keyboard.any_pressed(TURN_LEFT) {
        1.0
    } else {
        0.0
    } + if keyboard.any_pressed(TURN_RIGHT) {
        -1.0
    } else {
        0.0
    };
    let mut mouse_turn = 0.0;
    let mut mouse_pitch = 0.0;
    if grabbed.0 {
        for event in motion.iter() {
            mouse_turn -= event.delta.x;
            mouse_pitch += event.delta.y;
        }
    } else {
        motion.clear();
    }
    for (mut avatar, pitch) in &mut avatars {
        avatar.walking = walking;
        avatar.turning = turning + (mouse_turn * MOUSE_SENSITIVITY / avatar.turn_speed);
        if let Some(mut pitch) = pitch {
            pitch.pitch = (pitch.pitch + (mouse_pitch * 0.001)).clamp(-TAU / 4.0, TAU / 8.0);
        }
    }
}

fn move_avatars(
    mut query: Query<(&mut Transform, &mut Avatar, Option<&AvatarPitch>)>,
    time: Res<Time>,
) {
    let delta_time = time.delta_seconds();
    for (mut transform, mut avatar, pitch) in &mut query {
        avatar.facing += avatar.turning * avatar.turn_speed * delta_time;
        let unit_step = Quat::from_rotation_y(avatar.facing) * Vec3::Z;
        let step = unit_step * avatar.walk_speed * avatar.walking * delta_time;
        avatar.translation += step;
        let pitch_transform = pitch
            .map(|p| Quat::from_rotation_x(p.pitch))
            .unwrap_or(Quat::IDENTITY);
        let avatar_transform = Transform::from_translation(avatar.translation)
            .with_rotation(Quat::from_rotation_y(avatar.facing) * pitch_transform);
        transform.set_if_neq(avatar_transform * avatar.pre_transform);
    }
}

fn switch_camera(current: Res<CurrentView>, mut cameras: Query<(&mut Camera, &RestrictToView)>) {
    if !(current.is_added() || current.is_changed()) {
        return;
    }
    for (mut camera, restriction) in &mut cameras {
        camera.is_active = restriction.0 == current.0;
    }
}
