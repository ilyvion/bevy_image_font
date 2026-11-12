use bevy::color::palettes::css;
use bevy_color::Color;
use float_eq::assert_float_eq;

use super::*;
use crate::tests::utils::{
    COMPARISON_TOLERANCE, MONOSPACE_FONT_WIDTH, VARIABLE_WIDTH_FONT_CHARACTER_WIDTHS,
    VARIABLE_WIDTH_FONT_HEIGHT,
};

/// Creates a [`RenderContextTester`] that applies per-character offsets.
fn tester(app: &App, handle: Handle<ImageFont>) -> RenderContextTester<'_> {
    let mut tester = RenderContextTester::new(app, handle);
    tester.render_config.offset_characters = true;
    tester
}

#[test]
#[cfg_attr(feature = "gizmos", ignore = "test cannot run with `gizmos` feature")]
#[expect(
    clippy::cast_precision_loss,
    reason = "the magnitude of the numbers we're working on here are too small to lose \
        anything"
)]
fn scale() {
    let (app, handle) = initialize_app_with_loaded_example_font(ExampleFont::Monospace);
    let render_context_tester = tester(&app, handle);

    render_context_tester.modify_and_then_test_with(
        |tester| {
            tester.image_font_text.font_height = Some(100.0);
        },
        |render_context| {
            let render_context = render_context.unwrap();

            // Verify scale for valid font height
            let scale = render_context.scale();
            assert_float_eq!(
                scale,
                100.0 / render_context.max_height() as f32,
                abs <= COMPARISON_TOLERANCE
            );
        },
    );

    render_context_tester.modify_and_then_test_with(
        |tester| {
            tester.image_font_text.font_height = None;
        },
        |render_context| {
            let render_context = render_context.unwrap();

            // Verify scale defaults to 1.0 for no font height
            let scale = render_context.scale();
            assert_float_eq!(scale, 1.0, abs <= COMPARISON_TOLERANCE);
        },
    );
}

#[test]
#[cfg_attr(feature = "gizmos", ignore = "test cannot run with `gizmos` feature")]
#[expect(
    clippy::cast_precision_loss,
    reason = "the magnitude of the numbers we're working on here are too small to lose \
        anything"
)]
fn extreme_scale_works() {
    let (app, handle) = initialize_app_with_loaded_example_font(ExampleFont::Monospace);
    let render_context_tester = tester(&app, handle);

    render_context_tester.modify_and_then_test_with(
        |tester| {
            // Very small height
            tester.image_font_text.font_height = Some(1e-6);
        },
        |render_context| {
            let render_context = render_context.unwrap();
            let scale = render_context.scale();
            assert_float_eq!(
                scale,
                1e-6 / MONOSPACE_FONT_HEIGHT as f32,
                abs <= COMPARISON_TOLERANCE
            );
        },
    );

    render_context_tester.modify_and_then_test_with(
        |tester| {
            // Very large height
            tester.image_font_text.font_height = Some(1e6);
        },
        |render_context| {
            let render_context = render_context.unwrap();
            let scale = render_context.scale();
            assert_float_eq!(
                scale,
                1e6 / MONOSPACE_FONT_HEIGHT as f32,
                abs <= COMPARISON_TOLERANCE
            );
        },
    );
}

#[test]
#[cfg_attr(feature = "gizmos", ignore = "test cannot run with `gizmos` feature")]
#[expect(
    clippy::cast_precision_loss,
    reason = "the magnitude of the numbers we're working on here are too small to lose \
        anything"
)]
fn text_width() {
    let (app, handle) = initialize_app_with_loaded_example_font(ExampleFont::Monospace);
    let render_context_tester = tester(&app, handle);

    render_context_tester.test_with_defaults(|render_context| {
        let width = render_context.text_width();

        // Verify the total width is computed correctly
        assert_float_eq!(
            width,
            render_context.filtered_text.filtered_chars().count() as f32
                * MONOSPACE_FONT_WIDTH as f32,
            abs <= COMPARISON_TOLERANCE
        );
    });
}

