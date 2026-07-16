mod names;
mod params;
mod rng;

use bevy::{
    asset::AssetMetaCheck,
    prelude::*,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    sprite_render::{Material2d, Material2dPlugin},
};

use params::{CustomParams, Param, ShaderParams};

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
        .init_resource::<CustomParams>()
        .init_resource::<AnimTime>()
        .add_plugins(Material2dPlugin::<NeonLandscapeMaterial>::default())
        .add_systems(Startup, (setup, setup_ui, setup_settings_ui))
        .add_systems(
            Update,
            (
                fit_background,
                generate_button,
                scramble,
                toggle_settings,
                spin_icon,
                adjust_params,
                reset_params,
                settings_button_colors,
                sync_params,
                drive_material,
            ),
        )
        .run();
}

/// Fullscreen background driven by `assets/shaders/neon_landscape.wgsl`.
/// Time and viewport come from Bevy's view bindings.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct NeonLandscapeMaterial {
    #[uniform(0)]
    tint: Vec4,
    #[uniform(1)]
    params: ShaderParams,
}

/// Animation clock advanced by `time_scale`, fed to the shader so speed
/// changes never cause a phase jump.
#[derive(Resource, Default)]
struct AnimTime(f32);

impl Material2d for NeonLandscapeMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/neon_landscape.wgsl".into()
    }
}

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
        MeshMaterial2d(materials.add(NeonLandscapeMaterial {
            tint: Vec4::ONE,
            params: CustomParams::default().shader_params(0.0),
        })),
        Transform::from_xyz(0.0, 0.0, -100.0),
        Background,
    ));
}

const BUTTON_NORMAL: Color = Color::srgb(0.75, 0.1, 0.45);
const BUTTON_HOVERED: Color = Color::srgb(0.9, 0.2, 0.6);
const BUTTON_PRESSED: Color = Color::srgb(0.5, 0.05, 0.3);

/// Glyphs cycled through unrevealed characters during the scramble.
const GLYPHS: &[u8] = b"!<>-_\\/[]{}=+*^?#@$%&";

/// Marks the result text entity.
#[derive(Component)]
struct ResultText;

/// Marks the name-generation button.
#[derive(Component)]
struct GenerateButton;

/// Marks the settings toggle button (top-left gear).
#[derive(Component)]
struct SettingsButton;

/// Marks the gear icon image inside the settings button.
#[derive(Component)]
struct SettingsIcon;

/// Active spin animation on the gear icon.
#[derive(Component)]
struct IconSpin(Timer);

/// Marks the settings panel root.
#[derive(Component)]
struct SettingsPanel;

/// A +/- button that nudges one parameter (direction is -1.0 or 1.0).
#[derive(Component)]
struct ParamAdjust {
    param: Param,
    direction: f32,
}

/// A text node displaying one parameter's current value.
#[derive(Component)]
struct ParamValue(Param);

/// Panels whose background alpha follows the panel_opacity param.
#[derive(Component)]
struct DimmablePanel;

/// Marks the reset-to-defaults button in the settings panel.
#[derive(Component)]
struct ResetButton;

/// Active scramble animation on a text entity.
#[derive(Component)]
struct Scramble {
    target: String,
    timer: Timer,
}

fn setup_ui(mut commands: Commands) {
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|root| {
            root.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(18.0),
                    padding: UiRect::all(Val::Px(28.0)),
                    border_radius: BorderRadius::all(Val::Px(14.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.03, 0.0, 0.09, 0.8)),
                DimmablePanel,
            ))
            .with_children(|panel| {
                panel.spawn((
                    Text::new("Name your next project"),
                    TextFont::from_font_size(24.0),
                    TextColor(Color::srgb(0.92, 0.9, 1.0)),
                ));
                panel.spawn((
                    Text::new("..."),
                    TextFont::from_font_size(20.0),
                    TextColor(Color::srgb(0.15, 0.9, 1.0)),
                    ResultText,
                ));
                panel
                    .spawn((
                        Button,
                        GenerateButton,
                        Node {
                            padding: UiRect::axes(Val::Px(20.0), Val::Px(10.0)),
                            border_radius: BorderRadius::all(Val::Px(8.0)),
                            ..default()
                        },
                        BackgroundColor(BUTTON_NORMAL),
                    ))
                    .with_children(|button| {
                        button.spawn((
                            Text::new("Generate"),
                            TextFont::from_font_size(18.0),
                            TextColor(Color::srgb(1.0, 0.95, 1.0)),
                        ));
                    });
            });
        });
}

const PANEL_BG: Color = Color::srgba(0.03, 0.0, 0.09, 0.85);

