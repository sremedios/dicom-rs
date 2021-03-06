//! This module contains reusable components for encoding and decoding text in DICOM
//! data structures, including support for character repertoires.
//!
//! The Character Repertoires supported by DICOM are:
//! - ISO 8859
//! - JIS X 0201-1976 Code for Information Interchange
//! - JIS X 0208-1990 Code for the Japanese Graphic Character set for information interchange
//! - JIS X 0212-1990 Code of the supplementary Japanese Graphic Character set for information interchange
//! - KS X 1001 (registered as ISO-IR 149) for Korean Language
//! - TIS 620-2533 (1990) Thai Characters Code for Information Interchange
//! - ISO 10646-1, 10646-2, and their associated supplements and extensions for Unicode character set
//! - GB 18030
//! - GB2312
//!
//! At the moment, this library supports only IR-6 and IR-192.

use encoding::{DecoderTrap, EncoderTrap, Encoding, RawDecoder, StringWriter};
use encoding::all::{ISO_8859_1, UTF_8};
use error::{Result, TextEncodingError};
use std::fmt::Debug;

/// A holder of encoding and decoding mechanisms for text in DICOM content,
/// which according to the standard, depends on the specific character set.
pub trait TextCodec: Debug {
    /// Decode the given byte buffer as a single string. The resulting string
    /// _may_ contain backslash characters ('\') to delimit individual values,
    /// and should be split later on if required.
    fn decode(&self, text: &[u8]) -> Result<String>;

    /// Encode a text value into a byte vector. The input string can
    /// feature multiple text values by using the backslash character ('\')
    /// as the value delimiter.
    fn encode(&self, text: &str) -> Result<Vec<u8>>;
}

impl<T: ?Sized> TextCodec for Box<T>
where
    T: TextCodec,
{
    fn decode(&self, text: &[u8]) -> Result<String> {
        self.as_ref().decode(text)
    }

    fn encode(&self, text: &str) -> Result<Vec<u8>> {
        self.as_ref().encode(text)
    }
}

impl<'a, T: ?Sized> TextCodec for &'a T
where
    T: TextCodec,
{
    fn decode(&self, text: &[u8]) -> Result<String> {
        (*self).decode(text)
    }

    fn encode(&self, text: &str) -> Result<Vec<u8>> {
        (*self).encode(text)
    }
}

/// Type alias for a type erased text codec.
pub type DynamicTextCodec = Box<TextCodec>;

/// An enum type for the the supported character sets.
#[derive(Debug, Clone, Copy)]
pub enum SpecificCharacterSet {
    /// The default character set.
    Default, // TODO needs more
    /// The Unicode character set defined in ISO IR 192, based on the UTF-8 encoding.
    IsoIr192,
}

impl SpecificCharacterSet {
    pub fn from_code(uid: &str) -> Option<Self> {
        use self::SpecificCharacterSet::*;
        match uid {
            "Default" | "ISO_IR_6" => Some(Default),
            "ISO_IR_192" => Some(IsoIr192),
            _ => None,
        }
    }

    /// Retrieve the respective text codec.
    pub fn get_codec(&self) -> Option<Box<TextCodec>> {
        match *self {
            SpecificCharacterSet::Default => Some(Box::new(DefaultCharacterSetCodec)),
            SpecificCharacterSet::IsoIr192 => Some(Box::new(Utf8CharacterSetCodec)),
        }
    }
}

fn decode_text_trap(_decoder: &mut RawDecoder, input: &[u8], output: &mut StringWriter) -> bool {
    let c = input[0];
    let o0 = c & 7;
    let o1 = (c & 56) >> 3;
    let o2 = (c & 192) >> 6;
    output.write_char('\\');
    output.write_char((o2 + b'0') as char);
    output.write_char((o1 + b'0') as char);
    output.write_char((o0 + b'0') as char);
    true
}

impl Default for SpecificCharacterSet {
    fn default() -> SpecificCharacterSet {
        SpecificCharacterSet::Default
    }
}

