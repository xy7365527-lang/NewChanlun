//! #1394：受控 compact JSON 写出；I/O 错误直接持有，不经 serde_json 错误箱。
//! 普通 JSON、常规 Serde 容器及 collect_str 与锁定版输出一致。任意 Serialize
//! 的 Error::custom 仍保留 serde_json 完整诊断 owner，此分支未获零箱资格；
//! 底层 io::Error 自身的 owner 也不属于本模块的资格声明。
//!
//! Serializer/Compound/KeySerializer 与字符串扫描适配自 serde_json 1.0.151
//! src/ser.rs（https://github.com/serde-rs/json/tree/v1.0.151/src/ser.rs）。
//! 公开 CompactFormatter/Formatter 提供所有 JSON 字节格式；不复制其数字算法。
//! 上游 MIT 许可全文随适配源保留如下（原 LICENSE-MIT 无独立版权行）。

/*
Permission is hereby granted, free of charge, to any
person obtaining a copy of this software and associated
documentation files (the "Software"), to deal in the
Software without restriction, including without
limitation the rights to use, copy, modify, merge,
publish, distribute, sublicense, and/or sell copies of
the Software, and to permit persons to whom the Software
is furnished to do so, subject to the following
conditions:

The above copyright notice and this permission notice
shall be included in all copies or substantial portions
of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF
ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED
TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A
PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT
SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY
CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION
OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR
IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
DEALINGS IN THE SOFTWARE.
*/

use serde::ser::{self, Impossible, Serialize};
use serde_json::ser::{CharEscape, CompactFormatter, Formatter};
use std::fmt::{self, Display};
use std::io;
use std::num::FpCategory;
use std::{hint, str};

/// 受控写出的真实错误；Custom 是任意用户 Serialize 错误，未承诺零箱。
#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Custom(serde_json::Error),
    Unsupported(&'static str),
    Fmt(fmt::Error),
}
impl Error {
    fn io(error: io::Error) -> Self {
        Self::Io(error)
    }
}
impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => Display::fmt(e, f),
            Self::Custom(e) => Display::fmt(e, f),
            Self::Unsupported(message) => f.write_str(message),
            Self::Fmt(e) => Display::fmt(e, f),
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Custom(e) => Some(e),
            Self::Fmt(e) => Some(e),
            Self::Unsupported(_) => None,
        }
    }
}
impl ser::Error for Error {
    fn custom<T: Display>(message: T) -> Self {
        // Dynamic Display cannot be borrowed into this owner. Keep upstream full text
        // and its parse_line_col behavior, with no truncation or borrowed substitution.
        Self::Custom(<serde_json::Error as ser::Error>::custom(message))
    }
}
type Result<T> = std::result::Result<T, Error>;
macro_rules! tri {
    ($expression:expr) => {
        $expression?
    };
}
struct Serializer<W, F = CompactFormatter> {
    writer: W,
    formatter: F,
}

/// 写出 compact JSON；成功不代表 flush，失败不把 I/O 包进 serde_json::Error。
/// 不支持 serde_json 的私有 arbitrary_precision/raw_value token 扩展。
pub fn write_json<W: io::Write + ?Sized, T: Serialize + ?Sized>(
    writer: &mut W,
    value: &T,
) -> Result<()> {
    value.serialize(&mut Serializer {
        writer,
        formatter: CompactFormatter,
    })
}

