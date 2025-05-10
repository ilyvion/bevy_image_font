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
use bevy::color::palettes::tailwind;
use bevy::color::Srgba;
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
