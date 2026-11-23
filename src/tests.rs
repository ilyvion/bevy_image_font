#![allow(clippy::unwrap_used, reason = "test code panics to indicate errors")]
#![allow(clippy::expect_used, reason = "test code panics to indicate errors")]

use std::any::TypeId;

use bevy_asset::AssetServer;
use bevy_ecs::reflect::AppTypeRegistry;
use bevy_math::{URect, UVec2};

use super::*;
use crate::tests::utils::{
    ExampleFont, assert_in_image_font_set, initialize_app_with_example_font,
};

mod sync_texts_with_font_changes;
#[cfg(feature = "ui")]
mod ui_scale;
pub(crate) mod utils;

#[test]
fn mapped_atlas_layout_from_char_map_creates_correct_character_map_and_layout() {
    let size = UVec2::new(256, 256);
    let mut char_rect_map = HashMap::new();
    char_rect_map.insert('A', URect::new(0, 0, 16, 16));
    char_rect_map.insert('B', URect::new(16, 0, 32, 16));

    let (atlas_character_map, atlas_layout) = ImageFont::mapped_atlas_layout_from_char_map(
        0,
        size,
        char_rect_map.iter().map(|(&char, &rect)| (char, rect)),
    );

    assert_eq!(atlas_character_map.len(), 2);
    assert!(atlas_character_map.contains_key(&'A'));
    assert!(atlas_character_map.contains_key(&'B'));
    assert_eq!(atlas_layout.textures.len(), 2);
    assert_eq!(
        atlas_layout.textures[atlas_character_map[&'A'].character_index],
        char_rect_map[&'A']
    );
    assert_eq!(
        atlas_layout.textures[atlas_character_map[&'B'].character_index],
        char_rect_map[&'B']
    );
}

#[test]
fn image_font_plugin_initialization() {
    let (mut app, handle) = initialize_app_with_example_font(ExampleFont::Monospace);

    let asset_server = app.world().resource::<AssetServer>();
    let load_state = asset_server.get_load_state(handle.id());
    assert!(
        load_state.is_some(),
        "The `ImageFontPlugin` should allow loading `.image_font.ron` files; load state was \
        {load_state:?}, expected Some(_)."
    );

    // Verify that `ImageFont` and related types are registered with the reflection
    // system
    {
        let type_registry = app.world().resource::<AppTypeRegistry>().read();
        assert!(
            type_registry.contains(TypeId::of::<ImageFont>()),
            "The `ImageFontPlugin` should register `ImageFont` with the reflection system."
        );
        assert!(
            type_registry.contains(TypeId::of::<ImageFontText>()),
            "The `ImageFontPlugin` should register `ImageFontText` with the reflection system."
        );
    }

    // Verify that the app updates without errors (systems from the plugin are
    // functional)
    app.update();
}

#[test]
fn plugin_systems_are_in_image_font_set() {
    let (mut app, _) = initialize_app_with_example_font(ExampleFont::Monospace);

    assert_in_image_font_set(&mut app, PostUpdate, sync_texts_with_font_changes);
    #[cfg(feature = "rendered")]
    assert_in_image_font_set(&mut app, PostUpdate, rendered::render_text_to_sprite);
    #[cfg(all(feature = "rendered", feature = "ui"))]
    assert_in_image_font_set(&mut app, PostUpdate, rendered::render_text_to_image_node);
}

// This is mostly here for the sake of coverage.
#[test]
fn creating_image_font_works() {
    ImageFont::new(
        Vec::default(),
        HashMap::default(),
        Vec::default(),
        ImageSampler::default(),
    );
}
