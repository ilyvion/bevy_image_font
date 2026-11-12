//! This module provides functionality for rendering text as individual sprites
//! using the Bevy engine, utilizing custom image fonts.
//!
//! It breaks down text into individual characters and represents them as
//! sprites in the game world. This approach allows precise positioning and
//! styling of text at the character level, suitable for scenarios where text
//! needs to be rendered dynamically or interactively.
//!
//! Key Features:
//! - `ImageFontSpriteText` component: Allows customization of text rendering,
//!   such as color and anchor point.
//! - Systems for rendering text to sprite entities and updating their
//!   configuration when text changes.
//! - Optional gizmo rendering for debugging purposes, available with the
//!   "gizmos" feature flag.
//!
//! This module is intended for advanced text rendering use cases, offering
//! fine-grained control over how text is displayed in the game world.

#[cfg(feature = "gizmos")]
pub mod gizmos;

use std::fmt::Debug;

use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::Assets;
use bevy_camera::visibility::Visibility;
use bevy_color::Color;
use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::query::{Changed, Or};
use bevy_ecs::schedule::IntoScheduleConfigs as _;
use bevy_ecs::system::{Commands, Query, Res};
use bevy_image::TextureAtlasLayout;
use bevy_reflect::Reflect;
use bevy_sprite::{Anchor, Sprite};
use bevy_transform::components::Transform;
use derive_setters::Setters;

#[cfg(feature = "gizmos")]
use crate::atlas_sprites::gizmos::ImageFontTextGizmoData;
use crate::render_context::{RenderConfig, RenderContext};
use crate::{
    ImageFont, ImageFontScalingMode, ImageFontSet, ImageFontText, LetterSpacing,
    sync_texts_with_font_changes,
};

/// Internal plugin for conveniently organizing the code related to this
/// module's feature.
#[derive(Default)]
pub(crate) struct AtlasSpritesPlugin;

impl Plugin for AtlasSpritesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            set_up_sprites
                .after(sync_texts_with_font_changes)
                .in_set(ImageFontSet),
        );

        #[cfg(feature = "gizmos")]
        {
            gizmos::build(app);
        }
    }
}

/// Text rendered using an [`ImageFont`] as individual sprites.
///
/// This struct provides fields for customizing text rendering, such as
/// alignment, color, and scaling behavior.
///
/// - `anchor`: Specifies the alignment point of the text relative to its
///   position.
/// - `color`: Uniform tint applied to all glyphs.
/// - `scaling_mode`: Controls how scaling is applied to glyph dimensions.
#[derive(Debug, Clone, Reflect, Default, Component, Setters)]
#[setters(into)]
#[require(ImageFontText, Visibility)]
#[non_exhaustive]
pub struct ImageFontSpriteText {
    /// The alignment point of the text relative to its position. For example,
    /// `Anchor::TopLeft` aligns the text's top-left corner to its position.
    pub anchor: Anchor,

    /// The color applied to the rendered text. This color affects all glyphs
    /// equally, allowing you to tint the text uniformly.
    pub color: Color,

    /// The color of the outline, for fonts whose textures hold an outline
    /// around their glyphs (see [`ImageFont::outline_layers`]).
    ///
    /// When `None`, the outline keeps the color it has in the font texture,
    /// tinted by `color` like the rest of the glyph. When set, each glyph is
    /// drawn as two sprites: the outline in this color, and the inside of the
    /// glyph in `color` on top of it. Every outline sprite is placed slightly
    /// behind the glyph sprites, so outlines never cover neighboring glyphs.
    /// Fonts without an outline ignore this setting.
    ///
    /// The default value is `None`.
    pub outline_color: Option<Color>,

    /// Determines how scaling is applied to the glyph dimensions when adjusting
    /// them to match the desired font height.
    ///
    /// This field allows control over how fractional scaling values are
    /// handled, using the [`ImageFontScalingMode`] enum. It provides options to
    /// truncate, round, or retain precise fractional values, depending on
    /// your rendering requirements.
    ///
    /// The default value is `ImageFontScalingMode::Rounded`.
    pub scaling_mode: ImageFontScalingMode,

    /// Determines a constant kerning between characters. The spacing is given
    /// at the font's native height and is scaled proportionally based on the
    /// current font height.
    pub letter_spacing: LetterSpacing,
}

/// Stores a mapping between characters and their corresponding sprite entities.
/// This is used to manage text rendering at the entity level.
#[derive(Debug, Clone, Component)]
struct ImageFontTextData {
    /// The entity that owns this `ImageFontTextData` component.
    self_entity: Entity,

    /// Basically a map between character index and character sprite
    sprites: Vec<Entity>,

    /// The outline sprites of the characters drawn with an outline color, in
    /// text order.
    outline_sprites: Vec<Entity>,

