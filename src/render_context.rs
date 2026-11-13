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

#[cfg(feature = "atlas_sprites")]
mod anchors;
mod filtered_string;
#[cfg(feature = "atlas_sprites")]
mod sprites;

use std::cell::Cell;
use std::fmt::Debug;

use bevy_asset::Assets;
use bevy_image::TextureAtlasLayout;
use bevy_math::{URect, Vec2};

use crate::ImageFontScalingMode;
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
    /// The lines of the text, each filtered to include only supported
    /// characters in the font atlas.
    lines: Vec<FilteredString<'assets, &'assets str>>,

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

        let lines = image_font_text
            .text
            .split('\n')
            .map(|line| image_font.filter_string(line.strip_suffix('\r').unwrap_or(line)))
            .collect();

        Some(RenderContext {
            atlas_layouts,
            image_font,
            image_font_text,
            render_config,
            lines,

            max_height: CacheCell::default(),
        })
    }

    /// Calculates the height of the line of text at the font's native size.
    ///
    /// This is the font's [`ImageFont::line_height`] if it has one, and
    /// otherwise the height of the tallest glyph in the filtered text.
    ///
    /// # Returns
    /// The line height, in pixels.
    #[inline]
    pub(crate) fn max_height(&self) -> u32 {
        self.max_height.get_or_insert_with(|| {
            if let Some(line_height) = self.image_font.line_height {
                return line_height.max(1);
            }

            let mut max_height = 1;

            for character in self.filtered_chars() {
                let image_font_character = &self.image_font.atlas_character_map[&character];
                let rect = self.atlas_layouts[image_font_character.page_index].textures
                    [image_font_character.character_index];
                max_height = max_height.max(rect.height());
            }

            max_height
        })
    }

    /// Computes how far the pen moves after drawing a character: its x advance
    /// (or glyph width if it has none), plus letter spacing and the kerning
    /// towards `next_character`.
    ///
    /// When [`RenderConfig::apply_scaling`] is set and the text has a font
    /// height, the advance is scaled by the font height over the line height
    /// using [`RenderConfig::scaling_mode`].
    ///
    /// # Parameters
    /// - `character`: The character being drawn.
    /// - `next_character`: The character following this one, if any.
    ///
    /// # Returns
    /// The distance to move the pen, in pixels.
    #[expect(
        clippy::cast_precision_loss,
        reason = "the magnitude of the numbers we're working on here are too small to lose anything"
    )]
    pub(crate) fn character_advance(&self, character: char, next_character: Option<char>) -> f32 {
        let image_font_character = &self.image_font.atlas_character_map[&character];
        let rect = self.glyph_rect(character);
        let kerning = next_character
            .and_then(|next_character| self.character_kerning(character, next_character))
            .unwrap_or(0.0);
        let advance = image_font_character
            .x_advance
            .unwrap_or(rect.width() as f32)
            + self.render_config.letter_spacing
            + kerning;

        if let Some(font_height) = self.image_font_text.font_height
            && self.render_config.apply_scaling
        {
            let scale_factor = font_height / self.max_height() as f32;
            return self
                .render_config
                .scaling_mode
                .apply_scale(advance, scale_factor);
        }

        advance
    }

    /// Retrieves the region of the font texture holding a character's glyph.
    ///
    /// # Parameters
    /// - `character`: The character whose glyph region should be retrieved.
    ///
    /// # Returns
    /// The glyph's rectangle within its texture atlas page.
    #[inline]
    pub(crate) fn glyph_rect(&self, character: char) -> URect {
        let image_font_character = &self.image_font.atlas_character_map[&character];
        self.atlas_layouts[image_font_character.page_index].textures
            [image_font_character.character_index]
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

    /// Returns the lines of the text, each including only characters supported
    /// by the font.
    ///
    /// The text is split at every `\n`, and a `\r` directly before it is
    /// dropped. Each line starts at the left edge, [`Self::max_height`] below
    /// the one before it.
    #[inline]
    pub(crate) fn lines(&self) -> &[FilteredString<'assets, &'assets str>] {
        &self.lines
    }

    /// Returns the characters of every line that the font supports, in text
    /// order.
    pub(crate) fn filtered_chars(&self) -> impl Iterator<Item = char> + '_ {
        self.lines.iter().flat_map(FilteredString::filtered_chars)
    }

    /// Checks whether no line has any character the font supports.
    #[cfg(feature = "rendered")]
    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.lines.iter().all(FilteredString::is_empty)
    }

    /// Calculates the height of the text block at the font's native size: the
    /// number of lines times [`Self::max_height`].
    ///
    /// # Returns
    /// The text height, in pixels.
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a text with more than u32::MAX lines can't be rendered anyway"
    )]
    #[inline]
    pub(crate) fn text_height(&self) -> u32 {
        self.lines.len() as u32 * self.max_height()
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
    #[cfg(feature = "atlas_sprites")]
    pub text_anchor: bevy_sprite::Anchor,

    /// Determines whether individual characters should have per-character
    /// offsets applied.
    ///
    /// When `true`, additional spacing adjustments are made based on each
    /// character's properties. This setting affects how characters are
    /// positioned relative to each other.
    #[cfg(feature = "atlas_sprites")]
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
    #[cfg(feature = "atlas_sprites")]
    pub color: bevy_color::Color,

    /// The color of the outline of fonts that have one. When set, glyphs on
    /// textures with outline layers are drawn as an outline in this color
    /// with the glyph in [`Self::color`] on top.
    #[cfg(feature = "atlas_sprites")]
    pub outline_color: Option<bevy_color::Color>,
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
