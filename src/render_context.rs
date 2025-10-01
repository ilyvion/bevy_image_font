#![allow(clippy::unwrap_used, reason = "test code panics to indicate errors")]

//! This module provides the `RenderContext` type, which encapsulates
//! rendering-related computations and state for text sprites.
//!
//! # Purpose
//! The `RenderContext` simplifies and centralizes text rendering logic by
//! managing:
//! - Font assets, texture atlases, and associated metadata.
//! - Cached computations for glyph dimensions, text width, and alignment
//!   offsets.
//! - Handles text positioning and scaling for both `atlas_sprites`
//!   (sprite-based) and `rendered` (pre-rendered image-based) text rendering.
//!   This centralizes font asset access, text dimension calculations, and
//!   alignment logic.
//!
//! By consolidating these responsibilities, `RenderContext` reduces code
//! duplication and makes rendering logic more maintainable.
//!
//! # Key Features
//! - **Asset Management**: Fetches and stores references to the font and
//!   texture atlas assets.
//! - **Text Calculations**: Computes text dimensions, glyph dimensions, and
//!   scaling factors.
//! - **Unified Rendering Support**: Handles both `atlas_sprites` (sprite-based)
//!   and `rendered` (pre-rendered image-based) text rendering.
//! - **Caching**: Uses `CacheCell` to lazily compute and store values like
//!   maximum height and anchor offsets.
//!
//! This module is intended for internal use within the text rendering system
//! and is designed to work seamlessly with other components, such as
//! `SpriteContext`.

mod anchors;
mod filtered_string;

use std::cell::Cell;
use std::fmt::Debug;

use bevy_asset::Assets;
#[cfg(feature = "atlas_sprites")]
use bevy_asset::Handle;
use bevy_color::Color;
#[cfg(feature = "atlas_sprites")]
use bevy_image::Image;
use bevy_image::{TextureAtlas, TextureAtlasLayout};
use bevy_math::Vec2;
use bevy_sprite::Anchor;
use bevy_transform::components::Transform;

use crate::ImageFontScalingMode;
use crate::render_context::anchors::{AnchorExt as _, AnchorOffsets, ComputeTransformParams};
use crate::render_context::filtered_string::FilteredString;
use crate::{ImageFont, ImageFontText};

/// Groups font-related assets and configuration for rendering text sprites.
///
/// Includes references to the texture atlas layout, font asset, and the
/// font text component that defines the text content and font height.
pub(crate) struct RenderContext<'assets> {
    /// The texture atlas layout defining glyph placements.
    atlas_layouts: Vec<&'assets TextureAtlasLayout>,
    /// The font asset containing glyph metadata.
    image_font: &'assets ImageFont,
    /// The text component defining the content and font height.
    image_font_text: &'assets ImageFontText,
    /// Configuration for rendering the text, including anchor alignment,
    /// color, letter spacing, and scaling behavior.
    pub render_config: RenderConfig,
    /// The text filtered to include only supported characters in the font
    /// atlas.
    filtered_text: FilteredString<'assets, &'assets String>,

    /// Cached maximum glyph height.
    max_height: CacheCell<u32>,
}

impl<'assets> RenderContext<'assets> {
    /// Creates a new `RenderContext` for rendering text using an `ImageFont`.
    ///
    /// This function retrieves the necessary assets, filters the text to
    /// exclude unsupported characters, and initializes cached computations
    /// for rendering.
    ///
    /// # Parameters
    /// - `image_font`: A reference to the loaded `ImageFont` asset.
    /// - `image_font_text`: A reference to the `ImageFontText` component
    ///   containing the text.
    /// - `render_config`: Rendering options such as color, anchor alignment,
    ///   letter spacing, and scaling mode.
    /// - `texture_atlas_layouts`: The asset collection containing texture atlas
    ///   layouts.
    ///
    /// # Returns
    /// - `Some(RenderContext)`: If all required assets are available.
    /// - `None`: If the font or texture atlas layouts are missing.
    pub(crate) fn new(
        image_font: &'assets ImageFont,
        image_font_text: &'assets ImageFontText,
        render_config: RenderConfig,
        texture_atlas_layouts: &'assets Assets<TextureAtlasLayout>,
    ) -> Option<Self> {
        let atlas_layouts: Result<Vec<_>, _> = image_font
            .atlas_layouts
            .iter()
            .map(|texture_atlas_layout| {
                texture_atlas_layouts
                    .get(texture_atlas_layout)
                    .ok_or_else(|| {
                        format!("TextureAtlasLayout not loaded: {texture_atlas_layout:?}")
                    })
            })
            .collect();

        let atlas_layouts = match atlas_layouts {
            Ok(layout) => layout,
            Err(error) => {
                bevy_log::error!("{error}");
                return None;
            }
        };

        let filtered_text = image_font.filter_string(&image_font_text.text);

        Some(RenderContext {
            atlas_layouts,
            image_font,
            image_font_text,
            render_config,
            filtered_text,

            max_height: CacheCell::default(),
        })
    }