    /// Tracks whether a missing font asset has already been reported for this
    /// entity.
    ///
    /// This flag prevents repeated error messages when an `ImageFontText`
    /// component references a font asset that has not been loaded. Once an
    /// error is logged for a missing font, subsequent frames will not log
    /// additional errors for the same entity.
    ///
    /// # Default Behavior
    /// - Initially set to `false`, meaning no missing font error has been
    ///   reported.
    /// - Set to `true` after the first error message is logged.
    has_reported_missing_font: bool,

    /// Stores debugging information for visualizing `ImageFontSpriteText` in
    /// the scene.
    ///
    /// This field is only available when the `gizmos` feature is enabled. It
    /// tracks glyph dimensions and other relevant data used for rendering
    /// debug gizmos, such as bounding boxes and anchor points.
    #[cfg(feature = "gizmos")]
    gizmo_data: ImageFontTextGizmoData,
}
impl ImageFontTextData {
    /// Creates a new `ImageFontTextData` instance for a given entity.
    ///
    /// This function initializes an empty `ImageFontTextData` struct and
    /// associates it with the provided entity. It ensures that newly
    /// created text data always has a reference to its owning entity.
    ///
    /// # Parameters
    /// - `self_entity`: The entity that this `ImageFontTextData` belongs to.
    ///
    /// # Returns
    /// A new instance of `ImageFontTextData` with an empty sprite list and the
    /// specified `self_entity`.
    fn new(entity: Entity) -> Self {
        Self {
            self_entity: entity,
            sprites: Vec::default(),
            outline_sprites: Vec::default(),
            has_reported_missing_font: false,
            #[cfg(feature = "gizmos")]
            gizmo_data: ImageFontTextGizmoData::default(),
        }
    }
}

/// System that renders each [`ImageFontText`] as child [`Sprite`] entities,
/// where each sprite represents a character in the text. Each sprite is
/// positioned based on its order in the text, accounting for letter spacing,
/// scaling mode, and anchor alignment. This system only runs when the
/// `ImageFontText` or [`ImageFontSpriteText`] changes.
#[expect(
    clippy::missing_panics_doc,
    reason = "expect() is only used on a newly created Some() value"
)]
#[expect(
    private_interfaces,
    reason = "Systems are only `pub` for the sake of allowing dependent crates to use them for ordering"
)]
pub fn set_up_sprites(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &ImageFontText,
            &ImageFontSpriteText,
            Option<&mut ImageFontTextData>,
        ),
        Or<(Changed<ImageFontText>, Changed<ImageFontSpriteText>)>,
    >,
    mut child_query: Query<ChildSprite>,
    image_fonts: Res<Assets<ImageFont>>,
    texture_atlas_layouts: Res<Assets<TextureAtlasLayout>>,
) {
    for (entity, image_font_text, image_font_sprite_text, mut image_font_text_data) in &mut query {
        let mut maybe_new_image_font_text_data = None;
        let image_font_text_data = if let Some(image_font_text_data) = image_font_text_data.as_mut()
        {
            &mut *image_font_text_data
        } else {
            maybe_new_image_font_text_data = Some(ImageFontTextData::new(entity));
            #[expect(clippy::expect_used, reason = "newly created Some() value")]
            maybe_new_image_font_text_data
                .as_mut()
                .expect("newly created Some() value")
        };

        let render_config = RenderConfig {
            letter_spacing: image_font_sprite_text.letter_spacing.to_f32(),
            offset_characters: true,
            apply_scaling: true,
            scaling_mode: image_font_sprite_text.scaling_mode,
            color: image_font_sprite_text.color,
            outline_color: image_font_sprite_text.outline_color,
            text_anchor: image_font_sprite_text.anchor,
        };

        let font_handle = &image_font_text.font;
        let Some(image_font) = image_fonts.get(font_handle) else {
            if !image_font_text_data.has_reported_missing_font {
                let font_handle_detail: &dyn Debug = if let Some(font_path) = font_handle.path() {
                    font_path
                } else {
                    &font_handle.id()
                };
                bevy_log::error!(
                    "ImageFont asset {font_handle_detail:?} is not loaded; can't render text for entity: {}",
                    image_font_text_data.self_entity
                );
                image_font_text_data.has_reported_missing_font = true;
            }
            continue;
        };

        let Some(render_context) = RenderContext::new(
            image_font,
            image_font_text,
            render_config,
            &texture_atlas_layouts,
        ) else {
            maybe_insert_new_image_font_text_data(
                &mut commands,
                entity,
                maybe_new_image_font_text_data,
            );
            continue;
        };

        let (glyph_sprites, outline_sprites) = text_sprites(&render_context, image_font_text_data);

        let mut sprite_sync = SpriteSync {
            commands: &mut commands,
            parent: entity,
            child_query: &mut child_query,
        };
        sprite_sync.sync(&mut image_font_text_data.sprites, glyph_sprites);
        sprite_sync.sync(&mut image_font_text_data.outline_sprites, outline_sprites);

        maybe_insert_new_image_font_text_data(
            &mut commands,
            entity,
            maybe_new_image_font_text_data,
        );
    }
}

