use bevy::color::palettes::css;
use float_eq::assert_float_eq;

use super::*;
use crate::tests::utils::{
    initialize_app_with_loaded_example_font, ExampleFont, COMPARISON_TOLERANCE,
    MONOSPACE_FONT_HEIGHT, MONOSPACE_FONT_WIDTH, VARIABLE_WIDTH_FONT_CHARACTER_WIDTHS,
    VARIABLE_WIDTH_FONT_HEIGHT,
};

#[test]
#[cfg_attr(feature = "gizmos", ignore = "test cannot run with `gizmos` feature")]
fn render_context_creation() {
    let (app, handle) = initialize_app_with_loaded_example_font(ExampleFont::Monospace);
    let render_context_tester = RenderContextTester::new(&app, handle);

    // Test valid creation
    render_context_tester.modify_and_then_test_with(
        |_| {},
        |render_context| {
            assert!(render_context.is_some());
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
fn scale() {
    let (app, handle) = initialize_app_with_loaded_example_font(ExampleFont::Monospace);
    let render_context_tester = RenderContextTester::new(&app, handle);

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
    let render_context_tester = RenderContextTester::new(&app, handle);

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
fn max_height() {
    let (app, handle) = initialize_app_with_loaded_example_font(ExampleFont::Monospace);
    let render_context_tester = RenderContextTester::new(&app, handle);

    render_context_tester.test_with_defaults(|render_context| {
        // Calculate max height
        let max_height = render_context.max_height();
        assert_eq!(max_height, MONOSPACE_FONT_HEIGHT);

        // Ensure the same (cached) value is returned on subsequent calls
        assert_eq!(render_context.max_height(), max_height);
    });
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
    let render_context_tester = RenderContextTester::new(&app, handle);

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
    let render_context_tester = RenderContextTester::new(&app, handle);

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
    let render_context_tester = RenderContextTester::new(&app, handle);

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
    let (app, handle) = initialize_app_with_loaded_example_font(ExampleFont::Monospace);
    let render_context_tester = RenderContextTester::new(&app, handle);

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
    let render_context_tester = RenderContextTester::new(&app, handle);

    render_context_tester.test_with_defaults(|render_context| {
        let mut x_pos = 0.0;
        let transform = render_context.transform(
            &mut x_pos,
            render_context.text().filtered_chars().next().unwrap(),
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
fn update_sprite_values() {
    let (app, handle) = initialize_app_with_loaded_example_font(ExampleFont::Monospace);
    let render_context_tester = RenderContextTester::new(&app, handle);

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
fn empty_text() {
    let (app, handle) = initialize_app_with_loaded_example_font(ExampleFont::Monospace);
    let render_context_tester = RenderContextTester::new(&app, handle);

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
            // Default min height
            assert_eq!(render_context.max_height(), 1);
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
    let render_context_tester = RenderContextTester::new(&app, handle);

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

#[derive(Clone)]
struct RenderContextTester<'app> {
    image_font_text: ImageFontText,
    image_font_assets: &'app Assets<ImageFont>,
    atlas_layout_assets: &'app Assets<TextureAtlasLayout>,
    render_config: RenderConfig,
}

impl<'app> RenderContextTester<'app> {
    fn new(app: &'app App, handle: Handle<ImageFont>) -> Self {
        let image_font_text = ImageFontText {
            text: String::from("Test"),
            font: handle.clone_weak(),
            font_height: None,
        };

        let image_font_assets = app.world().resource::<Assets<ImageFont>>();

        let atlas_layout_assets = app.world().resource::<Assets<TextureAtlasLayout>>();

        Self {
            image_font_text,
            render_config: RenderConfig {
                offset_characters: true,
                ..default()
            },
            image_font_assets,
            atlas_layout_assets,
        }
    }

    fn test_with_defaults(&self, test_func: impl FnOnce(RenderContext<'_>)) {
        let font_handle = &self.image_font_text.font;
        let image_font = self.image_font_assets.get(font_handle).unwrap();

        let render_context = RenderContext::new(
            image_font,
            &self.image_font_text,
            self.render_config,
            self.atlas_layout_assets,
        )
        .unwrap();

        test_func(render_context);
    }

    #[track_caller]
    fn modify_and_then_test_with(
        &self,
        modify_func: impl FnOnce(&mut Self),
        test_func: impl FnOnce(Option<RenderContext<'_>>),
    ) {
        let mut modified_clone = self.clone();
        modify_func(&mut modified_clone);

        let font_handle = &modified_clone.image_font_text.font;
        let image_font = modified_clone.image_font_assets.get(font_handle).unwrap();

        let render_context = RenderContext::new(
            image_font,
            &modified_clone.image_font_text,
            modified_clone.render_config,
            modified_clone.atlas_layout_assets,
        );

        test_func(render_context);
    }
}
