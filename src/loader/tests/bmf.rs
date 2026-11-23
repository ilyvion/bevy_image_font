use bevy_app::App;
use bevy_asset::{AssetLoadError, AssetPath, Assets, Handle};
use bevy_image::{Image, TextureAtlasLayout};
use bevy_math::{URect, UVec2, Vec2};

use crate::ImageFont;
use crate::loader::BmFontLoadError;
use crate::tests::utils::{initialize_app_with_font, wait_until_failed, wait_until_loaded};

/// Loads the font at `path` and returns the first pixel of the texture each of
/// `characters` is drawn from, along with that texture's index.
fn first_pixels(path: &str, characters: &[char]) -> Vec<(usize, [u8; 4])> {
    let (mut app, handle): (App, Handle<ImageFont>) = initialize_app_with_font(path);
    wait_until_loaded(&mut app, &handle);

    let world = app.world();
    let font = world
        .resource::<Assets<ImageFont>>()
        .get(&handle)
        .expect("font is loaded");
    let images = world.resource::<Assets<Image>>();
    characters
        .iter()
        .map(|character| {
            let page_index = font.atlas_character_map[character].page_index;
            let image = images
                .get(&font.textures[page_index])
                .expect("font texture is loaded");
            let data = image.data.as_ref().expect("font texture has data");
            (
                page_index,
                data[..4].try_into().expect("a pixel is 4 bytes"),
            )
        })
        .collect()
}

#[test]
fn font_with_glyph_past_page_edge_fails_to_load() {
    // The page is 2x2, and `A` is 2 pixels wide starting at x = 1.
    let (mut app, handle): (App, Handle<ImageFont>) =
        initialize_app_with_font("tests/glyph_out_of_bounds.txt.fnt");

    let error = wait_until_failed(&mut app, &handle);

    let AssetLoadError::AssetLoaderError(ref loader_error) = error else {
        panic!("expected a loader error, got {error}");
    };
    let bmf_error = loader_error
        .error()
        .downcast_ref::<BmFontLoadError>()
        .expect("the loader error is a BmFontLoadError");
    assert!(
        matches!(
            *bmf_error,
            BmFontLoadError::CharacterRectOutOfBounds {
                character: 'A',
                rect: URect {
                    min: UVec2 { x: 1, y: 0 },
                    max: UVec2 { x: 3, y: 2 }
                },
                page: 0,
                page_size: UVec2 { x: 2, y: 2 },
            }
        ),
        "{bmf_error:?}"
    );
}

#[test]
fn font_with_char_on_missing_page_fails_to_load() {
    // The font has only page 0, and `B` is on page 1.
    let (mut app, handle): (App, Handle<ImageFont>) =
        initialize_app_with_font("tests/char_on_missing_page.txt.fnt");

    let error = wait_until_failed(&mut app, &handle);

    let AssetLoadError::AssetLoaderError(ref loader_error) = error else {
        panic!("expected a loader error, got {error}");
    };
    let bmf_error = loader_error
        .error()
        .downcast_ref::<BmFontLoadError>()
        .expect("the loader error is a BmFontLoadError");
    assert!(
        matches!(
            *bmf_error,
            BmFontLoadError::BmFontError(bmfont_rs::Error::InvalidCharPage {
                char_id: 66,
                page_id: 1,
            })
        ),
        "{bmf_error:?}"
    );
}

#[test]
fn font_with_unsupported_extension_fails_to_load() {
    // The `.meta` file routes the `.fnt` file to `BmFontLoader`.
    let (mut app, handle): (App, Handle<ImageFont>) =
        initialize_app_with_font("tests/unsupported_extension.fnt");

    let error = wait_until_failed(&mut app, &handle);

    let AssetLoadError::AssetLoaderError(ref loader_error) = error else {
        panic!("expected a loader error, got {error}");
    };
    let bmf_error = loader_error
        .error()
        .downcast_ref::<BmFontLoadError>()
        .expect("the loader error is a BmFontLoadError");
    assert!(
        matches!(
            *bmf_error,
            BmFontLoadError::UnsupportedExtension(ref path)
                if *path == AssetPath::from("tests/unsupported_extension.fnt")
        ),
        "{bmf_error:?}"
    );
}

