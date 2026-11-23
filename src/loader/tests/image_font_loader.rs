use bevy::asset::io::VecReader;

use super::*;
use crate::tests::utils::{initialize_app_with_font, wait_until_failed};

#[test]
fn test_image_font_loader_extensions() {
    let loader = ImageFontLoader;
    assert_eq!(loader.extensions(), &["image_font.ron"]);
}

#[tokio::test]
async fn image_font_loader_read_and_validate_font_descriptor_works() {
    let data = std::fs::read("assets/example_font.image_font.ron").unwrap();
    let mut reader = VecReader::new(data);

    read_and_validate_font_descriptor(&mut reader)
        .await
        .unwrap();
}

#[tokio::test]
async fn image_font_loader_read_and_validate_font_descriptor_fails_on_invalid_ron_data() {
    let mut reader = VecReader::new(Vec::new());

    let result = read_and_validate_font_descriptor(&mut reader).await;

    assert!(
        matches!(result, Err(ImageFontLoadError::ParseFailure(_))),
        "{result:?}"
    );
}

#[tokio::test]
async fn image_font_loader_read_and_validate_font_descriptor_fails_on_invalid_descriptor() {
    let mut reader = VecReader::new(Vec::from("(layout: Automatic(\"\"), image: \"\")"));

    let result = read_and_validate_font_descriptor(&mut reader).await;

    assert!(
        matches!(
            result,
            Err(ImageFontLoadError::DescriptorValidationError(_))
        ),
        "{result:?}"
    );
}

#[test]
fn descriptor_to_character_map_and_layout_succeeds_on_valid_descriptor() {
    let font_descriptor = ImageFontDescriptor::new(
        Utf8PathBuf::from("path/to/image.png"),
        ImageFontLayout::Automatic(String::from("ABCD")),
    )
    .expect("valid descriptor");
    let image_size = UVec2::new(100, 50);

    let result = descriptor_to_character_map_and_layout(font_descriptor, image_size);

    assert!(result.is_ok());
}

#[test]
fn descriptor_to_character_map_and_layout_fails_on_invalid_layout() {
    let font_descriptor = ImageFontDescriptor::new(
        Utf8PathBuf::from("path/to/image.png"),
        ImageFontLayout::Automatic(String::from("ABCD")),
    )
    .expect("valid descriptor");
    let invalid_image_size = UVec2::new(101, 50); // Invalid dimensions

    let result = descriptor_to_character_map_and_layout(font_descriptor, invalid_image_size);

    assert!(
        matches!(result, Err(ImageFontLoadError::LayoutValidationError(_))),
        "{result:?}"
    );
}

#[test]
fn font_with_missing_image_fails_to_load() {
    let (mut app, handle): (bevy_app::App, bevy_asset::Handle<ImageFont>) =
        initialize_app_with_font("tests/missing_image.image_font.ron");

    let error = wait_until_failed(&mut app, &handle);

    let bevy_asset::AssetLoadError::AssetLoaderError(ref loader_error) = error else {
        panic!("expected a loader error, got {error}");
    };
    let load_error = loader_error
        .error()
        .downcast_ref::<ImageFontLoadError>()
        .expect("the loader error is an ImageFontLoadError");
    assert!(
        matches!(*load_error, ImageFontLoadError::LoadDirect(_)),
        "{load_error:?}"
    );
}
