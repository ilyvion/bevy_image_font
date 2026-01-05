#![allow(unused, reason = "not all items are used in every test case")]
//
// Based on
// <https://github.com/bevyengine/bevy/blob/main/examples/app/headless_renderer.rs>

//! 1. Render from camera to gpu-image render target
//! 2. Copy from gpu image to buffer using `ImageCopyDriver` node in
//!    `RenderGraph`
//! 3. Copy from buffer to channel using `receive_image_from_buffer` after
//!    `RenderSystems::Render`
//! 4. Save from channel to random named file using `scene::update` at
//!    `PostUpdate` in `MainWorld`
//! 5. Exit if `single_image` setting is set

use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy_asset_loader::asset_collection::AssetCollection;
#[cfg(feature = "atlas_sprites")]
use bevy_image_font::atlas_sprites::ImageFontSpriteText;
use bevy_image_font::rendered::ImageFontPreRenderedText;
use bevy_image_font::{ImageFont, ImageFontText, LetterSpacing};
use itertools::Itertools as _;

#[expect(unused_macro_rules, reason = "only one rule is used per test file")]
macro_rules! test_case {
    ($category:ident, $name:ident) => {
        paste::paste! {
            #[test]
            #[cfg_attr(ci, ignore = "Does not work when headless")]
            fn [< $category _ $name >]() {
                $crate::setup::prepare_app(
                    stringify!($category),
                    stringify!($name),
                    $crate::visual_acceptance_testing::[< setup _ $category _ $name >],
                );
            }
        }
    };
    ($category:ident, $name:ident:$custom_name:ident:$val:expr_2021) => {
        paste::paste! {
            #[test]
            #[cfg_attr(ci, ignore = "Does not work when headless")]
            fn [< $category _ $name >]() {
                use bevy_ecs::system::IntoSystem;
                $crate::setup::prepare_app(
                    stringify!($category),
                    stringify!($name),
                    (|| $val).pipe($crate::visual_acceptance_testing::[< setup _ $category _ $custom_name >]));
            }
        }
    };
}

enum AnchorWithFormat {
    Custom(Anchor),
    Named(Anchor),
}

impl AnchorWithFormat {
    fn as_vec(&self) -> Vec2 {
        #[expect(
            clippy::match_same_arms,
            reason = "the arms represent different types of anchors"
        )]
        match *self {
            AnchorWithFormat::Custom(anchor) => anchor.as_vec(),
            AnchorWithFormat::Named(anchor) => anchor.as_vec(),
        }
    }
}

pub(crate) fn setup_rendered_base_alignment(commands: Commands, assets: Res<TestAssets>) {
    setup_base_alignment(commands, assets, |anchor| {
        (
            ImageFontPreRenderedText::default(),
            anchor,
            Sprite::default(),
        )
    });
}

#[cfg(feature = "atlas_sprites")]
pub(crate) fn setup_sprites_base_alignment(commands: Commands, assets: Res<TestAssets>) {
    setup_base_alignment(commands, assets, |anchor| {
        ImageFontSpriteText::default().anchor(anchor)
    });
}

fn setup_base_alignment<B: Bundle>(
    mut commands: Commands,
    assets: Res<TestAssets>,
    mut setup_component: impl FnMut(Anchor) -> B,
) {
    for anchor in [
        Anchor::CENTER,
        Anchor::BOTTOM_LEFT,
        Anchor::BOTTOM_CENTER,
        Anchor::BOTTOM_RIGHT,
        Anchor::CENTER_LEFT,
        Anchor::CENTER_RIGHT,
        Anchor::TOP_LEFT,
        Anchor::TOP_CENTER,
        Anchor::TOP_RIGHT,
    ] {
        setup_anchored_text(
            &mut commands,
            &assets,
            AnchorWithFormat::Named(anchor),
            setup_component(anchor),
        );
    }
}

pub(crate) fn setup_rendered_custom_alignment(
    steps: In<i8>,
    commands: Commands,
    assets: Res<TestAssets>,
) {
    setup_custom_alignment(steps.0, commands, assets, |anchor| {
        (
            ImageFontPreRenderedText::default(),
            anchor,
            Sprite::default(),
        )
    });
}