/// Loads the font at `path` and returns its glyphs: each character's page,
/// rectangle on that page, offsets, advance and kernings, sorted by character.
fn glyphs(path: &str) -> Vec<(char, usize, URect, Vec2, Option<f32>, Vec<(char, f32)>)> {
    let (mut app, handle): (App, Handle<ImageFont>) = initialize_app_with_font(path);
    wait_until_loaded(&mut app, &handle);

    let world = app.world();
    let font = world.resource::<Assets<ImageFont>>().get(&handle).unwrap();
    let layouts = world.resource::<Assets<TextureAtlasLayout>>();
    let mut glyphs: Vec<_> = font
        .atlas_character_map
        .iter()
        .map(|(&character, glyph)| {
            let layout = layouts.get(&font.atlas_layouts[glyph.page_index]).unwrap();
            let mut kernings: Vec<_> = glyph
                .kernings
                .iter()
                .map(|(&next, &kerning)| (next, kerning))
                .collect();
            kernings.sort_by_key(|&(next, _)| next);
            (
                character,
                glyph.page_index,
                layout.textures[glyph.character_index],
                glyph.offsets,
                glyph.x_advance,
                kernings,
            )
        })
        .collect();
    glyphs.sort_by_key(|&(character, ..)| character);
    glyphs
}

#[test]
fn xml_and_binary_fonts_load_the_same_glyphs_as_the_text_font() {
    let text_glyphs = glyphs("bmf.txt.fnt");
    assert_ne!(text_glyphs.len(), 0);

    assert_eq!(glyphs("bmf.xml.fnt"), text_glyphs);
    assert_eq!(glyphs("bmf.bin.fnt"), text_glyphs);
}

#[test]
fn characters_with_invalid_ids_are_skipped() {
    // Character id 55296 (U+D800) is a surrogate, which is not a `char`.
    let glyphs = glyphs("tests/invalid_char_id.txt.fnt");

    let characters: Vec<_> = glyphs.iter().map(|&(character, ..)| character).collect();
    assert_eq!(characters, ['A']);
}

#[test]
fn packed_font_draws_each_glyph_from_its_channel() {
    // Every pixel of the page is (200, 100, 50, 25). `A` is in the red
    // channel, `B` in the green channel and `C` in all channels.
    let pixels = first_pixels("tests/channels_packed.txt.fnt", &['A', 'B', 'C']);

    assert_eq!(pixels[0].1, [255, 255, 255, 200]);
    assert_eq!(pixels[1].1, [255, 255, 255, 100]);
    assert_eq!(pixels[2].1, [200, 100, 50, 25]);

    assert_ne!(pixels[0].0, pixels[1].0);
    assert_ne!(pixels[0].0, pixels[2].0);
    assert_ne!(pixels[1].0, pixels[2].0);
}

#[test]
fn font_with_alpha_set_to_one_draws_glyph_from_color_channels() {
    // Every pixel of the page is (200, 200, 200, 255).
    let pixels = first_pixels("tests/channels_alpha_one.txt.fnt", &['A']);

    assert_eq!(pixels[0].1, [255, 255, 255, 200]);
}

/// The pixel data of a font texture and its outline layers.
struct TextureData {
    /// The texture's pixel data.
    texture: Vec<u8>,
    /// The pixel data of the texture's outline layers, if it has them.
    layers: Option<LayerData>,
}

/// The pixel data of a texture's outline layers.
struct LayerData {
    /// The outline layer's pixel data.
    outline: Vec<u8>,
    /// The glyph layer's pixel data.
    glyph: Vec<u8>,
}

