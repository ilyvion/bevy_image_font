#![allow(clippy::unwrap_used, reason = "test code panics to indicate errors")]

use bmfont_rs::Charset;

use super::*;

/// An `info` block with the given charset settings.
#[cfg(feature = "bmf_charsets")]
fn info(charset: Charset, unicode: bool) -> Info {
    Info {
        charset,
        unicode,
        ..Info::default()
    }
}

/// The mapping for a non-Unicode font with `charset` and the given settings.
#[cfg(feature = "bmf_charsets")]
fn char_ids_with(charset: Charset, settings: &ImageFontLoaderSettings) -> CharIds {
    CharIds::new(&info(charset, false), settings).unwrap()
}

/// The mapping for a non-Unicode font with `charset` and the default
/// settings.
#[cfg(feature = "bmf_charsets")]
fn char_ids(charset: Charset) -> CharIds {
    char_ids_with(charset, &ImageFontLoaderSettings::default())
}

#[test]
fn unicode_ids_are_code_points() {
    let info = Info {
        charset: Charset::Null,
        unicode: true,
        ..Info::default()
    };
    let char_ids = CharIds::new(&info, &ImageFontLoaderSettings::default()).unwrap();

    assert_eq!(char_ids.to_char(0x41), Some('A'));
    assert_eq!(char_ids.to_char(0x3042), Some('あ'));
    assert_eq!(char_ids.to_char(0xD800), None);
}

#[cfg(not(feature = "bmf_charsets"))]
#[test]
fn non_unicode_fonts_are_rejected_without_the_feature() {
    let info = Info {
        charset: Charset::Tagged(bmfont_rs::ANSI),
        unicode: false,
        ..Info::default()
    };

    assert!(matches!(
        CharIds::new(&info, &ImageFontLoaderSettings::default()),
        Err(BmFontLoadError::CharsetUnsupported)
    ));
}

#[cfg(feature = "bmf_charsets")]
#[test]
fn double_byte_charsets_decode_lead_byte_in_high_byte() {
    let shift_jis = char_ids(Charset::Tagged(bmfont_rs::SHIFTJIS));
    assert_eq!(shift_jis.to_char(0x82A0), Some('あ'));
    assert_eq!(shift_jis.to_char(0x41), Some('A'));
    assert_eq!(shift_jis.to_char(0xB1), Some('ｱ'));
    // A lead byte on its own is not a character.
    assert_eq!(shift_jis.to_char(0x82), None);
    assert_eq!(shift_jis.to_char(0x0001_82A0), None);

    assert_eq!(
        char_ids(Charset::Tagged(bmfont_rs::GB2312)).to_char(0xB0A1),
        Some('啊')
    );
    assert_eq!(
        char_ids(Charset::Tagged(bmfont_rs::CHINESEBIG5)).to_char(0xA440),
        Some('一')
    );
    assert_eq!(
        char_ids(Charset::Tagged(bmfont_rs::HANGUL)).to_char(0xB0A1),
        Some('가')
    );
}

#[cfg(feature = "bmf_charsets")]
#[test]
fn single_byte_charsets_decode_their_code_page() {
    for (tag, id, expected) in [
        (bmfont_rs::MAC, 0x80, 'Ä'),
        (bmfont_rs::GREEK, 0xC1, 'Α'),
        (bmfont_rs::TURKISH, 0xD0, 'Ğ'),
        (bmfont_rs::VIETNAMESE, 0xD5, 'Ơ'),
        (bmfont_rs::HEBREW, 0xE0, 'א'),
        (bmfont_rs::ARABIC, 0xC7, 'ا'),
        (bmfont_rs::BALTIC, 0xC0, 'Ą'),
        (bmfont_rs::RUSSIAN, 0xC0, 'А'),
        (bmfont_rs::THAI, 0xA1, 'ก'),
        (bmfont_rs::EASTEUROPE, 0x8A, 'Š'),
    ] {
        let char_ids = char_ids(Charset::Tagged(tag));
        assert_eq!(char_ids.to_char(id), Some(expected), "charset {tag}");
        assert_eq!(char_ids.to_char(0x41), Some('A'), "charset {tag}");
    }
}

