use bevy::{
    core_pipeline::{bloom::BloomSettings, tonemapping::Tonemapping},
    prelude::*,
    sprite::MaterialMesh2dBundle,
};
// use bevy::render::camera::RenderTarget;
use ray_light_2d::prelude::*;

fn main() {
    // Basic setup.
    App::new()
        .insert_resource(ClearColor(Color::srgba(0.0, 0.0, 0.0, 0.0)))
        // ! MSAA needs to be off for the jump flood to work for now
        .insert_resource(Msaa::Off)
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: (800., 800.).into(),
                    title: "Vordie: Minimal Example".into(),
                    resizable: false,
                    ..default()
                }),
                ..default()
            }),
            VordieLight2DPlugin,
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // camera
    commands.spawn((
        Camera2dBundle {
            camera: Camera {
                hdr: true, // 1. HDR is required for bloom
                ..default()
            },
            tonemapping: Tonemapping::TonyMcMapface, // 2. Using a tonemapper that desaturates to white is recommended
            ..default()
        },
        BloomSettings::NATURAL, // 3. Enable bloom for the camera
        VordieLightSettings { ..default() },
    ));

    // Light
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(Circle::new(10.)).into(),
        // 4. Put something bright in a dark environment to see the effect
        material: materials.add(Color::srgb(0.0, 0.5, 1.5)),
        transform: Transform::from_translation(Vec3::new(-200., -200., 0.)),
        ..default()
    });

    // Light
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(Circle::new(10.)).into(),
        material: materials.add(Color::srgb(1.5, 0.0, 0.5)),
        transform: Transform::from_translation(Vec3::new(200., 100., 0.)),
        ..default()
    });

    // Light occluder
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(Circle::new(20.)).into(),
        material: materials.add(Color::srgb(0.0, 0.0, 0.0)),
        transform: Transform::from_translation(Vec3::new(-200., 200., 0.)),
        ..default()
    });

    // Light occluder
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(Circle::new(20.)).into(),
        material: materials.add(Color::srgb(0.0, 0.0, 0.0)),
        transform: Transform::from_translation(Vec3::new(200., -100., 0.)),
        ..default()
    });
}