impl<'a, W, F> ser::Serializer for &'a mut Serializer<W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Compound<'a, W, F>;
    type SerializeTuple = Compound<'a, W, F>;
    type SerializeTupleStruct = Compound<'a, W, F>;
    type SerializeTupleVariant = Compound<'a, W, F>;
    type SerializeMap = Compound<'a, W, F>;
    type SerializeStruct = Compound<'a, W, F>;
    type SerializeStructVariant = Compound<'a, W, F>;

    #[inline]
    fn serialize_bool(self, value: bool) -> Result<()> {
        self.formatter
            .write_bool(&mut self.writer, value)
            .map_err(Error::io)
    }

    #[inline]
    fn serialize_i8(self, value: i8) -> Result<()> {
        self.formatter
            .write_i8(&mut self.writer, value)
            .map_err(Error::io)
    }

    #[inline]
    fn serialize_i16(self, value: i16) -> Result<()> {
        self.formatter
            .write_i16(&mut self.writer, value)
            .map_err(Error::io)
    }

    #[inline]
    fn serialize_i32(self, value: i32) -> Result<()> {
        self.formatter
            .write_i32(&mut self.writer, value)
            .map_err(Error::io)
    }

    #[inline]
    fn serialize_i64(self, value: i64) -> Result<()> {
        self.formatter
            .write_i64(&mut self.writer, value)
            .map_err(Error::io)
    }

    fn serialize_i128(self, value: i128) -> Result<()> {
        self.formatter
            .write_i128(&mut self.writer, value)
            .map_err(Error::io)
    }

    #[inline]
    fn serialize_u8(self, value: u8) -> Result<()> {
        self.formatter
            .write_u8(&mut self.writer, value)
            .map_err(Error::io)
    }

    #[inline]
    fn serialize_u16(self, value: u16) -> Result<()> {
        self.formatter
            .write_u16(&mut self.writer, value)
            .map_err(Error::io)
    }

    #[inline]
    fn serialize_u32(self, value: u32) -> Result<()> {
        self.formatter
            .write_u32(&mut self.writer, value)
            .map_err(Error::io)
    }

    #[inline]
    fn serialize_u64(self, value: u64) -> Result<()> {
        self.formatter
            .write_u64(&mut self.writer, value)
            .map_err(Error::io)
    }

    fn serialize_u128(self, value: u128) -> Result<()> {
        self.formatter
            .write_u128(&mut self.writer, value)
            .map_err(Error::io)
    }

    #[inline]
    fn serialize_f32(self, value: f32) -> Result<()> {
        match value.classify() {
            FpCategory::Nan | FpCategory::Infinite => self
                .formatter
                .write_null(&mut self.writer)
                .map_err(Error::io),
            _ => self
                .formatter
                .write_f32(&mut self.writer, value)
                .map_err(Error::io),
        }
    }

    #[inline]
    fn serialize_f64(self, value: f64) -> Result<()> {
        match value.classify() {
            FpCategory::Nan | FpCategory::Infinite => self
                .formatter
                .write_null(&mut self.writer)
                .map_err(Error::io),
            _ => self
                .formatter
                .write_f64(&mut self.writer, value)
                .map_err(Error::io),
        }
    }

    #[inline]
    fn serialize_char(self, value: char) -> Result<()> {
        // A char encoded as UTF-8 takes 4 bytes at most.
        let mut buf = [0; 4];
        self.serialize_str(value.encode_utf8(&mut buf))
    }

    #[inline]
    fn serialize_str(self, value: &str) -> Result<()> {
        format_escaped_str(&mut self.writer, &mut self.formatter, value).map_err(Error::io)
    }

    #[inline]
    fn serialize_bytes(self, value: &[u8]) -> Result<()> {
        self.formatter
            .write_byte_array(&mut self.writer, value)
            .map_err(Error::io)
    }

    #[inline]
    fn serialize_unit(self) -> Result<()> {
        self.formatter
            .write_null(&mut self.writer)
            .map_err(Error::io)
    }

    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        self.serialize_str(variant)
    }

    /// Serialize newtypes without an object wrapper.
    #[inline]
    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        tri!(self
            .formatter
            .begin_object(&mut self.writer)
            .map_err(Error::io));
        tri!(self
            .formatter
            .begin_object_key(&mut self.writer, true)
            .map_err(Error::io));
        tri!(self.serialize_str(variant));
        tri!(self
            .formatter
            .end_object_key(&mut self.writer)
            .map_err(Error::io));
        tri!(self
            .formatter
            .begin_object_value(&mut self.writer)
            .map_err(Error::io));
        tri!(value.serialize(&mut *self));
        tri!(self
            .formatter
            .end_object_value(&mut self.writer)
            .map_err(Error::io));
        self.formatter
            .end_object(&mut self.writer)
            .map_err(Error::io)
    }

    #[inline]
    fn serialize_none(self) -> Result<()> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        tri!(self
            .formatter
            .begin_array(&mut self.writer)
            .map_err(Error::io));
        if len == Some(0) {
            tri!(self
                .formatter
                .end_array(&mut self.writer)
                .map_err(Error::io));
            Ok(Compound::Map {
                ser: self,
                state: State::Empty,
            })
        } else {
            Ok(Compound::Map {
                ser: self,
                state: State::First,
            })
        }
    }

    #[inline]
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        tri!(self
            .formatter
            .begin_object(&mut self.writer)
            .map_err(Error::io));
        tri!(self
            .formatter
            .begin_object_key(&mut self.writer, true)
            .map_err(Error::io));
        tri!(self.serialize_str(variant));
        tri!(self
            .formatter
            .end_object_key(&mut self.writer)
            .map_err(Error::io));
        tri!(self
            .formatter
            .begin_object_value(&mut self.writer)
            .map_err(Error::io));
        self.serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        tri!(self
            .formatter
            .begin_object(&mut self.writer)
            .map_err(Error::io));
        if len == Some(0) {
            tri!(self
                .formatter
                .end_object(&mut self.writer)
                .map_err(Error::io));
            Ok(Compound::Map {
                ser: self,
                state: State::Empty,
            })
        } else {
            Ok(Compound::Map {
                ser: self,
                state: State::First,
            })
        }
    }

    #[inline]
    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        match name {
            _ => self.serialize_map(Some(len)),
        }
    }

    #[inline]
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        tri!(self
            .formatter
            .begin_object(&mut self.writer)
            .map_err(Error::io));
        tri!(self
            .formatter
            .begin_object_key(&mut self.writer, true)
            .map_err(Error::io));
        tri!(self.serialize_str(variant));
        tri!(self
            .formatter
            .end_object_key(&mut self.writer)
            .map_err(Error::io));
        tri!(self
            .formatter
            .begin_object_value(&mut self.writer)
            .map_err(Error::io));
        self.serialize_map(Some(len))
    }

    fn collect_str<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Display,
    {
        use self::fmt::Write;

        struct Adapter<'ser, W: 'ser, F: 'ser> {
            writer: &'ser mut W,
            formatter: &'ser mut F,
            error: Option<io::Error>,
        }

        impl<'ser, W, F> Write for Adapter<'ser, W, F>
        where
            W: io::Write,
            F: Formatter,
        {
            fn write_str(&mut self, s: &str) -> fmt::Result {
                if self.error.is_some() {
                    return Err(fmt::Error);
                }
                match format_escaped_str_contents(self.writer, self.formatter, s) {
                    Ok(()) => Ok(()),
                    Err(err) => {
                        self.error = Some(err);
                        Err(fmt::Error)
                    }
                }
            }
        }

        tri!(self
            .formatter
            .begin_string(&mut self.writer)
            .map_err(Error::io));
        let mut adapter = Adapter {
            writer: &mut self.writer,
            formatter: &mut self.formatter,
            error: None,
        };
        let formatted = write!(adapter, "{}", value);
        // Even a Display implementation that ignores write errors cannot hide the first I/O.
        if let Some(error) = adapter.error {
            return Err(Error::Io(error));
        }
        formatted.map_err(Error::Fmt)?;
        self.formatter
            .end_string(&mut self.writer)
            .map_err(Error::io)
    }
}