/// Loads the font at `path` and returns the pixel data of each of its textures
/// and their outline layers.
fn texture_and_layer_data(path: &str) -> Vec<TextureData> {
    let (mut app, handle): (App, Handle<ImageFont>) = initialize_app_with_font(path);
    wait_until_loaded(&mut app, &handle);

    let world = app.world();
    let font = world
        .resource::<Assets<ImageFont>>()
        .get(&handle)
        .expect("font is loaded");
    let images = world.resource::<Assets<Image>>();
    let data = |handle: &Handle<Image>| {
        images
            .get(handle)
            .expect("font image is loaded")
            .data
            .clone()
            .expect("font image has data")
    };
    font.textures
        .iter()
        .enumerate()
        .map(|(page_index, texture)| {
            let layers = font
                .outline_layers
                .get(page_index)
                .and_then(Option::as_ref)
                .map(|layers| LayerData {
                    outline: data(&layers.outline),
                    glyph: data(&layers.glyph),
                });
            TextureData {
                texture: data(texture),
                layers,
            }
        })
        .collect()
}

#[test]
fn font_with_outline_in_alpha_has_outline_and_glyph_layers() {
    // The page's pixels are an outline (0, 0, 0, 255), a glyph
    // (255, 255, 255, 255), nothing (0, 0, 0, 0) and a partly covered glyph
    // inside the outline (100, 100, 100, 255).
    let textures = texture_and_layer_data("tests/channels_outline.txt.fnt");

    assert_eq!(textures.len(), 1);
    assert_eq!(
        textures[0].texture,
        [
            0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 0, 100, 100, 100, 255
        ]
    );
    let layers = textures[0]
        .layers
        .as_ref()
        .expect("the texture has outline layers");
    assert_eq!(
        layers.outline,
        [
            255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 255, 255, 255, 255
        ]
    );
    assert_eq!(
        layers.glyph,
        [
            255, 255, 255, 0, 255, 255, 255, 255, 255, 255, 255, 0, 255, 255, 255, 100
        ]
    );
}

#[test]
fn packed_font_with_glyph_and_outline_has_layers_for_every_channel() {
    let textures = texture_and_layer_data("bmf_packed_outline.txt.fnt");

    assert_eq!(textures.len(), 4);
    for texture in &textures {
        let layers = texture
            .layers
            .as_ref()
            .expect("the texture has outline layers");
        for ((pixel, outline_pixel), glyph_pixel) in texture
            .texture
            .as_chunks::<4>()
            .0
            .iter()
            .zip(layers.outline.as_chunks::<4>().0)
            .zip(layers.glyph.as_chunks::<4>().0)
        {
            assert_eq!(*outline_pixel, [255, 255, 255, pixel[3]]);
            assert_eq!(*glyph_pixel, [255, 255, 255, pixel[0]]);
        }
    }
}

#[test]
fn fonts_without_outline_have_no_outline_layers() {
    for path in ["bmf.txt.fnt", "bmf_packed.txt.fnt"] {
        let textures = texture_and_layer_data(path);
        assert!(
            textures.iter().all(|texture| texture.layers.is_none()),
            "{path}"
        );
    }
}

#[test]
fn each_character_gets_only_the_kernings_it_is_first_in() {
    // `A` is kerned with `B` and `C`, `B` with `A` and `C`, and `C` with
    // nothing. The kernings of `A` and `B` are interleaved in the file.
    let (mut app, handle): (App, Handle<ImageFont>) =
        initialize_app_with_font("tests/kernings.txt.fnt");
    wait_until_loaded(&mut app, &handle);

    let font = app
        .world()
        .resource::<Assets<ImageFont>>()
        .get(&handle)
        .expect("font is loaded");

    let kernings = |character: char| {
        let mut kernings: Vec<(char, f32)> = font.atlas_character_map[&character]
            .kernings
            .iter()
            .map(|(&second, &amount)| (second, amount))
            .collect();
        kernings.sort_unstable_by_key(|&(second, _)| second);
        kernings
    };
    assert_eq!(kernings('A'), [('B', -1.0), ('C', -2.0)]);
    assert_eq!(kernings('B'), [('A', 3.0), ('C', 4.0)]);
    assert_eq!(kernings('C'), []);
}

