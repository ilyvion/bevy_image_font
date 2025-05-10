//! Demonstrates rendering image font text at both its 'native' height and a
//! scaled-up height.

#![expect(
    clippy::mod_module_files,
    reason = "if present as common.rs, cargo thinks it's an example binary"
)]

use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy_asset_loader::prelude::AssetCollectionApp as _;
use bevy_image_font::rendered::ImageFontPreRenderedText;
use bevy_image_font::{ImageFontPlugin, ImageFontText};

use crate::common::{DemoAssets, FONT_WIDTH, TEXT};

mod common;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, ImageFontPlugin))
        .init_collection::<DemoAssets>()
        .add_systems(Startup, spawn_text)
        .insert_resource(ClearColor(Color::srgb(0.2, 0.2, 0.2)))
        .run();
}

/// Spawns the text entities for the example.
///
/// This system creates two text entities:
/// 1. A text entity rendered at a scaled-up height for demonstration purposes.
/// 2. A text entity rendered at its native height, demonstrating pixel-perfect
///    alignment.
///
/// The first entity shows how to shift the position by 0.5 pixels to align with
/// pixel boundaries. The second demonstrates the use of an anchor to achieve
/// proper alignment.
fn spawn_text(mut commands: Commands, assets: Res<DemoAssets>) {
    commands.spawn(Camera2d);

    commands.spawn((
        ImageFontPreRenderedText::default(),
        ImageFontText::default()
            .text(TEXT)
            .font(assets.example.clone())
            .font_height(36.0),
        // Our font is 45 pixels wide per character, and with an odd number of characters, the
        // text aligns to the middle of a pixel, causing imperfect rendering. Shifting the
        // position by 0.5 pixels ensures alignment to pixel boundaries, resulting in crisp,
        // pixel-perfect rendering.
        Transform::from_translation(Vec3::new(0.5, 0., 0.)),
    ));
    commands.spawn((
        ImageFontPreRenderedText::default(),
        ImageFontText::default()
            .text(TEXT)
            .font(assets.example.clone()),
        // Instead of shifting the character by 0.5 pixels when the text lands in the middle of
        // a pixel, we can anchor the sprite to an edge and move it by a whole number of pixels.
        // To center it with the text above, we shift it left by half its width.
        Sprite {
            anchor: Anchor::CenterLeft,
            ..default()
        },
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
}
