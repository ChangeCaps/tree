mod plant;
mod ron_loader;
mod shadow_render_resources;
mod sky;
mod sun;
mod terrain;

use bevy::prelude::*;
use rand::prelude::*;

fn main() {
    App::build()
        .insert_resource(Msaa { samples: 4 })
        .insert_resource(ClearColor(Color::rgba(0.0, 0.0, 0.0, 0.0)))
        .insert_resource(WindowDescriptor {
            vsync: false,
            ..Default::default()
        })
        // plugins
        .add_plugins(sky::Plugins)
        .add_plugin(sun::SunPlugin)
        .add_plugin(plant::PlantPlugin)
        // startup systems
        .add_startup_system(setup.system())
        .add_startup_system(bevy_mod_debugdump::print_render_graph.system())
        // system
        .add_system(character_system.system())
        .add_system(cursor_grab_system.system())
        .add_system(plant_mesh_system.system())
        .add_system(plant_growth_system.system())
        .add_system(terrain::terrain_system.system())
        // run
        .run();
}

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, asset_server: Res<AssetServer>) {
    let player = commands
        .spawn()
        .insert(Player {})
        .insert(Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)))
        .insert(GlobalTransform::default())
        .id();

    let _player_camera = commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_translation(Vec3::new(0.0, 2.0, 0.0)),
            ..PerspectiveCameraBundle::new_3d()
        })
        .insert(PlayerCamera::new())
        .insert(Parent(player))
        .id();

    let mut transform = Transform::from_translation(Vec3::new(-50.0, 50.0, -50.0));
    transform.look_at(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));

    let _sun = commands
        .spawn()
        .insert(sun::Sun)
        .insert(transform)
        .insert(GlobalTransform::default())
        .insert(Parent(player))
        .id();

    let mut rng = thread_rng();

    const SPREAD: f32 = 4.0;
        
    for x in -3..3 {
        for z in -3..3 {
            let x = x as f32 * SPREAD + rng.gen_range(-4.0..4.0) + SPREAD / 2.0;
            let z = z as f32 * SPREAD + rng.gen_range(-4.0..4.0) + SPREAD / 2.0;

            let mut transform = Transform::from_translation(Vec3::new(x, -0.1, z));
            transform.rotation = Quat::from_rotation_y(rng.gen_range(0.0..std::f32::consts::TAU));

            commands
                .spawn_bundle(plant::PlantBundle {
                    material: plant::PlantMaterial::new(
                        asset_server.load("textures/bark.png"),
                        asset_server.load("textures/leaf_front.png"),
                    ),
                    transform,
                    ..Default::default()
                })
                .insert(asset_server.load::<plant::Genome, _>("plants/test.gno"));
        }
    }

    commands.spawn_bundle(MeshBundle {
        mesh: bevy::sprite::QUAD_HANDLE.typed(),
        render_pipelines: RenderPipelines::from_pipelines(vec![
            bevy::render::pipeline::RenderPipeline::new(sky::SKY_PIPELINE.typed()),
        ]),
        ..Default::default()
    });

    commands.spawn_bundle(sky::PostBundle {
        ..Default::default()
    });

    commands.spawn_bundle(sun::ShadedBundle {
        mesh: meshes.add(shape::Plane { size: 2000.0 }.into()),
        ..Default::default()
    });
}

pub struct Player {
    //camera
}

pub struct PlayerCamera {
    head_bob: f32,
    state: Vec2,
}

impl PlayerCamera {
    pub fn new() -> Self {
        Self {
            head_bob: 2.0,
            state: Vec2::ZERO,
        }
    }
}

pub fn plant_growth_system(time: Res<Time>, mut query: Query<&mut plant::PlantMaterial>) {
    for mut plant_material in query.iter_mut() {
        plant_material.growth += time.delta_seconds() * 0.5;
    }
}

pub fn plant_mesh_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    gnomes: Res<Assets<plant::Genome>>,
    query: Query<(Entity, &Handle<plant::Genome>), Without<Handle<Mesh>>>,
) {
    for (entity, genome_handle) in query.iter() {
        if let Some(genome) = gnomes.get(genome_handle) {
            let mesh_handle = meshes.add(genome.generate_mesh());

            commands.entity(entity).insert(mesh_handle);
        }
    }
}

pub fn cursor_grab_system(
    mut windows: ResMut<Windows>,
    btn: Res<Input<MouseButton>>,
    key: Res<Input<KeyCode>>,
) {
    let window = windows.get_primary_mut().unwrap();

    if btn.just_pressed(MouseButton::Left) {
        window.set_cursor_lock_mode(true);
        window.set_cursor_visibility(false);
    }

    if key.just_pressed(KeyCode::Escape) {
        window.set_cursor_lock_mode(false);
        window.set_cursor_visibility(true);
    }
}

pub fn character_system(
    mut mouse_events: EventReader<bevy::input::mouse::MouseMotion>,
    input: Res<Input<KeyCode>>,
    time: Res<Time>,
    windows: Res<Windows>,
    mut camera_query: Query<(Entity, &mut PlayerCamera), With<Transform>>,
    player_query: Query<(Entity, &Player, &Children), With<Transform>>,
    mut transform_query: Query<&mut Transform>,
) {
    for event in mouse_events.iter() {
        let window = windows.get_primary().unwrap();

        if !window.cursor_locked() {
            continue;
        }

        for (entity, mut camera) in camera_query.iter_mut() {
            let mut transform = transform_query.get_mut(entity).unwrap();

            let delta = -event.delta * 0.0005;
            camera.state += delta;
            let rotation = Quat::from_rotation_ypr(camera.state.x, camera.state.y, 0.0);

            transform.rotation = rotation;
        }
    }

    for (entity, camera) in camera_query.iter_mut() {
        let mut transform = transform_query.get_mut(entity).unwrap();

        transform.translation.y = 2.0 + (camera.head_bob.cos() - 1.0) * 0.02;
    }

    for (entity, _player, children) in player_query.iter() {
        let transform = transform_query.get_mut(children[0]).unwrap();

        let rotation_matrix = transform.rotation.clone();

        let mut transform = transform_query.get_mut(entity).unwrap();

        let mut movement = Vec3::ZERO;

        if input.pressed(KeyCode::W) {
            let dir = -Vec3::Z;

            movement += dir;
        }

        if input.pressed(KeyCode::S) {
            let dir = Vec3::Z;

            movement += dir;
        }

        if input.pressed(KeyCode::A) {
            let dir = -Vec3::X;

            movement += dir;
        }

        if input.pressed(KeyCode::D) {
            let dir = Vec3::X;

            movement += dir;
        }

        if movement.length() > 0.0 {
            movement = rotation_matrix * movement;
            movement.y = 0.0;

            transform.translation += movement.normalize() * time.delta_seconds() * 1.5;

            let (_, mut camera) = camera_query.get_mut(children[0]).unwrap();

            camera.head_bob += time.delta_seconds() * 10.0;
        }
    }
}