#[cfg(feature = "atlas_sprites")]
pub(crate) fn setup_sprites_custom_alignment(
    steps: In<i8>,
    commands: Commands,
    assets: Res<TestAssets>,
) {
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
        setup_anchored_text(
            &mut commands,
            &assets,
            AnchorWithFormat::Custom(anchor),
            setup_component(anchor),
        );
    }
}

fn custom(steps: i8) -> impl Iterator<Item = Anchor> {
    itertools::iproduct!(-steps..=steps, -steps..=steps).map(move |(x, y)| {
        Anchor(Vec2::new(
            f32::from(x) / f32::from(steps) / 2.,
            f32::from(y) / f32::from(steps) / 2.,
        ))
    })
}

fn setup_anchored_text(
    commands: &mut Commands,
    assets: &TestAssets,
    anchor: AnchorWithFormat,
    text_render_components: impl Bundle,
) {
    let anchor_vec = anchor.as_vec();
    let text = match anchor {
        AnchorWithFormat::Named(anchor) => match anchor {
            Anchor::CENTER => "Center".to_owned(),
            Anchor::BOTTOM_LEFT => "BottomLeft".to_owned(),
            Anchor::BOTTOM_CENTER => "BottomCenter".to_owned(),
            Anchor::BOTTOM_RIGHT => "BottomRight".to_owned(),
            Anchor::CENTER_LEFT => "CenterLeft".to_owned(),
            Anchor::CENTER_RIGHT => "CenterRight".to_owned(),
            Anchor::TOP_LEFT => "TopLeft".to_owned(),
            Anchor::TOP_CENTER => "TopCenter".to_owned(),
            Anchor::TOP_RIGHT => "TopRight".to_owned(),
            Anchor(anchor) => panic!("Non-named anchor passed as named: {anchor:?}"),
        },
        AnchorWithFormat::Custom(_anchor) => format!("({:.2}, {:.2})", anchor_vec.x, anchor_vec.y),
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

pub(crate) const SCREENSHOT_WIDTH: u32 = 1920;
pub(crate) const SCREENSHOT_HEIGHT: u32 = 1080;

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
pub(crate) fn setup_rendered_manual_positioning(mut commands: Commands, assets: Res<TestAssets>) {
    for (x, y) in (0..GRID_WIDTH).cartesian_product(0..GRID_HEIGHT) {
        let text = format!("{x:02}.{y:02}");
        let text_width = text.len() as f32 * CHARACTER_WIDTH as f32;
        commands.spawn((
            ImageFontPreRenderedText::default(),
            Anchor::TOP_LEFT,
            Sprite::default(),
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
#[cfg(feature = "atlas_sprites")]
pub(crate) fn setup_sprites_manual_positioning(mut commands: Commands, assets: Res<TestAssets>) {
    for (x, y) in (0..GRID_WIDTH).cartesian_product(0..GRID_HEIGHT) {
        let text = format!("{x:02}.{y:02}");
        let text_width = text.len() as f32 * CHARACTER_WIDTH as f32;
        commands.spawn((
            ImageFontSpriteText::default().anchor(Anchor::TOP_LEFT),
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
pub(crate) fn setup_rendered_sizes(mut commands: Commands, assets: Res<TestAssets>) {
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
            Anchor::TOP_LEFT,
            Sprite::default(),
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
#[cfg(feature = "atlas_sprites")]
pub(crate) fn setup_sprites_sizes(mut commands: Commands, assets: Res<TestAssets>) {
    let mut y = 0.;
    for size_multiplier in 1..14 {
        let size = CHARACTER_HEIGHT * size_multiplier;
        let text = format!("This text is size {size}");

        commands.spawn((
            ImageFontSpriteText::default().anchor(Anchor::TOP_LEFT),
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
#[cfg(feature = "atlas_sprites")]
pub(crate) fn setup_sprites_spacing(mut commands: Commands, assets: Res<TestAssets>) {
    let mut y = 0.;
    for spacing in 0..16 {
        let size = CHARACTER_HEIGHT * 4;
        let text = format!("This text has spacing {spacing}");

        commands.spawn((
            ImageFontSpriteText::default()
                .anchor(Anchor::TOP_LEFT)
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
pub(crate) struct TestAssets {
    #[asset(path = "example_font.image_font.ron")]
    image_font: Handle<ImageFont>,
}
