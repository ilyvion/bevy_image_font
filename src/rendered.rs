//! This module provides functionality for rendering text as images in the Bevy
//! engine, utilizing custom image fonts. It includes components for
//! pre-rendering text for both in-world and UI contexts, as well as systems to
//! update and render the text when changes occur.
//!
//! Key Features:
//! - `ImageFontPreRenderedText` and `ImageFontPreRenderedUiText` components for
//!   in-world and UI text rendering, respectively.
//! - Systems for rendering text updates to `Sprite` or `ImageNode` components
//!   dynamically.
//! - Integrates with the `image` crate for low-level image manipulation.

use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::{Assets, Handle, RenderAssetUsages};
use bevy_color::{Color, ColorToPacked as _, LinearRgba, Srgba};
use bevy_ecs::{
    component::Component,
    query::{Changed, Or},
    schedule::IntoScheduleConfigs as _,
    system::{Query, Res, ResMut},
};
use bevy_image::{Image, ImageSampler, TextureAtlasLayout};
use bevy_math::{I64Vec2, UVec2};
use bevy_reflect::Reflect;
use bevy_render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_sprite::Sprite;
#[cfg(feature = "ui")]
use bevy_ui::widget::ImageNode;
use derive_setters::Setters;
use image::{
    GenericImageView as _, ImageBuffer, ImageError, Rgba,
    imageops::{self, FilterType},
};
use thiserror::Error;

use crate::render_context::{RenderConfig, RenderContext};
use crate::{
    ImageFont, ImageFontScalingMode, ImageFontSet, ImageFontText, sync_texts_with_font_changes,
};

/// Internal plugin for conveniently organizing the code related to this
/// module's feature.
#[derive(Default)]
pub(crate) struct RenderedPlugin;

impl Plugin for RenderedPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            render_text_to_sprite
                .after(sync_texts_with_font_changes)
                .in_set(ImageFontSet),
        );

        #[cfg(feature = "ui")]
        {
            use bevy_ui::widget::update_image_content_size_system;
            app.add_systems(
                PostUpdate,
                render_text_to_image_node
                    .in_set(ImageFontSet)
                    .before(update_image_content_size_system)
                    .after(sync_texts_with_font_changes),
            );
        }
    }
}

/// A component for displaying in-world text that has been pre-rendered using an
/// image font.
///
/// This component requires an `ImageFontText` component for determining its
/// font and text. It renders its text into an image and sets it as the texture
/// on its `Sprite` component. The `Sprite`'s color tints the whole image,
/// including any outline drawn in `outline_color`.
#[derive(Component, Debug, Clone, Reflect, Setters)]
#[setters(into)]
#[require(ImageFontText, Sprite)]
#[non_exhaustive]
pub struct ImageFontPreRenderedText {
    /// The color the glyphs are drawn in, as a tint multiplied with the font
    /// texture.
    ///
    /// The default value is white, which draws the glyphs as they are in the
    /// font texture.
    pub color: Color,

    /// The color of the outline, for fonts whose textures hold an outline
    /// around their glyphs (see [`ImageFont::outline_layers`]).
    ///
    /// When `None`, the outline keeps the color it has in the font texture,
    /// tinted by `color` like the rest of the glyph. When set, every outline is
    /// drawn in this color first, then the inside of every glyph in `color` on
    /// top. Fonts without an outline ignore this setting.
    ///
    /// The default value is `None`.
    pub outline_color: Option<Color>,
}

impl Default for ImageFontPreRenderedText {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            outline_color: None,
        }
    }
}

/// A component for displaying UI text that has been pre-rendered using an image
/// font.
///
/// This component requires an `ImageFontText` component for determining its
/// font and text. It renders its text into an image and sets it as the texture
/// on its `ImageNode` component. The `ImageNode`'s color tints the whole image,
/// including any outline drawn in `outline_color`.
#[derive(Component, Debug, Clone, Reflect, Setters)]
#[setters(into)]
#[cfg(feature = "ui")]
#[require(ImageFontText, ImageNode)]
#[non_exhaustive]
pub struct ImageFontPreRenderedUiText {
    /// The color the glyphs are drawn in. See
    /// [`ImageFontPreRenderedText::color`].
    pub color: Color,

