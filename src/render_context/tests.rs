use bevy::asset::Handle;
use bevy_app::App;

use super::*;
use crate::tests::utils::{
    ExampleFont, MONOSPACE_FONT_HEIGHT, initialize_app_with_loaded_example_font,
};

#[cfg(feature = "atlas_sprites")]
mod sprites;

#[test]
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
fn empty_text() {
    let (app, handle) = initialize_app_with_loaded_example_font(ExampleFont::Monospace);
    let render_context_tester = RenderContextTester::new(&app, handle);

    render_context_tester.modify_and_then_test_with(
        |tester| {
            tester.image_font_text.text = String::new();
        },
        |render_context| {
            let render_context = render_context.unwrap();

            // Default min height
            assert_eq!(render_context.max_height(), 1);
        },
    );
}

#[cfg(feature = "bmf")]
mod bmfont {
    use super::*;
    use crate::tests::utils::BMFONT_LINE_HEIGHT;

    pub(super) fn tester<'app>(
        app: &'app App,
        handle: Handle<ImageFont>,
        text: &str,
    ) -> RenderContextTester<'app> {
        let mut tester = RenderContextTester::new(app, handle);
        tester.image_font_text.text = String::from(text);
        tester
    }

    #[test]
    fn line_height_does_not_depend_on_text() {
        let (app, handle) = initialize_app_with_loaded_example_font(ExampleFont::BmFont);

        // "aaa" has no glyph as tall as the "g" in "Ag".
        for text in ["aaa", "Ag"] {
            tester(&app, handle.clone(), text).test_with_defaults(|render_context| {
                assert_eq!(render_context.max_height(), BMFONT_LINE_HEIGHT, "{text}");
            });
        }
    }
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
            font: handle.clone(),
            font_height: None,
        };

        let image_font_assets = app.world().resource::<Assets<ImageFont>>();

        let atlas_layout_assets = app.world().resource::<Assets<TextureAtlasLayout>>();

        Self {
            image_font_text,
            render_config: RenderConfig::default(),
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