    /// Computes the uniform scaling factor for text glyphs.
    ///
    /// Determines the scaling factor to apply to glyph dimensions based on
    /// the specified font height and the maximum glyph height.
    ///
    /// # Returns
    /// An `f32` representing the uniform scaling factor for text sprites.
    #[expect(
        clippy::cast_precision_loss,
        reason = "`max_height` won't ever be particularly large"
    )]
    #[inline]
    pub(crate) fn scale(&self) -> f32 {
        let max_height = self.max_height();
        self.image_font_text
            .font_height
            .map_or(1.0, |font_height| font_height / max_height as f32)
    }

    /// Calculates the maximum height of the filtered text.
    ///
    /// Iterates over the filtered text characters to determine the overall
    /// height based on glyph sizes in the texture atlas.
    ///
    /// # Returns
    /// The height of the tallest glyph
    #[inline]
    pub(crate) fn max_height(&self) -> u32 {
        self.max_height.get_or_insert_with(|| {
            let mut max_height = 1;

            for character in self.filtered_text.filtered_chars() {
                let image_font_character = &self.image_font.atlas_character_map[&character];
                let rect = self.atlas_layouts[image_font_character.page_index].textures
                    [image_font_character.character_index];
                max_height = max_height.max(rect.height());
            }

            max_height
        })
    }

    /// Calculates the total width of the rendered text based on the filtered
    /// characters and glyph dimensions stored in the context.
    ///
    /// # Returns
    /// The total width of the rendered text, in pixels, after applying the
    /// scaling factor.
    #[inline]
    pub(crate) fn text_width(&self) -> f32 {
        let mut text_width = 0.;

        for character in self.filtered_text.filtered_chars() {
            let (width, _) = self.character_dimensions(character);
            text_width += width;
        }
        text_width
    }

    /// Computes the dimensions of a glyph for a given character, applying
    /// scaling if a specific font height is provided.
    ///
    /// The dimensions are determined using the character's bounding rectangle
    /// in the texture atlas, the configured letter spacing, and the
    /// selected `ScalingMode`. Additionally, if
    /// `RenderConfig::apply_scaling` is `true`, width scaling is
    /// applied before rounding or truncation, ensuring consistent proportions
    /// in certain scaling modes.
    ///
    /// # Parameters
    /// - `character`: The character whose dimensions are being computed.
    ///
    /// # Returns
    /// A tuple `(width, height)` representing the computed dimensions of the
    /// glyph, where width scaling behavior depends on
    /// `RenderConfig::apply_scaling`.
    #[expect(
        clippy::cast_precision_loss,
        reason = "the magnitude of the numbers we're working on here are too small to lose anything"
    )]
    pub(crate) fn character_dimensions(&self, character: char) -> (f32, f32) {
        let image_font_character = &self.image_font.atlas_character_map[&character];
        let rect = self.atlas_layouts[image_font_character.page_index].textures
            [image_font_character.character_index];
        let letter_spacing = self.render_config.letter_spacing;
        let width = rect.width() as f32 + letter_spacing;
        let height = rect.height() as f32;
        let max_height = self.max_height() as f32;

        if let Some(font_height) = self.image_font_text.font_height
            && self.render_config.apply_scaling
        {
            let scaling_mode = self.render_config.scaling_mode;
            let scale_factor = font_height / max_height;
            return (
                scaling_mode.apply_scale(width, scale_factor),
                scaling_mode.apply_scale(height, scale_factor),
            );
        }

        (width, height)
    }

    /// Retrieves the offset for positioning a specific character in the text
    /// layout.
    ///
    /// This offset is used to correctly align the character within the text,
    /// based on font metadata. It accounts for individual glyph positioning
    /// adjustments relative to the baseline.
    ///
    /// # Parameters
    /// - `character`: The character whose offset should be retrieved.
    ///
    /// # Returns
    /// A [`Vec2`] containing the X and Y offsets for the character.
    #[inline]
    pub(crate) fn character_offsets(&self, character: char) -> Vec2 {
        let image_font_character = &self.image_font.atlas_character_map[&character];
        image_font_character.offsets
    }

    /// Retrieves the horizontal advance for a given character.
    ///
    /// The advance value determines how much horizontal space the character
    /// should take up in the rendered text. It is used to correctly space
    /// characters relative to each other.
    ///
    /// # Parameters
    /// - `character`: The character whose advance width should be retrieved.
    ///
    /// # Returns
    /// - `Some(f32)`: If the font specifies an advance width for the character.
    /// - `None`: If no specific advance width is defined.
    #[inline]
    pub(crate) fn character_x_advance(&self, character: char) -> Option<f32> {
        let image_font_character = &self.image_font.atlas_character_map[&character];
        image_font_character.x_advance
    }

    /// Retrieves the kerning value between two characters, if defined in the
    /// font.
    ///
    /// Kerning adjusts the spacing between specific pairs of characters to
    /// improve the visual appearance of text. This function looks up the
    /// kerning value for the given character pair, returning the adjustment
    /// amount if it exists.
    ///
    /// # Parameters
    /// - `character`: The current character in the text.
    /// - `next_character`: The character immediately following `character`.
    ///
    /// # Returns
    /// - `Some(f32)`: The kerning adjustment to apply between the two
    ///   characters, if defined.
    /// - `None`: If no kerning adjustment is specified for the character pair.
    #[inline]
    pub(crate) fn character_kerning(&self, character: char, next_character: char) -> Option<f32> {
        let image_font_character = &self.image_font.atlas_character_map[&character];
        image_font_character.kernings.get(&next_character).copied()
    }

    /// Retrieves the handle to the font texture image.
    ///
    /// This handle is used to assign the appropriate image to a text sprite.
    #[inline]
    #[cfg(feature = "atlas_sprites")]
    pub(crate) fn font_image(&self, character: char) -> Handle<Image> {
        let image_font_character = &self.image_font.atlas_character_map[&character];

        self.image_font.textures[image_font_character.page_index].clone_weak()
    }

    /// Constructs the texture atlas entry for a specific character.
    ///
    /// This method provides the texture atlas layout and the character's index
    /// within the atlas.
    ///
    /// # Parameters
    /// - `character`: The character whose texture atlas entry is needed.
    ///
    /// # Returns
    /// A [`TextureAtlas`] structure containing the layout and character index.
    #[inline]
    pub(crate) fn font_texture_atlas(&self, character: char) -> TextureAtlas {
        let image_font_character = &self.image_font.atlas_character_map[&character];
        TextureAtlas {
            layout: self.image_font.atlas_layouts[image_font_character.page_index].clone_weak(),
            index: image_font_character.character_index,
        }
    }

    /// Returns the filtered text, which includes only characters supported by
    /// the font.
    ///
    /// The filtered text excludes unsupported or invalid characters, ensuring
    /// that only renderable glyphs are processed.
    #[inline]
    pub(crate) fn text(&self) -> &FilteredString<'_, &String> {
        &self.filtered_text
    }

    /// Updates the texture index for the specified character and assigns the
    /// configured color.
    ///
    /// This function assigns the correct glyph index from the font atlas to
    /// `texture_atlas`, ensuring that the correct character is selected for
    /// rendering. Additionally, it assigns the configured text color from
    /// `RenderConfig` to `color`.
    ///
    /// # Parameters
    /// - `character`: The character whose corresponding glyph should be used.
    /// - `texture_atlas`: The texture atlas entry that will be updated with the
    ///   character's index.
    /// - `color`: The variable that will be assigned the value of
    ///   `RenderConfig::color`.
    #[inline]
    pub(crate) fn update_render_values(
        &self,
        character: char,
        texture_atlas: &mut TextureAtlas,
        color: &mut Color,
    ) {
        texture_atlas.index = self.image_font.atlas_character_map[&character].character_index;
        *color = self.render_config.color;
    }

    /// Computes or retrieves the cached anchor offsets for the text and glyph
    /// alignment.
    ///
    /// The computed offsets depend on the `render_config.offset_characters`
    /// setting:
    /// - If `true`, per-character offsets are applied to adjust positioning.
    /// - If `false`, glyphs are aligned strictly according to the text anchor.
    ///
    /// # Returns
    /// An [`AnchorOffsets`] struct containing:
    /// - `whole`: Offset for aligning the entire text block.
    /// - `individual`: Offset for aligning each individual glyph.
    #[inline]
    pub(crate) fn anchor_offsets(&self) -> AnchorOffsets {
        self.render_config
            .text_anchor
            .to_anchor_offsets(self.render_config.offset_characters)
    }

    /// Computes the transform for positioning and scaling a text sprite.
    ///
    /// This method calculates the sprite's translation and scale based on:
    /// - The x-position of the sprite.
    /// - The dimensions of the character's glyph.
    /// - The alignment offsets and scaling configuration.
    ///
    /// # Parameters
    /// - `x_pos`: A mutable reference to the current x-position of the sprite.
    ///   This value is updated to reflect the position of the next sprite.
    /// - `character`: The character associated with the sprite.
    ///
    /// # Returns
    /// A [`Transform`] representing the position and scale of the sprite.
    #[inline]
    pub(crate) fn transform(
        &self,
        x_pos: &mut f32,
        character: char,
        next_character: Option<char>,
    ) -> Transform {
        let x = *x_pos;
        let (width, height) = self.character_dimensions(character);
        *x_pos += self.character_x_advance(character).unwrap_or(width);

        let params = ComputeTransformParams {
            x_pos: x,
            scaled_text_width: self.text_width(),
            scaled_width: width,
            scaled_height: height,
            max_height: self.max_height(),
            character_offsets: self.character_offsets(character),
            scale: self.scale(),
        };
        if let Some(next_character) = next_character
            && let Some(kerning) = self.character_kerning(character, next_character)
        {
            *x_pos += kerning;
        }
        self.anchor_offsets().compute_transform(params)
    }
}

