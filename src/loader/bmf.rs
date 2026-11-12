//! Code for parsing an [`ImageFont`] off of an on-disk representation in `fnt`
//! format.

use bevy_asset::AssetPath;
use bevy_asset::{AssetLoader, Handle, LoadContext, io::Reader};
use bevy_image::Image;
use bevy_image::TextureAtlasLayout;
use bevy_log::warn;
use bevy_math::Vec2;
use bevy_math::{URect, UVec2};
use bevy_platform::collections::HashMap;
use bevy_reflect::TypePath;
use strum::{AsRefStr, EnumIter, IntoEnumIterator as _, VariantNames};
use thiserror::Error;

use self::channels::{ChannelSettings, PageDecoding, outline_layers};
use self::charset::CharIds;
use crate::{
    ImageFont,
    loader::{ImageFontLoadError, ImageFontLoaderSettings},
};
use crate::{ImageFontCharacter, ImageFontOutlineLayers};

mod channels;
mod charset;

/// Loader for [`ImageFont`]s.
#[derive(Debug, Default, TypePath)]
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

    /// The loaded font is not a Unicode font, and the `bmf_charsets` feature,
    /// which [`BmFontLoader`] needs to load such fonts, is not enabled.
    #[error("BmFontLoader only supports unicode fonts without the `bmf_charsets` feature")]
    CharsetUnsupported,

    /// The loaded font is not a Unicode font, and its `info.charset` is not a
    /// charset `BMFont` defines.
    #[cfg(feature = "bmf_charsets")]
    #[error("unknown BMFont charset {0:?}")]
    UnknownCharset(String),

    /// The loaded font is not a Unicode font, and the code page its charset
    /// stands for can't be decoded. The `JOHAB` charset (code page 1361) is
    /// not supported, and neither are code pages in
    /// [`ImageFontLoaderSettings::bmf_ansi_code_page`] or
    /// [`ImageFontLoaderSettings::bmf_oem_code_page`] that neither
    /// `encoding_rs` nor `oem_cp` can decode.
    #[cfg(feature = "bmf_charsets")]
    #[error("BMFont code page {0} is not supported")]
    CodePageUnsupported(u16),
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
        let char_ids = CharIds::new(&bm_font.info, settings)?;

        let mut atlas_character_map = process_bmf_characters(&bm_font, char_ids);
        let textures =
            load_images_and_textures(&bm_font, &mut atlas_character_map, settings, load_context)
                .await?;

        Ok(construct_image_font(
            &bm_font,
            char_ids,
            textures,
            atlas_character_map,
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

    /// Extracts the `BmFontExtension` from a `AssetPath`, if possible.
    ///
    /// This function looks at the file name, extracts its two-dot extension,
    /// and attempts to match it against known BMF font extensions.
    ///
    /// # Parameters
    /// - `path`: A reference to an `AssetPath` representing the file path.
    ///
    /// # Returns
    /// - `Some(BmFontExtension)` if the file has a recognized extension.
    /// - `None` if the file's extension is not recognized or the path has no
    ///   valid file name.
    fn from_asset_path(path: &AssetPath) -> Option<Self> {
        let file_name = path.get_full_extension()?;
        let (base, ext2) = file_name.rsplit_once('.')?;
        let ext1 = base.rsplit_once('.').map_or(base, |(_, ext)| ext);
        Self::from_tuple((ext1, ext2))
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

/// Reads the BMF font data from the reader.
async fn read_bmf_data(reader: &mut dyn Reader) -> Result<Vec<u8>, ImageFontLoadError> {
    let mut data = Vec::new();
    reader.read_to_end(&mut data).await?;
    Ok(data)
}

/// Parses the BMF font data.
fn parse_bmf_data(
    data: &[u8],
    load_context: &mut LoadContext<'_>,
) -> Result<bmfont_rs::Font, BmFontLoadError> {
    let path = load_context.path();

    let from_bytes = match BmFontExtension::from_asset_path(path) {
        Some(BmFontExtension::Text) => bmfont_rs::text::from_bytes,
        Some(BmFontExtension::Xml) => bmfont_rs::xml::from_bytes,
        Some(BmFontExtension::Binary) => bmfont_rs::binary::from_bytes,
        extension => unreachable!("{extension:?}"),
    };

    Ok(from_bytes(data)?)
}

/// A BMF character and where its glyph is in the font's textures.
#[derive(Debug)]
struct AtlasCharacter<'font> {
    /// The character's definition in the font file.
    font_char: &'font bmfont_rs::Char,
    /// The glyph's rectangle on its page.
    rect: URect,
    /// The index of the glyph's texture and its index in that texture's atlas
    /// layout, set by `load_images_and_textures`.
    placement: Option<(usize, usize)>,
}

/// Processes BMF characters into an atlas character map.
fn process_bmf_characters(
    bm_font: &bmfont_rs::Font,
    char_ids: CharIds,
) -> HashMap<char, AtlasCharacter<'_>> {
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

        if let Some(character) = char_ids.to_char(char.id) {
            atlas_character_map.insert(
                character,
                AtlasCharacter {
                    font_char: char,
                    rect,
                    placement: None,
                },
            );
        } else {
            warn!(
                "Skipping invalid character id {}. Full char definition: {char:?} ",
                char.id
            );
        }
    }

    atlas_character_map
}