    /// The color of the outline of fonts that have one. See
    /// [`ImageFontPreRenderedText::outline_color`].
    pub outline_color: Option<Color>,
}

#[cfg(feature = "ui")]
impl Default for ImageFontPreRenderedUiText {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            outline_color: None,
        }
    }
}

/// The colors pre-rendered text is drawn in.
#[derive(Debug, Clone, Copy, PartialEq)]
struct TextColors {
    /// The color the glyphs are tinted with.
    color: Color,
    /// The color of the outline of fonts that have one, if it is drawn in its
    /// own color.
    outline_color: Option<Color>,
}

impl Default for TextColors {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            outline_color: None,
        }
    }
}

impl From<&ImageFontPreRenderedText> for TextColors {
    fn from(text: &ImageFontPreRenderedText) -> Self {
        Self {
            color: text.color,
            outline_color: text.outline_color,
        }
    }
}

#[cfg(feature = "ui")]
impl From<&ImageFontPreRenderedUiText> for TextColors {
    fn from(text: &ImageFontPreRenderedUiText) -> Self {
        Self {
            color: text.color,
            outline_color: text.outline_color,
        }
    }
}

/// System that renders each [`ImageFontText`] into its [`Sprite`], in the
/// colors of its [`ImageFontPreRenderedText`]. This system only runs when the
/// `ImageFontText` or the `ImageFontPreRenderedText` changes.
pub fn render_text_to_sprite(
    mut query: Query<
        (
            &ImageFontText,
            Option<&ImageFontPreRenderedText>,
            &mut Sprite,
        ),
        Or<(Changed<ImageFontText>, Changed<ImageFontPreRenderedText>)>,
    >,
    image_fonts: Res<Assets<ImageFont>>,
    mut images: ResMut<Assets<Image>>,
    layouts: Res<Assets<TextureAtlasLayout>>,
) {
    render_text_to_image_holder(
        query
            .iter_mut()
            .map(|(image_font_text, pre_rendered, sprite)| {
                (
                    image_font_text,
                    pre_rendered.map_or_else(TextColors::default, TextColors::from),
                    sprite.into_inner(),
                )
            }),
        &image_fonts,
        &mut images,
        &layouts,
    );
}

#[cfg(feature = "ui")]
/// System that renders each [`ImageFontText`] into its [`ImageNode`], in the
/// colors of its [`ImageFontPreRenderedUiText`]. This system only runs when
/// the `ImageFontText` or the `ImageFontPreRenderedUiText` changes.
pub fn render_text_to_image_node(
    mut query: Query<
        (
            &ImageFontText,
            Option<&ImageFontPreRenderedUiText>,
            &mut ImageNode,
        ),
        Or<(Changed<ImageFontText>, Changed<ImageFontPreRenderedUiText>)>,
    >,
    image_fonts: Res<Assets<ImageFont>>,
    mut images: ResMut<Assets<Image>>,
    layouts: Res<Assets<TextureAtlasLayout>>,
) {
    render_text_to_image_holder(
        query
            .iter_mut()
            .map(|(image_font_text, pre_rendered, image_node)| {
                (
                    image_font_text,
                    pre_rendered.map_or_else(TextColors::default, TextColors::from),
                    image_node.into_inner(),
                )
            }),
        &image_fonts,
        &mut images,
        &layouts,
    );
}

