//! # Bevy Image Font Example
//!
//! This example demonstrates how to use `bevy_image_font` to render text
//! with bitmap fonts in a Bevy application. It loads multiple font formats
//! and displays multilingual pangrams to test font rendering across different
//! scripts and writing systems.
//!
//! ## Highlight: Pangrams for Multilingual Script Testing
//! A **pangram** is a sentence that contains every letter of an alphabet at
//! least once. This example renders pangrams from various languages and
//! scripts, including:
//! - Latin-based languages (English, French, German, etc.).
//! - Cyrillic (Russian).
//! - Greek.
//! - Hebrew and Arabic (right-to-left scripts).
//! - Thai and Armenian.
//!
//! By displaying pangrams, this example provides a visually rich way to test
//! font rendering, spacing, and character coverage across different writing
//! systems.

#![expect(
    clippy::mod_module_files,
    reason = "if present as common.rs, cargo thinks it's an example binary"
)]

use bevy::prelude::*;
use bevy_asset_loader::prelude::AssetCollectionApp as _;
#[cfg(feature = "gizmos")]
use bevy_gizmos::config::GizmoConfigStore;
use bevy_image_font::atlas_sprites::ImageFontSpriteText;
use bevy_image_font::{ImageFontPlugin, ImageFontText};

use crate::common::{DemoAssets, PANGRAMS};

mod common;

fn main() {
    let mut app = App::new();

    app.add_plugins((
        DefaultPlugins.set(ImagePlugin::default_nearest()),
        ImageFontPlugin,
    ))
    .init_collection::<DemoAssets>()
    .add_systems(Startup, setup)
    .insert_resource(ClearColor(Color::srgb(0.2, 0.2, 0.2)));

    #[cfg(feature = "gizmos")]
    app.add_systems(Update, common::gizmos::toggle_gizmos);

    app.run();
}

/// Spawns the text entities for the example.
///
/// This system creates two text entities:
/// 1. A text entity rendered at a scaled height with animated colors.
/// 2. A text entity rendered at its native height with animated content.
fn setup(
    mut commands: Commands,
    assets: Res<DemoAssets>,
    #[cfg(feature = "gizmos")] store: ResMut<GizmoConfigStore>,
) {
    #[cfg(feature = "gizmos")]
    common::gizmos::configure_gizmo_defaults(store);

    commands.spawn(Camera2d);

    for (i, &pangram) in PANGRAMS.iter().enumerate() {
        let font = match i % 3 {
            0 => assets.bmf_txt.clone(),
            1 => assets.bmf_xml.clone(),
            2 => assets.bmf_bin.clone(),
            _ => unreachable!(),
        };

        commands.spawn((
            ImageFontSpriteText::default(),
            ImageFontText::default().text(pangram).font(font),
            #[expect(
                clippy::cast_precision_loss,
                reason = "the magnitude of the numbers we're working on here are too small to lose \
                anything"
            )]
            Transform::from_translation(Vec3::new(
                0.,
                PANGRAMS.len() as f32 * 20. + -(i as f32) * 40.,
                0.,
            )),
        ));
    }
}
