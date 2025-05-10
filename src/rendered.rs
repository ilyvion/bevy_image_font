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

use bevy::{
    prelude::*,
    render::{
        render_asset::RenderAssetUsages,
        render_resource::{Extent3d, TextureDimension, TextureFormat},
    },
};
use bevy_image::{Image, ImageSampler};
use image::{
    imageops::{self, FilterType},
    GenericImage as _, GenericImageView as _, ImageBuffer, ImageError, Rgba,
};
use thiserror::Error;

use crate::{sync_texts_with_font_changes, ImageFont, ImageFontSet, ImageFontText};

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
            use bevy::ui::widget::update_image_content_size_system;
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
/// on its `Sprite` component.
#[derive(Component, Debug, Default, Clone, Reflect)]
#[require(ImageFontText, Sprite)]
pub struct ImageFontPreRenderedText;

/// A component for displaying UI text that has been pre-rendered using an image
/// font.
///
/// This component requires an `ImageFontText` component for determining its
/// font and text. It renders its text into an image and sets it as the texture
/// on its `ImageNode` component.
#[derive(Component, Debug, Default, Clone, Reflect)]
#[cfg(feature = "ui")]
#[require(ImageFontText, ImageNode)]
pub struct ImageFontPreRenderedUiText;

/// System that renders each [`ImageFontText`] into its [`Sprite`]. This system
/// only runs when the `ImageFontText` changes.
pub fn render_text_to_sprite(
    mut query: Query<(&ImageFontText, &mut Sprite), Changed<ImageFontText>>,
    image_fonts: Res<Assets<ImageFont>>,
    mut images: ResMut<Assets<Image>>,
    layouts: Res<Assets<TextureAtlasLayout>>,
) {
    render_text_to_image_holder(
        query
            .iter_mut()
            .map(|(image_font_text, sprite)| (image_font_text, sprite.into_inner())),
        &image_fonts,
        &mut images,
        &layouts,
    );
}

#[cfg(feature = "ui")]
/// System that renders each [`ImageFontText`] into its [`ImageNode`]. This
/// system only runs when the `ImageFontText` changes.
pub fn render_text_to_image_node(
    mut query: Query<(&ImageFontText, &mut ImageNode), Changed<ImageFontText>>,
    image_fonts: Res<Assets<ImageFont>>,
    mut images: ResMut<Assets<Image>>,
    layouts: Res<Assets<TextureAtlasLayout>>,
) {
    render_text_to_image_holder(
        query
            .iter_mut()
            .map(|(image_font_text, image_node)| (image_font_text, image_node.into_inner())),
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
/// - `font_text_to_image_iter`: An iterator over pairs of [`ImageFontText`] and
///   mutable references to objects implementing [`ImageHandleHolder`]. Each
///   item in the iterator represents a text-to-image mapping to be rendered.
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
/// If text rendering fails for an item, an error message is logged, and the
/// corresponding holder is not updated.
fn render_text_to_image_holder<'borrow>(
    font_text_to_image_iter: impl Iterator<
        Item = (
            &'borrow ImageFontText,
            &'borrow mut (impl ImageHandleHolder + 'borrow),
        ),
    >,
    image_fonts: &Assets<ImageFont>,
    images: &mut Assets<Image>,
    layouts: &Assets<TextureAtlasLayout>,
) {
    for (image_font_text, image_handle_holder) in font_text_to_image_iter {
        debug!("Rendering [{}]", image_font_text.text);
        match render_text_to_image(image_font_text, image_fonts, images, layouts) {
            Ok(image) => {
                image_handle_holder.set_image_handle(images.add(image));
            }
            Err(error) => {
                error!(
                    "Error when rendering image font text {:?}: {}",
                    image_font_text, error
                );
            }
        }
    }
}

/// Renders the text inside the [`ImageFontText`] to a single output image. You
/// don't need to use this if you're using the built-in functionality, but if
/// you want to use this for some other custom plugin/system, you can call this.
fn render_text_to_image(
    image_font_text: &ImageFontText,
    image_fonts: &Assets<ImageFont>,
    images: &Assets<Image>,
    layouts: &Assets<TextureAtlasLayout>,
) -> Result<Image, ImageFontRenderError> {
    let image_font = image_fonts
        .get(&image_font_text.font)
        .ok_or(ImageFontRenderError::MissingImageFontAsset)?;
    let font_texture = images
        .get(&image_font.texture)
        .ok_or(ImageFontRenderError::MissingTextureAsset)?;
    #[expect(clippy::expect_used, reason = "handle is kept alive by ImageFont")]
    let layout = layouts
        .get(&image_font.atlas_layout)
        .expect("handle is kept alive by ImageFont");

    let text = image_font.filter_string(&image_font_text.text);

    if text.is_empty() {
        // can't make a 0x0 image, so make a 1x1 transparent black pixel
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

    // as wide as the sum of all characters, as tall as the tallest one
    #[expect(
        clippy::expect_used,
        reason = "we've verified !text.is_empty() already"
    )]
    let height = text
        .filtered_chars()
        .map(|character| layout.textures[image_font.atlas_character_map[&character]].height())
        .reduce(u32::max)
        .expect("we've verified !text.is_empty() already");
    #[expect(
        clippy::expect_used,
        reason = "we've verified !text.is_empty() already"
    )]
    let width = text
        .filtered_chars()
        .map(|character| layout.textures[image_font.atlas_character_map[&character]].width())
        .reduce(|accumulator, value| accumulator + value)
        .expect("we've verified !text.is_empty() already");
    let Some(data) = &font_texture.data else {
        return Err(ImageFontRenderError::MissingTextureAsset);
    };

    let mut output_image = image::RgbaImage::new(width, height);
    let font_texture: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_raw(
        font_texture.width(),
        font_texture.height(),
        data.as_slice(),
    )
    .ok_or(ImageFontRenderError::UnknownError)?;

    let mut x = 0;
    for character in text.filtered_chars() {
        let rect = layout.textures[image_font.atlas_character_map[&character]];
        let width = rect.width();
        let height = rect.height();
        output_image.copy_from(
            &*font_texture.view(rect.min.x, rect.min.y, width, height),
            x,
            0,
        )?;
        x += width;
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss,
        reason = "the magnitude of the numbers we're working on here are too small to lose anything"
    )]
    if let Some(font_height) = image_font_text.font_height {
        let width = output_image.width() as f32 * font_height / output_image.height() as f32;
        output_image = imageops::resize(
            &output_image,
            width as u32,
            font_height as u32,
            FilterType::Nearest,
        );
    }

    let mut bevy_image = Image::new(
        Extent3d {
            // these might have changed because of the resize
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
    #[error("failed to copy from atlas")]
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
