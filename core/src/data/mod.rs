//! This modules contains everything needed to access and manipulate DICOM data elements.
//! It comprises a variety of basic data types, such as the DICOM attribute tag, the
//! element header, and element composite types.

pub mod dataset;
pub mod decode;
pub mod encode;
pub mod parser;
pub mod text;
pub mod value;

use data::value::{DicomValueType, PrimitiveValue, Value};
use error::{Error, Result};
use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt;
use std::str::from_utf8;

/// A trait for a data type containing a DICOM header.
pub trait Header {
    /// Retrieve the element's tag as a `(group, element)` tuple.
    fn tag(&self) -> Tag;

    /// Retrieve the value data's length as specified by the data element, in bytes.
    /// According to the standard, the concrete value size may be undefined,
    /// which can be the case for sequence elements or specific primitive values.
    fn len(&self) -> Length;

    /// Check whether this is the header of an item.
    fn is_item(&self) -> bool {
        self.tag() == Tag(0xFFFE, 0xE000)
    }

    /// Check whether this is the header of an item delimiter.
    fn is_item_delimiter(&self) -> bool {
        self.tag() == Tag(0xFFFE, 0xE00D)
    }

    /// Check whether this is the header of a sequence delimiter.
    fn is_sequence_delimiter(&self) -> bool {
        self.tag() == Tag(0xFFFE, 0xE0DD)
    }
}

/// A data type that represents and owns a DICOM data element.
#[derive(Debug, PartialEq, Clone)]
pub struct DataElement<I> {
    header: DataElementHeader,
    value: Value<I>,
}

/// A data type that represents and owns a DICOM data element
/// containing a primitive value.
#[derive(Debug, PartialEq, Clone)]
pub struct PrimitiveDataElement {
    header: DataElementHeader,
    value: PrimitiveValue,
}

impl<I> From<PrimitiveDataElement> for DataElement<I> {
    fn from(o: PrimitiveDataElement) -> Self {
        DataElement {
            header: o.header,
            value: o.value.into(),
        }
    }
}

/// A data type that represents a DICOM data element with
/// a borrowed value.
#[derive(Debug, PartialEq, Clone)]
pub struct DataElementRef<'v, I: 'v> {
    header: DataElementHeader,
    value: &'v Value<I>,
}

/// A data type that represents a DICOM data element with
/// a borrowed primitive value.
#[derive(Debug, PartialEq, Clone)]
pub struct PrimitiveDataElementRef<'v> {
    header: DataElementHeader,
    value: &'v PrimitiveValue,
}

impl<I> Header for DataElement<I> {
    #[inline]
    fn tag(&self) -> Tag {
        self.header.tag()
    }

    #[inline]
    fn len(&self) -> Length {
        self.header.len()
    }
}

impl<'a, I> Header for &'a DataElement<I> {
    #[inline]
    fn tag(&self) -> Tag {
        (**self).tag()
    }

    #[inline]
    fn len(&self) -> Length {
        (**self).len()
    }
}

impl<I> Header for Box<DataElement<I>> {
    #[inline]
    fn tag(&self) -> Tag {
        (**self).tag()
    }

    #[inline]
    fn len(&self) -> Length {
        (**self).len()
    }
}

impl<I> Header for ::std::rc::Rc<DataElement<I>> {
    #[inline]
    fn tag(&self) -> Tag {
        (**self).tag()
    }

    #[inline]
    fn len(&self) -> Length {
        (**self).len()
    }
}

impl<'v, I> Header for DataElementRef<'v, I> {
    #[inline]
    fn tag(&self) -> Tag {
        self.header.tag()
    }

    #[inline]
    fn len(&self) -> Length {
        self.header.len()
    }
}

