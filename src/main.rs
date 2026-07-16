mod names;
mod params;
mod rng;
mod svg;

use bevy::{
    asset::AssetMetaCheck,
    prelude::*,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
    sprite_render::{Material2d, Material2dPlugin},
};

use params::{CustomParams, Param, ShaderParams};
use svg::svg_icon;

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
                copy_result,
                copy_flash,
                copy_button_hover,
                scramble,
                toggle_settings,
                spin_icon,
                github_button,
                button_cursor,
                show_adapter_info,
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

/// Marks the copy-to-clipboard button next to the result text.
#[derive(Component)]
struct CopyButton;

/// Marks the copy button's icon.
#[derive(Component)]
struct CopyLabel;

/// Reverts the copy icon tint after the success flash.
#[derive(Component)]
struct CopyFlash(Timer);

/// Marks the settings toggle button (top-left gear).
#[derive(Component)]
struct SettingsButton;

/// Marks the GitHub link button next to the gear.
#[derive(Component)]
struct GithubButton;

/// Marks the gear icon image inside the settings button.
#[derive(Component)]
struct SettingsIcon;

/// Active spin animation on the gear icon.
#[derive(Component)]
struct IconSpin(Timer);

/// Marks the settings panel root.
#[derive(Component)]
struct SettingsPanel;

/// Text node showing the GPU adapter name, filled in once the renderer
/// is up.
#[derive(Component)]
struct AdapterNameText;

/// Text node showing the GPU backend, filled in once the renderer is up.
#[derive(Component)]
struct AdapterBackendText;

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

