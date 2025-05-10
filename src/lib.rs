#![doc = include_str!("../README.md")]
//
// only enables the `doc_cfg` feature when
// the `docsrs` configuration attribute is defined
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

use bevy::{
    prelude::*,
    platform::collections::{HashMap, HashSet},
};
use bevy_image::{Image, ImageSampler};
use derive_setters::Setters;

#[cfg(any(feature = "rendered", feature = "atlas_sprites"))]
mod filtered_string;

pub mod loader;
#[cfg(feature = "fnt")]
pub mod loader_fnt;

#[cfg(feature = "rendered")]
pub mod rendered;

#[cfg(feature = "atlas_sprites")]
pub mod atlas_sprites;

/// A Bevy plugin for rendering image-based fonts.
///
/// This plugin enables support for fonts stored as single images (e.g., PNG),
/// where each glyph is represented by a section of the image. It handles:
///
/// - Loading `ImageFont` assets, which describe the glyph layout.
/// - Registering the `ImageFont` and `ImageFontText` types for use in your app.
/// - Marking updated fonts as dirty, ensuring proper re-rendering.
///
/// ### Features
/// The plugin conditionally includes additional functionality based on enabled
/// features:
/// - `rendered`: Enables support for rendering image fonts.
/// - `atlas_sprites`: Enables support for more advanced atlas-based sprite
///   functionality.
///
/// ### Usage
/// To use this plugin, add it to your Bevy app:
/// ```rust,no_run
/// use bevy::prelude::*;
/// use bevy_image_font::ImageFontPlugin;
///
/// App::new()
///     .add_plugins(DefaultPlugins)
///     .add_plugins(ImageFontPlugin)
///     .run();
/// ```
///
/// Ensure that `ImageFont` assets are properly loaded and configured using the
/// asset system, and consider the relevant examples in the documentation for
/// advanced use cases.

#[derive(Debug, Default)]
pub struct ImageFontPlugin;

impl Plugin for ImageFontPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<ImageFont>()
            .init_asset_loader::<loader::ImageFontLoader>()
            .register_type::<ImageFont>()
            .register_type::<ImageFontText>()
            .add_systems(PostUpdate, sync_texts_with_font_changes);

        #[cfg(feature = "rendered")]
        app.add_plugins(rendered::RenderedPlugin);

        #[cfg(feature = "fnt")]
        app.init_asset_loader::<loader_fnt::ImageFntFontLoader>();

        #[cfg(feature = "atlas_sprites")]
        app.add_plugins(atlas_sprites::AtlasSpritesPlugin);
    }
}

/// A system set containing all systems related to the [`ImageFontPlugin`].
///
/// This can be used to group, disable, or reorder systems provided by
/// the plugin.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, SystemSet)]
pub struct ImageFontSet;

/// An image font as well as the mapping of characters to regions inside it.
#[derive(Debug, Clone, Reflect, Asset)]
#[reflect(opaque)]
pub struct ImageFont {
    /// The layout of the texture atlas, describing the positioning and sizes
    /// of glyphs within the image font. This layout is used to map character
    /// regions within the texture.
    pub atlas_layout: Handle<TextureAtlasLayout>,
    /// The image that contains the font glyphs. Each glyph is a section of
    /// this texture, defined by the `atlas_layout` and `atlas_character_map`.
    pub texture: Handle<Image>,
    /// The glyph used to render `c` is contained in the part of the image
    /// pointed to by `atlas.textures[atlas_character_map[c]]`.
    pub atlas_character_map: HashMap<char, usize>,
    /// The [`ImageSampler`] to use during font image rendering. The default is
    /// `nearest`, which scales an image without blurring, keeping the text
    /// crisp and pixellated.
    pub image_sampler: ImageSampler,
}

impl ImageFont {
    /// Creates a character map and a texture atlas layout from a map of
    /// character rectangles.
    ///
    /// This method processes a map of characters to their bounding rectangles
    /// within a texture and generates both:
    /// 1. A character map (`atlas_character_map`) that maps each character to
    ///    its index in the texture atlas.
    /// 2. A texture atlas layout (`TextureAtlasLayout`) containing the bounding
    ///    rectangles of all characters.
    ///
    /// # Parameters
    /// - `size`: The size of the texture (width and height in pixels).
    /// - `char_rect_map`: A map of characters to their corresponding bounding
    ///   rectangles in the texture.
    ///
    /// # Returns
    /// A tuple containing:
    /// - `HashMap<char, usize>`: A map of characters to their indices in the
    ///   atlas.
    /// - `TextureAtlasLayout`: The texture atlas layout with the bounding
    ///   rectangles.
    fn mapped_atlas_layout_from_char_map(
        size: UVec2,
        char_rect_map: &HashMap<char, URect>,
    ) -> (HashMap<char, usize>, TextureAtlasLayout) {
        let mut atlas_character_map = HashMap::new();
        let mut atlas_layout = TextureAtlasLayout::new_empty(size);
        for (&character, &rect) in char_rect_map {
            atlas_character_map.insert(character, atlas_layout.add_texture(rect));
        }

        (atlas_character_map, atlas_layout)
    }

    /// Constructs an `ImageFont` instance from a precomputed atlas layout and
    /// character map.
    ///
    /// This function creates an `ImageFont` by combining the texture containing
    /// the font, a character map that maps characters to indices, a precomputed
    /// texture atlas layout, and an image sampler for scaling behavior.
    ///
    /// # Parameters
    /// - `texture`: A handle to the texture containing the font glyphs.
    /// - `atlas_character_map`: A map of characters to their indices in the
    ///   texture atlas.
    /// - `atlas_layout`: A handle to the texture atlas layout describing the
    ///   glyph bounds.
    /// - `image_sampler`: The image sampler used for scaling during rendering.
    ///
    /// # Returns
    /// An `ImageFont` instance ready to be used for rendering text.
    fn from_mapped_atlas_layout(
        texture: Handle<Image>,
        atlas_character_map: HashMap<char, usize>,
        atlas_layout: Handle<TextureAtlasLayout>,
        image_sampler: ImageSampler,
    ) -> Self {
        Self {
            atlas_layout,
            texture,
            atlas_character_map,
            image_sampler,
        }
    }