#[derive(Eq, PartialEq)]
enum State {
    Empty,
    First,
    Rest,
}

enum Compound<'a, W: 'a, F: 'a> {
    Map {
        ser: &'a mut Serializer<W, F>,
        state: State,
    },
}

impl<'a, W, F> ser::SerializeSeq for Compound<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        match self {
            Compound::Map { ser, state } => {
                tri!(ser
                    .formatter
                    .begin_array_value(&mut ser.writer, *state == State::First)
                    .map_err(Error::io));
                *state = State::Rest;
                tri!(value.serialize(&mut **ser));
                ser.formatter
                    .end_array_value(&mut ser.writer)
                    .map_err(Error::io)
            }
        }
    }

    #[inline]
    fn end(self) -> Result<()> {
        match self {
            Compound::Map { ser, state } => match state {
                State::Empty => Ok(()),
                _ => ser.formatter.end_array(&mut ser.writer).map_err(Error::io),
            },
        }
    }
}

impl<'a, W, F> ser::SerializeTuple for Compound<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<()> {
        ser::SerializeSeq::end(self)
    }
}

impl<'a, W, F> ser::SerializeTupleStruct for Compound<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<()> {
        ser::SerializeSeq::end(self)
    }
}

impl<'a, W, F> ser::SerializeTupleVariant for Compound<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        ser::SerializeSeq::serialize_element(self, value)
    }

    #[inline]
    fn end(self) -> Result<()> {
        match self {
            Compound::Map { ser, state } => {
                match state {
                    State::Empty => {}
                    _ => tri!(ser.formatter.end_array(&mut ser.writer).map_err(Error::io)),
                }
                tri!(ser
                    .formatter
                    .end_object_value(&mut ser.writer)
                    .map_err(Error::io));
                ser.formatter.end_object(&mut ser.writer).map_err(Error::io)
            }
        }
    }
}

impl<'a, W, F> ser::SerializeMap for Compound<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        match self {
            Compound::Map { ser, state } => {
                tri!(ser
                    .formatter
                    .begin_object_key(&mut ser.writer, *state == State::First)
                    .map_err(Error::io));
                *state = State::Rest;

                tri!(key.serialize(KeySerializer { ser: *ser }));

                ser.formatter
                    .end_object_key(&mut ser.writer)
                    .map_err(Error::io)
            }
        }
    }

    #[inline]
    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        match self {
            Compound::Map { ser, .. } => {
                tri!(ser
                    .formatter
                    .begin_object_value(&mut ser.writer)
                    .map_err(Error::io));
                tri!(value.serialize(&mut **ser));
                ser.formatter
                    .end_object_value(&mut ser.writer)
                    .map_err(Error::io)
            }
        }
    }

    #[inline]
    fn end(self) -> Result<()> {
        match self {
            Compound::Map { ser, state } => match state {
                State::Empty => Ok(()),
                _ => ser.formatter.end_object(&mut ser.writer).map_err(Error::io),
            },
        }
    }
}

