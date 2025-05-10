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

use bevy::color::palettes::css;
use bevy::prelude::*;

use crate::atlas_sprites::ImageFontTextData;
use crate::render_context::RenderContext;
use crate::ImageFontText;

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
        .add_systems(Update, (render_all_sprite_gizmos, render_sprite_gizmos));
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
#[expect(clippy::min_ident_chars, reason = "clippy bug")]
mod clippy_bug {
    use bevy::prelude::*;

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
    /// The width of the gizmo, representing the rendered font's bounding box
    /// or visualized area in the scene.
    width: f32,

    /// The height of the gizmo, representing the rendered font's bounding box
    /// or visualized area in the scene.
    height: f32,
}

pub(crate) fn record_character_dimensions(
    render_context: &RenderContext,
    character: char,
    gizmo_data: &mut ImageFontTextGizmoData,
) {
    let (new_width, new_height) = render_context.character_dimensions(character);

    gizmo_data.width = new_width;
    gizmo_data.height = new_height;
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
        (&GlobalTransform, &Children, &ImageFontTextData),
        (With<ImageFontText>, Without<ShowAtlasSpritesGizmos>),
    >,
    child_query: Query<&GlobalTransform, Without<ImageFontText>>,
) {
    for (global_transform, children, data) in &query {
        render_gizmos(
            &mut gizmos,
            &child_query,
            None,
            global_transform,
            children,
            data,
        );
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
            &Children,
            &ImageFontTextData,
        ),
        (With<ImageFontText>, With<ShowAtlasSpritesGizmos>),
    >,
    child_query: Query<&GlobalTransform, Without<ImageFontText>>,
) {
    for (sprite_gizmos, global_transform, children, data) in &query {
        render_gizmos(
            &mut gizmos,
            &child_query,
            Some(sprite_gizmos),
            global_transform,
            children,
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
    ($gizmos:expr, $sprite_gizmos:expr, $field:ident) => {
        $sprite_gizmos
            .and_then(|sprite_gizmos| sprite_gizmos.$field)
            .unwrap_or($gizmos.config_ext.$field)
    };
}

/// X
fn render_gizmos(
    gizmos: &mut Gizmos<AtlasSpritesGizmoConfigGroup>,
    child_query: &Query<&GlobalTransform, Without<ImageFontText>>,
    sprite_gizmos: Option<&ShowAtlasSpritesGizmos>,
    global_transform: &GlobalTransform,
    children: &Children,
    data: &ImageFontTextData,
) {
    for &child in children {
        if let Ok(child_global_transform) = child_query.get(child) {
            let width = data.gizmo_data.width;
            let height = data.gizmo_data.height;

            if gizmo_config_value!(gizmos, sprite_gizmos, render_character_box) {
                gizmos.rect_2d(
                    Isometry2d::from_translation(child_global_transform.translation().truncate()),
                    Vec2::new(width, height),
                    gizmo_config_value!(gizmos, sprite_gizmos, character_box_color),
                );
            }

            if gizmo_config_value!(gizmos, sprite_gizmos, render_character_anchor_point) {
                gizmos.cross_2d(
                    Isometry2d::from_translation(child_global_transform.translation().truncate()),
                    5.,
                    gizmo_config_value!(gizmos, sprite_gizmos, character_anchor_point_color),
                );
            }
        }
    }

    if gizmo_config_value!(gizmos, sprite_gizmos, render_text_anchor_point) {
        gizmos.cross_2d(
            Isometry2d::from_translation(global_transform.translation().truncate()),
            10.,
            gizmo_config_value!(gizmos, sprite_gizmos, text_anchor_point_color),
        );
    }
}