    /// Filters a string to include only characters present in the font's
    /// character map.
    ///
    /// This function returns a
    /// [`FilteredString`](filtered_string::FilteredString) containing only the
    /// characters from the input string that exist in the font's
    /// `atlas_character_map`. It ensures that unsupported characters are
    /// excluded during rendering.
    ///
    /// # Parameters
    /// - `string`: The input string to filter.
    ///
    /// # Returns
    /// A `FilteredString` returning only characters supported by the font.
    ///
    /// # Notes
    /// This function requires either the `rendered` or `atlas_sprites` feature
    /// to be enabled.
    #[cfg(any(feature = "rendered", feature = "atlas_sprites"))]
    fn filter_string<S: AsRef<str>>(&self, string: S) -> filtered_string::FilteredString<'_, S> {
        filtered_string::FilteredString::new(string, &self.atlas_character_map)
    }
}

/// Text rendered using an [`ImageFont`].
#[derive(Debug, Clone, Reflect, Default, Component, Setters)]
#[setters(into)]
pub struct ImageFontText {
    /// The string of text to be rendered. Each character in the string is
    /// mapped to a corresponding glyph in the associated [`ImageFont`].
    pub text: String,
    /// The handle to the [`ImageFont`] used to render this text. The font's
    /// texture and atlas mapping determine how characters are displayed.
    pub font: Handle<ImageFont>,
    /// If set, overrides the height the font is rendered at. This should be an
    /// integer multiple of the 'native' height if you want pixel accuracy,
    /// but we allow float values for things like animations.
    pub font_height: Option<f32>,
}

/// How kerning between characters is specified.
#[derive(Debug, Clone, Copy, Reflect)]
pub enum LetterSpacing {
    /// Kerning as an integer value, use this when you want a pixel-perfect
    /// spacing between characters.
    Pixel(i16),
    /// Kerning as a floating point value, use this when you want precise
    /// control over the spacing between characters and don't care about
    /// pixel-perfectness.
    Floating(f32),
}

impl Default for LetterSpacing {
    /// Zero constant spacing between character
    fn default() -> Self {
        Self::Pixel(0)
    }
}

impl From<LetterSpacing> for f32 {
    fn from(spacing: LetterSpacing) -> f32 {
        match spacing {
            LetterSpacing::Pixel(pixels) => f32::from(pixels),
            LetterSpacing::Floating(value) => value,
        }
    }
}

/// Marks any text where the underlying [`ImageFont`] asset has changed as
/// changed, which will cause it to be re-rendered.
#[expect(
    private_interfaces,
    reason = "Systems are only `pub` for the sake of allowing dependent crates to use them for ordering"
)]
pub fn sync_texts_with_font_changes(
    mut events: EventReader<AssetEvent<ImageFont>>,
    mut query: Query<&mut ImageFontText>,
    mut changed_fonts: Local<CachedHashSet>,
) {
    // Extract relevant IDs from events
    for id in events.read().filter_map(extract_asset_id) {
        info!("Image font {id} finished loading; marking as dirty");
        changed_fonts.insert(id);
    }

    // Update query for affected fonts
    for mut image_font_text in &mut query {
        if changed_fonts.contains(&image_font_text.font.id()) {
            image_font_text.set_changed();
        }
    }

    // Reset the local state
    changed_fonts.clear();
}

/// Extracts the asset ID from an [`AssetEvent`] for an [`ImageFont`] asset.
///
/// This helper function processes asset events and retrieves the relevant
/// asset ID if the event indicates that the asset was either modified or
/// loaded. Other event types are ignored.
///
/// # Parameters
/// - `event`: An [`AssetEvent`] for an [`ImageFont`] asset. This event
///   represents changes to assets, such as when they are modified or loaded.
///
/// # Returns
/// An [`Option`] containing the asset ID if the event type is `Modified` or
/// `LoadedWithDependencies`; otherwise, `None`.
///
/// # Notes
/// - This function is used to track changes to `ImageFont` assets in order to
///   trigger updates in dependent components or systems.
/// - The other events are irrelevant to our needs.
#[inline]
fn extract_asset_id(event: &AssetEvent<ImageFont>) -> Option<AssetId<ImageFont>> {
    match *event {
        AssetEvent::Modified { id } | AssetEvent::LoadedWithDependencies { id } => Some(id),
        AssetEvent::Added { .. } | AssetEvent::Removed { .. } | AssetEvent::Unused { .. } => None,
    }
}

/// A cached set of asset IDs used for tracking changes to [`ImageFont`] assets.
///
/// This struct wraps a [`HashSet`] of asset IDs and is used  to temporarily
/// store and manage asset IDs during a single update cycle. It serves as a way
/// to avoid having to re-allocate a `HashSet` each time he sync function runs.
///
/// The cache is cleared at the end of each update cycle to ensure it does not
/// persist between runs.
///
/// # Notes
/// - This struct is primarily used in the [`sync_texts_with_font_changes`]
///   system to keep track of `ImageFont` assets that have been modified or
///   loaded.
#[derive(Default, Deref, DerefMut)]
struct CachedHashSet(HashSet<AssetId<ImageFont>>);

#[cfg(test)]
mod tests;
