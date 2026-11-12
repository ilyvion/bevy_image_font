//! Conversion of `BMFont` character ids into `char`s according to the font's
//! `info.unicode` and `info.charset` settings.

#[cfg(feature = "bmf_charsets")]
use bmfont_rs::Charset;
use bmfont_rs::Info;

use super::BmFontLoadError;
use crate::loader::ImageFontLoaderSettings;

/// How a font's character ids map to `char`s.
#[derive(Debug, Clone, Copy)]
pub(super) enum CharIds {
    /// Character ids are Unicode code points.
    Unicode,
    /// Character ids are codes in a code page. Ids above `0xFF` are double-byte
    /// codes, with the lead byte in the high byte.
    #[cfg(feature = "bmf_charsets")]
    CodePage(CodePage),
    /// Character ids are codes in a symbol font, which map to `U+F000` plus
    /// the code, as on Windows.
    #[cfg(feature = "bmf_charsets")]
    Symbol,
}

impl CharIds {
    /// The mapping for a font with the given `info` block.
    ///
    /// # Errors
    ///
    /// Fails if the font isn't a Unicode font and its charset can't be decoded.
    pub(super) fn new(
        info: &Info,
        #[cfg_attr(
            not(feature = "bmf_charsets"),
            expect(
                unused_variables,
                reason = "only non-Unicode charsets use the loader settings"
            )
        )]
        settings: &ImageFontLoaderSettings,
    ) -> Result<Self, BmFontLoadError> {
        if info.unicode {
            return Ok(Self::Unicode);
        }

        #[cfg(feature = "bmf_charsets")]
        {
            Self::from_charset(&info.charset, settings)
        }
        #[cfg(not(feature = "bmf_charsets"))]
        {
            Err(BmFontLoadError::CharsetUnsupported)
        }
    }

    /// The mapping for a non-Unicode font with the given charset. A missing
    /// charset is read as `ANSI`, which is charset 0 in the binary format.
    #[cfg(feature = "bmf_charsets")]
    fn from_charset(
        charset: &Charset,
        settings: &ImageFontLoaderSettings,
    ) -> Result<Self, BmFontLoadError> {
        let code_page = match *charset {
            Charset::Null | Charset::Tagged(bmfont_rs::ANSI | bmfont_rs::DEFAULT) => {
                settings.bmf_ansi_code_page
            }
            Charset::Tagged(bmfont_rs::SYMBOL) => return Ok(Self::Symbol),
            Charset::Tagged(bmfont_rs::OEM) => settings.bmf_oem_code_page,
            Charset::Tagged(tag) => windows_code_page(tag)
                .ok_or_else(|| BmFontLoadError::UnknownCharset(charset.to_string()))?,
            Charset::Undefined(ref name) => {
                return Err(BmFontLoadError::UnknownCharset(name.clone()));
            }
        };

        CodePage::new(code_page)
            .map(Self::CodePage)
            .ok_or(BmFontLoadError::CodePageUnsupported(code_page))
    }

    /// The `char` that the character id `id` stands for, if any.
    pub(super) fn to_char(self, id: u32) -> Option<char> {
        match self {
            Self::Unicode => char::from_u32(id),
            #[cfg(feature = "bmf_charsets")]
            Self::CodePage(code_page) => code_page.decode(id),
            #[cfg(feature = "bmf_charsets")]
            Self::Symbol => {
                let code = u8::try_from(id).ok()?;
                char::from_u32(0xF000 + u32::from(code))
            }
        }
    }
}

/// The Windows code page of the `BMFont` charset `tag`, for the charsets that
/// name a fixed code page.
#[cfg(feature = "bmf_charsets")]
fn windows_code_page(tag: u8) -> Option<u16> {
    Some(match tag {
        bmfont_rs::MAC => 10000,
        bmfont_rs::SHIFTJIS => 932,
        bmfont_rs::HANGUL => 949,
        bmfont_rs::JOHAB => 1361,
        bmfont_rs::GB2312 => 936,
        bmfont_rs::CHINESEBIG5 => 950,
        bmfont_rs::GREEK => 1253,
        bmfont_rs::TURKISH => 1254,
        bmfont_rs::VIETNAMESE => 1258,
        bmfont_rs::HEBREW => 1255,
        bmfont_rs::ARABIC => 1256,
        bmfont_rs::BALTIC => 1257,
        bmfont_rs::RUSSIAN => 1251,
        bmfont_rs::THAI => 874,
        bmfont_rs::EASTEUROPE => 1250,
        _ => return None,
    })
}

/// A decoder for a Windows code page.
#[cfg(feature = "bmf_charsets")]
#[derive(Debug, Clone, Copy)]
pub(super) enum CodePage {
    /// A code page `encoding_rs` decodes.
    Encoding(&'static encoding_rs::Encoding),
    /// A single-byte code page `oem_cp` decodes.
    Oem(&'static oem_cp::code_table_type::TableType),
}

#[cfg(feature = "bmf_charsets")]
impl CodePage {
    /// The decoder for code page number `code_page`, if it is supported.
    fn new(code_page: u16) -> Option<Self> {
        codepage::to_encoding_no_replacement(code_page)
            .map(Self::Encoding)
            .or_else(|| {
                oem_cp::code_table::DECODING_TABLE_CP_MAP
                    .get(&code_page)
                    .map(Self::Oem)
            })
    }

    /// The `char` that the single- or double-byte code `id` stands for, if it
    /// is a valid code for exactly one character.
    fn decode(self, id: u32) -> Option<char> {
        let pair = u16::try_from(id).ok()?.to_be_bytes();
        let bytes = if pair[0] == 0 { &pair[1..] } else { &pair[..] };

        match self {
            Self::Encoding(encoding) => {
                let decoded =
                    encoding.decode_without_bom_handling_and_without_replacement(bytes)?;
                let mut chars = decoded.chars();
                let character = chars.next()?;
                chars.next().is_none().then_some(character)
            }
            Self::Oem(table) => {
                let [byte] = *bytes else {
                    return None;
                };
                table.decode_char_checked(byte)
            }
        }
    }
}

#[cfg(test)]
mod tests;