impl<I> DataElement<I>
where
    I: DicomValueType,
{
    /// Create an empty data element.
    pub fn empty(tag: Tag, vr: VR) -> Self {
        DataElement {
            header: DataElementHeader {
                tag: tag,
                vr: vr,
                len: Length(0),
            },
            value: PrimitiveValue::Empty.into(),
        }
    }

    /// Create a primitive data element from the given parts. This method will not check
    /// whether the value representation is compatible with the given value.
    pub fn new(tag: Tag, vr: VR, value: Value<I>) -> Self {
        DataElement {
            header: DataElementHeader {
                tag: tag,
                vr: vr,
                len: value.size(),
            },
            value: value,
        }
    }

    /// Retrieve the element header.
    pub fn header(&self) -> &DataElementHeader {
        &self.header
    }

    /// Retrieve the data value.
    pub fn value(&self) -> &Value<I> {
        &self.value
    }

    /// Retrieve the value representation, which may be unknown or not
    /// applicable.
    pub fn vr(&self) -> VR {
        self.header.vr()
    }

    /// Retrieve the element's value as a string.
    pub fn as_string(&self) -> Result<Cow<str>> {
        self.value.as_string().map_err(From::from)
    }
}

impl<'v, I> DataElementRef<'v, I>
where
    I: DicomValueType,
{
    /// Create a data element from the given parts. This method will not check
    /// whether the value representation is compatible with the value. Caution
    /// is advised.
    pub fn new(tag: Tag, vr: VR, value: &'v Value<I>) -> Self {
        DataElementRef {
            header: DataElementHeader {
                tag: tag,
                vr: vr,
                len: value.size(),
            },
            value: value,
        }
    }

    /// Retrieves the element's value representation, which can be unknown.
    pub fn vr(&self) -> VR {
        self.header.vr()
    }

    /// Retrieves the DICOM value.
    pub fn value(&self) -> &Value<I> {
        &self.value
    }
}

/// A data structure for a data element header, containing
/// a tag, value representation and specified length.
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct DataElementHeader {
    /// DICOM tag
    pub tag: Tag,
    /// Value Representation
    pub vr: VR,
    /// Element length
    pub len: Length,
}

impl Header for DataElementHeader {
    fn tag(&self) -> Tag {
        self.tag
    }

    fn len(&self) -> Length {
        self.len
    }
}

impl DataElementHeader {
    /// Create a new data element header with the given properties.
    /// This is just a trivial constructor.
    pub fn new<T: Into<Tag>>(tag: T, vr: VR, len: Length) -> DataElementHeader {
        DataElementHeader {
            tag: tag.into(),
            vr,
            len,
        }
    }

    /// Retrieve the element's value representation, which can be unknown.
    pub fn vr(&self) -> VR {
        self.vr
    }
}

impl From<SequenceItemHeader> for DataElementHeader {
    fn from(value: SequenceItemHeader) -> DataElementHeader {
        DataElementHeader {
            tag: value.tag(),
            vr: VR::UN,
            len: value.len(),
        }
    }
}

/// Data type for describing a sequence item data element.
/// If the element represents an item, it will also contain
/// the specified length.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum SequenceItemHeader {
    /// The cursor contains an item.
    Item {
        /// the length of the item in bytes (can be 0xFFFFFFFF if undefined)
        len: Length,
    },
    /// The cursor read an item delimiter.
    /// The element ends here and should not be read any further.
    ItemDelimiter,
    /// The cursor read a sequence delimiter.
    /// The element ends here and should not be read any further.
    SequenceDelimiter,
}

impl SequenceItemHeader {
    /// Create a sequence item header using the element's raw properties.
    /// An error can be raised if the given properties do not relate to a
    /// sequence item, a sequence item delimiter or a sequence delimiter.
    pub fn new<T: Into<Tag>>(tag: T, len: Length) -> Result<SequenceItemHeader> {
        match tag.into() {
            Tag(0xFFFE, 0xE000) => {
                // item
                Ok(SequenceItemHeader::Item { len })
            }
            Tag(0xFFFE, 0xE00D) => {
                // item delimiter
                // delimiters should not have a positive length
                if len != Length(0) {
                    Err(Error::UnexpectedDataValueLength)
                } else {
                    Ok(SequenceItemHeader::ItemDelimiter)
                }
            }
            Tag(0xFFFE, 0xE0DD) => {
                // sequence delimiter
                Ok(SequenceItemHeader::SequenceDelimiter)
            }
            _ => Err(Error::UnexpectedElement),
        }
    }
}

impl Header for SequenceItemHeader {
    fn tag(&self) -> Tag {
        match *self {
            SequenceItemHeader::Item { .. } => Tag(0xFFFE, 0xE000),
            SequenceItemHeader::ItemDelimiter => Tag(0xFFFE, 0xE00D),
            SequenceItemHeader::SequenceDelimiter => Tag(0xFFFE, 0xE0DD),
        }
    }

