use std::fmt;

use serde::ser::{self, Serialize};

/// Why a value has no canonical bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalError {
    Unsized,
    Custom(String),
}

impl fmt::Display for CanonicalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsized => {
                f.write_str("a sequence or map of unknown length has no canonical form")
            }
            Self::Custom(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for CanonicalError {}

impl ser::Error for CanonicalError {
    fn custom<T: fmt::Display>(message: T) -> Self {
        Self::Custom(message.to_string())
    }
}

/// Appends the canonical bytes of `value` to `out`; fails on a sequence or map of unknown length.
pub fn canonical<T: Serialize + ?Sized>(
    value: &T,
    out: &mut Vec<u8>,
) -> Result<(), CanonicalError> {
    value.serialize(&mut Writer(out))
}

struct Writer<'a>(&'a mut Vec<u8>);

type Done = Result<(), CanonicalError>;

impl Writer<'_> {
    fn len(&mut self, len: usize) -> Done {
        let len = u64::try_from(len).map_err(|_| CanonicalError::Unsized)?;
        self.0.extend_from_slice(&len.to_le_bytes());
        Ok(())
    }

    fn bytes(&mut self, bytes: &[u8]) -> Done {
        self.len(bytes.len())?;
        self.0.extend_from_slice(bytes);
        Ok(())
    }
}

impl ser::Serializer for &mut Writer<'_> {
    type Ok = ();
    type Error = CanonicalError;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Done {
        self.0.push(u8::from(v));
        Ok(())
    }
    fn serialize_i8(self, v: i8) -> Done {
        self.0.extend_from_slice(&v.to_le_bytes());
        Ok(())
    }
    fn serialize_i16(self, v: i16) -> Done {
        self.0.extend_from_slice(&v.to_le_bytes());
        Ok(())
    }
    fn serialize_i32(self, v: i32) -> Done {
        self.0.extend_from_slice(&v.to_le_bytes());
        Ok(())
    }
    fn serialize_i64(self, v: i64) -> Done {
        self.0.extend_from_slice(&v.to_le_bytes());
        Ok(())
    }
    fn serialize_i128(self, v: i128) -> Done {
        self.0.extend_from_slice(&v.to_le_bytes());
        Ok(())
    }
    fn serialize_u8(self, v: u8) -> Done {
        self.0.push(v);
        Ok(())
    }
    fn serialize_u16(self, v: u16) -> Done {
        self.0.extend_from_slice(&v.to_le_bytes());
        Ok(())
    }
    fn serialize_u32(self, v: u32) -> Done {
        self.0.extend_from_slice(&v.to_le_bytes());
        Ok(())
    }
    fn serialize_u64(self, v: u64) -> Done {
        self.0.extend_from_slice(&v.to_le_bytes());
        Ok(())
    }
    fn serialize_u128(self, v: u128) -> Done {
        self.0.extend_from_slice(&v.to_le_bytes());
        Ok(())
    }
    fn serialize_f32(self, v: f32) -> Done {
        self.serialize_u32(v.to_bits())
    }
    fn serialize_f64(self, v: f64) -> Done {
        self.serialize_u64(v.to_bits())
    }
    fn serialize_char(self, v: char) -> Done {
        self.serialize_u32(u32::from(v))
    }
    fn serialize_str(self, v: &str) -> Done {
        self.bytes(v.as_bytes())
    }
    fn serialize_bytes(self, v: &[u8]) -> Done {
        self.bytes(v)
    }
    fn serialize_none(self) -> Done {
        self.0.push(0);
        Ok(())
    }
    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Done {
        self.0.push(1);
        value.serialize(self)
    }
    fn serialize_unit(self) -> Done {
        Ok(())
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Done {
        Ok(())
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
    ) -> Done {
        self.serialize_str(variant)
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Done {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        value: &T,
    ) -> Done {
        self.serialize_str(variant)?;
        value.serialize(self)
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Self, CanonicalError> {
        self.len(len.ok_or(CanonicalError::Unsized)?)?;
        Ok(self)
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self, CanonicalError> {
        Ok(self)
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self, CanonicalError> {
        Ok(self)
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self, CanonicalError> {
        self.serialize_str(variant)?;
        Ok(self)
    }
    fn serialize_map(self, len: Option<usize>) -> Result<Self, CanonicalError> {
        self.serialize_seq(len)
    }
    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self, CanonicalError> {
        Ok(self)
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self, CanonicalError> {
        self.serialize_str(variant)?;
        Ok(self)
    }
}

impl ser::SerializeSeq for &mut Writer<'_> {
    type Ok = ();
    type Error = CanonicalError;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Done {
        value.serialize(&mut **self)
    }
    fn end(self) -> Done {
        Ok(())
    }
}

impl ser::SerializeTuple for &mut Writer<'_> {
    type Ok = ();
    type Error = CanonicalError;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Done {
        value.serialize(&mut **self)
    }
    fn end(self) -> Done {
        Ok(())
    }
}

impl ser::SerializeTupleStruct for &mut Writer<'_> {
    type Ok = ();
    type Error = CanonicalError;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Done {
        value.serialize(&mut **self)
    }
    fn end(self) -> Done {
        Ok(())
    }
}

impl ser::SerializeTupleVariant for &mut Writer<'_> {
    type Ok = ();
    type Error = CanonicalError;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Done {
        value.serialize(&mut **self)
    }
    fn end(self) -> Done {
        Ok(())
    }
}

impl ser::SerializeMap for &mut Writer<'_> {
    type Ok = ();
    type Error = CanonicalError;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Done {
        key.serialize(&mut **self)
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Done {
        value.serialize(&mut **self)
    }
    fn end(self) -> Done {
        Ok(())
    }
}

impl ser::SerializeStruct for &mut Writer<'_> {
    type Ok = ();
    type Error = CanonicalError;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, _key: &'static str, value: &T) -> Done {
        value.serialize(&mut **self)
    }
    fn end(self) -> Done {
        Ok(())
    }
}

impl ser::SerializeStructVariant for &mut Writer<'_> {
    type Ok = ();
    type Error = CanonicalError;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, _key: &'static str, value: &T) -> Done {
        value.serialize(&mut **self)
    }
    fn end(self) -> Done {
        Ok(())
    }
}
