//! Decoding of `BMFont` texture channel settings (`common.packed`,
//! `alphaChnl`, `redChnl`, `greenChnl`, `blueChnl` and `char.chnl`) into
//! RGBA textures where the alpha channel is the drawn shape.

use bevy_image::Image;
use bevy_log::warn;
use bmfont_rs::{Chnl, Common, Packing};
use wgpu_types::TextureFormat;

/// Index of the red channel in an RGBA pixel.
const RED: usize = 0;
/// Index of the green channel in an RGBA pixel.
const GREEN: usize = 1;
/// Index of the blue channel in an RGBA pixel.
const BLUE: usize = 2;
/// Index of the alpha channel in an RGBA pixel.
const ALPHA: usize = 3;

/// The shape data a texture channel holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ChannelContent {
    /// The glyph's coverage.
    Glyph,
    /// The coverage of the outline and the glyph together.
    Outline,
    /// Glyph and outline in one value: up to half is outline coverage, above
    /// half is glyph intensity.
    GlyphOutline,
}

impl ChannelContent {
    /// The content of a channel with the given packing, or `None` if the
    /// channel is constant.
    fn from_packing(packing: Packing) -> Option<Self> {
        match packing {
            Packing::Glyph => Some(Self::Glyph),
            Packing::Outline => Some(Self::Outline),
            Packing::GlyphOutline => Some(Self::GlyphOutline),
            Packing::Zero | Packing::One => None,
        }
    }

    /// Decodes a channel value into an RGBA pixel. `Glyph` and `Outline`
    /// become a white shape; `GlyphOutline` becomes a white glyph with a black
    /// outline.
    fn decode(self, value: u8) -> [u8; 4] {
        match self {
            Self::Glyph | Self::Outline => [u8::MAX, u8::MAX, u8::MAX, value],
            Self::GlyphOutline => {
                let doubled = u16::from(value) * 2;
                let intensity =
                    u8::try_from(doubled.saturating_sub(u16::from(u8::MAX))).unwrap_or(u8::MAX);
                let alpha = u8::try_from(doubled).unwrap_or(u8::MAX);
                [intensity, intensity, intensity, alpha]
            }
        }
    }
}

/// How a font page's pixels are turned into the texture glyphs are drawn
/// from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PageDecoding {
    /// The page is used as-is.
    AsIs,
    /// The shape is read from a single channel.
    Channel {
        /// Index of the channel in an RGBA pixel.
        channel: usize,
        /// What the channel holds.
        content: ChannelContent,
    },
    /// Alpha is read from an outline channel and the color from a glyph
    /// channel, giving a white glyph with a black outline.
    OutlineAndGlyph {
        /// Index of the outline channel in an RGBA pixel.
        outline: usize,
        /// Index of the glyph channel in an RGBA pixel.
        glyph: usize,
    },
}

impl PageDecoding {
    /// The suffix added to the labels of a packed page's texture and layout,
    /// naming the channel the glyphs are read from.
    pub(super) fn packed_label_suffix(self) -> &'static str {
        match self {
            Self::Channel { channel, .. } => match channel {
                RED => "_red",
                GREEN => "_green",
                BLUE => "_blue",
                _ => "_alpha",
            },
            Self::AsIs | Self::OutlineAndGlyph { .. } => "",
        }
    }

    /// Whether a page decoded this way holds an outline around its glyphs,
    /// with the glyph in the color channels and the outline in alpha.
    pub(super) fn has_outline(self) -> bool {
        matches!(
            self,
            Self::OutlineAndGlyph { .. }
                | Self::Channel {
                    content: ChannelContent::GlyphOutline,
                    ..
                }
        )
    }

    /// Decodes a single RGBA pixel in place.
    fn decode_pixel(self, pixel: &mut [u8; 4]) {
        match self {
            Self::AsIs => {}
            Self::Channel { channel, content } => {
                *pixel = content.decode(pixel[channel]);
            }
            Self::OutlineAndGlyph { outline, glyph } => {
                let (outline, glyph) = (pixel[outline], pixel[glyph]);
                *pixel = [glyph, glyph, glyph, outline];
            }
        }
    }

    /// Decodes the pixels of `image` in place. Images that are not 8-bit RGBA
    /// are left as-is.
    pub(super) fn decode_image(self, image: &mut Image) {
        if self == Self::AsIs {
            return;
        }
        let format = image.texture_descriptor.format;
        if !matches!(
            format,
            TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb
        ) {
            warn!(
                "Can't decode BMFont channels of a page image in format {format:?}; only 8-bit \
                RGBA images are supported. Using the image as-is."
            );
            return;
        }
        let Some(data) = image.data.as_mut() else {
            warn!("Can't decode BMFont channels of a page image without data. Using it as-is.");
            return;
        };
        for pixel in data.as_chunks_mut::<4>().0 {
            self.decode_pixel(pixel);
        }
    }
}

