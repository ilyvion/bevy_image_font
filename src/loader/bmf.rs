// TODO: Remove this shit
// #![allow(
//     clippy::missing_docs_in_private_items,
//     missing_docs,
//     unused,
//     clippy::unwrap_used,
//     reason = "dev work"
// )]

//! Code for parsing an [`ImageFont`] off of an on-disk representation in `fnt`
//! format.

use bevy_asset::{AssetLoader, LoadContext, io::Reader};
use bevy_image::Image;
use bevy_image::TextureAtlasLayout;
use bevy_log::warn;
use bevy_math::Vec2;
use bevy_math::{URect, UVec2};
use bevy_platform::collections::HashMap;
use camino::Utf8Path;
use strum::{AsRefStr, EnumIter, IntoEnumIterator as _, VariantNames};
use thiserror::Error;

use crate::ImageFontCharacter;
use crate::{
    ImageFont,
    loader::{ImageFontLoadError, ImageFontLoaderSettings},
};

/// Loader for [`ImageFont`]s.
#[derive(Debug, Default)]
pub struct BmFontLoader;

/// An error type representing issues that may arise during the loading of BMF
/// fonts.
///
/// This includes errors encountered when processing the font file, handling
/// associated assets, or enforcing constraints specific to the
/// [`BmFontLoader`]. Some errors originate from lower-level font loading
/// mechanisms and are wrapped transparently.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum BmFontLoadError {
    /// Errors that can occur when loading an image font.
    #[error(transparent)]
    ImageFontLoadError(#[from] ImageFontLoadError),

    /// Errors that occur while parsing the BMF file format.
    #[error(transparent)]
    BmFontError(#[from] bmfont_rs::Error),

    /// [`BmFontLoader`] only supports Unicode fonts, but the loaded font is
    /// not.
    #[error("BmFontLoader only supports unicode fonts")]
    CharsetUnsupported,

    /// BMF fonts must not be packed, but the loaded font is packed.
    #[error("BmFontLoader only supports non-packed fonts")]
    PackedUnsupported,
}

impl AssetLoader for BmFontLoader {
    type Asset = ImageFont;

    type Settings = ImageFontLoaderSettings;

    type Error = BmFontLoadError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let data = read_bmf_data(reader).await?;
        let bm_font = parse_bmf_data(&data, load_context)?;

        let mut atlas_character_map = process_bmf_characters(&bm_font);
        let (image_handles, atlas_layout_handles) =
            load_images_and_textures(&bm_font, &mut atlas_character_map, settings, load_context)
                .await?;

        Ok(construct_image_font(
            &bm_font,
            image_handles,
            atlas_character_map,
            atlas_layout_handles,
            settings,
        ))
    }

    fn extensions(&self) -> &[&str] {
        BmFontExtension::VARIANTS
    }
}

/// Represents the supported file extensions for BMF font files.
///
/// This enum is used to ensure that only recognized file formats are parsed
/// correctly, preventing unsupported formats from being loaded.
#[derive(AsRefStr, Debug, Copy, Clone, EnumIter, PartialEq, Eq, VariantNames)]
#[repr(usize)]
enum BmFontExtension {
    /// The text-based `.txt.fnt` format.
    ///
    /// This format stores font metadata in a human-readable text format.
    #[strum(serialize = "txt.fnt")]
    Text,

    /// The XML-based `.xml.fnt` format.
    ///
    /// This format stores font metadata using XML syntax.
    #[strum(serialize = "xml.fnt")]
    Xml,

    /// The binary `.bin.fnt` format.
    ///
    /// This format stores font metadata in a compact binary representation.
    #[strum(serialize = "bin.fnt")]
    Binary,
}

impl BmFontExtension {
    /// Attempts to convert an extension tuple (`(&str, &str)`) into a
    /// `BmFontExtension`.
    ///
    /// # Parameters
    /// - `extension`: A tuple representing the two-part file extension, e.g.,
    ///   `("txt", "fnt")`.
    ///
    /// # Returns
    /// - `Some(BmFontExtension)` if the extension is recognized.
    /// - `None` if the extension does not match any known format.
    #[inline]
    fn from_tuple(extension: (&str, &str)) -> Option<Self> {
        Self::iter().find(|&self_extension| extension == self_extension.as_tuple())
    }