fn setup_ui(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
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
                panel
                    .spawn(Node {
                        width: Val::Percent(100.0),
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    })
                    .with_children(|row| {
                        row.spawn((
                            Text::new("..."),
                            TextFont::from_font_size(20.0),
                            TextColor(Color::srgb(0.15, 0.9, 1.0)),
                            ResultText,
                        ));
                        row.spawn((
                            Button,
                            CopyButton,
                            Node {
                                padding: UiRect::axes(Val::Px(10.0), Val::Px(5.0)),
                                position_type: PositionType::Absolute,
                                right: Val::Px(0.0),
                                border_radius: BorderRadius::all(Val::Px(6.0)),
                                ..default()
                            },
                        ))
                        .with_children(|button| {
                            button.spawn((
                                ImageNode::new(svg_icon(
                                    &mut images,
                                    include_str!("../assets/clipboard-copy.svg"),
                                )),
                                Node {
                                    width: Val::Px(16.0),
                                    height: Val::Px(16.0),
                                    ..default()
                                },
                                CopyLabel,
                            ));
                        });
                    });
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
                ImageNode::new(svg_icon(&mut images, include_str!("../assets/settings.svg"))),
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
            Button,
            GithubButton,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                left: Val::Px(60.0),
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
                ImageNode::new(svg_icon(&mut images, include_str!("../assets/github.svg"))),
                Node {
                    width: Val::Px(22.0),
                    height: Val::Px(22.0),
                    ..default()
                },
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
                Text::new("Adapter"),
                TextFont::from_font_size(15.0),
                TextColor(Color::srgb(0.92, 0.9, 1.0)),
                Node {
                    width: Val::Percent(100.0),
                    ..default()
                },
            ));
            // Column wrapper so name and backend stack on their own lines
            // inside the wrap-row panel.
            panel
                .spawn(Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    ..default()
                })
                .with_children(|adapter| {
                    adapter.spawn((
                        Text::new(""),
                        TextFont::from_font_size(11.0),
                        TextColor(Color::srgb(0.7, 0.65, 0.85)),
                        AdapterNameText,
                    ));
                    adapter.spawn((
                        Text::new(""),
                        TextFont::from_font_size(11.0),
                        TextColor(Color::srgb(0.7, 0.65, 0.85)),
                        AdapterBackendText,
                    ));
                });
            panel.spawn((
                Text::new("Params"),
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

/// Repository page opened by the GitHub button; single-sourced from the
/// `repository` field in Cargo.toml.
const REPO_URL: &str = env!("CARGO_PKG_REPOSITORY");

/// Open the repository page on GitHub button press.
fn github_button(
    interactions: Query<&Interaction, (Changed<Interaction>, With<GithubButton>)>,
) {
    for interaction in &interactions {
        if *interaction == Interaction::Pressed {
            // _self replaces the current tab on wasm; no effect natively.
            let mut options = webbrowser::BrowserOptions::new();
            options.with_target_hint("_self");
            if let Err(error) =
                webbrowser::open_browser_with_options(webbrowser::Browser::Default, REPO_URL, &options)
            {
                warn!("failed to open {REPO_URL}: {error}");
            }
        }
    }
}

/// Show a pointer cursor while hovering any button.
fn button_cursor(
    interactions: Query<&Interaction, With<Button>>,
    window: Single<Entity, With<Window>>,
    mut hovering: Local<bool>,
    mut commands: Commands,
) {
    use bevy::window::{CursorIcon, SystemCursorIcon};

    let now_hovering = interactions.iter().any(|i| *i != Interaction::None);
    if now_hovering != *hovering {
        *hovering = now_hovering;
        let icon = if now_hovering {
            SystemCursorIcon::Pointer
        } else {
            SystemCursorIcon::Default
        };
        commands.entity(*window).insert(CursorIcon::System(icon));
    }
}

/// Fill the adapter lines once the render resources exist (async on wasm).
fn show_adapter_info(
    adapter: Option<Res<bevy::render::renderer::RenderAdapterInfo>>,
    mut names: Query<&mut Text, With<AdapterNameText>>,
    // Without keeps the two mutable Text queries provably disjoint (B0001).
    mut backends: Query<&mut Text, (With<AdapterBackendText>, Without<AdapterNameText>)>,
) {
    let Some(adapter) = adapter else { return };
    // Guards so text change detection doesn't fire every frame.
    for mut name in &mut names {
        if name.0.is_empty() {
            name.0 = adapter.name.clone();
        }
    }
    for mut backend in &mut backends {
        if backend.0.is_empty() {
            backend.0 = if adapter.driver_info.is_empty() {
                adapter.backend.to_str().into()
            } else {
                adapter.driver_info.clone()
            };
        }
    }
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

/// Hover feedback for the frameless copy button: brighten the icon,
/// unless the green success flash is showing.
fn copy_button_hover(
    interactions: Query<(&Interaction, &Children), (Changed<Interaction>, With<CopyButton>)>,
    mut icons: Query<&mut ImageNode, (With<CopyLabel>, Without<CopyFlash>)>,
) {
    for (interaction, children) in &interactions {
        for child in children {
            if let Ok(mut icon) = icons.get_mut(*child) {
                icon.color = match interaction {
                    Interaction::None => Color::WHITE,
                    _ => Color::srgb(0.15, 0.9, 1.0),
                };
            }
        }
    }
}

/// Hover/press feedback for settings-related buttons.
fn settings_button_colors(
    mut interactions: Query<
        (&Interaction, &mut BackgroundColor),
        (
            Changed<Interaction>,
            With<Button>,
            Without<GenerateButton>,
            Without<SettingsButton>,
            Without<GithubButton>,
            Without<CopyButton>,
        ),
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

/// Copy the generated name to the clipboard (the scramble target while
/// the animation is still running), flashing the icon green.
fn copy_result(
    interactions: Query<&Interaction, (Changed<Interaction>, With<CopyButton>)>,
    result: Query<(&Text, Option<&Scramble>), With<ResultText>>,
    mut icons: Query<(Entity, &mut ImageNode), With<CopyLabel>>,
    mut clipboard: ResMut<bevy::clipboard::Clipboard>,
    mut commands: Commands,
) {
    for interaction in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }
        for (text, scramble) in &result {
            let name = scramble
                .map(|s| s.target.clone())
                .unwrap_or_else(|| text.0.clone());
            if name.is_empty() || name == "..." {
                continue;
            }
            match clipboard.set_text(name) {
                Ok(()) => {
                    for (entity, mut icon) in &mut icons {
                        icon.color = Color::srgb(0.3, 1.0, 0.5);
                        commands
                            .entity(entity)
                            .insert(CopyFlash(Timer::from_seconds(1.2, TimerMode::Once)));
                    }
                }
                Err(error) => warn!("clipboard write failed: {error}"),
            }
        }
    }
}

fn copy_flash(
    time: Res<Time>,
    mut commands: Commands,
    mut query: Query<(Entity, &mut CopyFlash, &mut ImageNode)>,
) {
    for (entity, mut flash, mut icon) in &mut query {
        flash.0.tick(time.delta());
        if flash.0.is_finished() {
            icon.color = Color::WHITE;
            commands.entity(entity).remove::<CopyFlash>();
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
