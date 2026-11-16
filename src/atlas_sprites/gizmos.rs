//! Debug visualization for `ImageFontSpriteText` components.
//!
//! This module provides systems and configurations for rendering **debug
//! gizmos** that help visualize the alignment, positioning, and bounding boxes
//! of `ImageFontSpriteText` elements in the scene.
//!
//! # Features
//! - **Anchor Point Visualization**: Draws small crosshairs to indicate text
//!   and character anchor positions.
//! - **Bounding Box Visualization**: Renders rectangles around individual
//!   glyphs to assist with alignment debugging.
//! - **Configurable Rendering**: Gizmos can be toggled globally via
//!   [`AtlasSpritesGizmoConfigGroup`] or on a per-entity basis using
//!   [`ShowAtlasSpritesGizmos`].

use std::fmt::Debug;

use bevy_app::{App, Update};
use bevy_color::Color;
use bevy_color::palettes::css;
use bevy_ecs::component::Component;
use bevy_ecs::query::{With, Without};
use bevy_ecs::resource::Resource;
use bevy_ecs::schedule::IntoScheduleConfigs as _;
use bevy_ecs::system::Query;
use bevy_gizmos::AppGizmoBuilder as _;
use bevy_gizmos::config::GizmoConfigGroup;
use bevy_gizmos::gizmos::Gizmos;
use bevy_math::Vec2;
use bevy_reflect::Reflect;
use bevy_transform::components::GlobalTransform;

use crate::atlas_sprites::ImageFontTextData;
use crate::render_context::RenderContext;
use crate::{ImageFontSystems, ImageFontText};

/// Initializes the debug gizmo system for `ImageFontSpriteText` components.
///
/// This function sets up the necessary configuration and systems to enable
/// debug visualization of `ImageFontSpriteText` elements in Bevy's scene.
/// It registers the `AtlasSpritesGizmoConfigGroup` and adds the rendering
/// systems for drawing sprite-based text gizmos.
///
/// # Parameters
/// - `app`: A mutable reference to the [`App`] to which the gizmo systems are
///   added.
///
/// # Behavior
/// - Registers `AtlasSpritesGizmoConfigGroup`, which controls whether various
///   text gizmos (such as anchor points and bounding boxes) are rendered.
/// - Adds the `render_all_sprite_gizmos` and `render_sprite_gizmos` systems to
///   the update schedule, enabling debug visualization.
pub fn build(app: &mut App) {
    app.init_gizmo_group::<AtlasSpritesGizmoConfigGroup>()
        .add_systems(
            Update,
            (render_all_sprite_gizmos, render_sprite_gizmos).in_set(ImageFontSystems),
        );
}

/// The [`GizmoConfigGroup`] used for debug visualizations of
/// [`ImageFontSpriteText`](crate::atlas_sprites::ImageFontSpriteText)
/// components on entities
#[derive(Debug, GizmoConfigGroup, Reflect, Resource)]
pub struct AtlasSpritesGizmoConfigGroup {
    /// Draws text anchor points in the scene when set to `true`.
    ///
    /// To draw a specific entity's text anchor point, you can add the
    /// [`ShowAtlasSpritesGizmos`] component.
    ///
    /// Defaults to `false`.
    pub render_text_anchor_point: bool,

    /// The color to draws the text anchor points in the scene with.
    ///
    /// Defaults to `css::RED`.
    pub text_anchor_point_color: Color,

    /// Draws character anchor points in the scene when set to `true`.
    ///
    /// To draw a specific entity's character anchor points, you can add the
    /// [`ShowAtlasSpritesGizmos`] component.
    ///
    /// Defaults to `false`.
    pub render_character_anchor_point: bool,

    /// The color to draws the character anchor points in the scene with.
    ///
    /// Defaults to `css::GREEN`.
    pub character_anchor_point_color: Color,

    /// Draws character bounding boxes in the scene when set to `true`.
    ///
    /// To draw a specific entity's character bounding boxes, you can add the
    /// [`ShowAtlasSpritesGizmos`] component.
    ///
    /// Defaults to `false`.
    pub render_character_box: bool,

    /// The color to draws the character bounding boxes in the scene with.
    ///
    /// Defaults to `css::PURPLE`.
    pub character_box_color: Color,
}

impl Default for AtlasSpritesGizmoConfigGroup {
    fn default() -> Self {
        Self {
            render_text_anchor_point: Default::default(),
            text_anchor_point_color: css::RED.into(),
            render_character_anchor_point: Default::default(),
            character_anchor_point_color: css::GREEN.into(),
            render_character_box: Default::default(),
            character_box_color: css::PURPLE.into(),
        }
    }
}

/// This module exists entirely to circumvent an annoying Clippy bug
// #[expect(clippy::min_ident_chars, reason = "clippy bug")]
mod clippy_bug {

