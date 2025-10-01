#![doc = include_str!("../README.md")]
//
// only enables the `doc_cfg` feature when
// the `docsrs` configuration attribute is defined
#![cfg_attr(docsrs, feature(doc_cfg))]

use bevy_app::{App, Plugin, PostUpdate};
#[cfg(feature = "rendered")]
use bevy_asset::Assets;
use bevy_asset::{Asset, AssetApp as _, AssetEvent, AssetId, Handle};
use bevy_derive::{Deref, DerefMut};
#[cfg(feature = "ui")]
use bevy_ecs::system::Res;
use bevy_ecs::{
    change_detection::DetectChangesMut as _,
    component::Component,
    event::EventReader,
    schedule::SystemSet,
    system::{Local, Query},
};
use bevy_image::{Image, ImageSampler, TextureAtlasLayout};
use bevy_math::{URect, UVec2, Vec2};
use bevy_platform::collections::{HashMap, HashSet};
use bevy_reflect::Reflect;
#[cfg(feature = "ui")]
use bevy_ui::UiScale;
use derive_setters::Setters;

mod letter_spacing;
#[cfg(any(feature = "rendered", feature = "atlas_sprites"))]
mod render_context;
mod scaling_mode;

pub use letter_spacing::*;
pub use scaling_mode::*;

pub mod loader;

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

        #[cfg(feature = "bmf")]
        app.init_asset_loader::<loader::BmFontLoader>();

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
#[derive(Debug, Clone, Reflect, Asset, Default)]
#[reflect(opaque)]
#[non_exhaustive]
pub struct ImageFont {
    /// The layouts of the texture atlases describing the positioning and sizes
    /// of glyphs within the image font. This layout is used to map characters
    /// to regions within the texture.
    pub atlas_layouts: Vec<Handle<TextureAtlasLayout>>,
    /// The images that contain the font glyphs. Each glyph is a section of one
    /// of these textures, as defined by the `atlas_layout` and
    /// `atlas_character_map` fields.
    pub textures: Vec<Handle<Image>>,
    /// The information required to render the character `c` in
    /// `atlas_character_map[c]` is stored here.
    pub atlas_character_map: HashMap<char, ImageFontCharacter>,
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
    /// - `HashMap<char, ImageFontCharacter>`: A map of characters to their
    ///   atlas placement, including texture page index and character index.
    /// - `TextureAtlasLayout`: The texture atlas layout with the bounding
    ///   rectangles.
    fn mapped_atlas_layout_from_char_map(
        page: usize,
        size: UVec2,
        char_rect_mapping: impl Iterator<Item = (char, URect)>,
    ) -> (HashMap<char, ImageFontCharacter>, TextureAtlasLayout) {
        let mut atlas_character_map = HashMap::new();
        let mut atlas_layout = TextureAtlasLayout::new_empty(size);
        for (character, rect) in char_rect_mapping {
            atlas_character_map.insert(
                character,
                ImageFontCharacter {
                    page_index: page,
                    character_index: atlas_layout.add_texture(rect),
                    ..Default::default()
                },
            );
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
    fn new(
        texture: Vec<Handle<Image>>,
        atlas_character_map: HashMap<char, ImageFontCharacter>,
        atlas_layout: Vec<Handle<TextureAtlasLayout>>,
        image_sampler: ImageSampler,
    ) -> Self {
        Self {
            atlas_layouts: atlas_layout,
            textures: texture,
            atlas_character_map,
            image_sampler,
            // size: default(),
            // padding: default(),
            // spacing: default(),
        }
    }

    /// Retrieves references to the font's textures.
    ///
    /// # Parameters
    /// - `image_assets`: The asset storage for images.
    ///
    /// # Returns
    /// A vector of references to the images in use by this font.
    #[cfg(feature = "rendered")]
    fn textures<'assets>(&self, image_assets: &'assets Assets<Image>) -> Vec<&'assets Image> {
        self.textures
            .iter()
            .map(|handle| {
                #[expect(clippy::expect_used, reason = "handle is kept alive by ImageFont")]
                image_assets
                    .get(handle)
                    .expect("handle is kept alive by ImageFont")
            })
            .collect()
    }
}

/// Represents a character in an [`ImageFont`], storing metadata required for
/// rendering.
///
/// This struct contains information about a specific character in the font,
/// including its location in the texture atlas and any additional properties
/// that may be useful for rendering, alignment, or future extensions.
///
/// # Fields
/// - `character_index`: The index of the character's glyph in the texture
///   atlas.
/// - `page_index`: The index of the texture atlas page where this character's
///   glyph is stored.
/// - *(Planned: Additional metadata fields, such as offsets, kerning, or
///   stylistic variants.)*
///
/// # Future Considerations
/// This struct is designed to be extensible. In the future, it may include:
/// - **Kerning Information:** Adjustments for character spacing.
/// - **Per-Character Offsets:** Fine-tuned positioning for different glyphs.
/// - **Stylistic Variants:** Alternative representations of characters.
#[derive(Clone, Debug, Default, Reflect)]
#[non_exhaustive]
pub struct ImageFontCharacter {
    /// The index of this character's glyph in the texture atlas given by
    /// `atlas_page`.
    ///
    /// This value corresponds to an entry in the [`TextureAtlasLayout`],
    /// determining the region of the texture where this character's glyph
    /// is located.
    pub character_index: usize,

    /// The index of the texture atlas page where this character's glyph is
    /// stored.
    ///
    /// When a font spans multiple textures, this field identifies which
    /// specific texture contains the glyph.
    pub page_index: usize,

    /// How to move the character relative to the baseline.
    pub offsets: Vec2,

    /// How much to advance the x position after rendering the character. `None`
    /// means use character width.
    pub x_advance: Option<f32>,

    /// Kerning information for this character. This is a map of following
    /// characters and the amount of kerning to apply between them.
    pub kernings: HashMap<char, f32>,
}

/// Text rendered using an [`ImageFont`].
#[derive(Debug, Clone, Reflect, Default, Component, Setters)]
#[setters(into)]
#[non_exhaustive]
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
    #[doc(alias = "line_height")]
    pub font_height: Option<f32>,
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
    #[cfg(feature = "ui")] ui_scale: Res<UiScale>,
) {
    #[cfg(feature = "ui")]
    {
        // If the UI scale changed, mark all texts as changed

        use bevy_ecs::change_detection::DetectChanges as _;
        if ui_scale.is_changed() && !ui_scale.is_added() {
            for mut image_font_text in &mut query {
                use bevy_ecs::change_detection::DetectChangesMut as _;

                image_font_text.set_changed();
            }
            changed_fonts.clear();
            return;
        }
    }

    // Extract relevant IDs from events
    for id in events.read().filter_map(extract_asset_id) {
        bevy_log::info!("Image font {id} finished loading; marking as dirty");
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
