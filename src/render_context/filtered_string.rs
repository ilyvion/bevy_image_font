#![allow(
    dead_code,
    reason = "private utility code that, depending on the activated feature set, will sometimes be missing uses"
)]
//! A module for filtering strings based on a character map.
//!
//! This module provides the [`FilteredString`] type, which offers a reusable
//! and efficient abstraction for working with strings filtered according to a
//! predefined character map. This is particularly useful in scenarios where
//! only a subset of characters are supported, such as rendering text with an
//! `ImageFont`.
//!
//! # Key Features
//! - **Efficient Filtering:** Filters strings without unnecessary allocations,
//!   making it suitable for performance-sensitive contexts.
//! - **Iterator Support:** Allows iteration over filtered characters for
//!   further processing.
//! - **Display Implementation:** Can be directly converted to a string
//!   representation of the filtered content.
//!
//! # Usage
//! This module is typically used in conjunction with image font rendering
//! systems, where only characters supported by the font's atlas are allowed. It
//! ensures unsupported characters are ignored while preserving the order of the
//! valid characters.

use std::fmt;

use bevy::platform::collections::HashMap;

use crate::{ImageFont, ImageFontCharacter};

/// A wrapper type for filtering characters from a string based on a character
/// map.
///
/// This type allows you to filter out characters from a string that do not
/// exist in the given `atlas_character_map`. It provides a reusable abstraction
/// for working with filtered strings efficiently, avoiding unnecessary
/// allocations.
#[derive(Debug)]
pub(crate) struct FilteredString<'map, S: AsRef<str>> {
    /// The input string to be filtered.
    ///
    /// This string serves as the source for filtering operations. Characters
    /// in this string are compared against the characters in
    /// `atlas_character_map`.
    string: S,

    /// A reference to a map of characters to their indices in a texture atlas.
    ///
    /// This map determines which characters from `string` are retained during
    /// filtering. Only characters present as keys in this map will be included
    /// in the filtered output.
    atlas_character_map: &'map HashMap<char, ImageFontCharacter>,
}

impl<'map, S: AsRef<str>> FilteredString<'map, S> {
    /// Creates a new `FilteredString` instance.
    ///
    /// # Parameters
    /// - `string`: The input string to be filtered.
    /// - `atlas_character_map`: A reference to a character map that determines
    ///   which characters are retained.
    ///
    /// # Returns
    /// A `FilteredString` instance that can produce iterators over the filtered
    /// characters.
    pub(crate) fn new(
        string: S,
        atlas_character_map: &'map HashMap<char, ImageFontCharacter>,
    ) -> Self {
        Self {
            string,
            atlas_character_map,
        }
    }

    /// Returns an iterator over the filtered characters.
    ///
    /// This method filters the input string to include only characters that
    /// exist in the `atlas_character_map`.
    ///
    /// # Returns
    /// An iterator that yields characters retained by the filter.
    pub(crate) fn filtered_chars(&self) -> impl Iterator<Item = char> + '_ {
        self.string
            .as_ref()
            .chars()
            .filter(|character| self.atlas_character_map.contains_key(character))
    }

    /// Checks if the filtered string is empty.
    ///
    /// # Returns
    /// `true` if there are no characters in the filtered string; otherwise,
    /// `false`.
    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.filtered_chars().next().is_none()
    }
}

impl<S: AsRef<str>> fmt::Display for FilteredString<'_, S> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for character in self.filtered_chars() {
            write!(formatter, "{character}")?;
        }
        Ok(())
    }
}

impl ImageFont {
    /// Filters a string to include only characters present in the font's
    /// character map.
    ///
    /// This function returns a
    /// [`FilteredString`](filtered_string::FilteredString) containing only the
    /// characters from the input string that exist in the font's
    /// `atlas_character_map`. It ensures that unsupported characters are
    /// excluded during rendering.
    ///
    /// # Parameters
    /// - `string`: The input string to filter.
    ///
    /// # Returns
    /// A `FilteredString` returning only characters supported by the font.
    ///
    /// # Notes
    /// This function requires either the `rendered` or `atlas_sprites` feature
    /// to be enabled.
    pub(super) fn filter_string<S: AsRef<str>>(&self, string: S) -> FilteredString<'_, S> {
        FilteredString::new(string, &self.atlas_character_map)
    }
}

#[cfg(test)]
mod tests;
