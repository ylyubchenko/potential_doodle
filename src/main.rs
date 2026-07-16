use bevy::{
    asset::AssetMetaCheck,
    prelude::*,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    sprite_render::{Material2d, Material2dPlugin},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(AssetPlugin {
            // Web servers (incl. trunk) answer missing .meta files with HTML
            // instead of 404, which breaks asset loading on wasm.
            meta_check: AssetMetaCheck::Never,
            ..default()
        }).set(WindowPlugin {
            primary_window: Some(Window {
                title: "potential_doodle".into(),
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(Material2dPlugin::<NeonLandscapeMaterial>::default())
        .add_systems(Startup, setup)
        .add_systems(Update, (rotate, fit_background))
        .run();
}

/// Fullscreen background driven by `assets/shaders/neon_landscape.wgsl`.
/// Time and viewport come from Bevy's view bindings.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct NeonLandscapeMaterial {
    #[uniform(0)]
    tint: Vec4,
}

impl Material2d for NeonLandscapeMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/neon_landscape.wgsl".into()
    }
}

#[derive(Component)]
struct Spinner;

#[derive(Component)]
struct Background;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<NeonLandscapeMaterial>>,
) {
    commands.spawn(Camera2d);
    commands.spawn((
        Mesh2d(meshes.add(Rectangle::default())),
        MeshMaterial2d(materials.add(NeonLandscapeMaterial { tint: Vec4::ONE })),
        Transform::from_xyz(0.0, 0.0, -100.0),
        Background,
    ));
    commands.spawn((
        Sprite::from_color(Color::srgb(0.9, 0.4, 0.3), Vec2::splat(150.0)),
        Spinner,
    ));
}

fn rotate(time: Res<Time>, mut query: Query<&mut Transform, With<Spinner>>) {
    for mut transform in &mut query {
        transform.rotate_z(time.delta_secs());
    }
}

/// Keep the background quad covering the whole window.
fn fit_background(
    window: Single<&Window>,
    mut query: Query<&mut Transform, With<Background>>,
) {
    for mut transform in &mut query {
        transform.scale = Vec3::new(window.width(), window.height(), 1.0);
    }
}