#[test]
#[cfg_attr(feature = "gizmos", ignore = "test cannot run with `gizmos` feature")]
#[expect(
    clippy::cast_precision_loss,
    reason = "the magnitude of the numbers we're working on here are too small to lose \
        anything"
)]
fn character_dimensions() {
    let (app, handle) = initialize_app_with_loaded_example_font(ExampleFont::Monospace);
    let render_context_tester = tester(&app, handle);

    render_context_tester.test_with_defaults(|render_context| {
        let (width, height) = render_context.character_dimensions('A');

        // Verify width and height are valid
        assert_float_eq!(
            width,
            MONOSPACE_FONT_WIDTH as f32,
            abs <= COMPARISON_TOLERANCE
        );
        assert_float_eq!(
            height,
            MONOSPACE_FONT_HEIGHT as f32,
            abs <= COMPARISON_TOLERANCE
        );
    });
}

#[test]
#[cfg_attr(feature = "gizmos", ignore = "test cannot run with `gizmos` feature")]
#[expect(
    clippy::cast_precision_loss,
    reason = "the magnitude of the numbers we're working on here are too small to lose \
        anything"
)]
fn test_mixed_character_widths() {
    let (app, handle) = initialize_app_with_loaded_example_font(ExampleFont::VariableWidth);
    let render_context_tester = tester(&app, handle);

    render_context_tester.modify_and_then_test_with(
        |tester| {
            tester.image_font_text.text = String::from("IIMMII");
        },
        |render_context| {
            let render_context = render_context.unwrap();

            // Calculate total width
            let total_width = render_context.text_width();
            assert_float_eq!(
                total_width,
                4. * VARIABLE_WIDTH_FONT_CHARACTER_WIDTHS[&'I'] as f32
                    + 2. * VARIABLE_WIDTH_FONT_CHARACTER_WIDTHS[&'M'] as f32,
                abs <= COMPARISON_TOLERANCE
            );

            // Verify individual dimensions
            for character in render_context.text().filtered_chars() {
                let (width, height) = render_context.character_dimensions(character);
                assert_float_eq!(
                    width,
                    VARIABLE_WIDTH_FONT_CHARACTER_WIDTHS[&character] as f32,
                    abs <= COMPARISON_TOLERANCE
                );
                assert_float_eq!(
                    height,
                    VARIABLE_WIDTH_FONT_HEIGHT as f32,
                    abs <= COMPARISON_TOLERANCE
                );
            }
        },
    );
}

#[test]
#[cfg_attr(feature = "gizmos", ignore = "test cannot run with `gizmos` feature")]
fn anchor_offsets() {
    use crate::render_context::anchors::AnchorExt as _;

    let (app, handle) = initialize_app_with_loaded_example_font(ExampleFont::Monospace);
    let render_context_tester = tester(&app, handle);

    render_context_tester.test_with_defaults(|render_context| {
        let offsets = render_context.anchor_offsets();

        // Verify the offsets match expected values
        assert_eq!(
            offsets,
            render_context
                .render_config
                .text_anchor
                .to_anchor_offsets(true)
        );
    });
}

#[test]
#[cfg_attr(feature = "gizmos", ignore = "test cannot run with `gizmos` feature")]
#[expect(
    clippy::cast_precision_loss,
    reason = "the magnitude of the numbers we're working on here are too small to lose \
        anything"
)]
fn transform() {
    let (app, handle) = initialize_app_with_loaded_example_font(ExampleFont::Monospace);
    let render_context_tester = tester(&app, handle);

    render_context_tester.test_with_defaults(|render_context| {
        let mut x_pos = 0.0;
        let transform = render_context.transform(
            &mut x_pos,
            render_context.text().filtered_chars().next().unwrap(),
            Some(render_context.text().filtered_chars().next().unwrap()),
        );

        // Verify the transform is calculated correctly
        assert_float_eq!(transform.translation.x, -7.5, abs <= COMPARISON_TOLERANCE);
        assert_float_eq!(transform.scale.x, 1.0, abs <= COMPARISON_TOLERANCE);

        // Verify x_pos is updated
        assert_float_eq!(
            x_pos,
            MONOSPACE_FONT_WIDTH as f32,
            abs <= COMPARISON_TOLERANCE
        );
    });
}