/// Configuration settings for rendering text using an `ImageFont`.
///
/// This struct controls how text is rendered, including alignment, spacing,
/// scaling, and color settings. It is passed to `RenderContext` to determine
/// rendering behavior.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct RenderConfig {
    /// The anchor point used to align the rendered text.
    ///
    /// This defines how the text is positioned relative to its origin.
    /// For example, `Anchor::TopLeft` aligns the text's top-left corner to its
    /// position.
    pub text_anchor: Anchor,

    /// Determines whether individual characters should have per-character
    /// offsets applied.
    ///
    /// When `true`, additional spacing adjustments are made based on each
    /// character's properties. This setting affects how characters are
    /// positioned relative to each other.
    pub offset_characters: bool,

    /// Controls whether glyph dimensions should be scaled when a specific font
    /// height is set.
    ///
    /// When `true`, glyphs are scaled proportionally based on the desired font
    /// height. The scaling behavior is further influenced by the
    /// `scaling_mode` setting.
    pub apply_scaling: bool,

    /// The amount of space added between characters when rendering text.
    ///
    /// This value is specified at the font’s native height and is scaled
    /// accordingly when the text is resized.
    pub letter_spacing: f32,

    /// Determines how fractional values are handled when scaling glyph
    /// dimensions.
    ///
    /// This setting controls whether scaled dimensions are rounded, truncated,
    /// or left as floating-point values, influencing text rendering
    /// precision.
    pub scaling_mode: ImageFontScalingMode,

    /// The color applied to the rendered text.
    ///
    /// This affects all glyphs uniformly, allowing text to be tinted or styled
    /// dynamically.
    pub color: Color,
}

