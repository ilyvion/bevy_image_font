//! Demonstrates multi-line text: each `\n` in the text starts a new line at
//! the left edge, one line height below the previous one.
//!
//! The same text is drawn three times:
//! - As sprite text (`ImageFontSpriteText`) with the example image font.
//! - As pre-rendered text (`ImageFontPreRenderedText`) with the example image
//!   font.
//! - As sprite text with a `BMFont` font, whose lines are spaced by the font's
//!   `lineHeight`. Its heading uses Windows (`\r\n`) line endings, which break
//!   lines the same way as `\n`.

#![expect(
    clippy::mod_module_files,
    reason = "if present as common.rs, cargo thinks it's an example binary"
)]

use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy_asset_loader::prelude::AssetCollectionApp as _;
use bevy_image_font::atlas_sprites::ImageFontSpriteText;
use bevy_image_font::rendered::ImageFontPreRenderedText;
use bevy_image_font::{ImageFontPlugin, ImageFontText};

use crate::common::{DemoAssets, RAINBOW};

mod common;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            ImageFontPlugin,
        ))
        .init_collection::<DemoAssets>()
        .add_systems(Startup, setup)
        .insert_resource(ClearColor(Color::srgb(0.2, 0.2, 0.2)))
        .run();
}

/// The text shown below each heading, split over several lines.
const TEXT: &str = "Sphinx of\nblack quartz,\njudge my vow!";

/// Spawns the three multi-line texts side by side, each anchored at its
/// top-left corner.
fn setup(mut commands: Commands, assets: Res<DemoAssets>) {
    commands.spawn(Camera2d);

    commands.spawn((
        ImageFontSpriteText::default()
            .anchor(Anchor::TOP_LEFT)
            .color(RAINBOW[0]),
        ImageFontText::default()
            .text(format!("Atlased sprites\n\n{TEXT}"))
            .font(assets.example.clone())
            .font_height(24.0),
        Transform::from_translation(Vec3::new(-600., 150., 0.)),
    ));

    // Pre-rendered text is a single sprite holding every line, so the sprite's
    // anchor positions the whole block.
    commands.spawn((
        ImageFontPreRenderedText::default().color(RAINBOW[3]),
        ImageFontText::default()
            .text(format!("Rendered sprite\n\n{TEXT}"))
            .font(assets.example.clone())
            .font_height(24.0),
        Anchor::TOP_LEFT,
        Transform::from_translation(Vec3::new(-200., 150., 0.)),
    ));

    commands.spawn((
        ImageFontSpriteText::default()
            .anchor(Anchor::TOP_LEFT)
            .color(RAINBOW[4]),
        ImageFontText::default()
            .text(format!("BMFont\r\n\r\n{TEXT}"))
            .font(assets.bmf_txt.clone()),
        Transform::from_translation(Vec3::new(200., 150., 0.)),
    ));
}
