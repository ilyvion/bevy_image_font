//! This module defines the `LetterSpacing` enum, which specifies how spacing
//! between characters in text is applied when rendering with image fonts.
//!
//! The `LetterSpacing` enum provides two variants:
//! - `Pixel(i16)`: Specifies spacing as an integer value, ideal for
//!   pixel-perfect alignment.
//! - `Floating(f32)`: Specifies spacing as a floating-point value, allowing for
//!   precise control.
//!
//! Key Features:
//! - Conversion to `f32` via the `to_f32` method, enabling consistent usage in
//!   rendering calculations.
//! - Default implementation (`Pixel(0)`), representing no spacing between
//!   characters.

use bevy::prelude::*;
/// Specifies the spacing between characters in text rendering.
///
/// This enum provides options for defining the kerning or spacing between
/// individual characters in a line of text. When using `Pixel(i16)`, the
/// spacing is specified in the font's native height and is scaled
/// proportionally based on the current font height.
///
/// It supports both
/// pixel-perfect alignment and precise floating-point adjustments, offering
/// flexibility for various rendering scenarios.
///
/// The choice of variant depends on the rendering requirements. For example,
/// pixel-based spacing is often used in retro-style or low-resolution contexts,
/// while floating-point spacing is better suited for high-resolution or
/// sub-pixel rendering.
#[derive(Debug, Clone, Copy, PartialEq, Reflect)]
pub enum LetterSpacing {
    /// Spacing as an integer value, ideal for pixel-perfect alignment.
    ///
    /// This variant ensures that spacing between characters is aligned to
    /// integer pixel values, making it suitable for retro-style games,
    /// low-resolution screens, or any scenario requiring precise, whole-pixel
    /// placement.
    Pixel(i16),
    /// Spacing as a floating-point value, offering precise control.
    ///
    /// This variant allows for fractional spacing between characters, making
    /// it ideal for high-resolution displays or scenarios where sub-pixel
    /// accuracy is required. It is especially useful for achieving smooth
    /// typography or applying gradual spacing adjustments.
    Floating(f32),
}

impl LetterSpacing {
    /// Converts the letter spacing into a floating-point value.
    #[must_use]
    pub fn to_f32(self) -> f32 {
        match self {
            LetterSpacing::Pixel(pixels) => f32::from(pixels),
            LetterSpacing::Floating(value) => value,
        }
    }
}

impl Default for LetterSpacing {
    /// Zero constant spacing between character
    fn default() -> Self {
        Self::Pixel(0)
    }
}

impl From<LetterSpacing> for f32 {
    fn from(spacing: LetterSpacing) -> f32 {
        spacing.to_f32()
    }
}

#[cfg(test)]
mod tests {
    use float_eq::assert_float_eq;

    use super::*;
    use crate::tests::utils::COMPARISON_TOLERANCE;

    #[test]
    fn to_f32_gives_expected_value() {
        // Test Pixel spacing
        assert_float_eq!(
            LetterSpacing::Pixel(0).to_f32(),
            0.0,
            abs <= COMPARISON_TOLERANCE
        );
        assert_float_eq!(
            LetterSpacing::Pixel(10).to_f32(),
            10.0,
            abs <= COMPARISON_TOLERANCE
        );
        assert_float_eq!(
            LetterSpacing::Pixel(-5).to_f32(),
            -5.0,
            abs <= COMPARISON_TOLERANCE
        );

        // Test Floating spacing
        assert_float_eq!(
            LetterSpacing::Floating(0.0).to_f32(),
            0.0,
            abs <= COMPARISON_TOLERANCE
        );
        assert_float_eq!(
            LetterSpacing::Floating(1.5).to_f32(),
            1.5,
            abs <= COMPARISON_TOLERANCE
        );
        assert_float_eq!(
            LetterSpacing::Floating(-3.2).to_f32(),
            -3.2,
            abs <= COMPARISON_TOLERANCE
        );
    }

    #[test]
    fn default_is_correct() {
        // Default value should be Pixel(0)
        assert_eq!(LetterSpacing::default(), LetterSpacing::Pixel(0));
    }

    #[test]
    fn conversion_gives_expected_value() {
        // Test conversion to f32
        let spacing_pixel: f32 = LetterSpacing::Pixel(10).into();
        assert_float_eq!(spacing_pixel, 10.0, abs <= COMPARISON_TOLERANCE);

        let spacing_floating: f32 = LetterSpacing::Floating(2.5).into();
        assert_float_eq!(spacing_floating, 2.5, abs <= COMPARISON_TOLERANCE);
    }

    #[test]
    fn extreme_pixel_spacing_gives_expected_value() {
        assert_float_eq!(
            LetterSpacing::Pixel(i16::MAX).to_f32(),
            f32::from(i16::MAX),
            abs <= COMPARISON_TOLERANCE
        );
        assert_float_eq!(
            LetterSpacing::Pixel(i16::MIN).to_f32(),
            f32::from(i16::MIN),
            abs <= COMPARISON_TOLERANCE
        );
    }
}
