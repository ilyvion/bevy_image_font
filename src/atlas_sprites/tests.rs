#![allow(clippy::unwrap_used, reason = "test code panics to indicate errors")]
#![allow(clippy::expect_used, reason = "test code panics to indicate errors")]

use bevy_app::App;
use bevy_asset::Handle;
use bevy_color::palettes::css;
use bevy_ecs::query::Added;

use super::*;
use crate::tests::utils::{initialize_app_with_font, wait_until_loaded};

/// Loads the font at `path` and spawns sprite text using it.
fn spawn_text(path: &str, text: &str, sprite_text: ImageFontSpriteText) -> (App, Entity) {
    let (mut app, font): (App, Handle<ImageFont>) = initialize_app_with_font(path);
    wait_until_loaded(&mut app, &font);

    let entity = app
        .world_mut()
        .spawn((ImageFontText::default().text(text).font(font), sprite_text))
        .id();
    app.update();

    (app, entity)
}

/// The font used by the text entity.
fn font(app: &App, entity: Entity) -> &ImageFont {
    let world = app.world();
    let handle = &world.get::<ImageFontText>(entity).unwrap().font;
    world.resource::<Assets<ImageFont>>().get(handle).unwrap()
}

/// A child sprite of a text entity.
struct ChildSpriteData {
    /// The child's sprite.
    sprite: Sprite,
    /// The child's transform.
    transform: Transform,
}

/// The glyph sprites and the outline sprites of the text entity.
fn sprites(app: &App, entity: Entity) -> (Vec<ChildSpriteData>, Vec<ChildSpriteData>) {
    let world = app.world();
    let data = world.get::<ImageFontTextData>(entity).unwrap();
    let sprite = |&entity: &Entity| ChildSpriteData {
        sprite: world.get::<Sprite>(entity).unwrap().clone(),
        transform: *world.get::<Transform>(entity).unwrap(),
    };
    (
        data.sprites.iter().map(sprite).collect(),
        data.outline_sprites.iter().map(sprite).collect(),
    )
}

#[test]
fn outline_color_draws_outlines_below_glyphs() {
    let (app, entity) = spawn_text(
        "tests/channels_outline.txt.fnt",
        "AB",
        ImageFontSpriteText::default()
            .color(css::BLUE)
            .outline_color(Color::from(css::RED)),
    );
    let layers = font(&app, entity).outline_layers[0].clone().unwrap();

    let (glyph_sprites, outline_sprites) = sprites(&app, entity);

    assert_eq!(glyph_sprites.len(), 2);
    assert_eq!(outline_sprites.len(), 2);
    for (glyph, outline) in glyph_sprites.iter().zip(&outline_sprites) {
        assert_eq!(glyph.sprite.image, layers.glyph);
        assert_eq!(glyph.sprite.color, Color::from(css::BLUE));
        assert_eq!(outline.sprite.image, layers.outline);
        assert_eq!(outline.sprite.color, Color::from(css::RED));
        assert_eq!(outline.sprite.texture_atlas, glyph.sprite.texture_atlas);
        assert_eq!(
            outline.transform.translation.truncate(),
            glyph.transform.translation.truncate()
        );
        assert_eq!(outline.transform.scale, glyph.transform.scale);
    }
    let lowest_glyph = glyph_sprites
        .iter()
        .map(|glyph| glyph.transform.translation.z)
        .fold(f32::INFINITY, f32::min);
    for outline in &outline_sprites {
        assert!(outline.transform.translation.z < lowest_glyph);
    }
}