/// Renders text into images and assigns the resulting image handles to their
/// holders.
///
/// This function is designed to work with any type that implements
/// [`ImageHandleHolder`], allowing it to be used with multiple components, such
/// as sprites and UI elements.
///
/// # Parameters
/// - `font_text_to_image_iter`: An iterator over each [`ImageFontText`], the
///   colors to draw it in, and a mutable reference to the object implementing
///   [`ImageHandleHolder`] that receives the rendered image.
/// - `image_fonts`: A reference to the font assets used for rendering.
/// - `images`: A mutable reference to the collection of image assets. This is
///   used to store the newly rendered images.
/// - `layouts`: A reference to the collection of texture atlas assets.
///
/// The function iterates over the provided items, renders the text for each
/// [`ImageFontText`], and updates the corresponding [`ImageHandleHolder`] with
/// the handle to the newly created image.
///
/// # Errors
/// If text rendering fails for an item (e.g., due to missing font assets
/// or invalid texture layouts), an error message is logged, and the
/// corresponding holder is not updated.
fn render_text_to_image_holder<'borrow>(
    font_text_to_image_iter: impl Iterator<
        Item = (
            &'borrow ImageFontText,
            TextColors,
            &'borrow mut (impl ImageHandleHolder + 'borrow),
        ),
    >,
    image_fonts: &Assets<ImageFont>,
    images: &mut Assets<Image>,
    layouts: &Assets<TextureAtlasLayout>,
) {
    for (image_font_text, colors, image_handle_holder) in font_text_to_image_iter {
        bevy_log::debug!("Rendering [{}]", image_font_text.text);
        match render_text_to_image(image_font_text, colors, image_fonts, images, layouts) {
            Ok(image) => {
                image_handle_holder.set_image_handle(images.add(image));
            }
            Err(error) => {
                bevy_log::error!(
                    "Error when rendering image font text {:?}: {}",
                    image_font_text,
                    error
                );
            }
        }
    }
}

/// Renders the text from an [`ImageFontText`] into a single image.
///
/// This function takes a reference to an [`ImageFontText`] component and
/// generates an image representation of the text. It applies font-specific
/// filtering, determines appropriate character placements, and composites the
/// final output.
///
/// # Parameters
/// - `image_font_text`: The text to render, along with its associated font.
/// - `colors`: The colors to draw the text in.
/// - `image_fonts`: The collection of available font assets.
/// - `images`: The collection of image assets used to retrieve font textures.
/// - `layouts`: The texture atlas layouts defining character positioning.
///
/// # Returns
/// A [`Result<Image, ImageFontRenderError>`] containing the generated image if
/// successful, or an error if rendering fails (e.g., due to missing assets).
///
/// # Behavior
/// - If the text is empty, a **1x1 transparent image** is returned to avoid
///   invalid texture sizes.
/// - The function leverages [`RenderContext`] to compute character positions
///   and generate the image.
fn render_text_to_image(
    image_font_text: &ImageFontText,
    colors: TextColors,
    image_fonts: &Assets<ImageFont>,
    images: &Assets<Image>,
    layouts: &Assets<TextureAtlasLayout>,
) -> Result<Image, ImageFontRenderError> {
    let (image_font, textures, render_context) =
        prepare_context(image_font_text, image_fonts, images, layouts)?;

    if render_context.is_empty() {
        // Can't make a 0x0 image, so make a 1x1 transparent black pixel
        return Ok(Image::new(
            Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            vec![0, 0, 0, 0],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        ));
    }

    let (placements, size) = layout_glyphs(&render_context);
    let mut output_image = image::RgbaImage::new(size.x, size.y);
    let pages = textures
        .iter()
        .enumerate()
        .map(|(page_index, texture)| {
            let texture = image_buffer(texture).ok_or(ImageFontRenderError::UnknownError)?;
            let outline_layers = colors
                .outline_color
                .and_then(|_| image_font.page_outline_layers(page_index))
                .map(|layers| {
                    let layer = |handle| {
                        let image = images
                            .get(handle)
                            .ok_or(ImageFontRenderError::MissingTextureAsset)?;
                        image_buffer(image).ok_or(ImageFontRenderError::UnknownError)
                    };
                    Ok::<_, ImageFontRenderError>(OutlineLayerBuffers {
                        outline: layer(&layers.outline)?,
                        glyph: layer(&layers.glyph)?,
                    })
                })
                .transpose()?;
            Ok(PageBuffers {
                texture,
                outline_layers,
            })
        })
        .collect::<Result<Vec<_>, ImageFontRenderError>>()?;

    render_glyphs_to_image(
        image_font,
        &render_context,
        &placements,
        &mut output_image,
        &pages,
        colors,
    );

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "the magnitude of the numbers we're working on here are too small to lose anything"
    )]
    if let Some(font_height) = image_font_text.font_height {
        let line_height = render_context.max_height() as f32;
        let scale_dimension =
            |dimension: u32| (dimension as f32 * font_height / line_height) as u32;

        output_image = imageops::resize(
            &output_image,
            scale_dimension(size.x),
            scale_dimension(size.y),
            FilterType::Nearest,
        );
    }

    let mut bevy_image = Image::new(
        Extent3d {
            width: output_image.width(),
            height: output_image.height(),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        output_image.into_vec(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    bevy_image.sampler = ImageSampler::nearest();

    Ok(bevy_image)
}

/// The position of a glyph's top-left corner in the rendered text image, at
/// the font's native size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GlyphPlacement {
    /// The character whose glyph is drawn.
    character: char,
    /// The glyph's position, in pixels from the image's top-left corner.
    position: I64Vec2,
}

/// Positions every glyph of the filtered text at the font's native size.
///
/// Each line starts at the left edge, one line height below the line before
/// it. Each glyph is drawn at the pen position plus its offsets, and the pen
/// then moves by the character's advance. The image covers every line (the
/// widest line's sum of advances by the line count times the line height) as
/// well as any glyph that extends past them.
///
/// # Parameters
/// - `render_context`: The context providing the filtered text and font
///   metrics.
///
/// # Returns
/// The glyph placements, and the size of the image that holds them.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    reason = "the magnitude of the numbers we're working on here are too small to lose anything"
)]
fn layout_glyphs(render_context: &RenderContext<'_>) -> (Vec<GlyphPlacement>, UVec2) {
    let mut placements = Vec::new();
    let line_height = i64::from(render_context.max_height());
    let mut min = I64Vec2::ZERO;
    let mut max = I64Vec2::new(0, i64::from(render_context.text_height()));

    for (line_index, line) in render_context.lines().iter().enumerate() {
        let line_top = line_index as i64 * line_height;
        let mut pen = 0.0;

        let mut characters = line.filtered_chars().peekable();
        while let Some(character) = characters.next() {
            let offsets = render_context.character_offsets(character);
            let position = I64Vec2::new(
                (pen + offsets.x).round() as i64,
                line_top + (-offsets.y).round() as i64,
            );
            let size = render_context.glyph_rect(character).size().as_i64vec2();

            min = min.min(position);
            max = max.max(position + size);
            placements.push(GlyphPlacement {
                character,
                position,
            });

            pen += render_context.character_advance(character, characters.peek().copied());
        }
        max.x = max.x.max(pen.round() as i64);
    }

    for placement in &mut placements {
        placement.position -= min;
    }

    let size = (max - min).as_uvec2();

    (placements, size)
}