    use bevy_color::Color;
    use bevy_ecs::component::Component;
    use bevy_ecs::reflect::ReflectComponent;
    use bevy_reflect::{Reflect, prelude::ReflectDefault};

    #[expect(unused_imports, reason = "used in intra-doc comment")]
    use super::AtlasSpritesGizmoConfigGroup;

    /// Add this [`Component`] to an entity to draw its gizmos.
    #[derive(Component, Reflect, Default, Debug)]
    #[reflect(Component, Default, Debug)]
    pub struct ShowAtlasSpritesGizmos {
        /// Draws text anchor points in the scene when set to `true`.
        ///
        /// The default value from the [`AtlasSpritesGizmoConfigGroup`] config
        /// is used if `None`,
        pub render_text_anchor_point: Option<bool>,

        /// The color to draws the text anchor points in the scene with.
        ///
        /// The default value from the [`AtlasSpritesGizmoConfigGroup`] config
        /// is used if `None`,
        pub text_anchor_point_color: Option<Color>,

        /// Draws character anchor points in the scene when set to `true`.
        ///
        /// The default value from the [`AtlasSpritesGizmoConfigGroup`] config
        /// is used if `None`,
        pub render_character_anchor_point: Option<bool>,

        /// The color to draws the character anchor points in the scene with.
        ///
        /// The default value from the [`AtlasSpritesGizmoConfigGroup`] config
        /// is used if `None`,
        pub character_anchor_point_color: Option<Color>,

        /// Draws character bounding boxes in the scene when set to `true`.
        ///
        /// The default value from the [`AtlasSpritesGizmoConfigGroup`] config
        /// is used if `None`,
        pub render_character_box: Option<bool>,

        /// The color to draws the character bounding boxes in the scene with.
        ///
        /// The default value from the [`AtlasSpritesGizmoConfigGroup`] config
        /// is used if `None`,
        pub character_box_color: Option<Color>,
    }
}
pub use clippy_bug::ShowAtlasSpritesGizmos;

/// Debugging data for visualizing an `ImageFontSpriteText` in a scene, enabled
/// by the `gizmos` feature.
#[derive(Debug, Clone, Default, Component)]
pub struct ImageFontTextGizmoData {
    /// The bounding box size of each glyph sprite, in the same order as the
    /// glyph sprites of the text.
    glyph_sizes: Vec<Vec2>,
}

impl ImageFontTextGizmoData {
    /// Forgets the recorded glyph sizes, before the sizes of a new layout are
    /// recorded.
    pub(crate) fn clear(&mut self) {
        self.glyph_sizes.clear();
    }
}

/// Records the width and height of the next glyph sprite for gizmo
/// visualization.
///
/// This function queries the `RenderContext` for the dimensions of the given
/// character and appends them to the provided `ImageFontTextGizmoData`.
pub(crate) fn record_character_dimensions(
    render_context: &RenderContext,
    character: char,
    gizmo_data: &mut ImageFontTextGizmoData,
) {
    let (width, height) = render_context.character_dimensions(character);
    gizmo_data.glyph_sizes.push(Vec2::new(width, height));
}

/// The position and bounding box size of each glyph sprite of a text.
pub(super) fn glyph_boxes<'data>(
    data: &'data ImageFontTextData,
    child_query: &'data Query<&GlobalTransform, Without<ImageFontText>>,
) -> impl Iterator<Item = (Vec2, Vec2)> + 'data {
    data.sprites
        .iter()
        .zip(&data.gizmo_data.glyph_sizes)
        .filter_map(|(&sprite, &size)| {
            let transform = child_query.get(sprite).ok()?;
            Some((transform.translation().truncate(), size))
        })
}

/// Renders gizmos for debugging `ImageFontText` and its associated glyphs in
/// the scene.
///
/// This function draws 2D rectangles and crosshairs to visualize the bounding
/// boxes and positions of rendered glyphs, aiding in debugging and alignment.
///
/// ### Gizmo Details
/// - Each child glyph is visualized as a purple rectangle using its dimensions
///   and position.
/// - The `ImageFontText` position is marked with a red cross for easier
///   identification.
fn render_all_sprite_gizmos(
    mut gizmos: Gizmos<AtlasSpritesGizmoConfigGroup>,
    query: Query<
        (&GlobalTransform, &ImageFontTextData),
        (With<ImageFontText>, Without<ShowAtlasSpritesGizmos>),
    >,
    child_query: Query<&GlobalTransform, Without<ImageFontText>>,
) {
    for (global_transform, data) in &query {
        render_gizmos(&mut gizmos, &child_query, None, global_transform, data);
    }
}

