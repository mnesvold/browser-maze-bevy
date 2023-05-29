use std::f32::consts::TAU;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
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
        walk_speed: 1.0 / 3.0,
        walking: 0.0,
        pre_transform: Transform::IDENTITY,
        translation: Vec3::ZERO,
        facing: 0.0,
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

    // Floor
    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Plane::from_size(50.0).into()),
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
    let walking = if keyboard.any_pressed(WALK_FORWARD) {
        1.0
    } else {
        0.0
    } + if keyboard.any_pressed(WALK_BACKWARD) {
        -1.0
    } else {
        0.0
    };
    for mut avatar in &mut avatars {
        avatar.walking = walking;
    }
}

fn move_avatars(mut query: Query<(&mut Transform, &mut Avatar)>, time: Res<Time>) {
    let delta_time = time.delta_seconds();
    for (mut transform, mut avatar) in &mut query {
        let unit_step = Quat::from_rotation_y(avatar.facing) * Vec3::Z;
        let step = unit_step * avatar.walk_speed * avatar.walking * delta_time;
        avatar.translation += step;
        transform
            .set_if_neq(Transform::from_translation(avatar.translation) * avatar.pre_transform);
    }
}