impl<'a, W, F> ser::SerializeStruct for Compound<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        match self {
            Compound::Map { .. } => ser::SerializeMap::serialize_entry(self, key, value),
        }
    }

    #[inline]
    fn end(self) -> Result<()> {
        match self {
            Compound::Map { .. } => ser::SerializeMap::end(self),
        }
    }
}

impl<'a, W, F> ser::SerializeStructVariant for Compound<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        match *self {
            Compound::Map { .. } => ser::SerializeStruct::serialize_field(self, key, value),
        }
    }

    #[inline]
    fn end(self) -> Result<()> {
        match self {
            Compound::Map { ser, state } => {
                match state {
                    State::Empty => {}
                    _ => tri!(ser.formatter.end_object(&mut ser.writer).map_err(Error::io)),
                }
                tri!(ser
                    .formatter
                    .end_object_value(&mut ser.writer)
                    .map_err(Error::io));
                ser.formatter.end_object(&mut ser.writer).map_err(Error::io)
            }
        }
    }
}

struct KeySerializer<'a, W: 'a, F: 'a> {
    ser: &'a mut Serializer<W, F>,
}

fn key_must_be_a_string() -> Error {
    Error::Unsupported("key must be a string")
}

fn float_key_must_be_finite() -> Error {
    Error::Unsupported("float key must be finite")
}