/// Renders gizmos for debugging `ImageFontText` and its associated glyphs in
/// the scene.
///
/// This function draws 2D rectangles and crosshairs to visualize the bounding
/// boxes and positions of rendered glyphs, aiding in debugging and alignment.
///
/// ### Gizmo Details
/// - Each child glyph is visualized as a purple rectangle using its dimensions
///   and position.
/// - The `ImageFontText` position is marked with a red cross for easier
///   identification.
fn render_sprite_gizmos(
    mut gizmos: Gizmos<AtlasSpritesGizmoConfigGroup>,
    query: Query<
        (
            &ShowAtlasSpritesGizmos,
            &GlobalTransform,
            &ImageFontTextData,
        ),
        (With<ImageFontText>, With<ShowAtlasSpritesGizmos>),
    >,
    child_query: Query<&GlobalTransform, Without<ImageFontText>>,
) {
    for (sprite_gizmos, global_transform, data) in &query {
        render_gizmos(
            &mut gizmos,
            &child_query,
            Some(sprite_gizmos),
            global_transform,
            data,
        );
    }
}

/// Retrieves a gizmo configuration value, prioritizing per-entity settings.
///
/// This macro allows a [`ShowAtlasSpritesGizmos`] component (if present) to
/// override the default configuration from [`AtlasSpritesGizmoConfigGroup`].
///
/// # Parameters
/// - `$gizmos`: A reference to the [`Gizmos<AtlasSpritesGizmoConfigGroup>`]
///   instance.
/// - `$sprite_gizmos`: An `Option<&ShowAtlasSpritesGizmos>` for the entity.
/// - `$field`: The field name to retrieve.
///
/// # Behavior
/// - If the entity has a [`ShowAtlasSpritesGizmos`] component **and** the field
///   is set (`Some(value)`), the entity-specific value is used.
/// - Otherwise, it falls back to the global configuration in
///   [`AtlasSpritesGizmoConfigGroup`].
macro_rules! gizmo_config_value {
    ($gizmos:expr_2021, $sprite_gizmos:expr_2021, $field:ident) => {
        $sprite_gizmos
            .and_then(|sprite_gizmos| sprite_gizmos.$field)
            .unwrap_or($gizmos.config_ext.$field)
    };
}

/// Renders debug gizmos for a single `ImageFontText` entity and its glyph
/// sprites.
///
/// This function draws rectangles and crosshairs to visualize the bounding
/// boxes and anchor points of each glyph sprite and the text entity itself,
/// according to the current gizmo configuration. Per-entity overrides are
/// supported via `ShowAtlasSpritesGizmos`.
fn render_gizmos(
    gizmos: &mut Gizmos<AtlasSpritesGizmoConfigGroup>,
    child_query: &Query<&GlobalTransform, Without<ImageFontText>>,
    sprite_gizmos: Option<&ShowAtlasSpritesGizmos>,
    global_transform: &GlobalTransform,
    data: &ImageFontTextData,
) {
    for (position, size) in glyph_boxes(data, child_query) {
        if gizmo_config_value!(gizmos, sprite_gizmos, render_character_box) {
            let color = gizmo_config_value!(gizmos, sprite_gizmos, character_box_color);
            gizmos.rect_2d(
                bevy_math::Isometry2d::from_translation(position),
                size,
                color,
            );
        }

        if gizmo_config_value!(gizmos, sprite_gizmos, render_character_anchor_point) {
            let color = gizmo_config_value!(gizmos, sprite_gizmos, character_anchor_point_color);
            gizmos.cross_2d(bevy_math::Isometry2d::from_translation(position), 5., color);
        }
    }

    if gizmo_config_value!(gizmos, sprite_gizmos, render_text_anchor_point) {
        let color = gizmo_config_value!(gizmos, sprite_gizmos, text_anchor_point_color);
        gizmos.cross_2d(
            bevy_math::Isometry2d::from_translation(global_transform.translation().truncate()),
            10.,
            color,
        );
    }
}

#[cfg(test)]
mod tests {
    use bevy::DefaultPlugins;
    use bevy::app::PluginGroup as _;
    use bevy::gizmos_render::GizmoRenderPlugin;
    use bevy_app::Update;

    use super::{render_all_sprite_gizmos, render_sprite_gizmos};
    use crate::tests::utils::{assert_in_image_font_set, initialize_app_with_font};

    #[test]
    fn gizmo_systems_are_in_image_font_set() {
        let (mut app, _) = initialize_app_with_font("example_font.image_font.ron");

        assert_in_image_font_set(&mut app, Update, render_all_sprite_gizmos);
        assert_in_image_font_set(&mut app, Update, render_sprite_gizmos);
    }

    /// The examples draw gizmos through `DefaultPlugins`, which only renders
    /// them when the `gizmos` feature enables Bevy's gizmo rendering.
    #[test]
    fn gizmos_feature_makes_default_plugins_render_gizmos() {
        assert!(
            DefaultPlugins.build().enabled::<GizmoRenderPlugin>(),
            "`DefaultPlugins` should include `GizmoRenderPlugin` when the `gizmos` feature is \
            enabled, or the examples draw no gizmos"
        );
    }
}
