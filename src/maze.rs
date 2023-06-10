use std::{f32::consts::TAU, ops::RangeInclusive};

use bevy::{
    prelude::*,
    utils::{HashMap, HashSet},
};
use bevy_rapier3d::prelude::*;
use petgraph::{algo::floyd_warshall, graph::NodeIndex, Graph, Undirected};
use rand::{rngs::SmallRng, seq::IteratorRandom, SeedableRng};

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Room {
    // The lower bound of this room's `x` coordinates.
    pub west_edge: i32,
    // The lower bound of this room`s `z` coordinates.
    pub south_edge: i32,
}

#[derive(Copy, Clone, Debug)]
struct Wall {
    sw_corner: (i32, i32),
    orientation: WallOrientation,
    disposition: Disposition,
}

#[derive(Copy, Clone, Debug)]
enum WallOrientation {
    ParallelToX,
    ParallelToZ,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
enum Disposition {
    Present,
    Absent,
    Unknown,
}

#[derive(Debug)]
pub struct Sizes {
    pub room_side_length: f32,
    pub wall_radius: f32,
}

#[derive(Debug)]
pub struct SpawnPositions {
    pub start: Room,
    pub goal: Room,
}

pub fn generate_walls(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    x_range: RangeInclusive<i32>,
    z_range: RangeInclusive<i32>,
    seed: u64,
    sizes: &Sizes,
) -> SpawnPositions {
    let border_walls = iter_border_walls(x_range.clone(), z_range.clone());
    let graph = choose_walls(x_range.clone(), z_range.clone(), seed);
    let inner_walls = graph.edge_weights().copied();
    build_walls(
        commands,
        meshes,
        materials,
        x_range,
        z_range,
        border_walls.chain(inner_walls),
        sizes,
    );
    choose_spawn_positions(&graph)
}

fn iter_border_walls(
    x_range: RangeInclusive<i32>,
    z_range: RangeInclusive<i32>,
) -> impl Iterator<Item = Wall> {
    let (x_min, x_max) = (*x_range.start(), *x_range.end());
    let (z_min, z_max) = (*z_range.start(), *z_range.end());

    let ns_walls = (x_min..x_max).flat_map(move |x| {
        [z_min, z_max].into_iter().map(move |z| Wall {
            sw_corner: (x, z),
            orientation: WallOrientation::ParallelToX,
            disposition: Disposition::Present,
        })
    });
    let ew_walls = (z_min..z_max).flat_map(move |z| {
        [x_min, x_max].into_iter().map(move |x| Wall {
            sw_corner: (x, z),
            orientation: WallOrientation::ParallelToZ,
            disposition: Disposition::Present,
        })
    });

    ns_walls.chain(ew_walls)
}

fn choose_walls(
    x_range: RangeInclusive<i32>,
    z_range: RangeInclusive<i32>,
    seed: u64,
) -> Graph<Room, Wall, Undirected> {
    let (x_min, x_max) = (*x_range.start(), *x_range.end());
    let (z_min, z_max) = (*z_range.start(), *z_range.end());

    let mut graph = Graph::<Room, Wall, Undirected>::new_undirected();
    let mut ids_by_room = HashMap::<Room, NodeIndex>::new();

    // Define rooms
    for x in x_min..x_max {
        for z in z_min..z_max {
            let room = Room {
                west_edge: x,
                south_edge: z,
            };
            let room_id = graph.add_node(room);
            ids_by_room.insert(room, room_id);
        }
    }

    // Define (potential) walls
    for x in x_min..x_max {
        for z in z_min..z_max {
            let r0 = *ids_by_room
                .get(&Room {
                    west_edge: x,
                    south_edge: z,
                })
                .unwrap();
            if z > z_min {
                let wall = Wall {
                    sw_corner: (x, z),
                    orientation: WallOrientation::ParallelToX,
                    disposition: Disposition::Unknown,
                };
                let r1 = *ids_by_room
                    .get(&Room {
                        west_edge: x,
                        south_edge: z - 1,
                    })
                    .unwrap();
                graph.add_edge(r0, r1, wall);
            }
            if x > x_min {
                let wall = Wall {
                    sw_corner: (x, z),
                    orientation: WallOrientation::ParallelToZ,
                    disposition: Disposition::Unknown,
                };
                let r1 = *ids_by_room
                    .get(&Room {
                        west_edge: x - 1,
                        south_edge: z,
                    })
                    .unwrap();
                graph.add_edge(r0, r1, wall);
            }
        }
    }

    let mut unfinished_rooms = ids_by_room.into_values().collect::<HashSet<_>>();
    let mut rooms_in_progress = HashSet::<NodeIndex>::new();
    let mut finished_rooms = HashSet::<NodeIndex>::new();

    let mut rng = SmallRng::seed_from_u64(seed);
    {
        let start_room = *unfinished_rooms.iter().choose(&mut rng).unwrap();
        unfinished_rooms.remove(&start_room);
        rooms_in_progress.insert(start_room);
    }

    while let Some(room) = rooms_in_progress.iter().choose(&mut rng).copied() {
        let Some((neighbor, wall)) = graph
            .neighbors(room)
            .map(|neighbor| {
                let edge_index = graph.find_edge(room, neighbor).unwrap();
                let wall = graph.edge_weight(edge_index).unwrap();
                (neighbor, edge_index, wall)
            })
            .filter(|(_, _, wall)| wall.disposition == Disposition::Unknown)
            .map(|(neighbor, edge_id, _)| (neighbor, edge_id))
            .choose(&mut rng) else {
                rooms_in_progress.remove(&room);
                finished_rooms.insert(room);
                continue;
            };
        let wall = graph.edge_weight_mut(wall).unwrap();
        if unfinished_rooms.contains(&neighbor) {
            wall.disposition = Disposition::Absent;
            unfinished_rooms.remove(&neighbor);
            rooms_in_progress.insert(neighbor);
        } else {
            wall.disposition = Disposition::Present;
        }
    }

    assert!(unfinished_rooms.is_empty());
    assert!(rooms_in_progress.is_empty());

    graph
}

fn choose_spawn_positions(graph: &Graph<Room, Wall, Undirected>) -> SpawnPositions {
    // To keep things interesting, we want to choose two rooms that are as far
    // away as possible (in terms of path length, not Euclidean distance).

    let distances = floyd_warshall(graph, |edge_ref| {
        if edge_ref.weight().disposition == Disposition::Absent {
            1
        } else {
            i32::MAX
        }
    })
    .unwrap();
    let ((start_index, goal_index), _distance) =
        distances.iter().max_by_key(|item| *item.1).unwrap();
    let start = *graph.node_weight(*start_index).unwrap();
    let goal = *graph.node_weight(*goal_index).unwrap();
    SpawnPositions { start, goal }
}

fn build_walls(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    x_range: RangeInclusive<i32>,
    z_range: RangeInclusive<i32>,
    walls: impl Iterator<Item = Wall>,
    sizes: &Sizes,
) {
    let (x_min, x_max) = (*x_range.start(), *x_range.end());
    let (z_min, z_max) = (*z_range.start(), *z_range.end());

    let corner_mesh = meshes.add(
        shape::UVSphere {
            radius: sizes.wall_radius,
            sectors: 8,
            stacks: 8,
        }
        .into(),
    );
    let corner_material = materials.add(Color::BLUE.into());

    let wall_mesh = meshes.add(
        shape::Cylinder {
            radius: sizes.wall_radius,
            height: sizes.room_side_length,
            resolution: 8,
            segments: 1,
        }
        .into(),
    );
    let wall_material = materials.add(Color::BLUE.into());

    for wall in walls.filter(|w| w.disposition == Disposition::Present) {
        let mut transform = Transform::from_xyz(
            wall.sw_corner.0 as f32 * sizes.room_side_length,
            0.0,
            wall.sw_corner.1 as f32 * sizes.room_side_length,
        );
        transform.rotate_z(TAU / 4.0);
        match wall.orientation {
            WallOrientation::ParallelToX => {
                transform.translation += Vec3::X * sizes.room_side_length * 0.5;
            }
            WallOrientation::ParallelToZ => {
                transform.translation += Vec3::Z * sizes.room_side_length * 0.5;
                transform.rotate_y(TAU / 4.0);
            }
        }

        commands.spawn((
            PbrBundle {
                mesh: wall_mesh.clone(),
                material: wall_material.clone(),
                transform,
                ..default()
            },
            Collider::cylinder(sizes.room_side_length * 0.5, sizes.wall_radius),
        ));
    }

    for x in x_min..=x_max {
        for z in z_min..=z_max {
            commands.spawn((
                PbrBundle {
                    mesh: corner_mesh.clone(),
                    material: corner_material.clone(),
                    transform: Transform::from_xyz(
                        x as f32 * sizes.room_side_length,
                        0.0,
                        z as f32 * sizes.room_side_length,
                    ),
                    ..default()
                },
                Collider::ball(sizes.wall_radius),
            ));
        }
    }
}
