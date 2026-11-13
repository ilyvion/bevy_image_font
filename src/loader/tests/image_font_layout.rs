use super::*;

/// Test `ImageFontLayout::Automatic` creates the correct character map.
#[test]
fn layout_automatic() {
    let layout = ImageFontLayout::Automatic("ABCD\nEFGH\nIJKL".to_string());

    // Each char is 100x100
    let image_size = UVec2::new(400, 300); // 4 characters wide, 3 rows
    let char_map = layout
        .into_character_rect_map(image_size)
        .expect("valid layout");

    assert_eq!(char_map.len(), 12);
    assert!(('A'..'L').all(|character| char_map.contains_key(&character)));
    assert_eq!(
        char_map[&'A'],
        // A is at 0 right, 0 down
        URect::new(0, 0, 100, 100),
        "Character 'A' has incorrect bounds."
    );
    assert_eq!(
        char_map[&'H'],
        // H is at 3 right, 1 down
        URect::new(300, 100, 400, 200),
        "Character 'H' has incorrect bounds."
    );
    assert_eq!(
        char_map[&'J'],
        // H is at 1 right, 2 down
        URect::new(100, 200, 200, 300),
        "Character 'J' has incorrect bounds."
    );
}

#[test]
fn layout_automatic_invalid_image_width() {
    let layout = ImageFontLayout::Automatic("AB\nCD".to_string());
    let image_size = UVec2::new(301, 200); // 301 / 2 is not a whole number
    let result = layout.into_character_rect_map(image_size);

    assert!(matches!(
        result,
        Err(ImageFontLayoutValidationError::InvalidImageWidth {
            width: 301,
            per_line_character_count: 2
        })
    ));
}

#[test]
fn layout_automatic_invalid_image_height() {
    let layout = ImageFontLayout::Automatic("AB\nCD".to_string());
    let image_size = UVec2::new(300, 201); // 201 / 2 is not a whole number
    let result = layout.into_character_rect_map(image_size);

    assert!(matches!(
        result,
        Err(ImageFontLayoutValidationError::InvalidImageHeight {
            height: 201,
            line_count: 2
        })
    ));
}

#[test]
fn layout_automatic_repeated_characters_error() {
    // Define a layout string with repeated characters
    let layout = ImageFontLayout::Automatic("AB\nAC".to_string());

    let image_size = UVec2::new(200, 100); // Each cell is 100x100 pixels
    let result = layout.into_character_rect_map(image_size);

    // Verify that an error is returned due to repeated characters
    assert!(
        matches!(
            result,
            Err(ImageFontLayoutValidationError::AutomaticRepeatedCharacter {
                row: 1,
                column: 0,
                character: 'A'
            })
        ),
        "{result:?}"
    );
}

#[test]
fn layout_manual_monospace() {
    let layout = ImageFontLayout::ManualMonospace {
        size: UVec2::new(10, 20),
        coords: HashMap::from([('a', UVec2::new(0, 0)), ('b', UVec2::new(10, 0))]),
    };

    let image_size = UVec2::new(100, 50);
    let char_map = layout
        .into_character_rect_map(image_size)
        .expect("valid layout");

    assert_eq!(char_map.len(), 2);
    assert_eq!(char_map[&'a'], URect::new(0, 0, 10, 20));
    assert_eq!(char_map[&'b'], URect::new(10, 0, 20, 20));
}

#[test]
fn test_image_font_layout_manual() {
    let layout = ImageFontLayout::Manual(HashMap::from([
        ('x', URect::new(0, 0, 5, 5)),
        ('y', URect::new(5, 5, 15, 15)),
    ]));

    let image_size = UVec2::new(100, 100);
    let char_map = layout
        .into_character_rect_map(image_size)
        .expect("valid layout");

    assert_eq!(char_map.len(), 2);
    assert_eq!(char_map[&'x'], URect::new(0, 0, 5, 5));
    assert_eq!(char_map[&'y'], URect::new(5, 5, 15, 15));
}

#[test]
fn layout_manual_accepts_rect_touching_image_edges() {
    let layout = ImageFontLayout::Manual(HashMap::from([('x', URect::new(0, 0, 10, 20))]));

    let char_map = layout
        .into_character_rect_map(UVec2::new(10, 20))
        .expect("valid layout");

    assert_eq!(char_map[&'x'], URect::new(0, 0, 10, 20));
}

#[test]
fn layout_manual_rect_past_image_edge_is_an_error() {
    for rect in [URect::new(5, 0, 11, 20), URect::new(0, 5, 10, 21)] {
        let layout = ImageFontLayout::Manual(HashMap::from([('x', rect)]));

        let result = layout.into_character_rect_map(UVec2::new(10, 20));

        assert!(
            matches!(
                result,
                Err(ImageFontLayoutValidationError::CharacterRectOutOfBounds {
                    character: 'x',
                    rect: error_rect,
                    image_size: UVec2 { x: 10, y: 20 },
                }) if error_rect == rect
            ),
            "{rect:?}: {result:?}"
        );
    }
}

#[test]
fn layout_manual_inverted_rect_is_an_error() {
    // `URect::new` would normalize the corners.
    let rect = URect {
        min: UVec2::new(5, 0),
        max: UVec2::new(4, 10),
    };
    let layout = ImageFontLayout::Manual(HashMap::from([('x', rect)]));

    let result = layout.into_character_rect_map(UVec2::new(10, 10));

    assert!(
        matches!(
            result,
            Err(ImageFontLayoutValidationError::InvertedCharacterRect {
                character: 'x',
                rect: error_rect,
            }) if error_rect == rect
        ),
        "{result:?}"
    );
}

#[test]
fn layout_manual_monospace_rect_past_image_edge_is_an_error() {
    let layout = ImageFontLayout::ManualMonospace {
        size: UVec2::new(10, 20),
        coords: HashMap::from([('a', UVec2::new(0, 0)), ('b', UVec2::new(95, 0))]),
    };

    let result = layout.into_character_rect_map(UVec2::new(100, 50));

    assert!(
        matches!(
            result,
            Err(ImageFontLayoutValidationError::CharacterRectOutOfBounds {
                character: 'b',
                rect: URect {
                    min: UVec2 { x: 95, y: 0 },
                    max: UVec2 { x: 105, y: 20 }
                },
                image_size: UVec2 { x: 100, y: 50 },
            })
        ),
        "{result:?}"
    );
}

#[test]
fn layout_manual_monospace_rect_past_u32_max_is_an_error() {
    let layout = ImageFontLayout::ManualMonospace {
        size: UVec2::new(10, 20),
        coords: HashMap::from([('a', UVec2::new(u32::MAX - 5, 0))]),
    };

    let result = layout.into_character_rect_map(UVec2::new(100, 50));

    assert!(
        matches!(
            result,
            Err(ImageFontLayoutValidationError::CharacterRectOutOfBounds { character: 'a', .. })
        ),
        "{result:?}"
    );
}

#[test]
fn layout_manual_empty_map() {
    let layout = ImageFontLayout::Manual(HashMap::new());
    let image_size = UVec2::new(100, 100);
    let char_map = layout
        .into_character_rect_map(image_size)
        .expect("valid layout");

    assert!(
        char_map.is_empty(),
        "Expected empty char map for empty input."
    );
}