/// Rasterize `assets/settings.svg` (white, 2x for crisp scaling) into a
/// Bevy image. Bevy has no native SVG support, so the icon is rendered
/// with resvg at startup.
fn settings_icon(images: &mut Assets<Image>) -> Handle<Image> {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

    let svg = include_str!("../assets/settings.svg").replace("currentColor", "#ffffff");
    let tree = resvg::usvg::Tree::from_str(&svg, &resvg::usvg::Options::default())
        .expect("settings.svg parses");
    let size = 48u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size).expect("pixmap");
    let scale = size as f32 / tree.size().width();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    // tiny-skia produces premultiplied alpha; Bevy expects straight.
    let mut data = pixmap.take();
    for px in data.chunks_exact_mut(4) {
        let a = px[3] as u32;
        if a > 0 {
            px[0] = (px[0] as u32 * 255 / a).min(255) as u8;
            px[1] = (px[1] as u32 * 255 / a).min(255) as u8;
            px[2] = (px[2] as u32 * 255 / a).min(255) as u8;
        }
    }
    images.add(Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        Default::default(),
    ))
}

fn setup_settings_ui(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    commands
        .spawn((
            Button,
            SettingsButton,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                left: Val::Px(12.0),
                width: Val::Px(40.0),
                height: Val::Px(40.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(PANEL_BG),
        ))
        .with_children(|button| {
            button.spawn((
                ImageNode::new(settings_icon(&mut images)),
                Node {
                    width: Val::Px(22.0),
                    height: Val::Px(22.0),
                    ..default()
                },
                UiTransform::IDENTITY,
                SettingsIcon,
            ));
        });

    commands
        .spawn((
            SettingsPanel,
            DimmablePanel,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(60.0),
                left: Val::Px(12.0),
                width: Val::Px(520.0),
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                column_gap: Val::Px(18.0),
                row_gap: Val::Px(10.0),
                padding: UiRect::all(Val::Px(16.0)),
                border_radius: BorderRadius::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(PANEL_BG),
            Visibility::Hidden,
        ))
        .with_children(|panel| {
            panel.spawn((
                Text::new("params"),
                TextFont::from_font_size(15.0),
                TextColor(Color::srgb(0.92, 0.9, 1.0)),
                Node {
                    width: Val::Percent(100.0),
                    ..default()
                },
            ));
            for param in Param::ALL {
                param_cell(panel, param);
            }
            panel
                .spawn((
                    Button,
                    ResetButton,
                    Node {
                        padding: UiRect::axes(Val::Px(14.0), Val::Px(7.0)),
                        border_radius: BorderRadius::all(Val::Px(6.0)),
                        ..default()
                    },
                    BackgroundColor(BUTTON_NORMAL),
                ))
                .with_children(|button| {
                    button.spawn((
                        Text::new("reset defaults"),
                        TextFont::from_font_size(12.0),
                        TextColor(Color::srgb(1.0, 0.95, 1.0)),
                    ));
                });
        });
}

/// One settings cell: label above a [-] value [+] row.
fn param_cell(panel: &mut ChildSpawnerCommands, param: Param) {
    let (label, ..) = param.spec();
    panel
        .spawn(Node {
            width: Val::Px(225.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            ..default()
        })
        .with_children(|cell| {
            cell.spawn((
                Text::new(label),
                TextFont::from_font_size(11.0),
                TextColor(Color::srgb(0.7, 0.65, 0.85)),
            ));
            cell.spawn(Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(10.0),
                ..default()
            })
            .with_children(|row| {
                adjust_button(row, ParamAdjust { param, direction: -1.0 }, "-");
                row.spawn((
                    Text::new(""),
                    TextFont::from_font_size(12.0),
                    TextColor(Color::srgb(0.15, 0.9, 1.0)),
                    Node {
                        min_width: Val::Px(90.0),
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    ParamValue(param),
                ));
                adjust_button(row, ParamAdjust { param, direction: 1.0 }, "+");
            });
        });
}

fn adjust_button(row: &mut ChildSpawnerCommands, adjust: ParamAdjust, sign: &str) {
    row.spawn((
        Button,
        adjust,
        Node {
            width: Val::Px(24.0),
            height: Val::Px(24.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            border_radius: BorderRadius::all(Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(BUTTON_NORMAL),
    ))
    .with_children(|b| {
        b.spawn((
            Text::new(sign),
            TextFont::from_font_size(13.0),
            TextColor(Color::srgb(1.0, 0.95, 1.0)),
        ));
    });
}

/// Show/hide the settings panel on gear press, and spin the gear.
fn toggle_settings(
    interactions: Query<&Interaction, (Changed<Interaction>, With<SettingsButton>)>,
    mut panel: Query<&mut Visibility, With<SettingsPanel>>,
    icon: Query<Entity, With<SettingsIcon>>,
    mut commands: Commands,
) {
    for interaction in &interactions {
        if *interaction == Interaction::Pressed {
            for mut visibility in &mut panel {
                *visibility = match *visibility {
                    Visibility::Hidden => Visibility::Visible,
                    _ => Visibility::Hidden,
                };
            }
            for entity in &icon {
                commands
                    .entity(entity)
                    .insert(IconSpin(Timer::from_seconds(0.45, TimerMode::Once)));
            }
        }
    }
}

/// Ease the gear through a half turn (the icon is 180°-symmetric, so it
/// lands back on itself).
fn spin_icon(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut IconSpin, &mut UiTransform)>,
) {
    for (entity, mut spin, mut transform) in &mut query {
        spin.0.tick(time.delta());
        if spin.0.is_finished() {
            transform.rotation = Rot2::IDENTITY;
            commands.entity(entity).remove::<IconSpin>();
            continue;
        }
        // Cubic ease-out.
        let eased = 1.0 - (1.0 - spin.0.fraction()).powi(3);
        transform.rotation = Rot2::radians(eased * std::f32::consts::PI);
    }
}

/// Restore every param to its default on reset press.
fn reset_params(
    interactions: Query<&Interaction, (Changed<Interaction>, With<ResetButton>)>,
    mut params: ResMut<CustomParams>,
) {
    for interaction in &interactions {
        if *interaction == Interaction::Pressed {
            *params = CustomParams::default();
        }
    }
}

/// Apply +/- presses to the params resource.
fn adjust_params(
    interactions: Query<(&Interaction, &ParamAdjust), Changed<Interaction>>,
    mut params: ResMut<CustomParams>,
) {
    for (interaction, adjust) in &interactions {
        if *interaction == Interaction::Pressed {
            params.nudge(adjust.param, adjust.direction);
        }
    }
}

/// Hover/press feedback for settings-related buttons.
fn settings_button_colors(
    mut interactions: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>, Without<GenerateButton>, Without<SettingsButton>),
    >,
) {
    for (interaction, mut background) in &mut interactions {
        *background = BackgroundColor(match interaction {
            Interaction::Pressed => BUTTON_PRESSED,
            Interaction::Hovered => BUTTON_HOVERED,
            Interaction::None => BUTTON_NORMAL,
        });
    }
}

/// Push params into the value labels and panel opacity whenever they
/// change (also runs once on startup).
fn sync_params(
    params: Res<CustomParams>,
    mut values: Query<(&ParamValue, &mut Text)>,
    mut panels: Query<&mut BackgroundColor, With<DimmablePanel>>,
) {
    if !params.is_changed() {
        return;
    }
    for (value, mut text) in &mut values {
        text.0 = params.display(value.0);
    }
    for mut background in &mut panels {
        *background = BackgroundColor(Color::srgba(0.03, 0.0, 0.09, params.panel_opacity));
    }
}

/// Advance the animation clock and feed the shader uniforms every frame.
fn drive_material(
    time: Res<Time>,
    params: Res<CustomParams>,
    mut anim: ResMut<AnimTime>,
    background: Query<&MeshMaterial2d<NeonLandscapeMaterial>, With<Background>>,
    mut materials: ResMut<Assets<NeonLandscapeMaterial>>,
) {
    anim.0 += time.delta_secs() * params.time_scale;
    for handle in &background {
        if let Some(mut material) = materials.get_mut(&handle.0) {
            material.params = params.shader_params(anim.0);
        }
    }
}

/// Button feedback + kick off a scramble toward a fresh name on press.
fn generate_button(
    mut interactions: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<GenerateButton>),
    >,
    result: Query<Entity, With<ResultText>>,
    time: Res<Time>,
    params: Res<CustomParams>,
    mut commands: Commands,
) {
    for (interaction, mut background) in &mut interactions {
        match *interaction {
            Interaction::Pressed => {
                *background = BackgroundColor(BUTTON_PRESSED);
                let name = names::generate_name(
                    time.elapsed().as_nanos() as u64,
                    params.name_style as u8,
                    names::SEPARATORS[params.name_separator as usize],
                );
                for entity in &result {
                    commands.entity(entity).insert(Scramble {
                        target: name.clone(),
                        timer: Timer::from_seconds(params.scramble_duration, TimerMode::Once),
                    });
                }
            }
            Interaction::Hovered => *background = BackgroundColor(BUTTON_HOVERED),
            Interaction::None => *background = BackgroundColor(BUTTON_NORMAL),
        }
    }
}

/// Reveal the target left to right while unrevealed characters cycle
/// through random glyphs (motion.dev-style text scramble).
fn scramble(
    time: Res<Time>,
    params: Res<CustomParams>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut Scramble, &mut Text)>,
) {
    for (entity, mut anim, mut text) in &mut query {
        anim.timer.tick(time.delta());
        if anim.timer.is_finished() {
            text.0 = anim.target.clone();
            commands.entity(entity).remove::<Scramble>();
            continue;
        }
        let chars: Vec<char> = anim.target.chars().collect();
        let revealed = (anim.timer.fraction() * chars.len() as f32) as usize;

        // Quantize the seed so glyphs only switch every some ms instead of
        // every frame.
        let interval = (params.scramble_interval_ms as u64).max(1);
        let mut rng = rng::Rng::new(time.elapsed().as_millis() as u64 / interval);
        let mut out = String::with_capacity(chars.len());
        for (i, c) in chars.iter().enumerate() {
            let glyph = *rng.pick(GLYPHS) as char;
            if i < revealed {
                out.push(*c);
            } else {
                out.push(glyph);
            }
        }
        text.0 = out;
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
