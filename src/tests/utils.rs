#![allow(
    dead_code,
    reason = "private utility code that, depending on the activated feature set, will sometimes be missing uses"
)]

use std::any::type_name;
use std::sync::LazyLock;

use bevy_asset::{AssetLoadError, AssetPlugin, AssetServer, LoadState};
use bevy_ecs::schedule::ScheduleLabel;
use bevy_ecs::system::IntoSystem;
use bevy_image::{CompressedImageFormats, ImageLoader};
use bevy_log::LogPlugin;
use bevy_platform::collections::HashMap;
use camino::Utf8Path;

use super::*;

pub(crate) fn initialize_app_with_example_font(font: ExampleFont) -> (App, Handle<ImageFont>) {
    let font = match font {
        ExampleFont::Monospace => "example_font.image_font.ron",
        ExampleFont::VariableWidth => "example_variable_width_font.image_font.ron",
        #[cfg(feature = "bmf")]
        ExampleFont::BmFont => "bmf.txt.fnt",
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
    /// `bmf.txt.fnt`, whose glyphs have offsets, advances and kerning. See
    /// [`BMFONT_LINE_HEIGHT`].
    #[cfg(feature = "bmf")]
    BmFont,
}

pub(crate) fn wait_until_loaded<T: Asset>(app: &mut App, handle: &Handle<T>) {
    while {
        let asset_server = app.world().resource::<AssetServer>();
        let load_state = asset_server
            .get_load_state(handle.id())
            .expect("Expected load_state to be Some");
        if let LoadState::Failed(ref error) = load_state {
            panic!("Asset failed to load: {error}");
        }

        !load_state.is_loaded()
    } {
        app.update();
    }
}

/// Updates `app` until the asset of `handle` fails to load, and returns the
/// error. Panics if it loads.
pub(crate) fn wait_until_failed<T: Asset>(app: &mut App, handle: &Handle<T>) -> AssetLoadError {
    loop {
        let asset_server = app.world().resource::<AssetServer>();
        let load_state = asset_server
            .get_load_state(handle.id())
            .expect("Expected load_state to be Some");
        if let LoadState::Failed(ref error) = load_state {
            return (**error).clone();
        }
        assert!(!load_state.is_loaded(), "Asset loaded without failing");

        app.update();
    }
}

pub(crate) fn initialize_app_with_font(
    font_path: impl AsRef<Utf8Path>,
) -> (App, Handle<ImageFont>) {
    let mut app = App::new();

    app.add_plugins((
        bevy_app::TaskPoolPlugin::default(),
        bevy_time::TimePlugin,
        bevy_app::ScheduleRunnerPlugin::default(),
        AssetPlugin::default(),
        LogPlugin::default(),
    ));
    #[cfg(feature = "gizmos")]
    app.init_asset::<bevy_gizmos::GizmoAsset>();
    app.add_plugins(ImageFontPlugin);

    app.register_asset_loader(ImageLoader::new(CompressedImageFormats::NONE))
        .init_asset::<TextureAtlasLayout>()
        .init_asset::<Image>();

    // Verify that `ImageFont` is registered as an asset by attempting to load one
    let asset_server = app.world().resource::<AssetServer>();

    let handle: Handle<ImageFont> =
        asset_server.load(font_path.as_ref().to_owned().into_std_path_buf());

    (app, handle)
}

/// Asserts that `app` runs `system` in its `schedule`, as part of
/// [`ImageFontSet`].
pub(crate) fn assert_in_image_font_set<Marker, S: IntoSystem<(), (), Marker>>(
    app: &mut App,
    schedule: impl ScheduleLabel,
    system: S,
) {
    let system_name = type_name::<S>();
    let system_type = system.system_type_id();
    app.world_mut().schedule_scope(schedule, |world, schedule| {
        schedule.initialize(world).unwrap();
        let matching_systems: Vec<_> = schedule
            .systems()
            .unwrap()
            .filter(|&(_, schedule_system)| schedule_system.system_type() == system_type)
            .map(|(key, _)| key)
            .collect();
        assert_eq!(
            matching_systems.len(),
            1,
            "Expected `{system_name}` to be in the schedule exactly once"
        );
        let systems_in_set = schedule
            .graph()
            .systems_in_set(ImageFontSet.intern())
            .unwrap();
        assert!(
            systems_in_set.contains(&matching_systems[0]),
            "Expected `{system_name}` to be in `ImageFontSet`"
        );
    });
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

/// The `lineHeight` of the [`ExampleFont::BmFont`] font.
///
/// Glyph metrics used by tests, as `(width, height, xoffset, yoffset,
/// xadvance)`: `A` (23, 23, -1, 6, 21), `V` (22, 23, 0, 6, 21), `.` (5, 3, 2,
/// 26, 9), `a` (16, 17, 0, 12, 17), `g` (16, 23, 0, 12, 17), `H` (19, 23, 2, 6,
/// 23), `e` and `o` (17, 17, 0, 12, 17), `l` (5, 23, 1, 6, 7). The kerning
/// between `A` and `V` is -2.
#[cfg(feature = "bmf")]
pub(crate) const BMFONT_LINE_HEIGHT: u32 = 36;

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