    fn len(&self) -> Length {
        match *self {
            SequenceItemHeader::Item { len } => len,
            SequenceItemHeader::ItemDelimiter | SequenceItemHeader::SequenceDelimiter => Length(0),
        }
    }
}

/// An enum type for a DICOM value representation.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum VR {
    /// Application Entity
    AE,
    /// Age String
    AS,
    /// Attribute Tag
    AT,
    /// Code String
    CS,
    /// Date
    DA,
    /// Decimal String
    DS,
    /// Date Time
    DT,
    /// Floating Point Single
    FL,
    /// Floating Point Double
    FD,
    /// Integer String
    IS,
    /// Long String
    LO,
    /// Long Text
    LT,
    /// Other Byte
    OB,
    /// Other Double
    OD,
    /// Other Float
    OF,
    /// Other Long
    OL,
    /// Other Word
    OW,
    /// Person Name
    PN,
    /// Short String
    SH,
    /// Signed Long
    SL,
    /// Sequence of Items
    SQ,
    /// Signed Short
    SS,
    /// Short Text
    ST,
    /// Time
    TM,
    /// Unlimited Characters
    UC,
    /// Unique Identifier (UID)
    UI,
    /// Unsigned Long
    UL,
    /// Unknown
    UN,
    /// Universal Resource Identifier or Universal Resource Locator (URI/URL)
    UR,
    /// Unsigned Short
    US,
    /// Unlimited Text
    UT,
}

impl VR {
    /// Obtain the value representation corresponding to the given two bytes.
    /// Each byte should represent an alphabetic character in upper case.
    pub fn from_binary(chars: [u8; 2]) -> Option<VR> {
        from_utf8(chars.as_ref()).ok().and_then(|s| VR::from_str(s))
    }

    /// Obtain the value representation corresponding to the given string.
    /// The string should hold exactly two UTF-8 encoded alphabetic characters
    /// in upper case, otherwise no match is made.
    pub fn from_str(string: &str) -> Option<VR> {
        match string {
            "AE" => Some(VR::AE),
            "AS" => Some(VR::AS),
            "AT" => Some(VR::AT),
            "CS" => Some(VR::CS),
            "DA" => Some(VR::DA),
            "DS" => Some(VR::DS),
            "DT" => Some(VR::DT),
            "FL" => Some(VR::FL),
            "FD" => Some(VR::FD),
            "IS" => Some(VR::IS),
            "LO" => Some(VR::LO),
            "LT" => Some(VR::LT),
            "OB" => Some(VR::OB),
            "OD" => Some(VR::OD),
            "OF" => Some(VR::OF),
            "OL" => Some(VR::OL),
            "OW" => Some(VR::OW),
            "PN" => Some(VR::PN),
            "SH" => Some(VR::SH),
            "SL" => Some(VR::SL),
            "SQ" => Some(VR::SQ),
            "SS" => Some(VR::SS),
            "ST" => Some(VR::ST),
            "TM" => Some(VR::TM),
            "UC" => Some(VR::UC),
            "UI" => Some(VR::UI),
            "UL" => Some(VR::UL),
            "UN" => Some(VR::UN),
            "UR" => Some(VR::UR),
            "US" => Some(VR::US),
            "UT" => Some(VR::UT),
            _ => None,
        }
    }

    /// Retrieve a string representation of this VR.
    pub fn to_string(&self) -> &'static str {
        match *self {
            VR::AE => "AE",
            VR::AS => "AS",
            VR::AT => "AT",
            VR::CS => "CS",
            VR::DA => "DA",
            VR::DS => "DS",
            VR::DT => "DT",
            VR::FL => "FL",
            VR::FD => "FD",
            VR::IS => "IS",
            VR::LO => "LO",
            VR::LT => "LT",
            VR::OB => "OB",
            VR::OD => "OD",
            VR::OF => "OF",
            VR::OL => "OL",
            VR::OW => "OW",
            VR::PN => "PN",
            VR::SH => "SH",
            VR::SL => "SL",
            VR::SQ => "SQ",
            VR::SS => "SS",
            VR::ST => "ST",
            VR::TM => "TM",
            VR::UC => "UC",
            VR::UI => "UI",
            VR::UL => "UL",
            VR::UN => "UN",
            VR::UR => "UR",
            VR::US => "US",
            VR::UT => "UT",
        }
    }

    /// Retrieve a copy of this VR's byte representation.
    /// The function returns two alphabetic characters in upper case.
    pub fn to_bytes(&self) -> [u8; 2] {
        let bytes = self.to_string().as_bytes();
        [bytes[0], bytes[1]]
    }
}

