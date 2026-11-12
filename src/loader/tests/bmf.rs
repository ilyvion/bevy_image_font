use bevy_app::App;
use bevy_asset::{Assets, Handle};
use bevy_image::{Image, TextureAtlasLayout};

use crate::ImageFont;
use crate::tests::utils::{initialize_app_with_font, wait_until_loaded};

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
