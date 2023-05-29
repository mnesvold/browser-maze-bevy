use std::{f32::consts::TAU, ops::RangeInclusive};

use bevy::prelude::*;

pub fn generate_walls(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    x_range: RangeInclusive<i32>,
    z_range: RangeInclusive<i32>,
) {
    let (x_min, x_max) = (*x_range.start(), *x_range.end());
    let (z_min, z_max) = (*z_range.start(), *z_range.end());

    let corner_mesh = meshes.add(
        shape::UVSphere {
            radius: 0.1,
            sectors: 8,
            stacks: 8,
        }
        .into(),
    );
    let corner_material = materials.add(Color::BLUE.into());

    let wall_mesh = meshes.add(shape::Box::new(0.7, 0.2, 0.2).into());
    let wall_material = materials.add(Color::BLUE.into());

    for x in x_min..=x_max {
        for z in z_min..=z_max {
            commands.spawn(PbrBundle {
                mesh: corner_mesh.clone(),
                material: corner_material.clone(),
                transform: Transform::from_xyz(x as _, 0.0, z as _),
                ..default()
            });

            if x < x_max {
                commands.spawn(PbrBundle {
                    mesh: wall_mesh.clone(),
                    material: wall_material.clone(),
                    transform: Transform::from_xyz(x as f32 + 0.5, 0.0, z as _),
                    ..default()
                });
            }

            if z < z_max {
                commands.spawn(PbrBundle {
                    mesh: wall_mesh.clone(),
                    material: wall_material.clone(),
                    transform: Transform::from_xyz(x as _, 0.0, z as f32 + 0.5)
                        .with_rotation(Quat::from_rotation_y(TAU / 4.0)),
                    ..default()
                });
            }
        }
    }
}