#[test]
fn outline_sprites_follow_the_outline_color() {
    let (mut app, entity) = spawn_text(
        "tests/channels_outline.txt.fnt",
        "AB",
        ImageFontSpriteText::default(),
    );
    let texture = font(&app, entity).textures[0].clone();
    let layers = font(&app, entity).outline_layers[0].clone().unwrap();

    let (glyph_sprites, outline_sprites) = sprites(&app, entity);
    assert_eq!(outline_sprites.len(), 0);
    assert!(
        glyph_sprites
            .iter()
            .all(|glyph| glyph.sprite.image == texture)
    );

    app.world_mut()
        .get_mut::<ImageFontSpriteText>(entity)
        .unwrap()
        .outline_color = Some(css::RED.into());
    app.update();

    let (glyph_sprites, outline_sprites) = sprites(&app, entity);
    assert_eq!(outline_sprites.len(), 2);
    assert!(
        glyph_sprites
            .iter()
            .all(|glyph| glyph.sprite.image == layers.glyph)
    );

    let outline_entities = app
        .world()
        .get::<ImageFontTextData>(entity)
        .unwrap()
        .outline_sprites
        .clone();
    app.world_mut()
        .get_mut::<ImageFontSpriteText>(entity)
        .unwrap()
        .outline_color = None;
    app.update();

    let (glyph_sprites, outline_sprites) = sprites(&app, entity);
    assert_eq!(outline_sprites.len(), 0);
    assert!(
        glyph_sprites
            .iter()
            .all(|glyph| glyph.sprite.image == texture)
    );
    for outline_entity in outline_entities {
        assert!(app.world().get_entity(outline_entity).is_err());
    }
}

#[test]
fn sprites_switch_texture_when_their_character_moves_to_another_texture() {
    // `A` and `B` are packed into different channels, so they are drawn from
    // different textures.
    let (mut app, entity) = spawn_text(
        "tests/channels_packed.txt.fnt",
        "AB",
        ImageFontSpriteText::default(),
    );

    app.world_mut()
        .get_mut::<ImageFontText>(entity)
        .unwrap()
        .text = String::from("BA");
    app.update();

    let image_font = font(&app, entity);
    let (glyph_sprites, _) = sprites(&app, entity);
    for (glyph, character) in glyph_sprites.iter().zip(['B', 'A']) {
        let page_index = image_font.atlas_character_map[&character].page_index;
        assert_eq!(
            glyph.sprite.image, image_font.textures[page_index],
            "{character}"
        );
        assert_eq!(
            glyph.sprite.texture_atlas.as_ref().unwrap().layout,
            image_font.atlas_layouts[page_index],
            "{character}"
        );
    }
}

/// Marks a sprite that [`insert_marker_on_new_sprites`] saw.
#[derive(Component)]
struct SeenSprite;

/// Queues an `insert` on every newly added sprite, like Bevy's sprite
/// rendering does, running in `PostUpdate` alongside [`set_up_sprites`]
/// without seeing its commands applied.
fn insert_marker_on_new_sprites(mut commands: Commands, sprites: Query<Entity, Added<Sprite>>) {
    for entity in &sprites {
        commands.entity(entity).insert(SeenSprite);
    }
}

#[test]
fn removing_a_new_sprite_does_not_break_other_systems_inserting_on_it() {
    let (mut app, entity) = spawn_text("bmf.txt.fnt", "A", ImageFontSpriteText::default());
    app.add_systems(
        PostUpdate,
        insert_marker_on_new_sprites.after_ignore_deferred(set_up_sprites),
    );

    app.world_mut()
        .get_mut::<ImageFontText>(entity)
        .unwrap()
        .text = String::from("AV");
    app.update();
    let removed_sprite = app
        .world()
        .get::<ImageFontTextData>(entity)
        .unwrap()
        .sprites[1];

    app.world_mut()
        .get_mut::<ImageFontText>(entity)
        .unwrap()
        .text = String::from("A");
    app.update();

    assert!(app.world().get_entity(removed_sprite).is_err());
    let (glyph_sprites, _) = sprites(&app, entity);
    assert_eq!(glyph_sprites.len(), 1);
}

#[cfg(feature = "gizmos")]
mod gizmos {
    use bevy_ecs::hierarchy::Children;
    use bevy_ecs::query::Without;
    use bevy_ecs::system::RunSystemOnce as _;
    use bevy_math::Vec2;
    use bevy_transform::components::GlobalTransform;

    use super::*;
    use crate::atlas_sprites::gizmos::glyph_boxes;