impl fmt::Display for VR {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.to_string())
    }
}

/// Idiomatic alias for a tag's group number.
pub type GroupNumber = u16;
/// Idiomatic alias for a tag's element number.
pub type ElementNumber = u16;

/// The data type for DICOM data element tags.
///
/// Since  types will not have a monomorphized tag, and so will only support
/// a (group, element) pair. For this purpose, `Tag` also provides a method
/// for converting it to a tuple. Both `(u16, u16)` and `[u16; 2]` can be
/// efficiently converted to this type as well.
#[derive(Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Clone, Copy)]
pub struct Tag(pub GroupNumber, pub ElementNumber);

impl Tag {
    /// Getter for the tag's group value.
    #[inline]
    pub fn group(&self) -> GroupNumber {
        self.0
    }

    /// Getter for the tag's element value.
    #[inline]
    pub fn element(&self) -> ElementNumber {
        self.1
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({:04X},{:04X})", self.0, self.1)
    }
}

impl PartialEq<(u16, u16)> for Tag {
    fn eq(&self, other: &(u16, u16)) -> bool {
        self.0 == other.0 && self.1 == other.1
    }
}

impl PartialEq<[u16; 2]> for Tag {
    fn eq(&self, other: &[u16; 2]) -> bool {
        self.0 == other[0] && self.1 == other[1]
    }
}

impl From<(u16, u16)> for Tag {
    #[inline]
    fn from(value: (u16, u16)) -> Tag {
        Tag(value.0, value.1)
    }
}

impl From<[u16; 2]> for Tag {
    #[inline]
    fn from(value: [u16; 2]) -> Tag {
        Tag(value[0], value[1])
    }
}

/// A type for representing data set content length, in bytes.
/// An internal value of `0xFFFF_FFFF` represents an undefined
/// (unspecified) length, which would have to be determined
/// with a traversal based on the content's encoding.
///
/// This also means that numeric comparisons and arithmetic
/// do not function the same way as primitive number types:
///
/// Two length of undefined length are not equal.
///
/// ```
/// # use dicom_core::data::Length;
/// assert_ne!(Length::undefined(), Length::undefined());
/// ```
///
/// Any addition or substraction with at least one undefined
/// length results in an undefined length.
///
/// ```
/// # use dicom_core::data::Length;
/// assert!((Length::defined(64) + Length::undefined()).is_undefined());
/// assert!((Length::undefined() + 8).is_undefined());
/// ```
///
/// Comparing between at least one undefined length is always `false`.
///
/// ```
/// # use dicom_core::data::Length;
/// assert!(Length::defined(16) < Length::defined(64));
/// assert!(!(Length::undefined() < Length::defined(64)));
/// assert!(!(Length::undefined() > Length::defined(64)));
///
/// assert!(!(Length::undefined() < Length::undefined()));
/// assert!(!(Length::undefined() > Length::undefined()));
/// assert!(!(Length::undefined() <= Length::undefined()));
/// assert!(!(Length::undefined() >= Length::undefined()));
/// ```
///
#[derive(Clone, Copy, Hash)]
pub struct Length(pub u32);

const UNDEFINED_LEN: u32 = 0xFFFF_FFFF;

impl Length {
    /// Create a new length value from its internal representation.
    /// This is equivalent to `Length(len)`.
    pub fn new(len: u32) -> Self {
        Length(len)
    }

    /// Create an undefined length.
    pub fn undefined() -> Self {
        Length(UNDEFINED_LEN)
    }

    /// Create a new length value with the given numbe of bytes.
    ///
    /// # Panic
    ///
    /// This function will panic if `len` represents an undefined length.
    pub fn defined(len: u32) -> Self {
        assert_ne!(len, UNDEFINED_LEN);
        Length(len)
    }
}

