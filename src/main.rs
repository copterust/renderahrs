use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;

mod data;
use data::*;
mod intg;

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
        .add_system(update_accel)
        .add_system(run_animation)
        .add_system(camera_controller)
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .run();
}

#[derive(Resource)]
struct Anim(Box<dyn AnimSource + Sync + Send>);

#[derive(Component)]
struct QuatTarget;

#[derive(Component)]
struct IntegrateTarget;

#[derive(Component)]
struct AccelTarget;

#[derive(Component)]
struct MagTarget;

#[derive(Component, Default)]
struct Orbit {
    rot: Vec2,
}

impl Orbit {
    fn new(x: f32, y: f32) -> Orbit {
        Orbit {
            rot: Vec2::new(x, y)
        }
    }
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let arrow = asset_server.load("arrow.glb#Mesh0/Primitive0");

    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });
    // cube
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::rgb(0.8, 0.4, 0.4).into()),
            transform: Transform::from_xyz(1.0, 1.0, 0.0),
            ..default()
        })
        .insert(QuatTarget);
    // cube
    commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::rgb(0.4, 0.4, 0.8).into()),
            transform: Transform::from_xyz(-1.0, 1.0, 0.0),
            ..default()
        })
        .insert(IntegrateTarget);
    // accel vector
    commands
        .spawn(PbrBundle {
            mesh: arrow.clone(),
            material: materials.add(Color::rgb(0.8, 0.8, 0.8).into()),
            transform: Transform::from_xyz(0.0, 1.0, 1.5),
            ..default()
        })
        .insert(AccelTarget);
    // mag vector
    commands
        .spawn(PbrBundle {
            mesh: arrow.clone(),
            material: materials.add(Color::rgb(0.2, 0.2, 0.8).into()),
            transform: Transform::from_xyz(0.0, 1.0, 1.5),
            ..default()
        })
        .insert(MagTarget);
    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
    commands.spawn(
        TextBundle::from_section(
            "Frame counter",
            TextStyle {
                font: asset_server.load("luculent.ttf"),
                font_size: 32.0,
                color: Color::rgb(1., 1., 1.),
            },
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
        }),
    );

    // camera
    commands.spawn(Camera3dBundle {
        ..default()
    }).insert(Orbit::new(2.0, 0.3));
}

fn run_animation(mut data: ResMut<Anim>, mut text: Query<&mut Text>) {
    data.0.think(&mut text.single_mut().sections[0].value);
}

fn update_quat(mut quat: Query<(&mut Transform, With<QuatTarget>)>, data: Res<Anim>) {
    quat.single_mut().0.rotation = data.0.get_quat();
}

fn update_intg(mut intg: Query<(&mut Transform, With<IntegrateTarget>)>, data: Res<Anim>) {
    intg.single_mut().0.rotation = data.0.get_gyro();
}

fn update_accel(
    mut accel: Query<(&mut Transform, With<AccelTarget>, Without<MagTarget>)>,
    mut north: Query<(&mut Transform, With<MagTarget>, Without<AccelTarget>)>,
    data: Res<Anim>,
) {
    let accel = &mut accel.single_mut().0;
    let north = &mut north.single_mut().0;
    let [a, m] = data.0.get_arrows();
    let a = Vec3::new(a[0], a[1], a[2]);
    let m = Vec3::new(m[0], m[1], m[2]);
    let l = a.length();

    let r = Quat::from_rotation_x(3.14159 / 2.);

    if l > 0.1 {
        accel.scale = Vec3::new(l / 25., 0.01, 0.01);
        accel.rotation = r * Quat::from_rotation_arc(a.normalize(), Vec3::X);
    } else {
        accel.scale = Vec3::new(0.1, 0.1, 0.1);
        accel.rotation = Quat::default();
    }

    north.scale = Vec3::new(0.5, 0.01, 0.01);
    north.rotation = Quat::from_rotation_arc(m.normalize(), Vec3::X);
}

fn camera_controller(
    button: Res<Input<MouseButton>>,
    mut motion: EventReader<MouseMotion>,
    mut camera: Query<(&mut Transform, &mut Orbit)>,
) {
    let (mut trans, mut orbit) = camera.single_mut();

    if button.pressed(MouseButton::Left) {
        let rot = motion.iter().fold(Vec2::default(), |a, e| a + e.delta);
        orbit.rot += rot / 100.0;
        orbit.rot.x = orbit.rot.x.rem_euclid(std::f32::consts::TAU);
        orbit.rot.y = orbit.rot.y.clamp(0.1, 1.5);
    }

    let r = Quat::from_euler(EulerRot::XYZ, 0.0, -orbit.rot.x, orbit.rot.y);
    let p = r * Vec3::new(6.0, 0.0, 0.0);
    *trans = Transform::from_translation(p).looking_at(Vec3::ZERO, Vec3::Y);
}
