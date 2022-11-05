
use std::io::BufRead;

use bevy::prelude::*;

use serde::{Deserialize};
use serde_json;

#[derive(Deserialize)]
struct Sample {
    dt: f32,
    _accel: [f32; 3],
    gyro: [f32; 3],
    _mag: [f32; 3],
    state: [[f32; 7]; 1],
}

fn load_animation() -> Vec<Sample> {
    let name = std::env::args().skip(1).next();
    let name = name.as_deref().unwrap_or("tilt1.txt");
    let f = std::io::BufReader::new(std::fs::File::open(name).unwrap());
    let mut v = Vec::new();
    for l in f.lines() {
        let l = l.unwrap();
        let s: Sample = match serde_json::from_str(&l) {
            Ok(s) => s,
            Err(_) => continue,
        };
        v.push(s);
    }

    v
}

struct AnimationData {
    samples: Vec<Sample>,
    frame: usize,
}

fn main() {
    let samples = load_animation();

    App::new()
        .insert_resource(AnimationData {
            samples,
            frame: 0,
        })
        .add_system(update_quat)
        .add_system(update_intg)
        .add_system(run_animation)
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .run();
}

#[derive(Component)]
struct QuatTarget;

#[derive(Component)]
struct IntegrateTarget;

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // plane
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });
    // cube
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::rgb(0.8, 0.4, 0.4).into()),
        transform: Transform::from_xyz(1.0, 0.5, 0.0),
        ..default()
    }).insert(QuatTarget);
    // cube
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::rgb(0.4, 0.4, 0.8).into()),
        transform: Transform::from_xyz(-1.0, 0.5, 0.0),
        ..default()
    }).insert(IntegrateTarget);
    // light
    commands.spawn_bundle(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    commands.spawn_bundle(TextBundle::from_section(
        "Frame counter",
        TextStyle {
            font: asset_server.load("luculent.ttf"),
            font_size: 32.0,
            color: Color::rgb(1., 1., 1.),
        }
    )
    .with_text_alignment(TextAlignment::TOP_CENTER)
    .with_style(Style {
        position_type: PositionType::Absolute,
        position: UiRect {
            bottom: Val::Px(5.0),
            right: Val::Px(15.0),
            ..default()
        },
        ..default()
    }));

    // camera
    commands.spawn_bundle(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

fn run_animation(mut data: ResMut<AnimationData>, mut text: Query<&mut Text>) {
    use std::fmt::Write;

    data.frame = (data.frame + 1) % data.samples.len();
    let text = &mut text.single_mut().sections[0].value;
    text.clear();
    write!(text, "{} / {}", data.frame, data.samples.len()).unwrap();
}

fn update_quat(mut quat: Query<(&mut Transform, With<QuatTarget>)>, data: Res<AnimationData>) {
    let sample = &data.samples[data.frame];
    let q = &sample.state[0]; // wxyz
    quat.single_mut().0.rotation = Quat::from_xyzw(q[1], q[2], q[3], q[0]);
}

fn update_intg(mut intg: Query<(&mut Transform, With<IntegrateTarget>)>, data: Res<AnimationData>) {
    if data.frame == 0 {
        intg.single_mut().0.rotation = Quat::default();
    }
    let sample = &data.samples[data.frame];
    let dt = sample.dt / 1000.0;
    let g = &sample.gyro;
    let r = Quat::from_euler(EulerRot::XYZ, g[0] * dt, g[1] * dt, g[2] * dt);
    intg.single_mut().0.rotation *= r;
}

