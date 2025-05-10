//! Demonstrates showing a texture atlas-based image font text at both its
//! 'native' height and a scaled-up height, also demonstrating use of its
//! additional `color` value.

#![expect(
    clippy::mod_module_files,
    reason = "if present as common.rs, cargo thinks it's an example binary"
)]
#![expect(clippy::expect_used, reason = "only used when panics can't happen")]

use std::time::Duration;

use bevy::color::palettes::tailwind;
use bevy::color::ColorCurve;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy_asset_loader::prelude::AssetCollectionApp as _;
use bevy_image_font::atlas_sprites::ImageFontSpriteText;
use bevy_image_font::{ImageFontPlugin, ImageFontText, LetterSpacing};

use crate::common::{DemoAssets, FONT_WIDTH, RAINBOW, TEXT};

mod common;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            ImageFontPlugin,
        ))
        .init_collection::<DemoAssets>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                animate_text,
                animate_color,
                #[cfg(feature = "gizmos")]
                gizmos::toggle_gizmos,
            ),
        )
        .insert_resource(ClearColor(Color::srgb(0.2, 0.2, 0.2)))
        .run();
}

/// A component for animating text content.
///
/// The `&'static str` field represents the text to animate, while the `usize`
/// field tracks the current animation state, i.e. how many characters of the
/// text is currenly being displayed.
#[derive(Component)]
struct AnimateText(&'static str, usize);

/// A component for animating the color of text.
///
/// The `ColorCurve` field defines the color gradient used for the animation.
#[derive(Component)]
struct AnimateColor(ColorCurve<Srgba>);

/// Spawns the text entities for the example and configures gizmo defaults.
fn setup(
    mut commands: Commands,
    assets: Res<DemoAssets>,
    #[cfg(feature = "gizmos")] store: ResMut<GizmoConfigStore>,
) {
    #[cfg(feature = "gizmos")]
    gizmos::configure_gizmo_defaults(store);

    commands.spawn(Camera2d);

    commands.spawn((
        AnimateColor(ColorCurve::new(RAINBOW).expect("RAINBOW contains at least two colors")),
        ImageFontSpriteText::default(),
        ImageFontText::default()
            .text(TEXT)
            .font(assets.example.clone())
            .font_height(36.0),
        // Shift 0.5 pixels so our characters end up at integer pixel values. This is only
        // necessary because we're using a horizontally centered anchor combined with an
        // odd number of characters. If we used left or right alignment, we wouldn't need
        // to do this, even with an odd number of characters.
        Transform::from_translation(Vec3::new(0.5, 0., 0.)),
    ));
    commands.spawn((
        AnimateText(TEXT, 0),
        // This demonstrates using the `anchor` field on the `ImageFontSpriteText`.
        // To still center it horizontally with the text above, we shift it left by half its width.
        ImageFontSpriteText::default()
            .color(tailwind::AMBER_500)
            .anchor(Anchor::CenterLeft),
        ImageFontText::default().font(assets.example.clone()),
        #[expect(
            clippy::cast_precision_loss,
            reason = "the magnitude of the numbers we're working on here are too small to lose \
            anything"
        )]
        Transform::from_translation(Vec3::new(
            -((TEXT.chars().count() * FONT_WIDTH / 2) as f32),
            40.,
            0.,
        )),
    ));

    commands.spawn((
        ImageFontSpriteText::default()
            .color(tailwind::ZINC_500)
            .letter_spacing(LetterSpacing::Pixel(2)),
        ImageFontText::default()
            .text(TEXT)
            .font(assets.example.clone())
            .font_height(36.0),
        Transform::from_translation(Vec3::new(0.5, -40., 0.)),
    ));

    // This will currently render without spaces; I intend to add a new feature
    // shortly that will allow a font to have space without needing to have a
    // space in the font image itself.
    commands.spawn((
        AnimateColor(ColorCurve::new(RAINBOW).expect("RAINBOW contains at least two colors")),
        ImageFontSpriteText::default().letter_spacing(LetterSpacing::Pixel(1)),
        ImageFontText::default()
            .text(TEXT.to_uppercase())
            .font(assets.variable_width.clone())
            .font_height(32.0),
        Transform::from_translation(Vec3::new(0., -120., 0.)),
    ));
}

/// Animates the text content of entities with the `AnimateText` component.
///
/// This system modifies the `ImageFontText` component to display an animated
/// sequence of characters, cycling through the text content over time.
fn animate_text(
    mut query: Query<(&mut AnimateText, &mut ImageFontText)>,
    time: Res<Time>,
    mut timer: Local<Timer>,
) {
    if timer.duration().is_zero() || timer.mode() == TimerMode::Once {
        timer.set_duration(Duration::from_secs_f32(0.1));
        timer.set_mode(TimerMode::Repeating);
    }

    timer.tick(time.delta());
    if !timer.just_finished() {
        return;
    }

    for (mut animated_text, mut image_font_text) in &mut query {
        let char_count = animated_text.0.chars().count();

        animated_text.1 += 1;
        if animated_text.1 > char_count * 2 {
            animated_text.1 = 0;
        }

        let show_count = if animated_text.1 > char_count - 1 {
            char_count - (animated_text.1 - char_count)
        } else {
            animated_text.1
        };
        image_font_text.text = animated_text.0.chars().take(show_count).collect::<String>();
    }
}

