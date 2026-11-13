#![allow(clippy::unwrap_used, reason = "test code panics to indicate errors")]

use super::*;

/// A `Common` block with the given channel settings.
fn common(packed: bool, alpha: Packing, red: Packing, green: Packing, blue: Packing) -> Common {
    Common::new(10, 8, 16, 16, 1, packed, alpha, red, green, blue)
}

/// The decoding of a font that is not packed.
fn not_packed(alpha: Packing, red: Packing, green: Packing, blue: Packing) -> PageDecoding {
    match ChannelSettings::new(&common(false, alpha, red, green, blue)) {
        ChannelSettings::NotPacked(decoding) => decoding,
        ChannelSettings::Packed(_) => panic!("expected a font that is not packed"),
    }
}

#[test]
fn alpha_holding_shape_is_used_as_is() {
    for alpha in [Packing::Glyph, Packing::Outline] {
        assert_eq!(
            not_packed(alpha, Packing::One, Packing::One, Packing::One),
            PageDecoding::AsIs,
            "{alpha:?}"
        );
    }
    assert_eq!(
        not_packed(
            Packing::Glyph,
            Packing::Glyph,
            Packing::Glyph,
            Packing::Glyph
        ),
        PageDecoding::AsIs
    );
}

#[test]
fn outline_in_alpha_with_glyph_in_color_channels_is_read_as_outline_and_glyph() {
    assert_eq!(
        not_packed(
            Packing::Outline,
            Packing::Glyph,
            Packing::Glyph,
            Packing::Glyph
        ),
        PageDecoding::OutlineAndGlyph {
            outline: ALPHA,
            glyph: RED
        }
    );
    assert_eq!(
        not_packed(
            Packing::Outline,
            Packing::Zero,
            Packing::Zero,
            Packing::Glyph
        ),
        PageDecoding::OutlineAndGlyph {
            outline: ALPHA,
            glyph: BLUE
        }
    );
}

#[test]
fn only_decodings_with_glyph_and_outline_have_an_outline() {
    let channel = |content| PageDecoding::Channel {
        channel: RED,
        content,
    };
    assert!(channel(ChannelContent::GlyphOutline).has_outline());
    assert!(
        PageDecoding::OutlineAndGlyph {
            outline: ALPHA,
            glyph: RED
        }
        .has_outline()
    );
    assert!(!channel(ChannelContent::Glyph).has_outline());
    assert!(!channel(ChannelContent::Outline).has_outline());
    assert!(!PageDecoding::AsIs.has_outline());
}