/// Expands an 8-bit grayscale page image into 8-bit RGBA; other images are
/// left as-is.
///
/// A grayscale image without alpha is a `BMFont` 8-bit page, which holds the
/// alpha channel, when `alpha` (the font's `alphaChnl`) holds shape data; it
/// is then expanded to white with the gray value as alpha, and its content is
/// returned. Otherwise, the gray value is copied to the color channels with
/// opaque alpha, as is the gray value of a grayscale image with alpha.
pub(super) fn expand_grayscale_page(image: &mut Image, alpha: Packing) -> Option<ChannelContent> {
    let format = image.texture_descriptor.format;
    let data = image.data.as_mut()?;
    let alpha_content = ChannelContent::from_packing(alpha);
    if format == TextureFormat::R8Unorm {
        *data = data
            .iter()
            .flat_map(|&value| {
                if alpha_content.is_some() {
                    [u8::MAX, u8::MAX, u8::MAX, value]
                } else {
                    [value, value, value, u8::MAX]
                }
            })
            .collect();
    } else if format == TextureFormat::Rg8Unorm {
        *data = data
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|&[gray, alpha]| [gray, gray, gray, alpha])
            .collect();
    } else {
        return None;
    }
    image.texture_descriptor.format = TextureFormat::Rgba8UnormSrgb;
    alpha_content.filter(|_| format == TextureFormat::R8Unorm)
}

/// Splits a decoded page that holds an outline (see
/// [`PageDecoding::has_outline`]) into its outline layer and its glyph layer.
/// Both are white, with alpha holding the coverage of the outline and of the
/// glyph, respectively.
///
/// Returns `None` for images that are not 8-bit RGBA or have no data.
pub(super) fn outline_layers(image: &Image) -> Option<(Image, Image)> {
    if !matches!(
        image.texture_descriptor.format,
        TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb
    ) {
        return None;
    }
    let data = image.data.as_ref()?;

    let mut outline = image.clone();
    let mut glyph = image.clone();
    for (layer, coverage_channel) in [(&mut outline, ALPHA), (&mut glyph, RED)] {
        let layer_data = layer
            .data
            .as_mut()
            .unwrap_or_else(|| unreachable!("a clone of an image with data has data"));
        for (layer_pixel, pixel) in layer_data
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(data.as_chunks::<4>().0)
        {
            *layer_pixel = [u8::MAX, u8::MAX, u8::MAX, pixel[coverage_channel]];
        }
    }
    Some((outline, glyph))
}

/// A font's channel settings, deciding how each glyph's page is decoded.
#[derive(Debug, Clone, Copy)]
pub(super) enum ChannelSettings {
    /// Every glyph is drawn from all channels of its page, decoded the same
    /// way.
    NotPacked(PageDecoding),
    /// Each glyph is in the channel its `chnl` names, all of which hold the
    /// same content.
    Packed(ChannelContent),
}