    /// Extracts the `BmFontExtension` from a file path, if possible.
    ///
    /// This function looks at the file name, extracts its two-dot extension,
    /// and attempts to match it against known BMF font extensions.
    ///
    /// # Parameters
    /// - `path`: A reference to a `Utf8Path` representing the file path.
    ///
    /// # Returns
    /// - `Some(BmFontExtension)` if the file has a recognized extension.
    /// - `None` if the file's extension is not recognized or the path has no
    ///   valid file name.
    fn from_path(path: &Utf8Path) -> Option<Self> {
        let file_name = path.file_name()?;
        let tuple = file_name_to_extension_tuple(file_name)?;
        Self::from_tuple(tuple)
    }

    /// Converts a `BmFontExtension` into its corresponding two-dot extension
    /// tuple.
    ///
    /// # Returns
    /// A tuple containing the two parts of the extension, e.g., `("txt",
    /// "fnt")`.
    fn as_tuple(&self) -> (&str, &str) {
        match self.as_ref().split_once('.') {
            Some(tuple) => tuple,
            None => unreachable!("Every BmFontExtension must contain a '.'"),
        }
    }
}

/// Extracts a two-dot file extension from a file name.
///
/// This function assumes the file name follows the convention of having a
/// two-part extension, such as `"txt.fnt"`. It will return `None` if the file
/// name does not match this pattern.
///
/// # Parameters
/// - `file_name`: A string slice representing the file name.
///
/// # Returns
/// - `Some((&str, &str))` if a valid two-dot extension is found.
/// - `None` if the file name does not contain at least two dot-separated parts.
fn file_name_to_extension_tuple(file_name: &str) -> Option<(&str, &str)> {
    let (base, ext2) = file_name.rsplit_once('.')?;
    let (_, ext1) = base.rsplit_once('.')?;
    let tuple = (ext1, ext2);
    Some(tuple)
}

/// Reads the BMF font data from the reader.
async fn read_bmf_data(reader: &mut dyn Reader) -> Result<Vec<u8>, ImageFontLoadError> {
    let mut data = Vec::new();
    reader.read_to_end(&mut data).await?;
    Ok(data)
}

/// Parses the BMF font data and validates it.
fn parse_bmf_data(
    data: &[u8],
    load_context: &mut LoadContext<'_>,
) -> Result<bmfont_rs::Font, BmFontLoadError> {
    let path = load_context.path();
    let path = match <&Utf8Path>::try_from(path) {
        Ok(path) => path,
        Err(error) => return Err(ImageFontLoadError::InvalidPath(error).into()),
    };

    let from_bytes = match BmFontExtension::from_path(path) {
        Some(BmFontExtension::Text) => bmfont_rs::text::from_bytes,
        Some(BmFontExtension::Xml) => bmfont_rs::xml::from_bytes,
        Some(BmFontExtension::Binary) => bmfont_rs::binary::from_bytes,
        extension => unreachable!("{extension:?}"),
    };

    let bm_font = from_bytes(data)?;

    if !bm_font.info.unicode {
        return Err(BmFontLoadError::CharsetUnsupported);
    }
    if bm_font.common.packed {
        return Err(BmFontLoadError::PackedUnsupported);
    }

    Ok(bm_font)
}

/// Processes BMF characters into an atlas character map.
fn process_bmf_characters(
    bm_font: &bmfont_rs::Font,
) -> HashMap<char, (&bmfont_rs::Char, URect, Option<usize>)> {
    let mut atlas_character_map = HashMap::new();
    for char in &bm_font.chars {
        let x = u32::from(char.x);
        let y = u32::from(char.y);

        let rect = URect {
            min: UVec2 { x, y },
            max: UVec2 {
                x: x + u32::from(char.width),
                y: y + u32::from(char.height),
            },
        };

        if let Some(character) = char::from_u32(char.id) {
            atlas_character_map.insert(character, (char, rect, None));
        } else {
            warn!(
                "Skipping invalid character id {}. Full char definition: {char:?} ",
                char.id
            );
        }
    }

    atlas_character_map
}

