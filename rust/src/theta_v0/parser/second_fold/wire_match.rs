//! #1419：以 K1 的权威序列化视图核验借用 wire，不生成或复制 JSON 树。
//! 只接受该视图使用的整数、文本、bool/null、数组、struct 与 unit enum；新增 wire 类型须显式补齐。
use serde::ser::{self, Serialize};
use serde_json::Value;
use std::fmt::{self, Write};

#[derive(Debug)]
struct Mismatch;
impl fmt::Display for Mismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("K1 wire 不匹配")
    }
}
impl std::error::Error for Mismatch {}
impl ser::Error for Mismatch {
    fn custom<T: fmt::Display>(_: T) -> Self {
        Self
    }
}
type Result<T = ()> = std::result::Result<T, Mismatch>;

fn require(ok: bool) -> Result {
    if ok {
        Ok(())
    } else {
        Err(Mismatch)
    }
}

// K1 的整数最多 20 位、数字型摘要文本 16 位；更长的数字不可能匹配这些预期字段。
// 满则拒绝，不分配兜底。数字统一为十进制文本，与旧 normalized 的规则相同。
struct NumberText {
    bytes: [u8; 128],
    len: usize,
}
impl fmt::Write for NumberText {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let end = self.len.checked_add(s.len()).ok_or(fmt::Error)?;
        let dest = self.bytes.get_mut(self.len..end).ok_or(fmt::Error)?;
        dest.copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}
impl NumberText {
    fn new(value: impl fmt::Display) -> Result<Self> {
        let mut out = Self {
            bytes: [0; 128],
            len: 0,
        };
        write!(&mut out, "{value}").map_err(|_| Mismatch)?;
        Ok(out)
    }
    fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.len]).expect("数字格式为 ASCII")
    }
}

pub(super) fn matches<T: Serialize + ?Sized>(value: &T, wire: &Value) -> bool {
    value.serialize(Matcher(wire)).is_ok()
}

struct Matcher<'a>(&'a Value);
struct Array<'a> {
    values: &'a [Value],
    index: usize,
}
impl Array<'_> {
    fn item<T: Serialize + ?Sized>(&mut self, value: &T) -> Result {
        let wire = self.values.get(self.index).ok_or(Mismatch)?;
        value.serialize(Matcher(wire))?;
        self.index += 1;
        Ok(())
    }
    fn finish(self) -> Result {
        require(self.index == self.values.len())
    }
}
impl ser::SerializeSeq for Array<'_> {
    type Ok = ();
    type Error = Mismatch;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result {
        self.item(value)
    }
    fn end(self) -> Result {
        self.finish()
    }
}
impl ser::SerializeTuple for Array<'_> {
    type Ok = ();
    type Error = Mismatch;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result {
        self.item(value)
    }
    fn end(self) -> Result {
        self.finish()
    }
}

struct Object<'a> {
    fields: &'a serde_json::Map<String, Value>,
    remaining: usize,
}
impl ser::SerializeStruct for Object<'_> {
    type Ok = ();
    type Error = Mismatch;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, key: &'static str, value: &T) -> Result {
        let wire = self.fields.get(key).ok_or(Mismatch)?;
        value.serialize(Matcher(wire))?;
        self.remaining = self.remaining.checked_sub(1).ok_or(Mismatch)?;
        Ok(())
    }
    fn end(self) -> Result {
        require(self.remaining == 0)
    }
}

macro_rules! integer {
    ($($method:ident($ty:ty)),* $(,)?) => {$(
        fn $method(self, value: $ty) -> Result {
            self.serialize_str(NumberText::new(value)?.as_str())
        }
    )*};
}
impl<'a> ser::Serializer for Matcher<'a> {
    type Ok = ();
    type Error = Mismatch;
    type SerializeSeq = Array<'a>;
    type SerializeTuple = Array<'a>;
    type SerializeStruct = Object<'a>;
    type SerializeTupleStruct = ser::Impossible<(), Mismatch>;
    type SerializeTupleVariant = ser::Impossible<(), Mismatch>;
    type SerializeMap = ser::Impossible<(), Mismatch>;
    type SerializeStructVariant = ser::Impossible<(), Mismatch>;

    integer!(
        serialize_i8(i8),
        serialize_i16(i16),
        serialize_i32(i32),
        serialize_i64(i64),
        serialize_u8(u8),
        serialize_u16(u16),
        serialize_u32(u32),
        serialize_u64(u64)
    );
    fn serialize_bool(self, value: bool) -> Result {
        require(self.0.as_bool() == Some(value))
    }
    fn serialize_str(self, value: &str) -> Result {
        match self.0 {
            Value::String(text) => require(text == value),
            Value::Number(number) => require(NumberText::new(number)?.as_str() == value),
            _ => Err(Mismatch),
        }
    }
    fn serialize_none(self) -> Result {
        require(self.0.is_null())
    }
    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result {
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result {
        self.serialize_none()
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result {
        self.serialize_unit()
    }
    fn serialize_unit_variant(self, _: &'static str, _: u32, name: &'static str) -> Result {
        self.serialize_str(name)
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(self, _: &'static str, value: &T) -> Result {
        value.serialize(self)
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        let values = self.0.as_array().ok_or(Mismatch)?;
        require(len.map_or(true, |len| len == values.len()))?;
        Ok(Array { values, index: 0 })
    }
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }
    fn serialize_struct(self, _: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        let fields = self.0.as_object().ok_or(Mismatch)?;
        require(fields.len() == len)?;
        Ok(Object {
            fields,
            remaining: len,
        })
    }
    fn serialize_f32(self, _: f32) -> Result {
        Err(Mismatch)
    }
    fn serialize_f64(self, _: f64) -> Result {
        Err(Mismatch)
    }
    fn serialize_char(self, _: char) -> Result {
        Err(Mismatch)
    }
    fn serialize_bytes(self, _: &[u8]) -> Result {
        Err(Mismatch)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: &T,
    ) -> Result {
        Err(Mismatch)
    }
    fn serialize_tuple_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Err(Mismatch)
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(Mismatch)
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap> {
        Err(Mismatch)
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(Mismatch)
    }
    fn collect_str<T: fmt::Display + ?Sized>(self, _: &T) -> Result {
        Err(Mismatch)
    }
}