impl<'a, W, F> ser::Serializer for KeySerializer<'a, W, F>
where
    W: io::Write,
    F: Formatter,
{
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_str(self, value: &str) -> Result<()> {
        self.ser.serialize_str(value)
    }

    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        self.ser.serialize_str(variant)
    }

    #[inline]
    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    type SerializeSeq = Impossible<(), Error>;
    type SerializeTuple = Impossible<(), Error>;
    type SerializeTupleStruct = Impossible<(), Error>;
    type SerializeTupleVariant = Impossible<(), Error>;
    type SerializeMap = Impossible<(), Error>;
    type SerializeStruct = Impossible<(), Error>;
    type SerializeStructVariant = Impossible<(), Error>;

    fn serialize_bool(self, value: bool) -> Result<()> {
        tri!(self
            .ser
            .formatter
            .begin_string(&mut self.ser.writer)
            .map_err(Error::io));
        tri!(self
            .ser
            .formatter
            .write_bool(&mut self.ser.writer, value)
            .map_err(Error::io));
        self.ser
            .formatter
            .end_string(&mut self.ser.writer)
            .map_err(Error::io)
    }

    fn serialize_i8(self, value: i8) -> Result<()> {
        tri!(self
            .ser
            .formatter
            .begin_string(&mut self.ser.writer)
            .map_err(Error::io));
        tri!(self
            .ser
            .formatter
            .write_i8(&mut self.ser.writer, value)
            .map_err(Error::io));
        self.ser
            .formatter
            .end_string(&mut self.ser.writer)
            .map_err(Error::io)
    }

    fn serialize_i16(self, value: i16) -> Result<()> {
        tri!(self
            .ser
            .formatter
            .begin_string(&mut self.ser.writer)
            .map_err(Error::io));
        tri!(self
            .ser
            .formatter
            .write_i16(&mut self.ser.writer, value)
            .map_err(Error::io));
        self.ser
            .formatter
            .end_string(&mut self.ser.writer)
            .map_err(Error::io)
    }

    fn serialize_i32(self, value: i32) -> Result<()> {
        tri!(self
            .ser
            .formatter
            .begin_string(&mut self.ser.writer)
            .map_err(Error::io));
        tri!(self
            .ser
            .formatter
            .write_i32(&mut self.ser.writer, value)
            .map_err(Error::io));
        self.ser
            .formatter
            .end_string(&mut self.ser.writer)
            .map_err(Error::io)
    }

    fn serialize_i64(self, value: i64) -> Result<()> {
        tri!(self
            .ser
            .formatter
            .begin_string(&mut self.ser.writer)
            .map_err(Error::io));
        tri!(self
            .ser
            .formatter
            .write_i64(&mut self.ser.writer, value)
            .map_err(Error::io));
        self.ser
            .formatter
            .end_string(&mut self.ser.writer)
            .map_err(Error::io)
    }

    fn serialize_i128(self, value: i128) -> Result<()> {
        tri!(self
            .ser
            .formatter
            .begin_string(&mut self.ser.writer)
            .map_err(Error::io));
        tri!(self
            .ser
            .formatter
            .write_i128(&mut self.ser.writer, value)
            .map_err(Error::io));
        self.ser
            .formatter
            .end_string(&mut self.ser.writer)
            .map_err(Error::io)
    }

    fn serialize_u8(self, value: u8) -> Result<()> {
        tri!(self
            .ser
            .formatter
            .begin_string(&mut self.ser.writer)
            .map_err(Error::io));
        tri!(self
            .ser
            .formatter
            .write_u8(&mut self.ser.writer, value)
            .map_err(Error::io));
        self.ser
            .formatter
            .end_string(&mut self.ser.writer)
            .map_err(Error::io)
    }

    fn serialize_u16(self, value: u16) -> Result<()> {
        tri!(self
            .ser
            .formatter
            .begin_string(&mut self.ser.writer)
            .map_err(Error::io));
        tri!(self
            .ser
            .formatter
            .write_u16(&mut self.ser.writer, value)
            .map_err(Error::io));
        self.ser
            .formatter
            .end_string(&mut self.ser.writer)
            .map_err(Error::io)
    }

    fn serialize_u32(self, value: u32) -> Result<()> {
        tri!(self
            .ser
            .formatter
            .begin_string(&mut self.ser.writer)
            .map_err(Error::io));
        tri!(self
            .ser
            .formatter
            .write_u32(&mut self.ser.writer, value)
            .map_err(Error::io));
        self.ser
            .formatter
            .end_string(&mut self.ser.writer)
            .map_err(Error::io)
    }

    fn serialize_u64(self, value: u64) -> Result<()> {
        tri!(self
            .ser
            .formatter
            .begin_string(&mut self.ser.writer)
            .map_err(Error::io));
        tri!(self
            .ser
            .formatter
            .write_u64(&mut self.ser.writer, value)
            .map_err(Error::io));
        self.ser
            .formatter
            .end_string(&mut self.ser.writer)
            .map_err(Error::io)
    }

    fn serialize_u128(self, value: u128) -> Result<()> {
        tri!(self
            .ser
            .formatter
            .begin_string(&mut self.ser.writer)
            .map_err(Error::io));
        tri!(self
            .ser
            .formatter
            .write_u128(&mut self.ser.writer, value)
            .map_err(Error::io));
        self.ser
            .formatter
            .end_string(&mut self.ser.writer)
            .map_err(Error::io)
    }

    fn serialize_f32(self, value: f32) -> Result<()> {
        if !value.is_finite() {
            return Err(float_key_must_be_finite());
        }

        tri!(self
            .ser
            .formatter
            .begin_string(&mut self.ser.writer)
            .map_err(Error::io));
        tri!(self
            .ser
            .formatter
            .write_f32(&mut self.ser.writer, value)
            .map_err(Error::io));
        self.ser
            .formatter
            .end_string(&mut self.ser.writer)
            .map_err(Error::io)
    }

    fn serialize_f64(self, value: f64) -> Result<()> {
        if !value.is_finite() {
            return Err(float_key_must_be_finite());
        }

        tri!(self
            .ser
            .formatter
            .begin_string(&mut self.ser.writer)
            .map_err(Error::io));
        tri!(self
            .ser
            .formatter
            .write_f64(&mut self.ser.writer, value)
            .map_err(Error::io));
        self.ser
            .formatter
            .end_string(&mut self.ser.writer)
            .map_err(Error::io)
    }

    fn serialize_char(self, value: char) -> Result<()> {
        self.ser.serialize_str(value.encode_utf8(&mut [0u8; 4]))
    }

    fn serialize_bytes(self, _value: &[u8]) -> Result<()> {
        Err(key_must_be_a_string())
    }

    fn serialize_unit(self) -> Result<()> {
        Err(key_must_be_a_string())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        Err(key_must_be_a_string())
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(key_must_be_a_string())
    }

    fn serialize_none(self) -> Result<()> {
        Err(key_must_be_a_string())
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Err(key_must_be_a_string())
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Err(key_must_be_a_string())
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Err(key_must_be_a_string())
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(key_must_be_a_string())
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(key_must_be_a_string())
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Err(key_must_be_a_string())
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(key_must_be_a_string())
    }

    fn collect_str<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Display,
    {
        self.ser.collect_str(value)
    }
}

fn format_escaped_str<W, F>(writer: &mut W, formatter: &mut F, value: &str) -> io::Result<()>
where
    W: ?Sized + io::Write,
    F: ?Sized + Formatter,
{
    tri!(formatter.begin_string(writer));
    tri!(format_escaped_str_contents(writer, formatter, value));
    formatter.end_string(writer)
}