#[cfg(feature = "bmf_charsets")]
#[test]
fn non_unicode_font_maps_character_and_kerning_ids_through_its_charset() {
    // A Shift JIS font with `A` (65) and `あ` (0x82A0), kerned by -1.
    let (mut app, handle): (App, Handle<ImageFont>) =
        initialize_app_with_font("tests/charset_shiftjis.txt.fnt");
    wait_until_loaded(&mut app, &handle);

    let font = app
        .world()
        .resource::<Assets<ImageFont>>()
        .get(&handle)
        .expect("font is loaded");

    let mut characters: Vec<char> = font.atlas_character_map.keys().copied().collect();
    characters.sort_unstable();
    assert_eq!(characters, ['A', 'あ']);
    assert_eq!(
        font.atlas_character_map[&'A'].kernings.get(&'あ'),
        Some(&-1.0)
    );
}

#[cfg(feature = "bmf_charsets")]
#[test]
fn code_page_437_example_font_maps_every_code_to_its_glyph() {
    let (mut app, handle): (App, Handle<ImageFont>) =
        initialize_app_with_font("cp437_10x10.txt.fnt");
    wait_until_loaded(&mut app, &handle);

    let world = app.world();
    let font = world
        .resource::<Assets<ImageFont>>()
        .get(&handle)
        .expect("font is loaded");
    let images = world.resource::<Assets<Image>>();
    let layouts = world.resource::<Assets<TextureAtlasLayout>>();

    assert_eq!(font.atlas_character_map.len(), 256);

    // `█` (0xDB) fills its whole cell, ` ` (0x20) leaves it empty, `╔` (0xC9)
    // and `Ñ` (0xA5) are on the grid where code page 437 puts them.
    let image = images
        .get(&font.textures[0])
        .expect("font texture is loaded");
    let layout = layouts
        .get(&font.atlas_layouts[0])
        .expect("font layout is loaded");
    for (character, code) in [('█', 0xDB), ('╔', 0xC9), ('Ñ', 0xA5)] {
        let rect = layout.textures[font.atlas_character_map[&character].character_index];
        assert_eq!(
            (rect.min.x, rect.min.y),
            (code % 16 * 10, code / 16 * 10),
            "{character:?}"
        );
    }

    let data = image.data.as_ref().expect("font texture has data");
    for (character, alpha) in [('█', 255), (' ', 0)] {
        let rect = layout.textures[font.atlas_character_map[&character].character_index];
        for y in rect.min.y..rect.max.y {
            for x in rect.min.x..rect.max.x {
                let offset = usize::try_from((y * image.width() + x) * 4).expect("offset fits");
                assert_eq!(
                    data[offset..offset + 4],
                    [255, 255, 255, alpha],
                    "{character:?} at ({x}, {y})"
                );
            }
        }
    }
}

#[test]
fn packed_bmfont_output_draws_white_glyphs_from_four_textures() {
    // Generated by AngelCode BMFont from `bmf_packed.bmfc`, with glyphs in all
    // four channels of a single page.
    let (mut app, handle): (App, Handle<ImageFont>) =
        initialize_app_with_font("bmf_packed.txt.fnt");
    wait_until_loaded(&mut app, &handle);

    let world = app.world();
    let font = world
        .resource::<Assets<ImageFont>>()
        .get(&handle)
        .expect("font is loaded");
    let images = world.resource::<Assets<Image>>();
    let layouts = world.resource::<Assets<TextureAtlasLayout>>();

    assert_eq!(font.textures.len(), 4);

    for (character, image_font_character) in &font.atlas_character_map {
        let image = images
            .get(&font.textures[image_font_character.page_index])
            .expect("font texture is loaded");
        let rect = layouts
            .get(&font.atlas_layouts[image_font_character.page_index])
            .expect("font layout is loaded")
            .textures[image_font_character.character_index];
        let data = image.data.as_ref().expect("font texture has data");
        let width = image.width();

        let mut has_coverage = false;
        for y in rect.min.y..rect.max.y {
            for x in rect.min.x..rect.max.x {
                let offset = usize::try_from((y * width + x) * 4).expect("offset fits in usize");
                let pixel = &data[offset..offset + 4];
                assert_eq!(pixel[..3], [255, 255, 255], "{character:?} at ({x}, {y})");
                has_coverage |= pixel[3] > 0;
            }
        }
        assert!(
            has_coverage || *character == ' ',
            "{character:?} has no visible pixels"
        );
    }
}