/// A font image's pixel data, borrowed from its [`Image`].
type ImageBufferRef<'image> = ImageBuffer<Rgba<u8>, &'image [u8]>;

/// Borrows the pixel data of an 8-bit RGBA image, or returns `None` if it has
/// no data.
fn image_buffer(image: &Image) -> Option<ImageBufferRef<'_>> {
    let data = image.data.as_ref()?;
    ImageBuffer::from_raw(image.width(), image.height(), data.as_slice())
}

/// The images glyphs on one font texture are drawn from.
struct PageBuffers<'image> {
    /// The font texture.
    texture: ImageBufferRef<'image>,
    /// The texture's outline layers, when the text is drawn with an outline
    /// color and the texture has them.
    outline_layers: Option<OutlineLayerBuffers<'image>>,
}

/// The outline and glyph layers of a font texture. See
/// [`ImageFontOutlineLayers`](crate::ImageFontOutlineLayers).
struct OutlineLayerBuffers<'image> {
    /// The outline layer.
    outline: ImageBufferRef<'image>,
    /// The glyph layer.
    glyph: ImageBufferRef<'image>,
}

/// Draws each placed glyph into the output image buffer, blending it over the
/// glyphs drawn before it.
///
/// Glyphs are tinted with `colors.color`. When the text has an outline color,
/// every outline is drawn first in that color, from the outline layers, and
/// then the inside of every glyph on top, from the glyph layers.
///
/// # Parameters
/// - `image_font`: The font asset containing glyph metadata and character maps.
/// - `render_context`: The context providing glyph texture regions.
/// - `placements`: Where to draw each glyph, from [`layout_glyphs`].
/// - `output_image`: The image buffer to which glyphs are rendered.
/// - `pages`: The images to draw from, one per font texture.
/// - `colors`: The colors to draw the text in.
fn render_glyphs_to_image(
    image_font: &ImageFont,
    render_context: &RenderContext<'_>,
    placements: &[GlyphPlacement],
    output_image: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    pages: &[PageBuffers<'_>],
    colors: TextColors,
) {
    if let Some(outline_color) = colors.outline_color {
        for placement in placements {
            let page_index = image_font.atlas_character_map[&placement.character].page_index;
            if let Some(ref layers) = pages[page_index].outline_layers {
                draw_glyph(
                    output_image,
                    &layers.outline,
                    render_context,
                    placement,
                    outline_color,
                );
            }
        }
    }

    for placement in placements {
        let page_index = image_font.atlas_character_map[&placement.character].page_index;
        let page = &pages[page_index];
        let source = page
            .outline_layers
            .as_ref()
            .map_or(&page.texture, |layers| &layers.glyph);
        draw_glyph(
            output_image,
            source,
            render_context,
            placement,
            colors.color,
        );
    }
}

/// Draws a placed glyph from `source`, tinted with `tint`, blending it over
/// the output image.
fn draw_glyph(
    output_image: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    source: &ImageBufferRef<'_>,
    render_context: &RenderContext<'_>,
    placement: &GlyphPlacement,
    tint: Color,
) {
    let rect = render_context.glyph_rect(placement.character);
    let glyph = source.view(rect.min.x, rect.min.y, rect.width(), rect.height());
    let (x, y) = (placement.position.x, placement.position.y);

    if tint == Color::WHITE {
        imageops::overlay(output_image, &*glyph, x, y);
    } else {
        let tint = tint.to_linear();
        let tinted = image::RgbaImage::from_fn(glyph.width(), glyph.height(), |x, y| {
            tint_pixel(glyph.get_pixel(x, y), tint)
        });
        imageops::overlay(output_image, &tinted, x, y);
    }
}

/// Multiplies an sRGB pixel by a tint in linear space, the way a sprite's
/// color tints its image.
fn tint_pixel(pixel: Rgba<u8>, tint: LinearRgba) -> Rgba<u8> {
    let [red, green, blue, alpha] = pixel.0;
    let linear = LinearRgba::from(Srgba::rgba_u8(red, green, blue, alpha));
    let tinted = LinearRgba::new(
        linear.red * tint.red,
        linear.green * tint.green,
        linear.blue * tint.blue,
        linear.alpha * tint.alpha,
    );
    Rgba(Srgba::from(tinted).to_u8_array())
}

/// Prepares the rendering context for image font text rendering.
///
/// This function retrieves the required font asset, associated textures, and
/// constructs a `RenderContext` for the given `ImageFontText`. It returns all
/// the necessary data to render the text as an image, including the font,
/// textures, and context with rendering configuration.
///
/// # Parameters
/// - `image_font_text`: The text and font information to render.
/// - `image_fonts`: The collection of available font assets.
/// - `images`: The collection of image assets used to retrieve font textures.
/// - `layouts`: The texture atlas layouts defining character positioning.
///
/// # Returns
/// A tuple containing:
/// - A reference to the `ImageFont` asset.
/// - A vector of references to the font's texture images.
/// - The constructed `RenderContext` for rendering.
///
/// # Errors
/// Returns an error if the font asset or required texture assets are missing.
fn prepare_context<'assets, 'image>(
    image_font_text: &'assets ImageFontText,
    image_fonts: &'assets Assets<ImageFont>,
    images: &'image Assets<Image>,
    layouts: &'assets Assets<TextureAtlasLayout>,
) -> Result<
    (
        &'assets ImageFont,
        Vec<&'image Image>,
        RenderContext<'assets>,
    ),
    ImageFontRenderError,
> {
    let image_font = image_fonts
        .get(&image_font_text.font)
        .ok_or(ImageFontRenderError::MissingImageFontAsset)?;
    let textures = image_font.textures(images);
    let render_config = RenderConfig {
        scaling_mode: ImageFontScalingMode::Truncated,
        ..RenderConfig::default()
    };
    let render_context = RenderContext::new(image_font, image_font_text, render_config, layouts)
        .ok_or(ImageFontRenderError::MissingTextureAsset)?;
    Ok((image_font, textures, render_context))
}

/// Errors that can occur during the rendering of an `ImageFont`.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ImageFontRenderError {
    /// The image could not be converted to a `DynamicImage`. This typically
    /// indicates an issue with the underlying image data or its format.
    #[error("failed to convert image to DynamicImage: {0}")]
    ImageConversion(String),

    /// The `ImageFont` asset required for rendering was not loaded. Ensure
    /// that the font asset is correctly loaded into the Bevy app. This should
    /// happend automatically when using the `AssetLoader` to load the font
    /// asset.
    #[error("ImageFont asset not loaded")]
    MissingImageFontAsset,

    /// The texture asset associated with the `ImageFont` was not loaded. This
    /// could indicate an issue with the asset pipeline or a missing dependency.
    /// This should happend automatically when using the `AssetLoader` to
    /// load the font asset.
    #[error("Font texture asset not loaded")]
    MissingTextureAsset,

    /// An unspecified internal error occurred during rendering. This may
    /// indicate a bug or unexpected state in the rendering system.
    #[error("internal error")]
    UnknownError,

    /// Failed to copy a character from the source font image texture to the
    /// target rendered text sprite image texture. This error typically occurs
    /// when there is an issue with the image data or the copying process.
    #[error("failed to copy from atlas: {0}")]
    CopyFailure(#[from] ImageError),
}

/// A helper trait that represents types that can hold an image handle.
///
/// This is used to abstract over different components (e.g., [`Sprite`] and
/// [`ImageNode`]) that need to store a handle to an image rendered from text.
trait ImageHandleHolder {
    /// Sets the handle for the image that this holder represents.
    ///
    /// This method is called after rendering text into an image
    /// to update the holder with the new image handle.
    ///
    /// # Parameters
    /// - `image`: The handle to the newly rendered image.
    fn set_image_handle(&mut self, image: Handle<Image>);
}

impl ImageHandleHolder for Sprite {
    fn set_image_handle(&mut self, image: Handle<Image>) {
        self.image = image;
    }
}

#[cfg(feature = "ui")]
impl ImageHandleHolder for ImageNode {
    fn set_image_handle(&mut self, image: Handle<Image>) {
        self.image = image;
    }
}

#[cfg(all(test, feature = "bmf"))]
mod tests {
    #![allow(clippy::unwrap_used, reason = "test code panics to indicate errors")]

    use bevy_app::App;
    use bevy_color::palettes::css;
    use image::RgbaImage;

    use super::*;
    use crate::tests::utils::{
        ExampleFont, initialize_app_with_font, initialize_app_with_loaded_example_font,
        wait_until_loaded,
    };

    fn render(
        app: &App,
        font: &Handle<ImageFont>,
        text: &str,
        font_height: Option<f32>,
    ) -> Result<RgbaImage, ImageFontRenderError> {
        render_with_colors(app, font, text, font_height, TextColors::default())
    }

    fn render_with_colors(
        app: &App,
        font: &Handle<ImageFont>,
        text: &str,
        font_height: Option<f32>,
        colors: TextColors,
    ) -> Result<RgbaImage, ImageFontRenderError> {
        let world = app.world();
        let image_font_text = ImageFontText {
            text: String::from(text),
            font: font.clone(),
            font_height,
        };
        let image = render_text_to_image(
            &image_font_text,
            colors,
            world.resource::<Assets<ImageFont>>(),
            world.resource::<Assets<Image>>(),
            world.resource::<Assets<TextureAtlasLayout>>(),
        )?;
        Ok(RgbaImage::from_raw(image.width(), image.height(), image.data.unwrap()).unwrap())
    }

    fn glyph(app: &App, font: &Handle<ImageFont>, character: char) -> RgbaImage {
        let world = app.world();
        let image_font = world.resource::<Assets<ImageFont>>().get(font).unwrap();
        let image_font_character = &image_font.atlas_character_map[&character];
        let rect = world
            .resource::<Assets<TextureAtlasLayout>>()
            .get(&image_font.atlas_layouts[image_font_character.page_index])
            .unwrap()
            .textures[image_font_character.character_index];
        let texture = world
            .resource::<Assets<Image>>()
            .get(&image_font.textures[image_font_character.page_index])
            .unwrap();
        let texture = RgbaImage::from_raw(
            texture.width(),
            texture.height(),
            texture.data.clone().unwrap(),
        )
        .unwrap();
        imageops::crop_imm(
            &texture,
            rect.min.x,
            rect.min.y,
            rect.width(),
            rect.height(),
        )
        .to_image()
    }

    #[test]
    fn renders_text_whose_advances_exceed_glyph_widths() {
        let (app, font) = initialize_app_with_loaded_example_font(ExampleFont::BmFont);

        // The advances of "Hello" add up to 71, while its glyphs are only 63
        // wide in total.
        let image = render(&app, &font, "Hello", None).unwrap();

        assert_eq!(image.dimensions(), (71, 36));
    }

    #[test]
    fn glyphs_are_drawn_at_their_offsets() {
        let (app, font) = initialize_app_with_loaded_example_font(ExampleFont::BmFont);
        let glyph = glyph(&app, &font, '.');
        assert!(glyph.pixels().any(|pixel| pixel[3] > 0));

        let image = render(&app, &font, ".", None).unwrap();

        // "." advances 9, the line is 36 tall, and the glyph is offset by (2, 26).
        assert_eq!(image.dimensions(), (9, 36));
        for (x, y, &pixel) in image.enumerate_pixels() {
            let expected =
                if (2..2 + glyph.width()).contains(&x) && (26..26 + glyph.height()).contains(&y) {
                    *glyph.get_pixel(x - 2, y - 26)
                } else {
                    Rgba([0, 0, 0, 0])
                };
            assert_eq!(pixel, expected, "pixel ({x}, {y})");
        }
    }

    /// Loads `tests/channels_outline.txt.fnt`, whose `A` is a 2×2 glyph made
    /// of an outline pixel, a glyph pixel, an empty pixel and a partly covered
    /// glyph pixel inside the outline, in that order.
    fn outline_font() -> (App, Handle<ImageFont>) {
        let (mut app, font) = initialize_app_with_font("tests/channels_outline.txt.fnt");
        wait_until_loaded(&mut app, &font);
        (app, font)
    }

    #[test]
    fn outline_color_draws_outline_and_glyph_in_their_own_colors() {
        let (app, font) = outline_font();

        let image = render_with_colors(
            &app,
            &font,
            "A",
            None,
            TextColors {
                color: css::BLUE.into(),
                outline_color: Some(css::RED.into()),
            },
        )
        .unwrap();

        assert_eq!(image.dimensions(), (2, 2));
        assert_eq!(*image.get_pixel(0, 0), Rgba([255, 0, 0, 255]));
        assert_eq!(*image.get_pixel(1, 0), Rgba([0, 0, 255, 255]));
        assert_eq!(*image.get_pixel(0, 1), Rgba([0, 0, 0, 0]));
        let partial = image.get_pixel(1, 1);
        assert!(
            partial[0] > 0 && partial[2] > 0 && partial[3] == 255,
            "{partial:?}"
        );
    }

    #[test]
    fn color_tints_glyph_and_keeps_outline_without_outline_color() {
        let (app, font) = outline_font();

        let image = render_with_colors(
            &app,
            &font,
            "A",
            None,
            TextColors {
                color: css::BLUE.into(),
                outline_color: None,
            },
        )
        .unwrap();

        assert_eq!(*image.get_pixel(0, 0), Rgba([0, 0, 0, 255]));
        assert_eq!(*image.get_pixel(1, 0), Rgba([0, 0, 255, 255]));
        assert_eq!(*image.get_pixel(0, 1), Rgba([0, 0, 0, 0]));
    }

    #[test]
    fn changing_the_outline_color_renders_the_text_again() {
        let (mut app, font) = outline_font();
        let entity = app
            .world_mut()
            .spawn((
                ImageFontPreRenderedText::default(),
                ImageFontText::default().text("A").font(font),
            ))
            .id();
        app.update();
        let first_image = app.world().get::<Sprite>(entity).unwrap().image.clone();

        app.world_mut()
            .get_mut::<ImageFontPreRenderedText>(entity)
            .unwrap()
            .outline_color = Some(css::RED.into());
        app.update();

        let world = app.world();
        let image = &world.get::<Sprite>(entity).unwrap().image;
        assert_ne!(*image, first_image);
        let image = world.resource::<Assets<Image>>().get(image).unwrap();
        assert_eq!(image.data.as_ref().unwrap()[..4], [255, 0, 0, 255]);
    }

    #[test]
    fn font_height_scales_the_whole_line() {
        let (app, font) = initialize_app_with_loaded_example_font(ExampleFont::BmFont);

        let image = render(&app, &font, ".", Some(72.0)).unwrap();

        assert_eq!(image.dimensions(), (18, 72));
    }

    #[test]
    fn each_line_starts_at_the_left_edge_one_line_height_below_the_previous() {
        let (app, font) = initialize_app_with_loaded_example_font(ExampleFont::BmFont);
        let glyph = glyph(&app, &font, '.');

        let image = render(&app, &font, "Hello\n.", None).unwrap();

        // "Hello" advances 71; the second line starts 36 down, and "." is
        // offset by (2, 26) from it.
        assert_eq!(image.dimensions(), (71, 72));
        for (x, y, &pixel) in glyph.enumerate_pixels() {
            assert_eq!(*image.get_pixel(2 + x, 36 + 26 + y), pixel, "({x}, {y})");
        }
    }

    #[test]
    fn crlf_breaks_lines_like_lf() {
        let (app, font) = initialize_app_with_loaded_example_font(ExampleFont::BmFont);

        let lf = render(&app, &font, "Hello\n.", None).unwrap();
        let crlf = render(&app, &font, "Hello\r\n.", None).unwrap();

        assert_eq!(crlf, lf);
    }

    #[test]
    fn empty_lines_take_up_a_line_height() {
        let (app, font) = initialize_app_with_loaded_example_font(ExampleFont::BmFont);

        assert_eq!(
            render(&app, &font, ".\n", None).unwrap().dimensions(),
            (9, 72)
        );
        assert_eq!(
            render(&app, &font, "\n\n.", None).unwrap().dimensions(),
            (9, 108)
        );
    }

    #[test]
    fn font_height_scales_every_line() {
        let (app, font) = initialize_app_with_loaded_example_font(ExampleFont::BmFont);

        let image = render(&app, &font, ".\n.", Some(72.0)).unwrap();

        assert_eq!(image.dimensions(), (18, 144));
    }

    #[test]
    fn overlapping_glyphs_do_not_erase_each_other() {
        let (app, font) = initialize_app_with_loaded_example_font(ExampleFont::BmFont);
        let glyph_a = glyph(&app, &font, 'A');
        let glyph_v = glyph(&app, &font, 'V');

        let image = render(&app, &font, "AV", None).unwrap();

        // A's x offset of -1 widens the image to the left. V starts at
        // A's advance (21) plus the kerning (-2), so its rectangle overlaps A's.
        assert_eq!(image.dimensions(), (42, 36));
        for (glyph, left) in [(&glyph_a, 0), (&glyph_v, 20)] {
            for (x, y, pixel) in glyph.enumerate_pixels() {
                if pixel[3] == u8::MAX {
                    assert_eq!(
                        image.get_pixel(left + x, 6 + y)[3],
                        u8::MAX,
                        "glyph pixel ({x}, {y})"
                    );
                }
            }
        }
    }
}
