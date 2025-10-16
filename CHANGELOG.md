# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **BMF kerning support:** Kerning pairs from BMF files are now parsed and applied during text rendering. This improves the visual appearance of text by adjusting spacing between specific character pairs according to the font's kerning data.
- The `ImageFontCharacter` struct now includes a `kernings` field, and the rendering pipeline (including both `atlas_sprites` and `rendered` modes) applies kerning adjustments between character pairs.

### Changed

- Crate updated to target Bevy 0.17.
- The text rendering logic in both sprite-based and pre-rendered modes now takes BMF kerning into account when positioning characters.

### Fixed

- Fixed a bug where text rendered with `bevy_image_font` would not update its size when `UiScale` changed, unless the text itself was also changed ([#17]). Now, all text is properly re-rendered when the UI scale changes.

[#17]: https://github.com/ilyvion/bevy_image_font/issues/17

## [0.9.0] - 2025-05-13

### Added

- `LetterSpacing` enum to allow customizable kerning between characters.
- `letter_spacing` field to `ImageFontSpriteText` for individual entity customization.
- Example in `atlased_sprite` to demonstrate `LetterSpacing` usage.
- Support for fonts that span multiple texture pages.
- `BmFontLoader` for loading bitmap font file formats (`txt.fnt`, `xml.fnt`, `bin.fnt`).
- Example (`bmf.rs`) for rendering bitmap fonts (`txt.fnt`, `xml.fnt`, `bin.fnt`). Uses multilingual pangrams to test font rendering in various scripts.
- Introduced `AtlasSpritesGizmoConfigGroup` for global configuration of debug rendering options.
- Gizmos can now be toggled per-entity via `ShowAtlasSpritesGizmos`.

### Changed

- Crate updated to target Bevy 0.16.
- Updated text rendering calculations to account for `letter_spacing`.
- No longer repeatedly prints error for missing font assets.
- `ImageFont` now supports multiple textures instead of a single one. (Currently only supported by `.fnt` definitions)
- Gizmos are now governed by settings in `AtlasSpritesGizmoConfigGroup`.

### Removed

- `ImageFntFontLoader` font loader, replaced by `BmFontLoader`.

### Breaking Changes

- `ImageFont` had two fields change name and type, `atlas_layout`→`atlas_layouts` and `texture`→`textures` and are now `Vec`s of their former type.
- Made most public types that are likely to be expanded with more values or variants in the near future `#[non_exhaustive]` to avoid that being a breaking change. These all implement `Default` and instances can thus still be created. Exact list of types changed below:
  - `atlas_sprites::ImageFontSpriteText`
  - `ImageFont`
  - `ImageFontText`
  - `loader::ImageFontLayout`
  - `loader::ImageFontLoaderSettings`
  - `rendered::ImageFontPreRenderedText`
  - `rendered::ImageFontPreRenderedUiText`
- Types have been moved:
  - `bevy_image_font::atlas_sprites::ScalingMode` to `bevy_image_font::ImageFontScalingMode`
  - `bevy_image_font::atlas_sprites::ImageFontGizmoData` to `bevy_image_font::atlas_sprites::gizmos::ImageFontGizmoData`
- `bevy_image_font::atlas_sprites::render_sprite_gizmos` is no longer part of the public API.

## [0.8.0] - 2025-01-24

### Added

- **Scaling Customization**:

  - Introduced the `ScalingMode` enum with options (`Truncated`, `Rounded`, `Smooth`) for controlling glyph scaling behavior in text rendering.
  - Enabled support for floating-point glyph dimensions to allow sub-pixel rendering.

### Changed

- **Rendering Precision**:

  - Replaced integer-based scaling (`u32`) with floating-point calculations (`f32`) to improve rendering precision.
  - Refactored text rendering logic to integrate the `ScalingMode` enum.

- **Code Cleanup**:

  - Removed all deprecated annotations and legacy comments associated with `ImageFontDescriptor`.

### Removed

- **Deprecated Fields and Types**:

  - Removed deprecated public fields `image` and `layout` from `ImageFontDescriptor`. Use accessor methods (`image`, `layout`) to retrieve their values.
  - Removed the deprecated type alias `ImageFontSettings`.
  - Removed unused error variants `EmptyImagePath` and `EmptyLayoutString`.

## [0.7.1] - 2025-01-24

### Added

- **Validation and Accessors**:

  - Introduced `ImageFontDescriptor::new` for creating validated instances.
  - Added accessor methods (`image`, `layout`) for retrieving the values of deprecated public fields.
  - Added `ValidationError` to `ImageFontLoadError` for encapsulating validation issues.

- **Core System Tests**:

  - Added tests for the `sync_texts_with_font_changes` system to validate:
    - Correct handling of `AssetEvent` variants.
    - Accurate change detection for `ImageFontText` components when their respective `ImageFont` is changed.

- **Component and Layout Tests**:

  - Introduced test modules for `ImageFontDescriptor` and `ImageFontLayout`:
    - Verified `ImageFontDescriptor::new` creation and validation logic.
    - Tested `ImageFontLayout`'s character map generation for all layout types (`Automatic`, `Manual`, `ManualMonospace`).
    - Added tests for edge cases like repeated characters and invalid image dimensions.

- **Loader Tests**:

  - Comprehensive coverage for `ImageFontLoader` functionality:
    - Validated `read_and_validate_font_descriptor` for proper descriptor parsing and validation.
    - Ensured `descriptor_to_character_map_and_layout` handles both valid and invalid inputs.

- **Integration Tests**:

  - Verified the behavior of `ImageFontPlugin` integration with the Bevy framework:
    - Tested asset loading setup.
    - Confirmed correct registration of `ImageFont` and `ImageFontText` types with the reflection system.
  - Ensured correctness of `ImageFont::filter_string` and its integration with layout character maps.

### Changed

- **Field Deprecation**:

  - Marked `ImageFontDescriptor` fields (`image` and `layout`) as deprecated, with a plan to make them private in version 8.0.
  - Updated documentation to guide users toward using `new`, `image`, and `layout` methods.

- **Gizmo Debugging**:

  - Added green cross markers to `render_sprite_gizmos` for enhanced visual debugging of sprite positions.

### Fixed

- Resolved an issue where incorrect width and anchor calculations caused text misalignment ([#10]).
- `ImageFontGizmoData` was not being updated in `update_existing_sprites`.

[#10]: https://github.com/ilyvion/bevy_image_font/issues/10

### Deprecated

- **Error Variants**:
  - Deprecated `ImageFontLoadError::EmptyImagePath` and `ImageFontLoadError::EmptyLayoutString` in favor of the `ValidationError` variant.

### Notes

- These changes improve the `ImageFontDescriptor` API by moving toward an encapsulated design while maintaining backward compatibility until version 8.0.

## [0.7.0] - 2025-01-01

### Added

#### Features

- Introduced the `rendered` feature for text rendering to `Sprite` and `ImageNode` components, using `ImageFontPreRenderedText` and `ImageFontPreRenderedUiText`.
- Added the **`atlas_sprites`** feature to enable text rendering with individual sprites for each character via a texture atlas. Includes:
  - `ImageFontSpriteText` component for sprite-based text rendering with configurable color and anchor.
  - Optional gizmo rendering via the `gizmos` feature.
- Introduced `ImageFontLoaderSettings` to allow custom `ImageSampler` specification for font asset loading.
- Added `ImageFontTextData` for tracking sprite-entity relationships and optimizing updates.
- Introduced a `validate` method to `ImageFontSettings` to ensure valid `image` paths and layout strings.

#### Documentation

- Enabled comprehensive documentation for all features on `docs.rs`, utilizing `doc_cfg` and `doc_auto_cfg` features.
- Expanded inline documentation for:
  - `ImageFont` and its fields (`atlas_layout`, `texture`, `atlas_character_map`).
  - `ImageFontLayout` variants and layout methods.
  - `ImageFontSettings`, detailing paths and layout mappings.
  - Error enums (`ImageFontLoadError` and `ImageFontRenderError`) with explanations of failure scenarios.
- Enhanced documentation for `ImageFontPlugin`, including usage examples and feature descriptions.

#### Tooling

- Added stricter linting rules in `Cargo.toml` to improve code quality.
- Added Rust lint configuration to `Cargo.toml`.
- Added the `camino` crate (version 1.1.9) with `serde1` feature for robust UTF-8 path handling.

### Changed

#### Core Library

- Removed `pub use` statements for `rendered` and `atlas_sprites` modules in `src/lib.rs`:
  - Users must now import items from these modules.

#### Examples

- Adjusted imports in `atlased_sprite.rs`, `rendered_sprite.rs`, and `rendered_ui.rs` to reflect the removal of `pub use` statements

#### Refactoring

- Refactored text rendering systems into the `rendered` module, conditional on the `rendered` feature.
- Renamed examples for clarity:
  - `sprite.rs` to `rendered_sprite.rs`.
  - `bevy_ui.rs` to `rendered_ui.rs`.

#### Features and APIs

- Made internal rendering text function private.
- Updated `Cargo.toml` to make the `image` dependency optional, activated only with the `rendered` feature.
- Replaced `PathBuf` with `Utf8PathBuf` in `ImageFontSettings` for stricter validation and compatibility with non-ASCII paths.
- Updated error messages in `ImageFontLoader` for better diagnostics.

### Fixed

- Resolved inconsistencies in type annotations, parameter naming, and error handling across core modules.
- Corrected link to `rendered_sprite.rs` in the "Note on Pixel Accuracy" section.

### Removed

- Removed redundant font setup logic from individual examples, consolidating it into a reusable `common` module.

### Breaking Changes

- Renamed `ImageFontSpriteText` to `ImageFontPreRenderedText` and `ImageFontUiText` to `ImageFontPreRenderedUiText`.
- Users of the library will need to update their import paths for components and related types from the `rendered` and `atlas_sprites` modules rather than from the root.

## [0.6.0] - 2024-12-31

### Added

- `ImageFontSpriteText` and `ImageFontUiText` were added as replacements for `ImageFontBundle` and `ImageFontUiBundle`. These now use Bevy 0.15's new required components to ensure that the entity has the components required to show the image font text.

### Changed

- Crate updated to target Bevy 0.14.

### Removed

- `ImageFontBundle` and `ImageFontUiBundle` have been removed.

## [0.5.1] - 2024-12-30

### Fixed

- Accidentally left behind some old repo links to the previous maintainer's repo.

## [0.5.0] - 2024-12-30

### Changed

- Crate renamed from `extol_image_font` to `bevy_image_font` and has a new maintainer.
- Crate updated to target Bevy 0.14.

### Fixed

- Only declare `ImageFontUiBundle` when feature `ui` is enabled, as it's useless otherwise and because `ImageBundle` is unavailable without `bevy`'s `bevy_ui` feature enabled.
- Clarify why text might sometimes render blurry in README. Update `sprite` example to illustrate work-arounds.

## [0.4.0] - 2024-04-04

- First public release; prior versions are not on Cargo.

[unreleased]: https://github.com/ilyvion/bevy_image_font/compare/0.9.0...HEAD
[0.9.0]: https://github.com/ilyvion/bevy_image_font/compare/0.8.0...0.9.0
[0.8.0]: https://github.com/ilyvion/bevy_image_font/compare/0.7.1...0.8.0
[0.7.1]: https://github.com/ilyvion/bevy_image_font/compare/0.7.0...0.7.1
[0.7.0]: https://github.com/ilyvion/bevy_image_font/compare/0.6.0...0.7.0
[0.6.0]: https://github.com/ilyvion/bevy_image_font/compare/0.5.1...0.6.0
[0.5.1]: https://github.com/ilyvion/bevy_image_font/compare/0.5.0...0.5.1
[0.5.0]: https://github.com/ilyvion/bevy_image_font/compare/v0.4.0...0.5.0
[0.4.0]: https://github.com/ilyvion/bevy_image_font/releases/tag/v0.4.0