fn format_escaped_str_contents<W, F>(
    writer: &mut W,
    formatter: &mut F,
    value: &str,
) -> io::Result<()>
where
    W: ?Sized + io::Write,
    F: ?Sized + Formatter,
{
    let mut bytes = value.as_bytes();

    let mut i = 0;
    while i < bytes.len() {
        let (string_run, rest) = bytes.split_at(i);
        let (&byte, rest) = rest.split_first().unwrap();

        let escape = ESCAPE[byte as usize];

        i += 1;
        if escape == 0 {
            continue;
        }

        bytes = rest;
        i = 0;

        // Safety: string_run is a valid utf8 string, since we only split on ascii sequences
        let string_run = unsafe { str::from_utf8_unchecked(string_run) };
        if !string_run.is_empty() {
            tri!(formatter.write_string_fragment(writer, string_run));
        }

        let char_escape = match escape {
            self::BB => CharEscape::Backspace,
            self::TT => CharEscape::Tab,
            self::NN => CharEscape::LineFeed,
            self::FF => CharEscape::FormFeed,
            self::RR => CharEscape::CarriageReturn,
            self::QU => CharEscape::Quote,
            self::BS => CharEscape::ReverseSolidus,
            self::UU => CharEscape::AsciiControl(byte),
            // Safety: the escape table does not contain any other type of character.
            _ => unsafe { hint::unreachable_unchecked() },
        };
        tri!(formatter.write_char_escape(writer, char_escape));
    }

    // Safety: bytes is a valid utf8 string, since we only split on ascii sequences
    let string_run = unsafe { str::from_utf8_unchecked(bytes) };
    if string_run.is_empty() {
        return Ok(());
    }

    formatter.write_string_fragment(writer, string_run)
}

const BB: u8 = b'b'; // \x08
const TT: u8 = b't'; // \x09
const NN: u8 = b'n'; // \x0A
const FF: u8 = b'f'; // \x0C
const RR: u8 = b'r'; // \x0D
const QU: u8 = b'"'; // \x22
const BS: u8 = b'\\'; // \x5C
const UU: u8 = b'u'; // \x00...\x1F except the ones above
const __: u8 = 0;

// Lookup table of escape sequences. A value of b'x' at index i means that byte
// i is escaped as "\x" in JSON. A value of 0 means that byte i is not escaped.
static ESCAPE: [u8; 256] = [
    //   1   2   3   4   5   6   7   8   9   A   B   C   D   E   F
    UU, UU, UU, UU, UU, UU, UU, UU, BB, TT, NN, UU, FF, RR, UU, UU, // 0
    UU, UU, UU, UU, UU, UU, UU, UU, UU, UU, UU, UU, UU, UU, UU, UU, // 1
    __, __, QU, __, __, __, __, __, __, __, __, __, __, __, __, __, // 2
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 3
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 4
    __, __, __, __, __, __, __, __, __, __, __, __, BS, __, __, __, // 5
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 6
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 7
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 8
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // 9
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // A
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // B
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // C
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // D
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // E
    __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, __, // F
];

#[cfg(test)]
mod tests {
    use super::*;
    use serde::ser::SerializeSeq;

    fn same<T: Serialize + ?Sized>(value: &T) -> Vec<u8> {
        let old = serde_json::to_vec(value).unwrap();
        let mut current = Vec::new();
        write_json(&mut current, value).unwrap();
        assert_eq!(current, old);
        current
    }