/// Loads font images and creates texture atlases.
async fn load_images_and_textures(
    bm_font: &bmfont_rs::Font,
    atlas_character_map: &mut HashMap<char, (&bmfont_rs::Char, URect, Option<usize>)>,
    settings: &ImageFontLoaderSettings,
    load_context: &mut LoadContext<'_>,
) -> Result<
    (
        Vec<bevy_asset::Handle<Image>>,
        Vec<bevy_asset::Handle<TextureAtlasLayout>>,
    ),
    BmFontLoadError,
> {
    let mut image_handles = Vec::new();
    let mut atlas_layout_handles = Vec::new();
    for (page_no, page) in bm_font.pages.iter().enumerate() {
        let image_path = load_context
            .path()
            .parent()
            .ok_or(ImageFontLoadError::MissingParentPath)?
            .join(page);

        let Some(mut image) = load_context
            .loader()
            .immediate()
            .with_unknown_type()
            .load(image_path.clone())
            .await
            .map_err(ImageFontLoadError::from)?
            .take::<Image>()
        else {
            return Err(ImageFontLoadError::NotAnImage(page.into()).into());
        };

        image.sampler = settings.image_sampler.clone();

        let size = image.size();
        let image_handle = load_context.add_labeled_asset(format!("texture_{page_no}"), image);
        image_handles.push(image_handle);

        let mut atlas_layout = TextureAtlasLayout::new_empty(size);
        for (_, &mut (_, rect, ref mut texture_handle)) in atlas_character_map
            .iter_mut()
            .filter(|&(_, &mut (char, _, _))| char.page as usize == page_no)
        {
            texture_handle.replace(atlas_layout.add_texture(rect));
        }

        let layout_handle =
            load_context.add_labeled_asset(format!("layout_{page_no}"), atlas_layout);
        atlas_layout_handles.push(layout_handle);
    }

    Ok((image_handles, atlas_layout_handles))
}

/// Constructs the final `ImageFont` asset.
fn construct_image_font(
    bm_font: &bmfont_rs::Font,
    image_handles: Vec<bevy_asset::Handle<Image>>,
    atlas_character_map: HashMap<char, (&bmfont_rs::Char, URect, Option<usize>)>,
    atlas_layout_handles: Vec<bevy_asset::Handle<TextureAtlasLayout>>,
    settings: &ImageFontLoaderSettings,
) -> ImageFont {
    ImageFont {
        textures: image_handles,
        atlas_character_map: atlas_character_map
            .into_iter()
            .map(|(char, (font_char, _, atlas_index))| {
                let kernings = bm_font.kernings.iter().filter_map(|kerning| {
                    if kerning.first == font_char.id {
                        if let Some(character) = char::from_u32(kerning.second) {
                            Some((character, f32::from(kerning.amount)))
                        } else {
                            warn!(
                                "Skipping invalid character id {} in kerning data.",
                                kerning.second
                            );
                            None
                        }
                    } else {
                        None
                    }
                });

                (
                    char,
                    #[expect(
                        clippy::unwrap_used,
                        reason = "all Nones replaced in load_images_and_textures"
                    )]
                    ImageFontCharacter {
                        character_index: atlas_index.unwrap(),
                        page_index: font_char.page as usize,
                        offsets: Vec2::new(
                            f32::from(font_char.xoffset),
                            -f32::from(font_char.yoffset),
                        ),
                        x_advance: Some(f32::from(font_char.xadvance)),
                        kernings: kernings.collect(),
                    },
                )
            })
            .collect(),
        atlas_layouts: atlas_layout_handles,
        image_sampler: settings.image_sampler.clone(),
    }
}