#[cfg(feature = "bmf_charsets")]
#[test]
fn single_byte_charsets_reject_double_byte_ids() {
    let russian = char_ids(Charset::Tagged(bmfont_rs::RUSSIAN));
    assert_eq!(russian.to_char(0xC0C0), None);

    let oem = char_ids(Charset::Tagged(bmfont_rs::OEM));
    assert_eq!(oem.to_char(0xFBFB), None);
}

#[cfg(feature = "bmf_charsets")]
#[test]
fn ansi_charsets_use_the_ansi_code_page_setting() {
    for charset in [
        Charset::Tagged(bmfont_rs::ANSI),
        Charset::Tagged(bmfont_rs::DEFAULT),
        Charset::Null,
    ] {
        assert_eq!(
            char_ids(charset.clone()).to_char(0x80),
            Some('€'),
            "{charset:?}"
        );

        let settings = ImageFontLoaderSettings {
            bmf_ansi_code_page: 1251,
            ..ImageFontLoaderSettings::default()
        };
        assert_eq!(
            char_ids_with(charset.clone(), &settings).to_char(0xC0),
            Some('А'),
            "{charset:?}"
        );
    }
}

#[cfg(feature = "bmf_charsets")]
#[test]
fn oem_charset_uses_the_oem_code_page_setting() {
    let charset = Charset::Tagged(bmfont_rs::OEM);
    assert_eq!(char_ids(charset.clone()).to_char(0xFB), Some('√'));
    assert_eq!(char_ids(charset.clone()).to_char(0x9B), Some('¢'));

    let cp850 = ImageFontLoaderSettings {
        bmf_oem_code_page: 850,
        ..ImageFontLoaderSettings::default()
    };
    assert_eq!(
        char_ids_with(charset.clone(), &cp850).to_char(0x9B),
        Some('ø')
    );

    // Code page 866 is decoded by `encoding_rs` rather than `oem_cp`.
    let cp866 = ImageFontLoaderSettings {
        bmf_oem_code_page: 866,
        ..ImageFontLoaderSettings::default()
    };
    assert_eq!(char_ids_with(charset, &cp866).to_char(0x80), Some('А'));
}

#[cfg(feature = "bmf_charsets")]
#[test]
fn symbol_charset_maps_to_private_use_area() {
    let symbol = char_ids(Charset::Tagged(bmfont_rs::SYMBOL));

    assert_eq!(symbol.to_char(0x41), Some('\u{F041}'));
    assert_eq!(symbol.to_char(0xFF), Some('\u{F0FF}'));
    assert_eq!(symbol.to_char(0x100), None);
}

#[cfg(feature = "bmf_charsets")]
#[test]
fn unsupported_charsets_are_rejected() {
    let settings = ImageFontLoaderSettings::default();

    assert!(matches!(
        CharIds::new(&info(Charset::Tagged(bmfont_rs::JOHAB), false), &settings),
        Err(BmFontLoadError::CodePageUnsupported(1361))
    ));
    assert!(matches!(
        CharIds::new(&info(Charset::Tagged(3), false), &settings),
        Err(BmFontLoadError::UnknownCharset(ref name)) if name == "3"
    ));
    assert!(matches!(
        CharIds::new(&info(Charset::Undefined("FOO".to_owned()), false), &settings),
        Err(BmFontLoadError::UnknownCharset(ref name)) if name == "FOO"
    ));

    let bad_code_page = ImageFontLoaderSettings {
        bmf_ansi_code_page: 1,
        ..ImageFontLoaderSettings::default()
    };
    assert!(matches!(
        CharIds::new(
            &info(Charset::Tagged(bmfont_rs::ANSI), false),
            &bad_code_page
        ),
        Err(BmFontLoadError::CodePageUnsupported(1))
    ));
}