/// The textures of a loaded font, with their atlas layouts and outline layers
/// at the same indices.
#[derive(Debug, Default)]
struct FontTextures {
    /// The font's textures.
    images: Vec<Handle<Image>>,
    /// The atlas layout of each texture.
    atlas_layouts: Vec<Handle<TextureAtlasLayout>>,
    /// The outline and glyph layers of each texture that holds an outline.
    outline_layers: Vec<Option<ImageFontOutlineLayers>>,
}

/// Loads font images and creates texture atlases.
///
/// Each page is decoded according to the font's channel settings. A packed
/// page becomes one texture per channel its glyphs are in. Textures that hold
/// an outline are also split into outline and glyph layers.
async fn load_images_and_textures(
    bm_font: &bmfont_rs::Font,
    atlas_character_map: &mut HashMap<char, AtlasCharacter<'_>>,
    settings: &ImageFontLoaderSettings,
    load_context: &mut LoadContext<'_>,
) -> Result<FontTextures, BmFontLoadError> {
    let channel_settings = ChannelSettings::new(&bm_font.common);
    let mut textures = FontTextures::default();
    for (page_no, page) in bm_font.pages.iter().enumerate() {
        let image_path = load_context
            .path()
            .resolve_embed_str(page)
            .map_err(ImageFontLoadError::from)?;

        let mut image: Image = load_context
            .load_builder()
            .load_value(image_path.clone())
            .await
            .map_err(ImageFontLoadError::from)?
            .take();

        image.sampler = settings.image_sampler.clone();

        let page_characters = atlas_character_map
            .values_mut()
            .filter(|character| usize::from(character.font_char.page) == page_no);
        let mut views: Vec<(PageDecoding, Vec<&mut AtlasCharacter<'_>>)> = Vec::new();
        if let ChannelSettings::NotPacked(decoding) = channel_settings {
            views.push((decoding, page_characters.collect()));
        } else {
            for character in page_characters {
                let decoding = channel_settings.decoding_for(character.font_char.chnl);
                match views.iter_mut().find(|&&mut (view, _)| view == decoding) {
                    Some(&mut (_, ref mut characters)) => characters.push(character),
                    None => views.push((decoding, vec![character])),
                }
            }
        }

        let size = image.size();
        let view_count = views.len();
        let mut page_image = Some(image);
        for (view_no, (decoding, characters)) in views.into_iter().enumerate() {
            let mut view_image = if view_no + 1 == view_count {
                page_image.take()
            } else {
                page_image.clone()
            }
            .unwrap_or_else(|| unreachable!("the page image is taken by the last view only"));
            decoding.decode_image(&mut view_image);

            let suffix = if channel_settings.is_packed() {
                decoding.packed_label_suffix()
            } else {
                ""
            };
            let label = format!("texture_{page_no}{suffix}");
            let layers = decoding
                .has_outline()
                .then(|| outline_layers(&view_image))
                .flatten()
                .map(|(outline, glyph)| ImageFontOutlineLayers {
                    outline: load_context.add_labeled_asset(format!("{label}_outline"), outline),
                    glyph: load_context.add_labeled_asset(format!("{label}_glyph"), glyph),
                });
            textures.outline_layers.push(layers);

            let texture_index = textures.images.len();
            textures
                .images
                .push(load_context.add_labeled_asset(label, view_image));

            let mut atlas_layout = TextureAtlasLayout::new_empty(size);
            for character in characters {
                character.placement =
                    Some((texture_index, atlas_layout.add_texture(character.rect)));
            }
            textures.atlas_layouts.push(
                load_context.add_labeled_asset(format!("layout_{page_no}{suffix}"), atlas_layout),
            );
        }
    }

    Ok(textures)
}

/// Constructs the final `ImageFont` asset.
fn construct_image_font(
    bm_font: &bmfont_rs::Font,
    char_ids: CharIds,
    textures: FontTextures,
    atlas_character_map: HashMap<char, AtlasCharacter<'_>>,
    settings: &ImageFontLoaderSettings,
) -> ImageFont {
    ImageFont {
        textures: textures.images,
        atlas_character_map: atlas_character_map
            .into_iter()
            .map(|(char, character)| {
                let font_char = character.font_char;
                let kernings = bm_font.kernings.iter().filter_map(|kerning| {
                    if kerning.first == font_char.id {
                        if let Some(character) = char_ids.to_char(kerning.second) {
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
                        character_index: character.placement.unwrap().1,
                        page_index: character.placement.unwrap().0,
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
        atlas_layouts: textures.atlas_layouts,
        image_sampler: settings.image_sampler.clone(),
        line_height: Some(u32::from(bm_font.common.line_height)),
        outline_layers: textures.outline_layers,
    }
}
