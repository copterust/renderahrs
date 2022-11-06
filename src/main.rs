use bevy::prelude::*;

mod data;
use data::*;

fn main() {
    let name = std::env::args().skip(1).next();
    let name = name.as_deref().unwrap_or("tilt1.txt");

    let anim = if name.starts_with("/dev/") {
        Anim(Box::new(Stream::start(name)))
    } else {
        Anim(Box::new(FileData::load(name)))
    };

    App::new()
        .insert_resource(anim)
        .add_system(update_quat)
        .add_system(update_intg)
        .add_system(run_animation)
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .run();
}

struct Anim(Box<dyn AnimSource + Sync + Send>);

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
        transform: Transform::from_xyz(1.0, 1.0, 0.0),
        ..default()
    }).insert(QuatTarget);
    // cube
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::rgb(0.4, 0.4, 0.8).into()),
        transform: Transform::from_xyz(-1.0, 1.0, 0.0),
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

fn run_animation(mut data: ResMut<Anim>, mut text: Query<&mut Text>) {
    data.0.think(&mut text.single_mut().sections[0].value);
}

fn update_quat(mut quat: Query<(&mut Transform, With<QuatTarget>)>, data: Res<Anim>) {
    let q = data.0.get_quat();
    quat.single_mut().0.rotation = Quat::from_xyzw(q[1], q[2], q[3], q[0]);
}

fn update_intg(mut intg: Query<(&mut Transform, With<IntegrateTarget>)>, data: Res<Anim>) {
    let intg = &mut intg.single_mut().0;

    let (g, reset) = data.0.get_gyro();
    if reset {
        intg.rotation = Quat::default();
    }

    let br = intg.rotation;
    let lx = Quat::from_axis_angle(br * Vec3::new(1., 0., 0.), g[0]);
    let ly = Quat::from_axis_angle(br * Vec3::new(0., 1., 0.), g[1]);
    let lz = Quat::from_axis_angle(br * Vec3::new(0., 0., 1.), g[2]);
    intg.rotation *= lx * ly * lz;
}
