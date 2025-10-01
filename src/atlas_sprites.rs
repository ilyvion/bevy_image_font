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
use std::iter;

use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::Assets;
use bevy_color::Color;
use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::query::{Changed, Or};
use bevy_ecs::schedule::IntoScheduleConfigs as _;
use bevy_ecs::system::{Commands, Query, Res};
use bevy_image::TextureAtlasLayout;
use bevy_reflect::Reflect;
use bevy_render::view::Visibility;
use bevy_sprite::{Anchor, Sprite};
use bevy_transform::components::Transform;
use derive_setters::Setters;
use itertools::Itertools as _;

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
    mut child_query: Query<(&mut Sprite, &mut Transform)>,
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

        let mut sprite_context = SpriteContext {
            entity,
            image_font_text_data,
        };

        let x_pos = update_existing_sprites(&mut child_query, &mut sprite_context, &render_context);

        adjust_sprite_count(
            x_pos,
            &mut commands,
            &mut sprite_context,
            &render_context,
            image_font_sprite_text,
        );

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

/// Updates existing sprites to match the filtered text content.
///
/// Adjusts the position, scale, and appearance of each sprite to reflect
/// the corresponding glyph in the text and texture atlas.
///
/// # Parameters
/// - `child_query`: Query for accessing child sprite components.
/// - `sprite_context`: Context for managing the entity and its sprite data.
/// - `render_context`: Context providing rendering-related information and
///   operations.
///
/// # Returns
/// The x-position to the right of the last processed sprite.
fn update_existing_sprites(
    child_query: &mut Query<(&mut Sprite, &mut Transform)>,
    sprite_context: &mut SpriteContext,
    render_context: &RenderContext,
) -> f32 {
    let SpriteContext {
        ref mut image_font_text_data,
        ..
    } = *sprite_context;

    let mut x_pos = 0.;

    for ((sprite_entity, character), (next_sprite_entity, next_character)) in image_font_text_data
        .sprites
        .iter()
        .copied()
        .zip(render_context.text().filtered_chars())
        .chain(iter::once((Entity::PLACEHOLDER, ' ')))
        .tuple_windows()
    {
        let (mut sprite, mut transform) = match child_query.get_mut(sprite_entity) {
            Ok(result) => result,
            Err(error) => {
                bevy_log::error!(
                    "An ImageFontSpriteText unexpectedly failed: {error}. This will likely cause rendering bugs."
                );
                continue;
            }
        };

        let sprite = &mut *sprite;
        let Some(sprite_texture) = sprite.texture_atlas.as_mut() else {
            bevy_log::error!(
                "An ImageFontSpriteText's child sprite was \
            unexpectedly missing a `texture_atlas`. This will likely cause rendering bugs."
            );
            continue;
        };

        render_context.update_render_values(character, sprite_texture, &mut sprite.color);

        *transform = render_context.transform(
            &mut x_pos,
            character,
            if next_sprite_entity == Entity::PLACEHOLDER {
                None
            } else {
                Some(next_character)
            },
        );

        #[cfg(feature = "gizmos")]
        gizmos::record_character_dimensions(
            render_context,
            character,
            &mut image_font_text_data.gizmo_data,
        );
    }

    x_pos
}

/// Ensures the number of sprites matches the number of characters in the text.
///
/// Adds missing sprites or removes excess sprites to maintain consistency
/// between the text content and the entity's children.
///
/// # Parameters
/// - `x_pos`: x-position of where the next sprite should go.
/// - `commands`: A command buffer for spawning or despawning sprites to
///   synchronize with the text content.
/// - `sprite_context`: Context for managing the entity and its sprite data.
/// - `render_context`: Context providing rendering-related information and
///   operations.
/// - `sprite_text`: Component defining text appearance (e.g., color).
#[inline]
fn adjust_sprite_count(
    x_pos: f32,
    commands: &mut Commands,
    sprite_context: &mut SpriteContext,
    render_context: &RenderContext,
    sprite_text: &ImageFontSpriteText,
) {
    use std::cmp::Ordering;

    let char_count = render_context.text().filtered_chars().count();
    let sprite_count = sprite_context.image_font_text_data.sprites.len();

    match sprite_count.cmp(&char_count) {
        Ordering::Greater => {
            remove_excess_sprites(commands, sprite_context, char_count);
        }
        Ordering::Less => {
            add_missing_sprites(x_pos, commands, sprite_context, render_context, sprite_text);
        }
        Ordering::Equal => {}
    }
}

/// Removes excess sprites from the text entity to match the new character
/// count.
///
/// # Parameters
/// - `commands`: Command buffer for despawning entities.
/// - `sprite_context`: Context for managing the entity and its sprite data.
/// - `char_count`: The number of characters in the filtered text.
///
/// # Side Effects
/// Excess sprites are despawned from the ECS.
#[inline]
fn remove_excess_sprites(
    commands: &mut Commands,
    sprite_context: &mut SpriteContext,
    char_count: usize,
) {
    for entity in sprite_context
        .image_font_text_data
        .sprites
        .drain(char_count..)
    {
        commands.entity(entity).despawn();
    }
}

/// Adds missing sprites to the text entity to match the new character count.
///
/// If the number of sprites is less than the number of characters in the text,
/// this function spawns new sprites for the remaining characters and updates
/// the sprite data accordingly.
///
/// # Parameters
/// - `x_pos`: x-position of where the next sprite should go.
/// - `commands`: Command buffer for spawning new sprite entities.
/// - `sprite_context`: Context for managing the entity and its sprite data.
/// - `render_context`: Context providing rendering-related information and
///   operations.
/// - `sprite_text`: Component defining text appearance (e.g., color).
fn add_missing_sprites(
    mut x_pos: f32,
    commands: &mut Commands,
    sprite_context: &mut SpriteContext,
    render_context: &RenderContext,
    sprite_text: &ImageFontSpriteText,
) {
    let SpriteContext {
        entity,
        ref mut image_font_text_data,
    } = *sprite_context;

    let current_sprite_count = image_font_text_data.sprites.len();

    commands.entity(entity).with_children(|parent| {
        for (character, next_character) in render_context
            .text()
            .filtered_chars()
            .skip(current_sprite_count)
            .map(Some)
            .chain(iter::once(None))
            .tuple_windows()
        {
            #[expect(
                clippy::expect_used,
                reason = "tuple_windows() guarantees that only the last next_character is None"
            )]
            let character = character.expect("Only the last character can be None");

            let transform = render_context.transform(&mut x_pos, character, next_character);
            let sprite = Sprite {
                image: render_context.font_image(character),
                texture_atlas: Some(render_context.font_texture_atlas(character)),
                color: sprite_text.color,
                ..Default::default()
            };

            let child = parent.spawn((sprite, transform));
            image_font_text_data.sprites.push(child.id());

            #[cfg(feature = "gizmos")]
            gizmos::record_character_dimensions(
                render_context,
                character,
                &mut image_font_text_data.gizmo_data,
            );
        }
    });
}

/// Represents the entity and its associated text sprites during rendering.
///
/// Manages the commands for modifying the entity, its sprite data, and the
/// filtered text to ensure the sprites match the text content.
struct SpriteContext<'data> {
    /// The entity associated with the text sprites.
    entity: Entity,
    /// The mutable text sprite data component for the entity.
    image_font_text_data: &'data mut ImageFontTextData,
}