/// Data type representing the default character set.
#[derive(Debug, Default, Clone, PartialEq, Eq, Copy)]
pub struct DefaultCharacterSetCodec;

impl TextCodec for DefaultCharacterSetCodec {
    fn decode(&self, text: &[u8]) -> Result<String> {
        ISO_8859_1
            .decode(text, DecoderTrap::Call(decode_text_trap))
            .map_err(|e| TextEncodingError::new(e).into())
    }

    fn encode(&self, text: &str) -> Result<Vec<u8>> {
        ISO_8859_1
            .encode(text, EncoderTrap::Strict)
            .map_err(|e| TextEncodingError::new(e).into())
    }
}

/// Data type representing the UTF-8 character set.
#[derive(Debug, Default, Clone, PartialEq, Eq, Copy)]
pub struct Utf8CharacterSetCodec;

impl TextCodec for Utf8CharacterSetCodec {
    fn decode(&self, text: &[u8]) -> Result<String> {
        UTF_8
            .decode(text, DecoderTrap::Call(decode_text_trap))
            .map_err(|e| TextEncodingError::new(e).into())
    }

    fn encode(&self, text: &str) -> Result<Vec<u8>> {
        UTF_8
            .encode(text, EncoderTrap::Strict)
            .map_err(|e| TextEncodingError::new(e).into())
    }
}

/// The result of a text validation procedure (please see [`validate_iso_8859`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TextValidationOutcome {
    /// The text is fully valid and can be safely decoded.
    Ok,
    /// Some characters may have to be replaced, other than that the text can be safely decoded.
    BadCharacters,
    /// The text cannot be decoded.
    NotOk,
}

/// Check whether the given byte slice contains valid text from the default character repertoire.
pub fn validate_iso_8859(text: &[u8]) -> TextValidationOutcome {
    if let Err(_) = ISO_8859_1.decode(text, DecoderTrap::Strict) {
        match ISO_8859_1.decode(text, DecoderTrap::Call(decode_text_trap)) {
            Ok(_) => TextValidationOutcome::BadCharacters,
            Err(_) => TextValidationOutcome::NotOk,
        }
    } else {
        TextValidationOutcome::Ok
    }
}

/// Check whether the given byte slice contains only valid characters for a
/// Date value representation.
pub fn validate_da(text: &[u8]) -> TextValidationOutcome {
    if text.into_iter().cloned().all(|c| c >= b'0' && c <= b'9') {
        TextValidationOutcome::Ok
    } else {
        TextValidationOutcome::NotOk
    }
}

/// Check whether the given byte slice contains only valid characters for a
/// Time value representation.
pub fn validate_tm(text: &[u8]) -> TextValidationOutcome {
    if text.into_iter().cloned().all(|c| match c {
        b'\\' | b'.' | b'-' | b' ' => true,
        c => c >= b'0' && c <= b'9',
    }) {
        TextValidationOutcome::Ok
    } else {
        TextValidationOutcome::NotOk
    }
}

/// Check whether the given byte slice contains only valid characters for a
/// Date Time value representation.
pub fn validate_dt(text: &[u8]) -> TextValidationOutcome {
    if text.into_iter().cloned().all(|c| match c {
        b'.' | b'-' | b'+' | b' ' | b'\\' => true,
        c => c >= b'0' && c <= b'9',
    }) {
        TextValidationOutcome::Ok
    } else {
        TextValidationOutcome::NotOk
    }
}

/// Check whether the given byte slice contains only valid characters for a
/// Code String value representation.
pub fn validate_cs(text: &[u8]) -> TextValidationOutcome {
    if text.into_iter().cloned().all(|c| match c {
        b' ' | b'_' => true,
        c => (c >= b'0' && c <= b'9') || (c >= b'A' && c <= b'Z'),
    }) {
        TextValidationOutcome::Ok
    } else {
        TextValidationOutcome::NotOk
    }
}
