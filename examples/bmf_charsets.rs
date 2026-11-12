//! Demonstrates a non-Unicode `BMFont` font, which needs the `bmf_charsets`
//! feature.
//!
//! `cp437_10x10.txt.fnt` is a code page 437 font: its `charset` is `OEM` and
//! its character ids are the font's 256 code page 437 codes, laid out as a
//! 16×16 grid of 10×10 glyphs in `cp437_10x10_rgba.png`. The loader decodes
//! the ids, so the text below is ordinary Unicode, including the box-drawing,
//! shading and Greek characters code page 437 is known for.
//!
//! The text is drawn twice, as sprite text and as pre-rendered text.

use bevy::color::palettes::tailwind;
use bevy::prelude::*;
use bevy_asset_loader::asset_collection::AssetCollection;
use bevy_asset_loader::prelude::AssetCollectionApp as _;
use bevy_image_font::atlas_sprites::ImageFontSpriteText;
use bevy_image_font::rendered::ImageFontPreRenderedText;
use bevy_image_font::{ImageFont, ImageFontPlugin, ImageFontText};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(ImagePlugin::default_nearest()),
            ImageFontPlugin,
        ))
        .init_collection::<Cp437Assets>()
        .add_systems(Startup, setup)
        .insert_resource(ClearColor(Color::srgb(0.1, 0.1, 0.3)))
        .run();
}

/// The font used in this example.
#[derive(AssetCollection, Resource)]
struct Cp437Assets {
    /// A code page 437 `BMFont` font.
    #[asset(path = "cp437_10x10.txt.fnt")]
    cp437: Handle<ImageFont>,
}

/// The lines of text inside the box.
const CONTENT: [&str; 4] = [
    "Code page 437 BMFont",
    "Ñandú, café, Smörgåsbord",
    "αßΓπΣσµΦΘΩδ ∞≈±√°½¼",
    "░▒▓█ ▄▌▐▀ ¢£¥₧ƒ",
];

/// The height each line is drawn at.
const LINE_HEIGHT: f32 = 20.;

/// The lines of [`CONTENT`] inside a box drawn with box-drawing characters.
fn boxed_lines() -> Vec<String> {
    let width = CONTENT
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or_default();
    let border = "═".repeat(width + 2);

    let mut lines = vec![format!("╔{border}╗")];
    lines.extend(CONTENT.iter().map(|line| {
        let padding = " ".repeat(width - line.chars().count());
        format!("║ {line}{padding} ║")
    }));
    lines.push(format!("╚{border}╝"));
    lines
}

/// Spawns the boxed text once as sprite text and once as pre-rendered text,
/// one entity per line.
fn setup(mut commands: Commands, assets: Res<Cp437Assets>) {
    commands.spawn(Camera2d);

    let lines = boxed_lines();
    #[expect(
        clippy::cast_precision_loss,
        reason = "the magnitude of the numbers we're working on here are too small to lose \
        anything"
    )]
    let box_height = lines.len() as f32 * LINE_HEIGHT;

    for (pre_rendered, box_top) in [(false, box_height + 20.), (true, -20.)] {
        for (line_no, line) in lines.iter().enumerate() {
            let font_text = ImageFontText::default()
                .text(line.clone())
                .font(assets.cp437.clone())
                .font_height(LINE_HEIGHT);
            #[expect(
                clippy::cast_precision_loss,
                reason = "the magnitude of the numbers we're working on here are too small to \
                lose anything"
            )]
            let transform = Transform::from_translation(Vec3::new(
                0.,
                box_top - line_no as f32 * LINE_HEIGHT,
                0.,
            ));

            if pre_rendered {
                commands.spawn((
                    ImageFontPreRenderedText::default().color(tailwind::SKY_300),
                    font_text,
                    transform,
                ));
            } else {
                commands.spawn((
                    ImageFontSpriteText::default().color(tailwind::YELLOW_300),
                    font_text,
                    transform,
                ));
            }
        }
    }
}