    /// The position and size of each glyph box the gizmos draw for the text
    /// entity.
    fn glyph_boxes_of(app: &mut App, entity: Entity) -> Vec<(Vec2, Vec2)> {
        app.world_mut()
            .run_system_once(
                move |texts: Query<&ImageFontTextData>,
                      child_query: Query<&GlobalTransform, Without<ImageFontText>>| {
                    glyph_boxes(texts.get(entity).unwrap(), &child_query).collect::<Vec<_>>()
                },
            )
            .unwrap()
    }

    /// The sizes of the glyph boxes the gizmos draw for the text entity.
    fn glyph_box_sizes(app: &mut App, entity: Entity) -> Vec<Vec2> {
        glyph_boxes_of(app, entity)
            .into_iter()
            .map(|(_, size)| size)
            .collect()
    }

    #[test]
    fn each_glyph_box_has_the_size_of_its_own_character() {
        // `#` is 19×23 and `'` is 4×8 in this font.
        let (mut app, entity) = spawn_text("bmf.txt.fnt", "#'", ImageFontSpriteText::default());

        assert_eq!(
            glyph_box_sizes(&mut app, entity),
            [Vec2::new(19., 23.), Vec2::new(4., 8.)]
        );

        app.world_mut()
            .get_mut::<ImageFontText>(entity)
            .unwrap()
            .text = String::from("'");
        app.update();

        assert_eq!(glyph_box_sizes(&mut app, entity), [Vec2::new(4., 8.)]);
    }

    #[test]
    fn outline_sprites_get_no_glyph_boxes() {
        let (mut app, entity) = spawn_text(
            "tests/channels_outline.txt.fnt",
            "AB",
            ImageFontSpriteText::default().outline_color(Color::from(css::RED)),
        );
        assert_eq!(app.world().get::<Children>(entity).unwrap().len(), 4);

        assert_eq!(glyph_boxes_of(&mut app, entity).len(), 2);
    }
}

/// Whether a missing font has been reported for the text entity, or `None`
/// if it has no `ImageFontTextData`.
fn has_reported_missing_font(app: &App, entity: Entity) -> Option<bool> {
    app.world()
        .get::<ImageFontTextData>(entity)
        .map(|data| data.has_reported_missing_font)
}

#[test]
fn a_missing_font_is_reported_once_until_the_font_is_found_again() {
    let (mut app, font): (App, Handle<ImageFont>) = initialize_app_with_font("bmf.txt.fnt");
    wait_until_loaded(&mut app, &font);
    let missing_font = Handle::<ImageFont>::default();

    let entity = app
        .world_mut()
        .spawn((
            ImageFontText::default()
                .text("A")
                .font(missing_font.clone()),
            ImageFontSpriteText::default(),
        ))
        .id();
    app.update();
    assert_eq!(has_reported_missing_font(&app, entity), Some(true));

    app.world_mut()
        .get_mut::<ImageFontText>(entity)
        .unwrap()
        .font = font;
    app.update();
    assert_eq!(has_reported_missing_font(&app, entity), Some(false));

    app.world_mut()
        .get_mut::<ImageFontText>(entity)
        .unwrap()
        .font = missing_font;
    app.update();
    assert_eq!(has_reported_missing_font(&app, entity), Some(true));
}

#[test]
fn each_line_starts_at_the_left_edge_one_line_height_below_the_previous() {
    let (app, entity) = spawn_text(
        "bmf.txt.fnt",
        "AV\r\nA\nA",
        ImageFontSpriteText::default().anchor(Anchor::TOP_LEFT),
    );

    let (glyph_sprites, _) = sprites(&app, entity);

    assert_eq!(glyph_sprites.len(), 4);
    let first_line_a = glyph_sprites[0].transform.translation;
    for (line_index, glyph) in [(1.0, &glyph_sprites[2]), (2.0, &glyph_sprites[3])] {
        let translation = glyph.transform.translation;
        assert_eq!(translation.x, first_line_a.x, "line {line_index}");
        assert_eq!(
            translation.y,
            first_line_a.y - line_index * 36.0,
            "line {line_index}"
        );
    }
}