/// Animates the color of text entities with the `AnimateColor` component.
///
/// This system modifies the `color` field of the `ImageFontSpriteText`
/// component to cycle through the colors defined in the `RAINBOW` palette.
#[expect(
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "the magnitude of the numbers we're working on here are too small to lose anything"
)]
fn animate_color(mut query: Query<(&AnimateColor, &mut ImageFontSpriteText)>, time: Res<Time>) {
    for (animate_color, mut image_sprite_font_text) in &mut query {
        let animation_progress = time.elapsed_secs() / RAINBOW.len() as f32;
        let len = (RAINBOW.len() - 1) as f32;
        if animation_progress.trunc() as u32 % 2 == 0 {
            image_sprite_font_text.color = animate_color
                .0
                .sample(animation_progress.fract() * len)
                .expect("fract() is [0,1) and `len` will always be within bounds")
                .into();
        } else {
            image_sprite_font_text.color = animate_color
                .0
                .sample(len - (animation_progress.fract() * len))
                .expect("fract() is [0,1) and `len` will always be within bounds")
                .into();
        }
    }
}

/// Gizmos related example code
#[cfg(feature = "gizmos")]
mod gizmos {

    use bevy_image_font::atlas_sprites::gizmos::AtlasSpritesGizmoConfigGroup;

    use super::{ButtonInput, GizmoConfigStore, KeyCode, Res, ResMut};

    /// Configures default gizmo rendering settings.
    ///
    /// This function initializes the default visibility settings for
    /// text-related gizmos, enabling anchor points and bounding box
    /// rendering by default.
    ///
    /// # Default Behavior
    /// - **Text anchor points**: Enabled (`render_text_anchor_point = true`).
    /// - **Character anchor points**: Enabled (`render_character_anchor_point =
    ///   true`).
    /// - **Character bounding boxes**: Enabled (`render_character_box = true`).
    ///
    /// This function is called at startup to ensure gizmos are visible by
    /// default.
    pub(super) fn configure_gizmo_defaults(mut store: ResMut<'_, GizmoConfigStore>) {
        let (_, atlas_sprites_config) = store.config_mut::<AtlasSpritesGizmoConfigGroup>();
        atlas_sprites_config.render_text_anchor_point = true;
        atlas_sprites_config.render_character_anchor_point = true;
        atlas_sprites_config.render_character_box = true;
    }

    /// Handles keyboard input to toggle gizmo rendering settings.
    ///
    /// This function listens for key presses and updates the gizmo
    /// configuration to enable or disable various debug visuals in real
    /// time.
    ///
    /// # Parameters
    /// - `input`: Reference to [`ButtonInput<KeyCode>`], used to detect key
    ///   presses.
    /// - `store`: Mutable reference to the [`GizmoConfigStore`] to update
    ///   settings.
    ///
    /// # Keyboard Shortcuts
    /// - **`G`**: Toggles all gizmo rendering on/off.
    /// - **`A`**: Toggles character anchor point gizmos.
    /// - **`B`**: Toggles character bounding box gizmos.
    /// - **`T`**: Toggles text anchor point gizmos.
    pub(super) fn toggle_gizmos(
        input: Res<ButtonInput<KeyCode>>,
        mut store: ResMut<GizmoConfigStore>,
    ) {
        let (config, atlas_sprites_config) = store.config_mut::<AtlasSpritesGizmoConfigGroup>();
        if input.just_pressed(KeyCode::KeyG) {
            config.enabled = !config.enabled;
            println!("Gizmos enabled: {}", config.enabled);
        }
        if input.just_pressed(KeyCode::KeyA) {
            atlas_sprites_config.render_character_anchor_point =
                !atlas_sprites_config.render_character_anchor_point;
            println!(
                "Character anchor point gizmo enabled: {}",
                atlas_sprites_config.render_character_anchor_point
            );
        }
        if input.just_pressed(KeyCode::KeyB) {
            atlas_sprites_config.render_character_box = !atlas_sprites_config.render_character_box;
            println!(
                "Character box gizmo enabled: {}",
                atlas_sprites_config.render_character_box
            );
        }
        if input.just_pressed(KeyCode::KeyT) {
            atlas_sprites_config.render_text_anchor_point =
                !atlas_sprites_config.render_text_anchor_point;
            println!(
                "Text anchor gizmo enabled: {}",
                atlas_sprites_config.render_text_anchor_point
            );
        }
    }
}
