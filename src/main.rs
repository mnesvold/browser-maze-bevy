use std::f32::consts::TAU;

use bevy::{prelude::*, window::close_on_esc};

mod maze;

use maze::generate_walls;

const SIDE_HALFLENGTH: i32 = 10;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(close_on_esc)
        .add_system(map_keyboard_input)
        .add_system(move_avatars)
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

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Player
    let avatar = Avatar {
        walk_speed: 1.0,
        walking: 0.0,
        turn_speed: TAU / 4.0,
        turning: 0.0,
        pre_transform: Transform::IDENTITY,
        translation: Vec3::new(
            -SIDE_HALFLENGTH as f32 + 0.5,
            0.0,
            -SIDE_HALFLENGTH as f32 + 0.5,
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
    );

    // Floor
    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Plane::from_size(SIDE_HALFLENGTH as f32 * 2.0).into()),
        material: materials.add(Color::SILVER.into()),
        ..default()
    });

    // Camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 6.0, 12.0).looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
        ..default()
    });
}

fn map_keyboard_input(keyboard: Res<Input<KeyCode>>, mut avatars: Query<&mut Avatar>) {
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
    for mut avatar in &mut avatars {
        avatar.walking = walking;
        avatar.turning = turning;
    }
}

fn move_avatars(mut query: Query<(&mut Transform, &mut Avatar)>, time: Res<Time>) {
    let delta_time = time.delta_seconds();
    for (mut transform, mut avatar) in &mut query {
        avatar.facing += avatar.turning * avatar.turn_speed * delta_time;
        let unit_step = Quat::from_rotation_y(avatar.facing) * Vec3::Z;
        let step = unit_step * avatar.walk_speed * avatar.walking * delta_time;
        avatar.translation += step;
        let avatar_transform = Transform::from_translation(avatar.translation)
            .with_rotation(Quat::from_rotation_y(avatar.facing));
        transform.set_if_neq(avatar_transform * avatar.pre_transform);
    }
}
