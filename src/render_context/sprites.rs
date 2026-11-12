//! [`RenderContext`] methods for positioning and styling text as individual
//! glyph sprites.

use bevy_image::TextureAtlas;
use bevy_sprite::Sprite;
use bevy_transform::components::Transform;

use super::RenderContext;
use super::anchors::{self, AnchorExt as _, AnchorOffsets, ComputeTransformParams};
use crate::ImageFontOutlineLayers;

impl RenderContext<'_> {
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

    /// Calculates the total width of the rendered text as the sum of every
    /// character's advance (see [`Self::character_advance`]).
    ///
    /// # Returns
    /// The total width of the rendered text, in pixels, after applying the
    /// scaling factor.
    #[inline]
    pub(crate) fn text_width(&self) -> f32 {
        let mut text_width = 0.;

        let mut characters = self.filtered_text.filtered_chars().peekable();
        while let Some(character) = characters.next() {
            text_width += self.character_advance(character, characters.peek().copied());
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
        let rect = self.glyph_rect(character);
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

    /// The outline and glyph layers to draw a character with, if the text has
    /// an outline color and the character's texture has outline layers.
    fn outline_layers(&self, character: char) -> Option<&ImageFontOutlineLayers> {
        self.render_config.outline_color?;
        let image_font_character = &self.image_font.atlas_character_map[&character];
        self.image_font
            .page_outline_layers(image_font_character.page_index)
    }

    /// Constructs the sprite that draws a character's glyph in the text's
    /// color. When the character is drawn with an outline color, the sprite
    /// draws only the inside of the glyph, from the glyph layer, and
    /// [`Self::outline_sprite`] draws the outline.
    pub(crate) fn glyph_sprite(&self, character: char) -> Sprite {
        let image = self.outline_layers(character).map_or_else(
            || {
                let image_font_character = &self.image_font.atlas_character_map[&character];
                self.image_font.textures[image_font_character.page_index].clone()
            },
            |layers| layers.glyph.clone(),
        );
        Sprite {
            image,
            texture_atlas: Some(self.font_texture_atlas(character)),
            color: self.render_config.color,
            ..Sprite::default()
        }
    }

    /// Constructs the sprite that draws a character's outline in the text's
    /// outline color, to be drawn below the glyph sprites. Returns `None` if
    /// the character is not drawn with an outline color.
    pub(crate) fn outline_sprite(&self, character: char) -> Option<Sprite> {
        let layers = self.outline_layers(character)?;
        Some(Sprite {
            image: layers.outline.clone(),
            texture_atlas: Some(self.font_texture_atlas(character)),
            color: self.render_config.outline_color?,
            ..Sprite::default()
        })
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
            layout: self.image_font.atlas_layouts[image_font_character.page_index].clone(),
            index: image_font_character.character_index,
        }
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
        *x_pos += self.character_advance(character, next_character);

        let params = ComputeTransformParams {
            x_pos: x,
            scaled_text_width: self.text_width(),
            scaled_width: width,
            scaled_height: height,
            max_height: self.max_height(),
            character_offsets: self.character_offsets(character),
            scale: self.scale(),
        };
        self.anchor_offsets().compute_transform(params)
    }

    /// Computes the transform for a glyph sprite, like [`Self::transform`],
    /// with the glyph's edges snapped to whole units.
    ///
    /// A glyph edge on a half unit makes each pixel sample exactly between two
    /// texels, where the renderer may pick either one per pixel, producing gaps,
    /// stretching, and bleeding from neighboring atlas glyphs.
    ///
    /// # Parameters
    /// - `x_pos`: A mutable reference to the current x-position of the sprite.
    ///   This value is updated to reflect the position of the next sprite.
    /// - `character`: The character associated with the sprite.
    /// - `next_character`: The character following this one, used for
    ///   kerning.
    ///
    /// # Returns
    /// A [`Transform`] representing the snapped position and scale of the
    /// sprite.
    #[inline]
    pub(crate) fn sprite_transform(
        &self,
        x_pos: &mut f32,
        character: char,
        next_character: Option<char>,
    ) -> Transform {
        let transform = self.transform(x_pos, character, next_character);
        let glyph_size = self.glyph_rect(character).size().as_vec2();

        anchors::snap_to_whole_units(transform, glyph_size * transform.scale.truncate())
    }
}
