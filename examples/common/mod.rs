//! Shared utilities and constants for example binaries.
//!
//! This module provides common functionality and configuration shared across
//! the example binaries for the project. It includes reusable constants,
//! such as default text and color palettes, that are utilized to ensure
//! consistency and reduce duplication across examples.
//!
//! # Key Features
//! - **Default Text:** Includes a pangram for rendering demonstrations and
//!   testing.
//! - **Font Configuration:** Provides the font width of the example font.
//! - **Rainbow Colors:** Supplies a palette of colors for visual styling in
//!   examples.
//!
//! # Usage
//! This module is intended for internal use by example binaries. It reduces
//! redundancy by centralizing common assets and configurations. Depending on
//! the active feature set, some utilities may not be used in certain examples.

#![allow(
    dead_code,
    reason = "private utility code that, depending on the activated feature set, \
    will sometimes be missing uses"
)]

use bevy::asset::Handle;
use bevy::color::Srgba;
use bevy::color::palettes::tailwind;
use bevy::prelude::Resource;
use bevy_asset_loader::asset_collection::AssetCollection;
use bevy_image_font::ImageFont;

/// A pangram used for rendering demonstrations or testing in examples.
///
/// This sentence contains all the letters of the English alphabet,
/// making it ideal for visualizing font rendering and character spacing.
pub(crate) const TEXT: &str = "Sphinx of black quartz, judge my vow!";

/// The standard width of characters in the example font.
///
/// This value is used to do math that involves the font width. Adjust as needed
/// if the example font is later changed.
pub(crate) const FONT_WIDTH: usize = 5;

/// A vibrant palette of rainbow colors for visual effects in examples.
pub(crate) const RAINBOW: [Srgba; 7] = [
    tailwind::RED_300,
    tailwind::ORANGE_300,
    tailwind::YELLOW_300,
    tailwind::GREEN_300,
    tailwind::BLUE_300,
    tailwind::INDIGO_300,
    tailwind::VIOLET_300,
];

/// A pangram or holoalphabetic sentence is a sentence using every letter of a
/// given alphabet at least once. Here's a list of pangrams in various languages
/// using various scripts.
pub(crate) static PANGRAMS: [&str; 15] = [
    // English
    "The quick brown fox jumps over the lazy dog",
    // French
    "Portez ce vieux whisky au juge blond qui fume",
    // German
    "Victor jagt zwölf Boxkämpfer quer über den großen Sylter Deich",
    // Spanish
    "El veloz murciélago hindú comía feliz cardillo y kiwi",
    // Italian
    "Pranzo d'acqua fa volti sghembi",
    // Dutch
    "Pa's wijze lynx bezag vroom het fikse aquaduct",
    // Danish
    "Høj bly gom vandt fræk sexquiz på wc",
    // Norwegian
    "Sær golfer med kølle vant sexquiz på wc i hjemby",
    // Polish
    "Pchnąć w tę łódź jeża lub ośm skrzyń fig",
    // Czech
    "Příliš žluťoučký kůň úpěl ďábelské ódy",
    // Arabic
    "نص حكيم له سر قاطع وذو شأن عظيم مكتوب على ثوب أخضر ومغلف بجلد أزرق",
    // Hebrew
    "דג סקרן שט בים, מאוכזב ולפתע מצא חברה",
    // Russian
    "Съешь ещё этих мягких французских булок, да выпей же чаю",
    // Greek
    "Ξεσκεπάζω την ψυχοφθόρα βδελυγμία",
    // Armenian
    "Ֆիզիկոս Մկրտիչը օճառաջուր ցողելով բժշկում է գնդապետ Հայկի փքված ձախ թևը",
];

/// A resource containing the image font asset used in this example.
///
/// This struct uses `bevy_asset_loader`'s `AssetCollection` to load the image
/// font asset automatically during startup.
#[derive(AssetCollection, Resource)]
pub(crate) struct DemoAssets {
    /// The handle to the image font asset loaded from the specified RON file.
    #[asset(path = "example_font.image_font.ron")]
    pub(crate) example: Handle<ImageFont>,
    /// The handle to the image font asset loaded from the specified FNT file.
    #[asset(path = "bmf.txt.fnt")]
    pub(crate) bmf_txt: Handle<ImageFont>,
    /// The handle to the image font asset loaded from the specified FNT file.
    #[asset(path = "bmf.xml.fnt")]
    pub(crate) bmf_xml: Handle<ImageFont>,
    /// The handle to the image font asset loaded from the specified FNT file.
    #[asset(path = "bmf.bin.fnt")]
    pub(crate) bmf_bin: Handle<ImageFont>,
    /// The handle to the image font asset loaded from the specified RON file.
    #[asset(path = "example_variable_width_font.image_font.ron")]
    pub(crate) variable_width: Handle<ImageFont>,
}

/// Gizmos related example code
#[cfg(all(feature = "gizmos", feature = "atlas_sprites"))]
pub(crate) mod gizmos {
    use bevy::{
        ecs::system::{Res, ResMut},
        gizmos::config::GizmoConfigStore,
        input::{ButtonInput, keyboard::KeyCode},
    };
    use bevy_image_font::atlas_sprites::gizmos::AtlasSpritesGizmoConfigGroup;

    //use super::{ButtonInput, GizmoConfigStore, KeyCode, Res, ResMut};

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
    pub(crate) fn configure_gizmo_defaults(mut store: ResMut<'_, GizmoConfigStore>) {
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
    pub(crate) fn toggle_gizmos(
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
