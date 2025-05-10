// Based on
// <https://github.com/bevyengine/bevy/blob/main/examples/app/headless_renderer.rs>

//! 1. Render from camera to gpu-image render target
//! 2. Copy from gpu image to buffer using `ImageCopyDriver` node in
//!    `RenderGraph`
//! 3. Copy from buffer to channel using `receive_image_from_buffer` after
//!    `RenderSet::Render`
//! 4. Save from channel to random named file using `scene::update` at
//!    `PostUpdate` in `MainWorld`
//! 5. Exit if `single_image` setting is set

use bevy::sprite::Anchor;
use bevy::{prelude::*, reflect::Enum as _};
use bevy_asset_loader::asset_collection::AssetCollection;
use bevy_image_font::atlas_sprites::ImageFontSpriteText;
use bevy_image_font::rendered::ImageFontPreRenderedText;
use bevy_image_font::{ImageFont, ImageFontText, LetterSpacing};
use itertools::Itertools as _;

use crate::setup::prepare_app;

mod setup;

macro_rules! test_cases {
    ($category:ident => {$($name:ident,)+ }) => {
        paste::paste! {
            $(
                #[test]
                #[cfg_attr(ci, ignore)]
                fn [< $category _ $name >]() {
                    prepare_app(stringify!($category), stringify!($name), [< setup _ $category _ $name >]);
                }
            )+
        }
    };
    ($category:ident => {$($name:ident:$custom_name:ident:$val:expr,)+ }) => {
        paste::paste! {
            $(
                #[test]
                #[cfg_attr(ci, ignore)]
                fn [< $category _ $name >]() {
                    prepare_app(
                        stringify!($category),
                        stringify!($name),
                        (|| $val).pipe([< setup _ $category _ $custom_name >]));
                }
            )+
        }
    };
}

test_cases!(rendered => {
    base_alignment,
    manual_positioning,
    sizes,
});

test_cases!(rendered => {
    thirds_alignment  :custom_alignment:3,
    quarters_alignment:custom_alignment:4,
    fifths_alignment  :custom_alignment:5,
});

test_cases!(sprites => {
    base_alignment,
    manual_positioning,
    sizes,
    spacing,
});

test_cases!(sprites => {
    thirds_alignment  :custom_alignment:3,
    quarters_alignment:custom_alignment:4,
    fifths_alignment  :custom_alignment:5,
});

fn setup_rendered_base_alignment(commands: Commands, assets: Res<TestAssets>) {
    setup_base_alignment(commands, assets, |anchor| {
        (
            ImageFontPreRenderedText::default(),
            Sprite {
                anchor,
                ..default()
            },
        )
    });
}

fn setup_sprites_base_alignment(commands: Commands, assets: Res<TestAssets>) {
    setup_base_alignment(commands, assets, |anchor| {
        ImageFontSpriteText::default().anchor(anchor)
    });
}

fn setup_base_alignment<B: Bundle>(
    mut commands: Commands,
    assets: Res<TestAssets>,
    mut setup_component: impl FnMut(Anchor) -> B,
) {
    use Anchor::*;
    for anchor in [
        Center,
        BottomLeft,
        BottomCenter,
        BottomRight,
        CenterLeft,
        CenterRight,
        TopLeft,
        TopCenter,
        TopRight,
    ] {
        setup_anchored_text(&mut commands, &assets, anchor, setup_component(anchor));
    }
}

fn setup_rendered_custom_alignment(steps: In<i8>, commands: Commands, assets: Res<TestAssets>) {
    setup_custom_alignment(steps.0, commands, assets, |anchor| {
        (
            ImageFontPreRenderedText::default(),
            Sprite {
                anchor,
                ..default()
            },
        )
    });
}

fn setup_sprites_custom_alignment(steps: In<i8>, commands: Commands, assets: Res<TestAssets>) {
    setup_custom_alignment(steps.0, commands, assets, |anchor| {
        ImageFontSpriteText::default().anchor(anchor)
    });
}

fn setup_custom_alignment<B: Bundle>(
    steps: i8,
    mut commands: Commands,
    assets: Res<TestAssets>,
    setup_component: impl Fn(Anchor) -> B,
) {
    for anchor in custom(steps) {
        setup_anchored_text(&mut commands, &assets, anchor, setup_component(anchor));
    }
}

fn custom(steps: i8) -> impl Iterator<Item = Anchor> {
    itertools::iproduct!(-steps..=steps, -steps..=steps).map(move |(x, y)| {
        Anchor::Custom(Vec2::new(
            f32::from(x) / f32::from(steps) / 2.,
            f32::from(y) / f32::from(steps) / 2.,
        ))
    })
}

fn setup_anchored_text(
    commands: &mut Commands,
    assets: &TestAssets,
    anchor: Anchor,
    text_render_components: impl Bundle,
) {
    let anchor_vec = anchor.as_vec();
    let text = match anchor {
        Anchor::Custom(vec) => format!("({:.2}, {:.2})", vec.x, vec.y),
        Anchor::Center
        | Anchor::BottomLeft
        | Anchor::BottomCenter
        | Anchor::BottomRight
        | Anchor::CenterLeft
        | Anchor::CenterRight
        | Anchor::TopLeft
        | Anchor::TopCenter
        | Anchor::TopRight => anchor.variant_name().to_owned(),
    };

    commands.spawn((
        text_render_components,
        ImageFontText::default()
            .text(text)
            .font(assets.image_font.clone()),
        #[expect(
            clippy::cast_precision_loss,
            reason = "the magnitude of the numbers we're working on here are too small to lose \
                anything"
        )]
        Transform::from_translation(Vec3::new(
            (anchor_vec.x * SCREENSHOT_WIDTH as f32).round(),
            (anchor_vec.y * SCREENSHOT_HEIGHT as f32).round(),
            0.0,
        )),
    ));
}

