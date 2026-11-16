//! Shows use of the plugin with `bevy_ui`.

#![expect(
    clippy::mod_module_files,
    reason = "if present as common.rs, cargo thinks it's an example binary"
)]
#![expect(clippy::expect_used, reason = "only used when panics can't happen")]

use bevy::input::common_conditions::input_just_pressed;
use bevy::prelude::*;
use bevy::scene::SceneList;
use bevy_asset_loader::prelude::AssetCollectionApp as _;
use bevy_image_font::rendered::ImageFontPreRenderedUiText;
use bevy_image_font::{ImageFontPlugin, ImageFontText};

use crate::common::DemoAssets;

mod common;

/// Tracks the number of vows judged during the application runtime.
///
/// This resource is updated when the user presses the SPACE key.
#[derive(Default, Debug, Resource)]
struct VowsJudged(u32);

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, ImageFontPlugin))
        .init_collection::<DemoAssets>()
        .insert_resource(ClearColor(Color::srgb(0.2, 0.2, 0.2)))
        .init_resource::<VowsJudged>()
        .add_systems(Startup, scene.spawn())
        .add_systems(
            Update,
            (
                judge.run_if(input_just_pressed(KeyCode::Space)),
                update_vows_node,
            )
                .chain(),
        )
        .run();
}

/// A marker component for the UI node that displays the number of vows judged.
///
/// Entities with this component have their text updated dynamically based
/// on the value of the [`VowsJudged`] resource.
#[derive(Component, FromTemplate)]
struct VowsNode;

/// Defines a UI layout scene for the example.
///
/// This scene list contains:
/// 1. A 2D camera with default settings.
/// 2. A root node with a prompt instructing the user to press SPACE.
/// 3. A dynamically updating node that displays the number of vows judged.
fn scene() -> impl SceneList {
    bsn_list! {
        Camera2d
        --
        @demo_root()
        --
        @demo_interactive()
    }
}

/// Returns a scene containing a centered UI node, along with a child node containing
/// `ImageFontText` that instructs the user to press SPACE.
fn demo_root() -> impl Scene {
    bsn! {
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        }
        Children [
            ImageNode
                Node {
                    position_type: PositionType::Relative,
                }
            ImageFontPreRenderedUiText
            ImageFontText {
                text: "Press SPACE to judge!",
                font: "example_font.image_font.ron",
                font_height: {Some(72.0)},
            }
        ]
    }
}

/// Returns a scene with an `ImageFontText` node that dynamically displays the number of vows
/// judged.
fn demo_interactive() -> impl Scene {
    bsn! {
        VowsNode
        ImageNode
        Node {
            position_type: PositionType::Absolute,
            left: Val::Auto,
            right: Val::Px(0.0),
        }
        ImageFontText {
            font: "example_font.image_font.ron",
            font_height: {Some(72.0)},
        }
    }
}

/// Increments the number of vows judged.
///
/// This system responds to the user pressing the SPACE key and increments
/// the `VowsJudged` resource by one.
fn judge(mut vows: ResMut<VowsJudged>) {
    vows.0 += 1;
}

/// Updates the text of the vows node when the number of vows judged changes.
///
/// This system listens for changes to the `VowsJudged` resource and updates the
/// text displayed by the UI node marked with [`VowsNode`].
fn update_vows_node(vows: Res<VowsJudged>, mut node: Query<&mut ImageFontText, With<VowsNode>>) {
    if vows.is_changed() {
        node.single_mut()
            .expect("VowsNode only has one ImageFontText")
            .text = format!("Vows judged: {}", vows.0);
    }
}