impl ChannelSettings {
    /// Reads the channel settings of a font.
    pub(super) fn new(common: &Common) -> Self {
        if common.packed {
            let content = ChannelContent::from_packing(common.alpha_chnl).unwrap_or_else(|| {
                warn!(
                    "Packed BMFont has alphaChnl {:?}, which holds no glyph data. Reading \
                    glyphs as if it were Glyph.",
                    common.alpha_chnl
                );
                ChannelContent::Glyph
            });
            Self::Packed(content)
        } else {
            Self::NotPacked(not_packed_decoding(common))
        }
    }

    /// The settings for a `BMFont` 8-bit page expanded by
    /// [`expand_grayscale_page`], whose alpha channel holds `alpha`.
    pub(super) fn for_8_bit_page(self, alpha: ChannelContent) -> Self {
        match self {
            Self::NotPacked(_) => Self::NotPacked(match alpha {
                ChannelContent::Glyph | ChannelContent::Outline => PageDecoding::AsIs,
                ChannelContent::GlyphOutline => PageDecoding::Channel {
                    channel: ALPHA,
                    content: ChannelContent::GlyphOutline,
                },
            }),
            Self::Packed(_) => self,
        }
    }

    /// Whether glyphs are packed into separate channels.
    pub(super) fn is_packed(self) -> bool {
        matches!(self, Self::Packed(_))
    }

    /// How the page of a glyph with the given `chnl` is decoded.
    pub(super) fn decoding_for(self, chnl: Chnl) -> PageDecoding {
        match self {
            Self::NotPacked(decoding) => decoding,
            Self::Packed(content) => {
                let channel = match chnl {
                    Chnl::RED => RED,
                    Chnl::GREEN => GREEN,
                    Chnl::BLUE => BLUE,
                    Chnl::ALPHA => ALPHA,
                    _ => return PageDecoding::AsIs,
                };
                PageDecoding::Channel { channel, content }
            }
        }
    }
}

/// How the pages of a font that is not packed are decoded.
fn not_packed_decoding(common: &Common) -> PageDecoding {
    let color_channels = [
        (RED, common.red_chnl),
        (GREEN, common.green_chnl),
        (BLUE, common.blue_chnl),
    ];
    let find = |content| {
        color_channels
            .iter()
            .find(|&&(_, packing)| ChannelContent::from_packing(packing) == Some(content))
            .map(|&(channel, _)| channel)
    };

    match ChannelContent::from_packing(common.alpha_chnl) {
        Some(ChannelContent::Glyph) => return PageDecoding::AsIs,
        Some(ChannelContent::Outline) => {
            return find(ChannelContent::Glyph).map_or(PageDecoding::AsIs, |glyph| {
                PageDecoding::OutlineAndGlyph {
                    outline: ALPHA,
                    glyph,
                }
            });
        }
        Some(ChannelContent::GlyphOutline) => {
            return PageDecoding::Channel {
                channel: ALPHA,
                content: ChannelContent::GlyphOutline,
            };
        }
        None => {}
    }

    if let Some(channel) = find(ChannelContent::GlyphOutline) {
        return PageDecoding::Channel {
            channel,
            content: ChannelContent::GlyphOutline,
        };
    }
    match (find(ChannelContent::Outline), find(ChannelContent::Glyph)) {
        (Some(outline), Some(glyph)) => PageDecoding::OutlineAndGlyph { outline, glyph },
        (Some(channel), None) => PageDecoding::Channel {
            channel,
            content: ChannelContent::Outline,
        },
        (None, Some(channel)) => PageDecoding::Channel {
            channel,
            content: ChannelContent::Glyph,
        },
        (None, None) => {
            warn!(
                "BMFont has no channel holding glyph data (alphaChnl {:?}, redChnl {:?}, \
                greenChnl {:?}, blueChnl {:?}). Using its pages as-is.",
                common.alpha_chnl, common.red_chnl, common.green_chnl, common.blue_chnl
            );
            PageDecoding::AsIs
        }
    }
}

#[cfg(test)]
mod tests;