const SCREENSHOT_WIDTH: u32 = 1920;
const SCREENSHOT_HEIGHT: u32 = 1080;

const CHARACTER_WIDTH: u32 = 5;
const CHARACTER_HEIGHT: u32 = 12;

#[expect(
    clippy::cast_precision_loss,
    reason = "the magnitude of the numbers we're working on here are too small to lose \
        anything"
)]
const TOP_LEFT_ORIGIN: Vec2 = Vec2::new(
    -(SCREENSHOT_WIDTH as f32 / 2.),
    SCREENSHOT_HEIGHT as f32 / 2.,
);

const GRID_WIDTH: u32 = 71;
const GRID_HEIGHT: u32 = 90;
const PADDING: f32 = 2.0;

#[expect(
    clippy::cast_precision_loss,
    reason = "the magnitude of the numbers we're working on here are too small to lose \
        anything"
)]
fn setup_rendered_manual_positioning(mut commands: Commands, assets: Res<TestAssets>) {
    for (x, y) in (0..GRID_WIDTH).cartesian_product(0..GRID_HEIGHT) {
        let text = format!("{x:02}.{y:02}");
        let text_width = text.len() as f32 * CHARACTER_WIDTH as f32;
        commands.spawn((
            ImageFontPreRenderedText::default(),
            Sprite {
                anchor: Anchor::TopLeft,
                ..default()
            },
            ImageFontText::default()
                .text(text)
                .font(assets.image_font.clone()),
            Transform::from_translation(
                (TOP_LEFT_ORIGIN
                    + Vec2::new(
                        x as f32 * (text_width + PADDING),
                        -(y as f32 * (CHARACTER_HEIGHT as f32) + PADDING),
                    ))
                .extend(0.),
            ),
        ));
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "the magnitude of the numbers we're working on here are too small to lose \
        anything"
)]
fn setup_sprites_manual_positioning(mut commands: Commands, assets: Res<TestAssets>) {
    for (x, y) in (0..GRID_WIDTH).cartesian_product(0..GRID_HEIGHT) {
        let text = format!("{x:02}.{y:02}");
        let text_width = text.len() as f32 * CHARACTER_WIDTH as f32;
        commands.spawn((
            ImageFontSpriteText::default().anchor(Anchor::TopLeft),
            ImageFontText::default()
                .text(text)
                .font(assets.image_font.clone()),
            Transform::from_translation(
                (TOP_LEFT_ORIGIN
                    + Vec2::new(
                        x as f32 * (text_width + PADDING),
                        -(y as f32 * (CHARACTER_HEIGHT as f32) + PADDING),
                    ))
                .extend(0.),
            ),
        ));
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "the magnitude of the numbers we're working on here are too small to lose \
        anything"
)]
fn setup_rendered_sizes(mut commands: Commands, assets: Res<TestAssets>) {
    let mut y = 0.;
    for size_multiplier in 1..14 {
        let size = CHARACTER_HEIGHT * size_multiplier;
        let text = format!("This text is size {size}");

        commands.spawn((
            ImageFontPreRenderedText::default(),
            ImageFontText::default()
                .text(text)
                .font(assets.image_font.clone())
                .font_height(size as f32),
            Sprite {
                anchor: Anchor::TopLeft,
                ..default()
            },
            Transform::from_translation((TOP_LEFT_ORIGIN + Vec2::new(0., -y)).extend(0.)),
        ));

        y += size as f32 + 2.;
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "the magnitude of the numbers we're working on here are too small to lose \
        anything"
)]
fn setup_sprites_sizes(mut commands: Commands, assets: Res<TestAssets>) {
    let mut y = 0.;
    for size_multiplier in 1..14 {
        let size = CHARACTER_HEIGHT * size_multiplier;
        let text = format!("This text is size {size}");

        commands.spawn((
            ImageFontSpriteText::default().anchor(Anchor::TopLeft),
            ImageFontText::default()
                .text(text)
                .font(assets.image_font.clone())
                .font_height(size as f32),
            Transform::from_translation((TOP_LEFT_ORIGIN + Vec2::new(0., -y)).extend(0.)),
        ));

        y += size as f32 + 2.;
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "the magnitude of the numbers we're working on here are too small to lose \
        anything"
)]
fn setup_sprites_spacing(mut commands: Commands, assets: Res<TestAssets>) {
    let mut y = 0.;
    for spacing in 0..16 {
        let size = CHARACTER_HEIGHT * 4;
        let text = format!("This text has spacing {spacing}");

        commands.spawn((
            ImageFontSpriteText::default()
                .anchor(Anchor::TopLeft)
                .letter_spacing(LetterSpacing::Pixel(spacing)),
            ImageFontText::default()
                .text(text)
                .font(assets.image_font.clone())
                .font_height(size as f32),
            Transform::from_translation((TOP_LEFT_ORIGIN + Vec2::new(0., -y)).extend(0.)),
        ));

        y += size as f32 + 20.;
    }
}

#[derive(AssetCollection, Resource)]
struct TestAssets {
    #[asset(path = "example_font.image_font.ron")]
    image_font: Handle<ImageFont>,
}