    struct Segments;
    impl Display for Segments {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("Q:")?;
            f.write_str("重/γ\"\\")?;
            f.write_str("\u{0000}\n\t")?;
            write!(f, "{}:{}", i64::MIN, u64::MAX)?;
            f.write_str("🧬\u{2028}\u{2029}")
        }
    }
    struct Collected<T>(T);
    impl<T: Display> Serialize for Collected<T> {
        fn serialize<S: ser::Serializer>(
            &self,
            serializer: S,
        ) -> std::result::Result<S::Ok, S::Error> {
            serializer.collect_str(&self.0)
        }
    }

    #[test]
    fn unicode_controls_escapes_and_segments_match_locked_oracle() {
        same("");
        same("\"\\\n\r\t\u{0008}\u{000c}/");
        same("中文/γ/🧬/\u{2028}/\u{2029}/\u{feff}/\u{007f}");
        let controls: String = (0..=127).map(char::from).collect();
        same(&controls);
        // All Unicode scalar values, in short batches, including multibyte runs
        // adjoining every ASCII escaping boundary.
        for start in (0..=0x10ffff).step_by(4096) {
            let mut text = String::new();
            for scalar in start..=(start + 4095).min(0x10ffff) {
                if let Some(c) = char::from_u32(scalar) {
                    text.push(c);
                }
            }
            text.push_str("\"\n\\\u{0000}");
            same(&text);
        }
        same(&Collected(Segments));
    }

    #[test]
    fn primitive_container_and_enum_bytes_match_locked_oracle() {
        same(&());
        same(&true);
        same(&false);
        same(&i8::MIN);
        same(&i16::MIN);
        same(&i32::MIN);
        same(&i64::MIN);
        same(&i128::MIN);
        same(&u8::MAX);
        same(&u16::MAX);
        same(&u32::MAX);
        same(&u64::MAX);
        same(&u128::MAX);
        for number in [
            0.0f64,
            -0.0,
            1.0,
            -1.25,
            f64::MIN_POSITIVE,
            f64::from_bits(1),
            f64::MAX,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
        ] {
            same(&number);
        }
        for number in [
            0.0f32,
            -0.0,
            1.0,
            -1.25,
            f32::MIN_POSITIVE,
            f32::from_bits(1),
            f32::MAX,
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
        ] {
            same(&number);
        }
        same(&'🧬');
        same(&None::<bool>);
        same(&Some("值"));
        same(&("tuple", true, -3i64));
        same(
            &serde_json::json!({"array": [null, true, 7, "\"\\\n", {}, []], "nested": {"中文": "🧬", "number": -3.25}}),
        );
        #[derive(serde::Serialize)]
        struct Record<'a> {
            name: &'a str,
            tuple: (i64, bool),
        }
        same(&Record {
            name: "record\n",
            tuple: (-7, false),
        });
        #[derive(serde::Serialize)]
        enum Variant {
            Unit,
            Newtype(u64),
            Tuple(bool, &'static str),
            Struct { key: &'static str },
            EmptyTuple(),
            EmptyStruct {},
        }
        for value in [
            Variant::Unit,
            Variant::Newtype(9),
            Variant::Tuple(true, "\n"),
            Variant::Struct { key: "值" },
            Variant::EmptyTuple(),
            Variant::EmptyStruct {},
        ] {
            same(&value);
        }
        let keys = std::collections::BTreeMap::from([(i64::MIN, "min"), (i64::MAX, "max")]);
        same(&keys);
        struct Bytes;
        impl Serialize for Bytes {
            fn serialize<S: ser::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
                s.serialize_bytes(&[0, 7, 255])
            }
        }
        same(&Bytes);
    }

    #[derive(Clone, Copy)]
    enum Failure {
        Error,
        Zero,
        Interrupted,
    }
    struct FaultWriter {
        bytes: Vec<u8>,
        offset: usize,
        chunk: usize,
        failure: Failure,
        failed: bool,
        interrupted: bool,
        after_failure: usize,
    }
    impl FaultWriter {
        fn new(offset: usize, chunk: usize, failure: Failure) -> Self {
            Self {
                bytes: Vec::new(),
                offset,
                chunk,
                failure,
                failed: false,
                interrupted: false,
                after_failure: 0,
            }
        }
    }
    impl io::Write for FaultWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.failed {
                self.after_failure += 1;
                return Err(io::ErrorKind::BrokenPipe.into());
            }
            if self.bytes.len() == self.offset {
                match self.failure {
                    Failure::Error => {
                        self.failed = true;
                        return Err(io::Error::new(
                            io::ErrorKind::BrokenPipe,
                            "原始 I/O 故障\n无删改",
                        ));
                    }
                    Failure::Zero => {
                        self.failed = true;
                        return Ok(0);
                    }
                    Failure::Interrupted if !self.interrupted => {
                        self.interrupted = true;
                        return Err(io::ErrorKind::Interrupted.into());
                    }
                    Failure::Interrupted => {}
                }
            }
            let remaining = if self.interrupted {
                usize::MAX
            } else {
                self.offset - self.bytes.len()
            };
            let n = bytes.len().min(self.chunk).min(remaining);
            self.bytes.extend_from_slice(&bytes[..n]);
            Ok(n)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    fn all_failure_offsets<T: Serialize + ?Sized>(value: &T) {
        let expected = same(value);
        for chunk in [1, 3, usize::MAX] {
            for offset in 0..expected.len() {
                for failure in [Failure::Error, Failure::Zero] {
                    let mut old = FaultWriter::new(offset, chunk, failure);
                    let old_error = serde_json::to_writer(&mut old, value).unwrap_err();
                    let mut current = FaultWriter::new(offset, chunk, failure);
                    let error = write_json(&mut current, value).unwrap_err();
                    let Error::Io(error) = error else {
                        panic!("I/O was wrapped into a different error: {error}");
                    };
                    assert_eq!(error.kind(), old_error.io_error_kind().unwrap());
                    assert_eq!(error.to_string(), old_error.to_string());
                    assert_eq!(current.bytes, expected[..offset]);
                    assert_eq!(current.bytes, old.bytes);
                    assert_eq!(current.after_failure, 0);
                    assert_eq!(old.after_failure, 0);
                }
                // write_all must retry Interrupted at exactly the interrupted offset.
                let mut old = FaultWriter::new(offset, chunk, Failure::Interrupted);
                serde_json::to_writer(&mut old, value).unwrap();
                let mut current = FaultWriter::new(offset, chunk, Failure::Interrupted);
                write_json(&mut current, value).unwrap();
                assert!(current.interrupted);
                assert_eq!(current.bytes, old.bytes);
                assert_eq!(current.bytes, expected);
            }
        }
    }

    #[test]
    fn inline_io_stops_at_every_offset_including_partial_zero_and_interrupted() {
        all_failure_offsets(
            &serde_json::json!({"a": [null, false, i64::MIN, u64::MAX, -0.25, "重\"\u{0001}\n🧬"], "empty": {}, "tail": "escaped\\"}),
        );
        all_failure_offsets(&Collected(Segments));
    }

    struct BrokenDisplay;
    impl Display for BrokenDisplay {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("partial")?;
            Err(fmt::Error)
        }
    }
    struct IgnoredDisplayFailure;
    impl Display for IgnoredDisplayFailure {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let _ = f.write_str("first");
            let _ = f.write_str("second");
            Ok(())
        }
    }
    #[test]
    fn collect_str_keeps_first_io_and_reports_bare_fmt_without_panicking() {
        let mut bytes = Vec::new();
        assert!(matches!(
            write_json(&mut bytes, &Collected(BrokenDisplay)),
            Err(Error::Fmt(_))
        ));
        assert_eq!(bytes, b"\"partial");
        let mut writer = FaultWriter::new(2, 1, Failure::Error);
        let error = write_json(&mut writer, &Collected(IgnoredDisplayFailure)).unwrap_err();
        assert!(matches!(error, Error::Io(_)));
        assert_eq!(error.to_string(), "原始 I/O 故障\n无删改");
        assert_eq!(writer.bytes, b"\"f");
        assert_eq!(writer.after_failure, 0);
    }

    #[test]
    fn custom_retains_full_upstream_text_and_line_column_owner() {
        struct Custom;
        impl Serialize for Custom {
            fn serialize<S: ser::Serializer>(&self, _: S) -> std::result::Result<S::Ok, S::Error> {
                Err(ser::Error::custom(
                    "完整动态错误：重🧬 at line 12 column 34",
                ))
            }
        }
        let old = serde_json::to_vec(&Custom).unwrap_err();
        let error = write_json(&mut Vec::new(), &Custom).unwrap_err();
        let Error::Custom(error) = error else {
            panic!("custom owner lost");
        };
        assert_eq!(error.to_string(), old.to_string());
        assert_eq!((error.line(), error.column()), (old.line(), old.column()));
        assert_eq!((error.line(), error.column()), (12, 34));
    }

    #[test]
    fn borrowed_json_view_and_nested_ref_extent_match_locked_oracle() {
        use crate::s_content::capture_allocation::audit_json::JsonView;
        let value =
            serde_json::json!({"z": [null, false, -12, 9007199254740993u64, 0.25], "重": "\"\n🧬"});
        let view = JsonView::from(&value);
        same(&view);
        let fields = [
            ("message_id", JsonView::String("m\n")),
            ("payload_hash", JsonView::String("h")),
            ("source_epoch", JsonView::String("1")),
            ("source_namespace", JsonView::String("ns")),
        ];
        same(&JsonView::object_fields(&fields).unwrap());
        let diagnostic = crate::s_content::RefDiagnostic::Extent(view);
        same(&Collected(diagnostic));
        struct Extent<'a>(JsonView<'a>);
        impl Display for Extent<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                Display::fmt(&crate::s_content::RefDiagnostic::Extent(self.0), f)
            }
        }
        all_failure_offsets(&Collected(Extent(view)));
    }

    #[test]
    fn sequence_value_custom_failure_does_not_write_closing_delimiter() {
        struct Fails;
        impl Serialize for Fails {
            fn serialize<S: ser::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
                let mut seq = s.serialize_seq(Some(2))?;
                seq.serialize_element(&true)?;
                seq.serialize_element(&Collected(BrokenDisplay))?;
                seq.end()
            }
        }
        let mut bytes = Vec::new();
        assert!(matches!(write_json(&mut bytes, &Fails), Err(Error::Fmt(_))));
        assert_eq!(bytes, b"[true,\"partial");
    }
}
