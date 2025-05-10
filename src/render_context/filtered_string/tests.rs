#![allow(clippy::unwrap_used, reason = "test code panics to indicate errors")]
use bevy::utils::default;

use super::*;

#[test]
fn filters_chars() {
    let mut atlas_character_map = HashMap::new();
    atlas_character_map.insert(
        'a',
        ImageFontCharacter {
            page_index: 0,
            character_index: 1,
            ..default()
        },
    );
    atlas_character_map.insert(
        'b',
        ImageFontCharacter {
            page_index: 0,
            character_index: 2,
            ..default()
        },
    );

    let filtered_string = FilteredString::new("abcd", &atlas_character_map);
    let filtered_chars: Vec<_> = filtered_string.filtered_chars().collect();

    assert_eq!(filtered_chars, vec!['a', 'b']);
}

#[test]
fn is_empty_when_no_characters_retained() {
    let mut atlas_character_map = HashMap::new();
    atlas_character_map.insert(
        'x',
        ImageFontCharacter {
            page_index: 0,
            character_index: 1,
            ..default()
        },
    );
    atlas_character_map.insert(
        'y',
        ImageFontCharacter {
            page_index: 0,
            character_index: 2,
            ..default()
        },
    );

    let filtered_string = FilteredString::new("abc", &atlas_character_map);

    assert!(filtered_string.is_empty());
}

#[test]
fn is_not_empty_when_characters_retained() {
    let mut atlas_character_map = HashMap::new();
    atlas_character_map.insert(
        'a',
        ImageFontCharacter {
            page_index: 0,
            character_index: 1,
            ..default()
        },
    );

    let filtered_string = FilteredString::new("abc", &atlas_character_map);

    assert!(!filtered_string.is_empty());
}

#[test]
fn display_shows_filtered_text() {
    let mut atlas_character_map = HashMap::new();
    atlas_character_map.insert(
        'a',
        ImageFontCharacter {
            page_index: 0,
            character_index: 1,
            ..default()
        },
    );
    atlas_character_map.insert(
        'b',
        ImageFontCharacter {
            page_index: 0,
            character_index: 2,
            ..default()
        },
    );

    let filtered_string = FilteredString::new("abcd", &atlas_character_map);

    assert_eq!(filtered_string.to_string(), "ab");
}

#[cfg(any(feature = "rendered", feature = "atlas_sprites"))]
#[test]
fn test_image_font_filter_string() {
    use bevy::asset::Handle;
    use bevy_image::ImageSampler;

    use crate::{ImageFont, ImageFontCharacter};

    let mut atlas_character_map = HashMap::new();
    atlas_character_map.insert(
        'A',
        ImageFontCharacter {
            page_index: 0,
            character_index: 0,
            ..default()
        },
    );
    atlas_character_map.insert(
        'B',
        ImageFontCharacter {
            page_index: 0,
            character_index: 1,
            ..default()
        },
    );
    let font = ImageFont {
        atlas_layouts: vec![Handle::default()],
        textures: vec![Handle::default()],
        atlas_character_map: atlas_character_map.clone(),
        image_sampler: ImageSampler::nearest(),
    };

    let input = "ABC";
    let filtered = font.filter_string(input);
    assert_eq!(filtered.to_string(), "AB");
}
