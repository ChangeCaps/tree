mod plant;
mod ron_loader;

use bevy::prelude::*;
use rand::prelude::*;

fn main() {
    App::build()
        .insert_resource(Msaa { samples: 8 })
        // plugins
        .add_plugins(DefaultPlugins)
        .add_plugin(plant::PlantPlugin)
        // startup systems
        .add_startup_system(setup.system())
        // system
        .add_system(character_system.system())
        .add_system(cursor_grab_system.system())
        // run
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_translation(Vec3::new(0.0, 2.0, 15.0)),
            ..PerspectiveCameraBundle::new_3d()
        })
        .insert(PlayerCamera::new());

    let mut rng = thread_rng();

    for _ in 0..50 {
        let x = rng.gen_range(-20.0..20.0);
        let z = rng.gen_range(-20.0..20.0);

        let mut transform = Transform::from_translation(Vec3::new(x, 0.0, z));
        transform.rotation = Quat::from_rotation_y(rng.gen_range(0.0..std::f32::consts::TAU));

        commands.spawn_bundle(plant::PlantBundle {
            mesh: asset_server.load("plants/test.gno"),
            transform,
            ..Default::default()
        });
    }

    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(shape::Plane { size: 2000.0 }.into()),
        material: materials.add(Color::rgb_linear(155.0, 118.0, 83.0).into()),
        ..Default::default()
    });
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
    mut query: Query<(&mut Transform, &mut PlayerCamera)>,
) {
    for event in mouse_events.iter() {
        let window = windows.get_primary().unwrap();

        if !window.cursor_locked() {
            continue;
        }

        for (mut transform, mut camera) in query.iter_mut() {
            let delta = -event.delta * 0.0005;
            camera.state += delta;
            let rotation = Quat::from_rotation_ypr(camera.state.x, camera.state.y, 0.0);

            transform.rotation = rotation;
        }
    }

    for (mut transform, mut camera) in query.iter_mut() {
        transform.translation.y = 2.0 + (camera.head_bob.cos() - 1.0) * 0.02;

        let mut movement = Vec3::ZERO;

        if input.pressed(KeyCode::W) {
            let dir = transform.rotation * -Vec3::Z;

            movement += dir;
        }

        if input.pressed(KeyCode::S) {
            let dir = transform.rotation * Vec3::Z;

            movement += dir;
        }

        if input.pressed(KeyCode::A) {
            let dir = transform.rotation * -Vec3::X;

            movement += dir;
        }

        if input.pressed(KeyCode::D) {
            let dir = transform.rotation * Vec3::X;

            movement += dir;
        }

        movement.y = 0.0;
        if movement.length() > 0.0 {
            transform.translation += movement.normalize() * time.delta_seconds() * 1.5;

            camera.head_bob += time.delta_seconds() * 10.0;
        }
    }
}