#[test]
#[cfg_attr(feature = "gizmos", ignore = "test cannot run with `gizmos` feature")]
fn sprite_transform_snaps_glyph_edges_to_whole_units() {
    let (app, handle) = initialize_app_with_loaded_example_font(ExampleFont::Monospace);
    let render_context_tester = tester(&app, handle);

    render_context_tester.modify_and_then_test_with(
        |tester| {
            // An odd number of glyphs centered on the origin puts glyph edges on half
            // units.
            tester.image_font_text.text = String::from("Tes");
        },
        |render_context| {
            let render_context = render_context.unwrap();
            let first_char = render_context.text().filtered_chars().next().unwrap();

            let unsnapped = render_context.transform(&mut 0.0, first_char, None);
            assert_float_eq!(unsnapped.translation.x, -5.0, abs <= COMPARISON_TOLERANCE);

            let mut x_pos = 0.0;
            let snapped = render_context.sprite_transform(&mut x_pos, first_char, None);

            // Left edge -7.5 snaps to -8.
            assert_float_eq!(snapped.translation.x, -5.5, abs <= COMPARISON_TOLERANCE);
            assert_float_eq!(snapped.translation.y, 0.0, abs <= COMPARISON_TOLERANCE);
            // Snapping only moves the glyph; the next glyph's position is unaffected.
            #[expect(
                clippy::cast_precision_loss,
                reason = "the magnitude of the numbers we're working on here are too small to \
                    lose anything"
            )]
            let expected_x_pos = MONOSPACE_FONT_WIDTH as f32;
            assert_float_eq!(x_pos, expected_x_pos, abs <= COMPARISON_TOLERANCE);
        },
    );
}

#[test]
#[cfg_attr(feature = "gizmos", ignore = "test cannot run with `gizmos` feature")]
fn update_sprite_values() {
    let (app, handle) = initialize_app_with_loaded_example_font(ExampleFont::Monospace);
    let render_context_tester = tester(&app, handle);

    render_context_tester.test_with_defaults(|render_context| {
        let first_char = render_context.text().filtered_chars().next().unwrap();

        let mut texture_atlas = render_context.font_texture_atlas('A');
        let mut color: Color = css::AZURE.into();

        // Make sure the values aren't what we expect after the change before we call
        // update_sprite_values
        assert_ne!(
            texture_atlas.index,
            render_context.image_font.atlas_character_map[&first_char].character_index
        );
        assert_ne!(color, render_context.render_config.color);

        render_context.update_render_values(first_char, &mut texture_atlas, &mut color);

        // Verify the texture atlas and color are updated
        assert_eq!(
            texture_atlas.index,
            render_context.image_font.atlas_character_map[&first_char].character_index
        );
        assert_eq!(color, render_context.render_config.color);
    });
}

#[test]
#[cfg_attr(feature = "gizmos", ignore = "test cannot run with `gizmos` feature")]
fn empty_text_has_no_width() {
    let (app, handle) = initialize_app_with_loaded_example_font(ExampleFont::Monospace);
    let render_context_tester = tester(&app, handle);

    render_context_tester.modify_and_then_test_with(
        |tester| {
            tester.image_font_text.text = String::new();
        },
        |render_context| {
            let render_context = render_context.unwrap();

            assert_float_eq!(
                render_context.text_width(),
                0.0,
                abs <= COMPARISON_TOLERANCE
            );
        },
    );
}

#[test]
#[cfg_attr(feature = "gizmos", ignore = "test cannot run with `gizmos` feature")]
#[expect(
    clippy::cast_precision_loss,
    reason = "the magnitude of the numbers we're working on here are too small to lose \
        anything"
)]
fn large_text_block() {
    let (app, handle) = initialize_app_with_loaded_example_font(ExampleFont::Monospace);
    let render_context_tester = tester(&app, handle);

    render_context_tester.modify_and_then_test_with(
        |tester| {
            tester.image_font_text.text = "A".repeat(10_000); // Large text
        },
        |render_context| {
            let render_context = render_context.unwrap();

            // Verify width scales linearly with text length
            let width = render_context.text_width();
            assert_float_eq!(
                width,
                10_000.0 * MONOSPACE_FONT_WIDTH as f32,
                abs <= COMPARISON_TOLERANCE
            );
        },
    );
}

#[cfg(feature = "bmf")]
mod bmfont {
    use bevy_sprite::Anchor;

    use super::*;