#[test]
fn outline_layers_split_glyph_from_outline() {
    use bevy_asset::RenderAssetUsages;
    use wgpu_types::{Extent3d, TextureDimension};

    let image = |data, format| {
        Image::new(
            Extent3d {
                width: 2,
                height: 1,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data,
            format,
            RenderAssetUsages::default(),
        )
    };

    let (outline, glyph) = outline_layers(&image(
        vec![0, 0, 0, 255, 100, 100, 100, 200],
        TextureFormat::Rgba8UnormSrgb,
    ))
    .unwrap();
    assert_eq!(
        outline.data.unwrap(),
        [255, 255, 255, 255, 255, 255, 255, 200]
    );
    assert_eq!(glyph.data.unwrap(), [255, 255, 255, 0, 255, 255, 255, 100]);

    assert!(outline_layers(&image(vec![0, 0], TextureFormat::R8Unorm)).is_none());
}

#[test]
fn alpha_holding_glyph_and_outline_is_decoded() {
    assert_eq!(
        not_packed(
            Packing::GlyphOutline,
            Packing::One,
            Packing::One,
            Packing::One
        ),
        PageDecoding::Channel {
            channel: ALPHA,
            content: ChannelContent::GlyphOutline
        }
    );
}

#[test]
fn constant_alpha_reads_shape_from_color_channels() {
    for alpha in [Packing::One, Packing::Zero] {
        assert_eq!(
            not_packed(alpha, Packing::Glyph, Packing::Glyph, Packing::Glyph),
            PageDecoding::Channel {
                channel: RED,
                content: ChannelContent::Glyph
            },
            "{alpha:?}"
        );
        assert_eq!(
            not_packed(alpha, Packing::Zero, Packing::Outline, Packing::Zero),
            PageDecoding::Channel {
                channel: GREEN,
                content: ChannelContent::Outline
            },
            "{alpha:?}"
        );
        assert_eq!(
            not_packed(alpha, Packing::Glyph, Packing::Zero, Packing::GlyphOutline),
            PageDecoding::Channel {
                channel: BLUE,
                content: ChannelContent::GlyphOutline
            },
            "{alpha:?}"
        );
        assert_eq!(
            not_packed(alpha, Packing::Glyph, Packing::Outline, Packing::Glyph),
            PageDecoding::OutlineAndGlyph {
                outline: GREEN,
                glyph: RED
            },
            "{alpha:?}"
        );
    }
}

#[test]
fn no_channel_with_shape_is_used_as_is() {
    assert_eq!(
        not_packed(Packing::One, Packing::One, Packing::Zero, Packing::One),
        PageDecoding::AsIs
    );
}

#[test]
fn packed_glyphs_are_read_from_their_channel() {
    let settings = ChannelSettings::new(&common(
        true,
        Packing::GlyphOutline,
        Packing::GlyphOutline,
        Packing::GlyphOutline,
        Packing::GlyphOutline,
    ));
    assert!(settings.is_packed());
    for (chnl, channel) in [
        (Chnl::RED, RED),
        (Chnl::GREEN, GREEN),
        (Chnl::BLUE, BLUE),
        (Chnl::ALPHA, ALPHA),
    ] {
        assert_eq!(
            settings.decoding_for(chnl),
            PageDecoding::Channel {
                channel,
                content: ChannelContent::GlyphOutline
            },
            "{chnl:?}"
        );
    }
    assert_eq!(settings.decoding_for(Chnl::ALL), PageDecoding::AsIs);
}

#[test]
fn packed_font_with_constant_alpha_reads_glyphs() {
    let settings = ChannelSettings::new(&common(
        true,
        Packing::One,
        Packing::One,
        Packing::One,
        Packing::One,
    ));
    assert_eq!(
        settings.decoding_for(Chnl::RED),
        PageDecoding::Channel {
            channel: RED,
            content: ChannelContent::Glyph
        }
    );
}

#[test]
fn not_packed_ignores_glyph_channel() {
    let settings = ChannelSettings::new(&common(
        false,
        Packing::One,
        Packing::Glyph,
        Packing::Glyph,
        Packing::Glyph,
    ));
    assert!(!settings.is_packed());
    assert_eq!(
        settings.decoding_for(Chnl::GREEN),
        PageDecoding::Channel {
            channel: RED,
            content: ChannelContent::Glyph
        }
    );
}

#[test]
fn glyph_and_outline_values_decode_to_white_glyph_with_black_outline() {
    let decode = |value| ChannelContent::GlyphOutline.decode(value);
    assert_eq!(decode(0), [0, 0, 0, 0]);
    assert_eq!(decode(64), [0, 0, 0, 128]);
    assert_eq!(decode(127), [0, 0, 0, 254]);
    assert_eq!(decode(128), [1, 1, 1, 255]);
    assert_eq!(decode(192), [129, 129, 129, 255]);
    assert_eq!(decode(255), [255, 255, 255, 255]);
}

#[test]
fn glyph_and_outline_content_decode_to_white_shape() {
    for content in [ChannelContent::Glyph, ChannelContent::Outline] {
        assert_eq!(content.decode(0), [255, 255, 255, 0], "{content:?}");
        assert_eq!(content.decode(100), [255, 255, 255, 100], "{content:?}");
    }
}

#[test]
fn pixels_decode_from_their_channels() {
    let decode = |decoding: PageDecoding| {
        let mut pixel = [10, 20, 30, 40];
        decoding.decode_pixel(&mut pixel);
        pixel
    };
    assert_eq!(decode(PageDecoding::AsIs), [10, 20, 30, 40]);
    assert_eq!(
        decode(PageDecoding::Channel {
            channel: BLUE,
            content: ChannelContent::Glyph
        }),
        [255, 255, 255, 30]
    );
    assert_eq!(
        decode(PageDecoding::OutlineAndGlyph {
            outline: GREEN,
            glyph: RED
        }),
        [10, 10, 10, 20]
    );
}

#[test]
fn packed_label_suffix_names_channel() {
    let suffix = |channel| {
        PageDecoding::Channel {
            channel,
            content: ChannelContent::Glyph,
        }
        .packed_label_suffix()
    };
    assert_eq!(suffix(RED), "_red");
    assert_eq!(suffix(GREEN), "_green");
    assert_eq!(suffix(BLUE), "_blue");
    assert_eq!(suffix(ALPHA), "_alpha");
    assert_eq!(PageDecoding::AsIs.packed_label_suffix(), "");
}

/// A 2×1 image with the given data and format.
fn two_pixel_image(data: Vec<u8>, format: TextureFormat) -> Image {
    use bevy_asset::RenderAssetUsages;
    use wgpu_types::{Extent3d, TextureDimension};

    Image::new(
        Extent3d {
            width: 2,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        format,
        RenderAssetUsages::default(),
    )
}

#[test]
fn gray_page_with_shape_in_alpha_expands_to_white_with_value_as_alpha() {
    for (alpha, content) in [
        (Packing::Glyph, ChannelContent::Glyph),
        (Packing::Outline, ChannelContent::Outline),
        (Packing::GlyphOutline, ChannelContent::GlyphOutline),
    ] {
        let mut image = two_pixel_image(vec![0, 200], TextureFormat::R8Unorm);
        assert_eq!(expand_grayscale_page(&mut image, alpha), Some(content));
        assert_eq!(
            image.texture_descriptor.format,
            TextureFormat::Rgba8UnormSrgb
        );
        assert_eq!(image.data.unwrap(), [255, 255, 255, 0, 255, 255, 255, 200]);
    }
}

#[test]
fn gray_page_with_constant_alpha_expands_to_opaque_gray() {
    for alpha in [Packing::One, Packing::Zero] {
        let mut image = two_pixel_image(vec![0, 200], TextureFormat::R8Unorm);
        assert_eq!(expand_grayscale_page(&mut image, alpha), None);
        assert_eq!(
            image.texture_descriptor.format,
            TextureFormat::Rgba8UnormSrgb
        );
        assert_eq!(image.data.unwrap(), [0, 0, 0, 255, 200, 200, 200, 255]);
    }
}

#[test]
fn gray_and_alpha_page_expands_to_gray_with_its_alpha() {
    let mut image = two_pixel_image(vec![0, 255, 100, 50], TextureFormat::Rg8Unorm);
    assert_eq!(expand_grayscale_page(&mut image, Packing::Glyph), None);
    assert_eq!(
        image.texture_descriptor.format,
        TextureFormat::Rgba8UnormSrgb
    );
    assert_eq!(image.data.unwrap(), [0, 0, 0, 255, 100, 100, 100, 50]);
}

#[test]
fn rgba_page_is_not_expanded() {
    let data = vec![1, 2, 3, 4, 5, 6, 7, 8];
    let mut image = two_pixel_image(data.clone(), TextureFormat::Rgba8UnormSrgb);
    assert_eq!(expand_grayscale_page(&mut image, Packing::Glyph), None);
    assert_eq!(image.data.unwrap(), data);
}

#[test]
fn eight_bit_page_is_decoded_from_alpha_content_only() {
    let page_decoding = |alpha, color| match ChannelSettings::new(&common(
        false,
        Packing::One,
        color,
        color,
        color,
    ))
    .for_8_bit_page(alpha)
    {
        ChannelSettings::NotPacked(decoding) => decoding,
        ChannelSettings::Packed(_) => panic!("expected a font that is not packed"),
    };
    for color in [Packing::One, Packing::Glyph, Packing::Outline] {
        for alpha in [ChannelContent::Glyph, ChannelContent::Outline] {
            assert_eq!(
                page_decoding(alpha, color),
                PageDecoding::AsIs,
                "alpha {alpha:?}, color {color:?}"
            );
        }
        assert_eq!(
            page_decoding(ChannelContent::GlyphOutline, color),
            PageDecoding::Channel {
                channel: ALPHA,
                content: ChannelContent::GlyphOutline
            },
            "color {color:?}"
        );
    }
}
