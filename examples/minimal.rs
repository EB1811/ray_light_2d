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
        .insert_resource(ClearColor(Color::rgba(0.0, 0.0, 0.0, 0.0)))
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: (1024., 1024.).into(),
                    title: "Bevy Magic Light 2D: Minimal Example".into(),
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
            // tonemapping: Tonemapping::TonyMcMapface, // 2. Using a tonemapper that desaturates to white is recommended
            ..default()
        },
        // BloomSettings::default(), // 3. Enable bloom for the camera
        VordieLightSettings {
            setting: 0.2,
            ..default()
        },
    ));

    // Light
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(Circle::new(50.)).into(),
        // 4. Put something bright in a dark environment to see the effect
        material: materials.add(Color::rgb(0.2, 0.9, 0.0)),
        transform: Transform::from_translation(Vec3::new(-300., -200., 0.)),
        ..default()
    });

    // Light occluder
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(Circle::new(50.)).into(),
        material: materials.add(Color::rgb(0.9, 0.2, 0.0)),
        transform: Transform::from_translation(Vec3::new(300., 200., 0.)),
        ..default()
    });
}
