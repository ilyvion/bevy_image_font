#![allow(
    dead_code,
    reason = "private utility code that, depending on the activated feature set, will sometimes be missing uses"
)]

use std::sync::LazyLock;

use bevy_asset::{AssetPlugin, AssetServer};
use bevy_image::{CompressedImageFormats, ImageLoader};
use bevy_log::LogPlugin;
use bevy_platform::collections::HashMap;
#[cfg(feature = "ui")]
use bevy_ui::UiScale;
use camino::Utf8Path;

use super::*;

pub(crate) fn initialize_app_with_example_font(font: ExampleFont) -> (App, Handle<ImageFont>) {
    let font = match font {
        ExampleFont::Monospace => "example_font.image_font.ron",
        ExampleFont::VariableWidth => "example_variable_width_font.image_font.ron",
    };
    initialize_app_with_font(font)
}

pub(crate) fn initialize_app_with_loaded_example_font(
    font: ExampleFont,
) -> (App, Handle<ImageFont>) {
    let (mut app, handle) = initialize_app_with_example_font(font);

    wait_until_loaded(&mut app, &handle);

    (app, handle)
}

pub(crate) enum ExampleFont {
    Monospace,
    VariableWidth,
}

pub(crate) fn wait_until_loaded<T: Asset>(app: &mut App, handle: &Handle<T>) {
    while {
        let asset_server = app.world().resource::<AssetServer>();
        let load_state = asset_server.get_load_state(handle.id());

        !load_state
            .expect("Expected load_state to be Some")
            .is_loaded()
    } {
        app.update();
    }
}

fn initialize_app_with_font(font_path: impl AsRef<Utf8Path>) -> (App, Handle<ImageFont>) {
    let mut app = App::new();

    app.add_plugins((
        bevy_app::TaskPoolPlugin::default(),
        bevy_time::TimePlugin,
        bevy_app::ScheduleRunnerPlugin::default(),
        AssetPlugin::default(),
        LogPlugin::default(),
    ));
    app.add_plugins(ImageFontPlugin);

    #[cfg(feature = "ui")]
    app.init_resource::<UiScale>();

    app.register_asset_loader(ImageLoader::new(CompressedImageFormats::NONE))
        .init_asset::<TextureAtlasLayout>()
        .init_asset::<Image>();

    // Verify that `ImageFont` is registered as an asset by attempting to load one
    let asset_server = app.world().resource::<AssetServer>();

    let handle: Handle<ImageFont> = asset_server.load(font_path.as_ref().as_std_path());

    (app, handle)
}

/// The standard width of characters in the monospace font.
///
/// This value is used to do math that involves the font width. Adjust as needed
/// if the monospace font is later changed.
pub(crate) const MONOSPACE_FONT_WIDTH: u32 = 5;

/// The standard height of characters in the monospace font.
///
/// This value is used to do math that involves the font height. Adjust as
/// needed if the monospace font is later changed.
pub(crate) const MONOSPACE_FONT_HEIGHT: u32 = 12;

/// The tolerance used for checking that floating point values are the expected
/// value
pub(crate) const COMPARISON_TOLERANCE: f32 = 0.01;

/// The standard height of characters in the variable width font.
///
/// This value is used to do math that involves the font height. Adjust as
/// needed if the monospace font is later changed.
pub(crate) const VARIABLE_WIDTH_FONT_HEIGHT: u32 = 8;

/// The width of characters in the variable width font.
///
/// This value is used to do math that involves the font width. Adjust as needed
/// if the variable width font is later changed.
pub(crate) static VARIABLE_WIDTH_FONT_CHARACTER_WIDTHS: LazyLock<HashMap<char, u32>> =
    LazyLock::new(|| {
        [
            ('!', 4),
            ('"', 7),
            ('#', 9),
            ('$', 7),
            ('%', 10),
            ('&', 9),
            ('\'', 4),
            ('(', 5),
            (')', 5),
            ('*', 6),
            ('+', 8),
            (',', 5),
            ('-', 6),
            ('.', 4),
            ('/', 6),
            ('0', 7),
            ('1', 4),
            ('2', 7),
            ('3', 7),
            ('4', 7),
            ('5', 7),
            ('6', 7),
            ('7', 7),
            ('8', 7),
            ('9', 7),
            (':', 4),
            (';', 4),
            ('<', 6),
            ('=', 6),
            ('>', 6),
            ('?', 8),
            ('@', 8),
            ('A', 7),
            ('B', 7),
            ('C', 7),
            ('D', 7),
            ('E', 7),
            ('F', 7),
            ('G', 7),
            ('H', 7),
            ('I', 4),
            ('J', 7),
            ('K', 7),
            ('L', 7),
            ('M', 9),
            ('N', 8),
            ('O', 7),
            ('P', 7),
            ('Q', 8),
            ('R', 7),
            ('S', 7),
            ('T', 8),
            ('U', 7),
            ('V', 7),
            ('W', 9),
            ('X', 7),
            ('Y', 8),
            ('Z', 7),
            ('[', 5),
            ('\\', 5),
            (']', 5),
            ('^', 8),
            ('_', 6),
            ('`', 5),
            ('{', 5),
            ('|', 4),
            ('}', 5),
            ('~', 8),
        ]
        .into()
    });
