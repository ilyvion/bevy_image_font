use bevy::camera::{Camera, Camera2d, ComputedCameraValues, RenderTargetInfo};
use bevy::input::InputPlugin;
use bevy::text::TextPlugin;
use bevy::transform::TransformPlugin;
use bevy_app::App;
use bevy_math::UVec2;
use bevy_ui::widget::ImageNode;
use bevy_ui::{ComputedNode, Node, UiPlugin, UiScale};

use crate::ImageFontText;
use crate::rendered::ImageFontPreRenderedUiText;
use crate::tests::utils::{
    ExampleFont, MONOSPACE_FONT_HEIGHT, MONOSPACE_FONT_WIDTH,
    initialize_app_with_loaded_example_font,
};

/// Adds `UiPlugin` and the plugins it depends on, and a camera whose render
/// target has a scale factor of 1.
fn add_ui_layout(app: &mut App) {
    app.add_plugins((InputPlugin, TransformPlugin, TextPlugin, UiPlugin));

    app.world_mut().spawn((
        Camera2d,
        Camera {
            computed: ComputedCameraValues {
                target_info: Some(RenderTargetInfo {
                    physical_size: UVec2::new(800, 600),
                    scale_factor: 1.,
                }),
                ..Default::default()
            },
            ..Default::default()
        },
    ));
}

/// Issue #17: pre-rendered UI text keeps its size in logical pixels when
/// `UiScale` changes, so its node grows with the scale, without the text being
/// rendered again.
#[test]
fn pre_rendered_ui_text_follows_ui_scale_without_re_rendering() {
    let (mut app, font) = initialize_app_with_loaded_example_font(ExampleFont::Monospace);
    add_ui_layout(&mut app);

    let text = "Hello";
    let entity = app
        .world_mut()
        .spawn((
            Node::default(),
            ImageFontText::default().text(text).font(font),
            ImageFontPreRenderedUiText::default(),
        ))
        .id();
    app.update();

    let unscaled_size = UVec2::new(
        MONOSPACE_FONT_WIDTH * u32::try_from(text.len()).unwrap(),
        MONOSPACE_FONT_HEIGHT,
    )
    .as_vec2();
    assert_eq!(
        app.world().get::<ComputedNode>(entity).unwrap().size,
        unscaled_size
    );
    let rendered_image = app.world().get::<ImageNode>(entity).unwrap().image.clone();

    app.world_mut().resource_mut::<UiScale>().0 = 2.;
    app.update();

    assert_eq!(
        app.world().get::<ComputedNode>(entity).unwrap().size,
        unscaled_size * 2.,
        "the node's physical size should follow `UiScale`"
    );
    assert_eq!(
        app.world().get::<ImageNode>(entity).unwrap().image,
        rendered_image,
        "a `UiScale` change should not render the text again"
    );
}
