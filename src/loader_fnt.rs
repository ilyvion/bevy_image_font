//! Code for parsing an [`ImageFont`] off of an on-disk representation in `fnt`
//! format.

use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt as _, LoadContext},
    image::Image,
    math::{URect, UVec2}, platform::collections::HashMap
};
use bevy_image::TextureAtlasLayout;

use crate::{
    loader::{
        ImageFontDescriptor, ImageFontDescriptorValidationError, ImageFontLayout,
        ImageFontLoadError, ImageFontLoaderSettings,
    },
    ImageFont,
};

/// Loader for [`ImageFont`]s.
#[derive(Debug, Default)]
pub struct ImageFntFontLoader;

impl AssetLoader for ImageFntFontLoader {
    type Asset = ImageFont;

    type Settings = ImageFontLoaderSettings;

    type Error = ImageFontLoadError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut str = String::new();
        reader.read_to_string(&mut str).await?;

        let mut map: HashMap<char, URect> = HashMap::new();
        let mut image_filename = String::new();

        for line in str.lines() {
            // Split the line into key-value pairs
            let parts: HashMap<&str, &str> = line
                .split_whitespace()
                .filter_map(|pair| {
                    let mut kv = pair.split('=');
                    Some((kv.next()?, kv.next()?))
                })
                .collect();
            if line.starts_with("page id=") {
                if let Some(filename) = parts.get("file") {
                    image_filename = filename.trim_matches('"').to_string();
                }
            } else if line.starts_with("char id=") {
                // Extract the ASCII id, x, y, width, and height
                if let (Some(&id), Some(&x), Some(&y), Some(&width), Some(&height)) = (
                    parts.get("id"),
                    parts.get("x"),
                    parts.get("y"),
                    parts.get("width"),
                    parts.get("height"),
                ) {
                    let id_char = id.parse::<u8>().unwrap_or_default() as char;
                    let x = x.parse::<u32>().unwrap_or(0);
                    let y = y.parse::<u32>().unwrap_or(0);
                    let width = width.parse::<u32>().unwrap_or(0);
                    let height = height.parse::<u32>().unwrap_or(0);

                    let rect = URect {
                        min: UVec2 { x, y },
                        max: UVec2 {
                            x: x + width,
                            y: y + height,
                        },
                    };

                    // Insert into the map
                    map.insert(id_char, rect);
                }
            }
        }
        if image_filename.is_empty() {
            return Err(ImageFontLoadError::DescriptorValidationError(
                ImageFontDescriptorValidationError::EmptyImagePath,
            ));
        }
        if map.is_empty() {
            return Err(ImageFontLoadError::DescriptorValidationError(
                ImageFontDescriptorValidationError::EmptyLayoutString,
            ));
        }
        let disk_format = ImageFontDescriptor::new(
            image_filename.clone().into(),
            ImageFontLayout::Manual(map.clone()),
        )?;

        // need the image loaded immediately because we need its size
        let image_path = load_context
            .path()
            .parent()
            .ok_or(ImageFontLoadError::MissingParentPath)?
            .join(disk_format.image());
        let Some(mut image) = load_context
            .loader()
            .immediate()
            .with_unknown_type()
            .load(image_path.clone())
            .await?
            .take::<Image>()
        else {
            return Err(ImageFontLoadError::NotAnImage(image_filename.into()));
        };

        image.sampler = settings.image_sampler.clone();

        let size = image.size();
        let image_handle = load_context.add_labeled_asset(String::from("texture"), image);
        let mut atlas_character_map = HashMap::new();
        let mut atlas_layout = TextureAtlasLayout::new_empty(size);
        for (&character, &rect) in &map {
            atlas_character_map.insert(character, atlas_layout.add_texture(rect));
        }

        let layout_handle = load_context.add_labeled_asset(String::from("layout"), atlas_layout);

        let image_font = ImageFont {
            texture: image_handle,
            atlas_character_map,
            atlas_layout: layout_handle,
            image_sampler: settings.image_sampler.clone(),
        };
        Ok(image_font)
    }

    fn extensions(&self) -> &[&str] {
        &["fnt"]
    }
}