    /// Creates a [`RenderContextTester`] for `text` in
    /// [`ExampleFont::BmFont`] that applies per-character offsets.
    fn tester<'app>(
        app: &'app App,
        handle: Handle<ImageFont>,
        text: &str,
    ) -> RenderContextTester<'app> {
        let mut tester = super::super::bmfont::tester(app, handle, text);
        tester.render_config.offset_characters = true;
        tester
    }

    #[test]
    #[cfg_attr(feature = "gizmos", ignore = "test cannot run with `gizmos` feature")]
    fn scale_is_relative_to_line_height() {
        let (app, handle) = initialize_app_with_loaded_example_font(ExampleFont::BmFont);

        // "aaa" has no glyph as tall as the "g" in "Ag".
        for text in ["aaa", "Ag"] {
            tester(&app, handle.clone(), text).modify_and_then_test_with(
                |tester| tester.image_font_text.font_height = Some(72.0),
                |render_context| {
                    let render_context = render_context.unwrap();
                    assert_float_eq!(render_context.scale(), 2.0, abs <= COMPARISON_TOLERANCE);
                },
            );
        }
    }

    #[test]
    #[cfg_attr(feature = "gizmos", ignore = "test cannot run with `gizmos` feature")]
    fn text_width_sums_advances_and_kerning() {
        let (app, handle) = initialize_app_with_loaded_example_font(ExampleFont::BmFont);

        tester(&app, handle, "AV").test_with_defaults(|render_context| {
            // A advance 21, kerning -2, V advance 21.
            assert_float_eq!(
                render_context.text_width(),
                40.0,
                abs <= COMPARISON_TOLERANCE
            );
        });
    }

    #[test]
    #[cfg_attr(feature = "gizmos", ignore = "test cannot run with `gizmos` feature")]
    fn advances_and_kerning_scale_with_font_height() {
        let (app, handle) = initialize_app_with_loaded_example_font(ExampleFont::BmFont);

        tester(&app, handle, "AV").modify_and_then_test_with(
            |tester| {
                tester.image_font_text.font_height = Some(72.0);
                tester.render_config.apply_scaling = true;
            },
            |render_context| {
                let render_context = render_context.unwrap();

                let mut x_pos = 0.0;
                render_context.transform(&mut x_pos, 'A', Some('V'));
                assert_float_eq!(x_pos, 38.0, abs <= COMPARISON_TOLERANCE);

                assert_float_eq!(
                    render_context.text_width(),
                    80.0,
                    abs <= COMPARISON_TOLERANCE
                );
            },
        );
    }

    #[test]
    #[cfg_attr(feature = "gizmos", ignore = "test cannot run with `gizmos` feature")]
    fn letter_spacing_is_added_to_advances() {
        let (app, handle) = initialize_app_with_loaded_example_font(ExampleFont::BmFont);

        tester(&app, handle, "AV").modify_and_then_test_with(
            |tester| tester.render_config.letter_spacing = 2.0,
            |render_context| {
                let render_context = render_context.unwrap();

                let mut x_pos = 0.0;
                render_context.transform(&mut x_pos, 'A', Some('V'));
                assert_float_eq!(x_pos, 21.0, abs <= COMPARISON_TOLERANCE);

                assert_float_eq!(
                    render_context.text_width(),
                    44.0,
                    abs <= COMPARISON_TOLERANCE
                );
            },
        );
    }

    #[test]
    #[cfg_attr(feature = "gizmos", ignore = "test cannot run with `gizmos` feature")]
    fn glyph_top_is_line_top_minus_y_offset() {
        let (app, handle) = initialize_app_with_loaded_example_font(ExampleFont::BmFont);

        tester(&app, handle, "A").modify_and_then_test_with(
            |tester| tester.render_config.text_anchor = Anchor::BOTTOM_LEFT,
            |render_context| {
                let render_context = render_context.unwrap();

                let transform = render_context.transform(&mut 0.0, 'A', None);

                // The line spans 0..36 above the anchor; A is 23 tall with a
                // y offset of 6, so its top is at 30.
                assert_float_eq!(
                    transform.translation.y + 23.0 / 2.0,
                    30.0,
                    abs <= COMPARISON_TOLERANCE
                );
            },
        );
    }
}