/// A lightweight wrapper around a [`Cell<Option<T>>`] for caching values.
///
/// This utility type provides a mechanism to lazily compute and cache a value
/// using a factory function. It is ideal for scenarios where a value is
/// expensive to compute but may not always be needed.
///
/// # Features
/// - Lazily initializes the cached value when accessed for the first time.
/// - Ensures the cached value is reused across multiple accesses.
/// - Supports types that implement the [`Copy`] trait.
struct CacheCell<T>(Cell<Option<T>>);

impl<T> CacheCell<T> {
    /// Retrieves the cached value, computing it if necessary.
    ///
    /// If the value is already cached, this method returns it directly. If not,
    /// it invokes the provided factory function to compute the value, caches
    /// it, and then returns it.
    ///
    /// # Parameters
    /// - `factory`: A closure that computes the value if it is not already
    ///   cached.
    ///
    /// # Returns
    /// The cached value, or the newly computed value if the cache was empty.
    fn get_or_insert_with<F>(&self, factory: F) -> T
    where
        T: Copy,
        F: FnOnce() -> T,
    {
        if let Some(value) = self.0.get() {
            value
        } else {
            let value = factory();
            self.0.set(Some(value));
            value
        }
    }
}

impl<T> Default for CacheCell<T> {
    /// Creates a new, empty `CacheCell`.
    ///
    /// The cache will initialize with no value and will compute the value on
    /// first access via `get_or_insert_with`.
    fn default() -> Self {
        Self(Cell::default())
    }
}

#[cfg(test)]
mod tests;
