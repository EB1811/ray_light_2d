use bevy::{
    core_pipeline::{bloom::BloomSettings, tonemapping::Tonemapping},
    prelude::*,
    sprite::MaterialMesh2dBundle,
    window::WindowResolution,
};
// use bevy::render::camera::RenderTarget;
use ray_light_2d::prelude::*;

#[derive(Component)]
pub struct SpriteCamera;

fn main() {
    // Basic setup.
    App::new()
        .insert_resource(ClearColor(Color::srgba(0.0, 0.0, 0.0, 0.0)))
        // ! MSAA needs to be off for the jump flood to work for now
        .insert_resource(Msaa::Off)
        .add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        resolution: (500., 500.).into(),
                        // resolution: WindowResolution::new(1600., 1600.)
                        //     .with_scale_factor_override(2.),
                        title: "Vordie: Minimal Example".into(),
                        resizable: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
            VordieLight2DPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, system_move_camera)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // camera
    commands
        .spawn((
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
        ))
        .insert(SpriteCamera);

    // Light
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(Circle::new(10.)).into(),
        // 4. Put something bright in a dark environment to see the effect
        material: materials.add(Color::srgb(0.5, 0.0, 0.9)),
        transform: Transform::from_translation(Vec3::new(-100., -100., 0.)),
        ..default()
    });

    // Light
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(Circle::new(10.)).into(),
        material: materials.add(Color::srgb(1.0, 0.0, 0.5)),
        transform: Transform::from_translation(Vec3::new(100., 100., 0.)),
        ..default()
    });

    // Walls

    // Light occluder top
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(Rectangle::new(300., 10.)).into(),
        material: materials.add(Color::srgb(0.0, 0.0, 0.0)),
        transform: Transform::from_translation(Vec3::new(0., 150., 0.)),
        ..default()
    });

    // Light occluder left
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(Rectangle::new(10., 250.)).into(),
        material: materials.add(Color::srgb(0.0, 0.0, 0.0)),
        transform: Transform::from_translation(Vec3::new(-150., 50., 0.)),
        ..default()
    });

    // Light occluder bottom
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(Rectangle::new(300., 10.)).into(),
        material: materials.add(Color::srgb(0.0, 0.0, 0.0)),
        transform: Transform::from_translation(Vec3::new(0., -150., 0.)),
        ..default()
    });
}

fn system_move_camera(
    mut camera_target: Local<Vec3>,
    mut query_camera: Query<&mut Transform, With<SpriteCamera>>,

    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if let Ok(mut camera_transform) = query_camera.get_single_mut() {
        let speed = 10.0;

        if keyboard.pressed(KeyCode::KeyW) {
            camera_target.y += speed;
        }
        if keyboard.pressed(KeyCode::KeyS) {
            camera_target.y -= speed;
        }
        if keyboard.pressed(KeyCode::KeyA) {
            camera_target.x -= speed;
        }
        if keyboard.pressed(KeyCode::KeyD) {
            camera_target.x += speed;
        }

        // Smooth camera.
        let blend_ratio = 0.18;
        let movement = (*camera_target - camera_transform.translation) * blend_ratio;
        camera_transform.translation.x += movement.x;
        camera_transform.translation.y += movement.y;
    }
}