impl From<u32> for Length {
    fn from(o: u32) -> Self {
        Length(o)
    }
}

impl PartialEq<Length> for Length {
    fn eq(&self, rhs: &Length) -> bool {
        match (self.0, rhs.0) {
            (UNDEFINED_LEN, _) | (_, UNDEFINED_LEN) => false,
            (l1, l2) => l1 == l2,
        }
    }
}

impl PartialOrd<Length> for Length {
    fn partial_cmp(&self, rhs: &Length) -> Option<Ordering> {
        match (self.0, rhs.0) {
            (UNDEFINED_LEN, _) | (_, UNDEFINED_LEN) => None,
            (l1, l2) => Some(l1.cmp(&l2)),
        }
    }
}

impl ::std::ops::Add<Length> for Length {
    type Output = Self;

    fn add(self, rhs: Length) -> Self::Output {
        match (self.0, rhs.0) {
            (UNDEFINED_LEN, _) | (_, UNDEFINED_LEN) => Length::undefined(),
            (l1, l2) => {
                let o = l1 + l2;
                debug_assert!(
                    o != UNDEFINED_LEN,
                    "integer overflow (0xFFFF_FFFF reserved for undefined length)"
                );
                Length(o)
            }
        }
    }
}

impl ::std::ops::Add<i32> for Length {
    type Output = Self;

    fn add(self, rhs: i32) -> Self::Output {
        match self.0 {
            UNDEFINED_LEN => Length::undefined(),
            len => {
                let o = (len as i32 + rhs) as u32;
                debug_assert!(
                    o != UNDEFINED_LEN,
                    "integer overflow (0xFFFF_FFFF reserved for undefined length)"
                );

                Length(o)
            }
        }
    }
}

impl ::std::ops::Sub<Length> for Length {
    type Output = Self;

    fn sub(self, rhs: Length) -> Self::Output {
        match (self.0, rhs.0) {
            (UNDEFINED_LEN, _) | (_, UNDEFINED_LEN) => Length::undefined(),
            (l1, l2) => {
                let o = l1 - l2;
                debug_assert!(
                    o != UNDEFINED_LEN,
                    "integer overflow (0xFFFF_FFFF reserved for undefined length)"
                );

                Length(o)
            }
        }
    }
}

impl ::std::ops::Sub<i32> for Length {
    type Output = Self;

    fn sub(self, rhs: i32) -> Self::Output {
        match self.0 {
            UNDEFINED_LEN => Length::undefined(),
            len => {
                let o = (len as i32 - rhs) as u32;
                debug_assert!(
                    o != UNDEFINED_LEN,
                    "integer overflow (0xFFFF_FFFF reserved for undefined length)"
                );

                Length(o)
            }
        }
    }
}

impl Length {
    /// Check whether this length is undefined.
    #[inline]
    pub fn is_undefined(&self) -> bool {
        self.0 == UNDEFINED_LEN
    }

    /// Check whether this length is well defined.
    #[inline]
    pub fn is_defined(&self) -> bool {
        !self.is_undefined()
    }

    /// Fetch the concrete length value, if available.
    /// Returns `None` if it represents an undefined length.
    #[inline]
    pub fn get(&self) -> Option<u32> {
        match self.0 {
            UNDEFINED_LEN => None,
            v => Some(v),
        }
    }
}

impl fmt::Debug for Length {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0 == UNDEFINED_LEN {
            f.write_str("Length(Undefined)")
        } else {
            f.debug_tuple("Length").field(&self.0).finish()
        }
    }
}

impl fmt::Display for Length {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0 == UNDEFINED_LEN {
            f.write_str("U/L")
        } else {
            write!(f, "{}", &self.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Tag;

    #[test]
    fn tag_from_u16_pair() {
        let t = Tag::from((0x0010u16, 0x0020u16));
        assert_eq!(0x0010u16, t.group());
        assert_eq!(0x0020u16, t.element());
    }

    #[test]
    fn tag_from_u16_array() {
        let t = Tag::from([0x0010u16, 0x0020u16]);
        assert_eq!(0x0010u16, t.group());
        assert_eq!(0x0020u16, t.element());
    }
}
