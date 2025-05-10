//! This module defines the `ScalingMode` enum, which specifies how scaling is
//! applied to character glyph dimensions during rendering.
//!
//! The `ScalingMode` enum provides three options for handling fractional values
//! when scaling glyph dimensions to match a target font height:
//! - `Truncated`: Scales values and truncates fractional parts for
//!   pixel-perfect rendering.
//! - `Rounded`: Scales values and rounds to the nearest whole number for
//!   balanced precision.
//! - `Smooth`: Retains full precision, ideal for high-quality or sub-pixel
//!   rendering.
//!
//! Key Features:
//! - The `apply_scale` method centralizes scaling logic for consistent
//!   behavior.
//! - Default implementation (`Rounded`) balances precision and visual quality.

use bevy::prelude::*;

/// Determines how scaling is applied when calculating the dimensions of a
/// character glyph. Scaling primarily affects width adjustments, while height
/// remains proportional to the original glyph aspect ratio.
///
/// This enum is used to control how fractional values are handled when scaling
/// glyph dimensions to fit a specified font height. It provides options for
/// truncating, rounding, or retaining precise values, offering flexibility
/// based on the rendering requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect, Default)]
pub enum ScalingMode {
    /// Truncates fractional values during scaling.
    ///
    /// This mode ensures that the width and height of the glyph are always
    /// rounded down to the nearest whole number. It can be useful for
    /// pixel-perfect rendering where fractional dimensions could cause
    /// visual artifacts.
    Truncated,

    /// Rounds fractional values during scaling.
    ///
    /// This mode rounds the width and height of the glyph to the nearest whole
    /// number. It offers a balance between precision and consistency, often
    /// used when slight inaccuracies are acceptable but extreme rounding
    /// errors need to be avoided.
    ///
    /// This is the default scaling mode.
    #[default]
    Rounded,

    /// Retains precise fractional values during scaling.
    ///
    /// This mode avoids rounding entirely, keeping the scaled dimensions as
    /// floating-point values. It is ideal for high-precision rendering or
    /// cases where exact scaling is necessary, such as when performing
    /// sub-pixel positioning.
    Smooth,
}

impl ScalingMode {
    /// Applies the scaling mode to the provided value given a scale factor.
    ///
    /// # Parameters
    /// - `value`: The value to be scaled.
    /// - `scale_factor`: The factor by which the value is scaled.
    ///
    /// # Returns
    /// The scaled value, adjusted according to the scaling mode.
    #[must_use]
    pub fn apply_scale(self, value: f32, scale_factor: f32) -> f32 {
        let scaled = value * scale_factor;
        match self {
            ScalingMode::Truncated => scaled.trunc(),
            ScalingMode::Rounded => scaled.round(),
            ScalingMode::Smooth => scaled,
        }
    }
}

#[cfg(test)]
mod tests {
    use float_eq::assert_float_eq;

    use super::*;
    use crate::tests::utils::COMPARISON_TOLERANCE;

    #[test]
    fn apply_scale_scales_correctly() {
        let value = 10.5;

        // Test Truncated
        assert_float_eq!(
            ScalingMode::Truncated.apply_scale(value, 2.0),
            21.0,
            abs <= COMPARISON_TOLERANCE
        );
        assert_float_eq!(
            ScalingMode::Truncated.apply_scale(value, 0.5),
            5.0,
            abs <= COMPARISON_TOLERANCE
        );

        // Test Rounded
        assert_float_eq!(
            ScalingMode::Rounded.apply_scale(value, 2.0),
            21.0,
            abs <= COMPARISON_TOLERANCE
        );
        assert_float_eq!(
            ScalingMode::Rounded.apply_scale(value, 0.5),
            5.0,
            abs <= COMPARISON_TOLERANCE
        );

        // Test Smooth
        assert_float_eq!(
            ScalingMode::Smooth.apply_scale(value, 2.0),
            21.0,
            abs <= COMPARISON_TOLERANCE
        );
        assert_float_eq!(
            ScalingMode::Smooth.apply_scale(value, 0.5),
            5.25,
            abs <= COMPARISON_TOLERANCE
        );
    }

    #[test]
    fn apply_scale_edge_cases() {
        let value = 0.0;

        // Test with zero value
        assert_float_eq!(
            ScalingMode::Truncated.apply_scale(value, 1.0),
            0.0,
            abs <= COMPARISON_TOLERANCE
        );
        assert_float_eq!(
            ScalingMode::Rounded.apply_scale(value, 1.0),
            0.0,
            abs <= COMPARISON_TOLERANCE
        );
        assert_float_eq!(
            ScalingMode::Smooth.apply_scale(value, 1.0),
            0.0,
            abs <= COMPARISON_TOLERANCE
        );

        // Test with zero scale factor
        let value = 10.0;
        assert_float_eq!(
            ScalingMode::Truncated.apply_scale(value, 0.0),
            0.0,
            abs <= COMPARISON_TOLERANCE
        );
        assert_float_eq!(
            ScalingMode::Rounded.apply_scale(value, 0.0),
            0.0,
            abs <= COMPARISON_TOLERANCE
        );
        assert_float_eq!(
            ScalingMode::Smooth.apply_scale(value, 0.0),
            0.0,
            abs <= COMPARISON_TOLERANCE
        );

        // Test with negative scale factor
        let value = 10.5;
        assert_float_eq!(
            ScalingMode::Truncated.apply_scale(value, -1.0),
            -10.0,
            abs <= COMPARISON_TOLERANCE
        );
        assert_float_eq!(
            ScalingMode::Rounded.apply_scale(value, -1.0),
            -11.0,
            abs <= COMPARISON_TOLERANCE
        );
        assert_float_eq!(
            ScalingMode::Smooth.apply_scale(value, -1.0),
            -10.5,
            abs <= COMPARISON_TOLERANCE
        );
    }
}