/// Inserts a newly created `ImageFontTextData` component into an entity if one
/// was generated.
///
/// This function takes an `Option<ImageFontTextData>`, which may contain a
/// newly created instance. If it is `Some`, the function inserts the
/// `ImageFontTextData` into the provided entity. If it is `None`, no action is
/// taken.
///
/// # Parameters
/// - `commands`: A command buffer used to insert the component if
///   `maybe_new_image_font_text_data` is `Some`.
/// - `entity`: The entity that will receive the `ImageFontTextData`.
/// - `maybe_new_image_font_text_data`: An `Option<ImageFontTextData>` that
///   contains a newly created instance if one was generated earlier.
///
/// # Notes
/// - This function does not check whether the entity already has an
///   `ImageFontTextData` component; it simply inserts the given value if it
///   exists.
/// - If a new component is inserted, a debug message is logged.
fn maybe_insert_new_image_font_text_data(
    commands: &mut Commands,
    entity: Entity,
    maybe_new_image_font_text_data: Option<ImageFontTextData>,
) {
    if let Some(new_image_font_text_data) = maybe_new_image_font_text_data {
        bevy_log::debug!("Inserted new ImageFontTextData for entity {:?}", entity);
        commands.entity(entity).insert(new_image_font_text_data);
    }
}

/// How far behind the glyph sprites the outline sprites are placed, so that
/// every outline is drawn before every glyph.
const OUTLINE_DEPTH: f32 = 0.001;

/// Computes the sprites that draw the filtered text: one glyph sprite per
/// character, and one outline sprite per character drawn with an outline
/// color, placed [`OUTLINE_DEPTH`] behind its glyph sprite.
///
/// # Returns
/// The glyph sprites and the outline sprites, each with its transform, in
/// text order.
fn text_sprites(
    render_context: &RenderContext,
    #[cfg_attr(
        not(feature = "gizmos"),
        expect(unused_variables, reason = "gizmos only")
    )]
    image_font_text_data: &mut ImageFontTextData,
) -> (Vec<(Sprite, Transform)>, Vec<(Sprite, Transform)>) {
    let mut glyph_sprites = Vec::new();
    let mut outline_sprites = Vec::new();
    let mut x_pos = 0.;

    let mut characters = render_context.text().filtered_chars().peekable();
    while let Some(character) = characters.next() {
        let transform =
            render_context.sprite_transform(&mut x_pos, character, characters.peek().copied());

        if let Some(outline_sprite) = render_context.outline_sprite(character) {
            let mut outline_transform = transform;
            outline_transform.translation.z -= OUTLINE_DEPTH;
            outline_sprites.push((outline_sprite, outline_transform));
        }
        glyph_sprites.push((render_context.glyph_sprite(character), transform));

        #[cfg(feature = "gizmos")]
        gizmos::record_character_dimensions(
            render_context,
            character,
            &mut image_font_text_data.gizmo_data,
        );
    }

    (glyph_sprites, outline_sprites)
}

/// The components of a text entity's child sprite that are kept in sync with
/// its text.
type ChildSprite = (&'static mut Sprite, &'static mut Transform);

/// Keeps a text entity's child sprites in sync with the sprites its text
/// should be drawn with.
struct SpriteSync<'borrow, 'world, 'state, 'query_world, 'query_state> {
    /// Command buffer for spawning and despawning sprites.
    commands: &'borrow mut Commands<'world, 'state>,
    /// The text entity the sprites are children of.
    parent: Entity,
    /// Query for updating existing child sprites.
    child_query: &'borrow mut Query<'query_world, 'query_state, ChildSprite>,
}

impl SpriteSync<'_, '_, '_, '_, '_> {
    /// Makes `entities` draw `sprites`: existing entities are updated in
    /// order, missing ones are spawned as children of the text entity, and
    /// excess ones are despawned.
    fn sync(&mut self, entities: &mut Vec<Entity>, sprites: Vec<(Sprite, Transform)>) {
        for entity in entities.drain(sprites.len().min(entities.len())..) {
            self.commands.entity(entity).despawn();
        }

        let mut sprites = sprites.into_iter();
        for (&entity, (new_sprite, new_transform)) in entities.iter().zip(sprites.by_ref()) {
            match self.child_query.get_mut(entity) {
                Ok((mut sprite, mut transform)) => {
                    sprite.image = new_sprite.image;
                    sprite.texture_atlas = new_sprite.texture_atlas;
                    sprite.color = new_sprite.color;
                    *transform = new_transform;
                }
                Err(error) => {
                    bevy_log::error!(
                        "An ImageFontSpriteText unexpectedly failed: {error}. This will likely \
                        cause rendering bugs."
                    );
                }
            }
        }

        self.commands.entity(self.parent).with_children(|parent| {
            for sprite in sprites {
                entities.push(parent.spawn(sprite).id());
            }
        });
    }
}

#[cfg(all(test, feature = "bmf"))]
mod tests;
