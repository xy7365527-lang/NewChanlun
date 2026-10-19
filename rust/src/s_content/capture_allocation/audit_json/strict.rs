//! #1394 Q Owned slice grammar, ported from serde_json 1.0.151 (Cargo.lock).
//! Only default/std, no arbitrary_precision/float_roundtrip/raw_value/unbounded_depth.
//! Branch correspondence: de.rs deserialize_any/parse_ident/SeqAccess/MapAccess/end_seq/
//! end_map/check_recursion; read.rs SliceRead::parse_str_bytes/decode_hex_escape and
//! validated parse_escape/parse_unicode_escape; error.rs make_error/parse_line_col/
//! fix_position/Display. Slice positioning counts bytes, including invalid UTF8.
//! SWAR string scanning is replaced by a byte loop with identical stop/consume points.
//! The StrictOwnerSeed Some visitor's construction order is retained: fresh scalar/
//! container owners, owned key before duplicate check, child before parent copy.
//! No serde Error, Value tree, fallback reparse, or global allocator redirection.
//! Upstream: https://github.com/serde-rs/json/tree/v1.0.151/src
//! Copyright serde_json contributors. Licensed under MIT (permission below).
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
use super::{number, CodecError, CodecVec, JsonView, OwnedBudget, OwnedJson, TextOwner};
use std::fmt;

#[derive(Debug)]
pub enum ParseFailure {
    Syntax(Diagnostic),
    Allocation(CodecError),
}
impl From<CodecError> for ParseFailure {
    fn from(error: CodecError) -> Self {
        Self::Allocation(error)
    }
}
type Result<T> = std::result::Result<T, ParseFailure>;

#[derive(Clone, Copy, Debug)]
enum Code {
    EofList,
    EofObject,
    EofString,
    EofValue,
    Colon,
    ListComma,
    ObjectComma,
    Ident,
    Value,
    Escape,
    Number,
    Range,
    Unicode,
    Control,
    Key,
    Surrogate,
    TrailingComma,
    TrailingCharacters,
    HexEnd,
    Recursion,
}
impl Code {
    fn message(self) -> &'static str {
        match self {
            Self::EofList => "EOF while parsing a list",
            Self::EofObject => "EOF while parsing an object",
            Self::EofString => "EOF while parsing a string",
            Self::EofValue => "EOF while parsing a value",
            Self::Colon => "expected `:`",
            Self::ListComma => "expected `,` or `]`",
            Self::ObjectComma => "expected `,` or `}`",
            Self::Ident => "expected ident",
            Self::Value => "expected value",
            Self::Escape => "invalid escape",
            Self::Number => "invalid number",
            Self::Range => "number out of range",
            Self::Unicode => "invalid unicode code point",
            Self::Control => "control character (\\u0000-\\u001F) found while parsing a string",
            Self::Key => "key must be a string",
            Self::Surrogate => "lone leading surrogate in hex escape",
            Self::TrailingComma => "trailing comma",
            Self::TrailingCharacters => "trailing characters",
            Self::HexEnd => "unexpected end of hex escape",
            Self::Recursion => "recursion limit exceeded",
        }
    }
}
const DUPLICATE_PREFIX: &str = "duplicate key ";
#[derive(Debug)]
enum Message {
    Static(Code),
    // Keep the complete user key alive. Only the borrowed display length is truncated by
    // upstream Error::custom's parse_line_col rule; no fixed user-message buffer exists.
    Duplicate { key: TextOwner, visible: usize },
}
/// Borrowed-format syntax diagnostic; owns only a moved, already charged duplicate key.
#[derive(Debug)]
pub struct Diagnostic {
    message: Message,
    line: usize,
    column: usize,
}
impl Diagnostic {
    fn parts(&self) -> (&str, &str) {
        match &self.message {
            Message::Static(code) => (code.message(), ""),
            Message::Duplicate { key, visible } => {
                let prefix = &DUPLICATE_PREFIX[..(*visible).min(DUPLICATE_PREFIX.len())];
                let key_len = visible.saturating_sub(DUPLICATE_PREFIX.len());
                (prefix, &key.as_str()[..key_len])
            }
        }
    }
    fn duplicate(key: TextOwner) -> Self {
        let parts = [DUPLICATE_PREFIX.as_bytes(), key.as_str().as_bytes()];
        let full = parts[0].len() + parts[1].len();
        let mut visible = full;
        let mut line = 0;
        let mut column = 0;
        // error.rs::parse_line_col uses the LAST occurrence, and on malformed/overflow
        // suffix preserves the full message. Include matches crossing the prefix/key seam.
        let byte = |i| joined_byte(&parts, i);
        let suffix = b" at line ";
        if let Some(start) = (0..=full.saturating_sub(suffix.len())).rev().find(|&i| {
            i + suffix.len() <= full && suffix.iter().enumerate().all(|(n, b)| byte(i + n) == *b)
        }) {
            let mut end_line = start + suffix.len();
            while end_line < full && byte(end_line).is_ascii_digit() {
                end_line += 1;
            }
            let delimiter = b" column ";
            if end_line + delimiter.len() <= full
                && delimiter
                    .iter()
                    .enumerate()
                    .all(|(n, b)| byte(end_line + n) == *b)
            {
                let start_column = end_line + delimiter.len();
                let mut end_column = start_column;
                while end_column < full && byte(end_column).is_ascii_digit() {
                    end_column += 1;
                }
                if end_column == full {
                    if let (Some(l), Some(c)) = (
                        joined_usize(&parts, start + suffix.len(), end_line),
                        joined_usize(&parts, start_column, end_column),
                    ) {
                        visible = start;
                        line = l;
                        column = c;
                    }
                }
            }
        }
        Self {
            message: Message::Duplicate { key, visible },
            line,
            column,
        }
    }
    /// Exact substring search over Display's message and optional location suffix.
    /// This borrows arbitrary-length user text and uses stack storage only for usize digits.
    pub fn contains(&self, needle: &str) -> bool {
        let (prefix, key) = self.parts();
        let mut line_digits = [0; 3 * std::mem::size_of::<usize>()];
        let mut column_digits = [0; 3 * std::mem::size_of::<usize>()];
        let line = decimal(self.line, &mut line_digits);
        let column = decimal(self.column, &mut column_digits);
        let parts: [&[u8]; 6] = if self.line == 0 {
            [prefix.as_bytes(), key.as_bytes(), b"", b"", b"", b""]
        } else {
            [
                prefix.as_bytes(),
                key.as_bytes(),
                b" at line ",
                line,
                b" column ",
                column,
            ]
        };
        let size: usize = parts.iter().map(|part| part.len()).sum();
        let needle = needle.as_bytes();
        if needle.len() > size {
            return false;
        }
        (0..=size - needle.len()).any(|start| {
            needle
                .iter()
                .enumerate()
                .all(|(i, b)| joined_byte(&parts, start + i) == *b)
        })
    }
    pub fn line(&self) -> usize {
        self.line
    }
    pub fn column(&self) -> usize {
        self.column
    }
}
fn joined_byte<const N: usize>(parts: &[&[u8]; N], mut i: usize) -> u8 {
    for part in parts {
        if i < part.len() {
            return part[i];
        }
        i -= part.len();
    }
    unreachable!("joined message byte in bounds")
}
fn joined_usize(parts: &[&[u8]; 2], start: usize, end: usize) -> Option<usize> {
    if start == end {
        return None;
    }
    let mut value = 0usize;
    for i in start..end {
        value = value
            .checked_mul(10)?
            .checked_add(usize::from(joined_byte(parts, i) - b'0'))?;
    }
    Some(value)
}
fn decimal(value: usize, buffer: &mut [u8]) -> &[u8] {
    let mut value = value;
    let mut start = buffer.len();
    loop {
        start -= 1;
        buffer[start] = b'0' + (value % 10) as u8;
        value /= 10;
        if value == 0 {
            return &buffer[start..];
        }
    }
}
impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (prefix, key) = self.parts();
        f.write_str(prefix)?;
        f.write_str(key)?;
        if self.line != 0 {
            write!(f, " at line {} column {}", self.line, self.column)?;
        }
        Ok(())
    }
}

/// Only the explicitly budgeted Q path enters here. Legacy audit parsing is unchanged.
pub fn parse(bytes: &[u8], budget: &OwnedBudget) -> Result<OwnedJson> {
    let mut parser = Parser {
        input: bytes,
        index: 0,
        remaining: 128,
        scratch: CodecVec::new(Some(budget)),
        budget,
    };
    let value = parser.value()?;
    if parser.whitespace().is_some() {
        return Err(parser.peek_error(Code::TrailingCharacters));
    }
    Ok(value)
}
struct Parser<'a> {
    input: &'a [u8],
    index: usize,
    remaining: u8,
    scratch: CodecVec<u8>,
    budget: &'a OwnedBudget,
}
#[derive(Clone, Copy)]
enum StringRef {
    Input { start: usize, end: usize },
    Scratch,
}
impl Parser<'_> {
    fn position(&self, peek: bool) -> (usize, usize) {
        let end = (self.index + usize::from(peek)).min(self.input.len());
        let mut line = 1;
        let mut column = 0;
        for &byte in &self.input[..end] {
            if byte == b'\n' {
                line += 1;
                column = 0;
            } else {
                column += 1;
            }
        }
        (line, column)
    }
    fn syntax(&self, code: Code, peek: bool) -> ParseFailure {
        let (line, column) = self.position(peek);
        ParseFailure::Syntax(Diagnostic {
            message: Message::Static(code),
            line,
            column,
        })
    }
    fn error(&self, code: Code) -> ParseFailure {
        self.syntax(code, false)
    }
    fn peek_error(&self, code: Code) -> ParseFailure {
        self.syntax(code, true)
    }
    fn fix_position(&self, mut error: ParseFailure) -> ParseFailure {
        if let ParseFailure::Syntax(ref mut diagnostic) = error {
            if diagnostic.line == 0 {
                (diagnostic.line, diagnostic.column) = self.position(false);
            }
        }
        error
    }
    fn peek(&self) -> Option<u8> {
        self.input.get(self.index).copied()
    }
    fn next(&mut self) -> Option<u8> {
        let result = self.peek();
        if result.is_some() {
            self.index += 1;
        }
        result
    }
    fn whitespace(&mut self) -> Option<u8> {
        loop {
            match self.peek() {
                Some(b' ' | b'\n' | b'\t' | b'\r') => self.index += 1,
                byte => return byte,
            }
        }
    }
    fn ident(&mut self, expected: &[u8]) -> Result<()> {
        for &expected in expected {
            match self.next() {
                Some(actual) if actual == expected => {}
                Some(_) => return Err(self.error(Code::Ident)),
                None => return Err(self.error(Code::EofValue)),
            }
        }
        Ok(())
    }
    fn scalar(&self, value: JsonView<'_>) -> Result<OwnedJson> {
        Ok(OwnedJson::from_view(value, self.budget)?)
    }
    fn value(&mut self) -> Result<OwnedJson> {
        let byte = self
            .whitespace()
            .ok_or_else(|| self.peek_error(Code::EofValue))?;
        let result = match byte {
            b'n' => {
                self.index += 1;
                self.ident(b"ull")?;
                self.scalar(JsonView::Null)
            }
            b't' => {
                self.index += 1;
                self.ident(b"rue")?;
                self.scalar(JsonView::Bool(true))
            }
            b'f' => {
                self.index += 1;
                self.ident(b"alse")?;
                self.scalar(JsonView::Bool(false))
            }
            b'-' | b'0'..=b'9' => {
                match number::parse_positioned(&self.input[self.index..]) {
                    Ok((number, consumed)) => {
                        self.index += consumed;
                        // Number is inline under the locked feature set; preserve scalar
                        // from_view construction without a Plain JSON tree.
                        self.scalar(JsonView::Number(&number))
                    }
                    Err(error) => {
                        self.index += error.consumed;
                        let code = match error.kind {
                            number::ErrorCode::InvalidNumber => Code::Number,
                            number::ErrorCode::EofWhileParsingValue => Code::EofValue,
                            number::ErrorCode::NumberOutOfRange => Code::Range,
                        };
                        Err(self.syntax(code, error.peek))
                    }
                }
            }
            b'"' => {
                self.index += 1;
                self.scratch.clear();
                let value = self.string()?;
                self.scalar(JsonView::String(self.string_ref(value)))
            }
            b'[' | b'{' => {
                // de.rs check_recursion: reject depth 128 BEFORE consuming its opener.
                self.remaining -= 1;
                if self.remaining == 0 {
                    return Err(self.peek_error(Code::Recursion));
                }
                self.index += 1;
                let result = if byte == b'[' {
                    self.sequence()
                } else {
                    self.map()
                };
                self.remaining += 1;
                // Visitor errors still run end_seq/end_map; first error wins. This movement
                // precedes fix_position and matters for duplicate custom suffix line zero.
                let end = if byte == b'[' {
                    self.end_sequence()
                } else {
                    self.end_map()
                };
                match (result, end) {
                    (Ok(value), Ok(())) => Ok(value),
                    (Err(error), _) | (_, Err(error)) => Err(error),
                }
            }
            _ => Err(self.peek_error(Code::Value)),
        };
        result.map_err(|error| self.fix_position(error))
    }
    fn sequence(&mut self) -> Result<OwnedJson> {
        let mut owner = OwnedJson::array(self.budget)?;
        let mut first = true;
        loop {
            let peek = self
                .whitespace()
                .ok_or_else(|| self.peek_error(Code::EofList))?;
            if peek == b']' {
                return Ok(owner);
            }
            if first {
                first = false;
            } else if peek == b',' {
                self.index += 1;
                match self.whitespace() {
                    Some(b']') => return Err(self.peek_error(Code::TrailingComma)),
                    Some(_) => {}
                    None => return Err(self.peek_error(Code::EofValue)),
                }
            } else {
                return Err(self.peek_error(Code::ListComma));
            }
            let child = self.value()?;
            owner.push(child.view())?;
        }
    }
    fn map(&mut self) -> Result<OwnedJson> {
        let mut owner = OwnedJson::object(self.budget)?;
        let mut first = true;
        loop {
            let peek = self
                .whitespace()
                .ok_or_else(|| self.peek_error(Code::EofObject))?;
            if peek == b'}' {
                return Ok(owner);
            }
            if first {
                first = false;
                if peek != b'"' {
                    return Err(self.peek_error(Code::Key));
                }
            } else if peek == b',' {
                self.index += 1;
                match self.whitespace() {
                    Some(b'"') => {}
                    Some(b'}') => return Err(self.peek_error(Code::TrailingComma)),
                    Some(_) => return Err(self.peek_error(Code::Key)),
                    None => return Err(self.peek_error(Code::EofValue)),
                }
            } else {
                return Err(self.peek_error(Code::ObjectComma));
            }
            // MapKey::deserialize_string forwards to deserialize_any: quote already checked.
            self.index += 1;
            self.scratch.clear();
            let string = self.string()?;
            let key = TextOwner::copy_from(self.string_ref(string), self.budget)?;
            if owner.view().get(key.as_str()).is_some() {
                return Err(ParseFailure::Syntax(Diagnostic::duplicate(key)));
            }
            match self.whitespace() {
                Some(b':') => self.index += 1,
                Some(_) => return Err(self.peek_error(Code::Colon)),
                None => return Err(self.peek_error(Code::EofObject)),
            }
            let child = self.value()?;
            owner.insert_field(key.as_str(), child.view())?;
        }
    }
    fn end_sequence(&mut self) -> Result<()> {
        match self.whitespace() {
            Some(b']') => {
                self.index += 1;
                Ok(())
            }
            Some(b',') => {
                self.index += 1;
                let code = if self.whitespace() == Some(b']') {
                    Code::TrailingComma
                } else {
                    Code::TrailingCharacters
                };
                Err(self.peek_error(code))
            }
            Some(_) => Err(self.peek_error(Code::TrailingCharacters)),
            None => Err(self.peek_error(Code::EofList)),
        }
    }
    fn end_map(&mut self) -> Result<()> {
        match self.whitespace() {
            Some(b'}') => {
                self.index += 1;
                Ok(())
            }
            Some(b',') => Err(self.peek_error(Code::TrailingComma)),
            Some(_) => Err(self.peek_error(Code::TrailingCharacters)),
            None => Err(self.peek_error(Code::EofObject)),
        }
    }
    fn string_ref(&self, value: StringRef) -> &str {
        let bytes = match value {
            StringRef::Input { start, end } => &self.input[start..end],
            StringRef::Scratch => &self.scratch,
        };
        std::str::from_utf8(bytes).expect("string was validated after consuming closing quote")
    }
    fn string(&mut self) -> Result<StringRef> {
        let mut start = self.index;
        loop {
            // SliceRead::skip_to_escape stops at quote, backslash or control byte.
            while let Some(byte) = self.peek() {
                if byte == b'"' || byte == b'\\' || byte < 0x20 {
                    break;
                }
                self.index += 1;
            }
            match self.peek() {
                None => return Err(self.error(Code::EofString)),
                Some(b'"') => {
                    let end = self.index;
                    let result = if self.scratch.is_empty() {
                        StringRef::Input { start, end }
                    } else {
                        self.scratch.extend_from_slice(&self.input[start..end])?;
                        StringRef::Scratch
                    };
                    self.index += 1;
                    let bytes = match result {
                        StringRef::Input { start, end } => &self.input[start..end],
                        StringRef::Scratch => &self.scratch,
                    };
                    // read.rs::as_str validates only AFTER all escapes and the closing quote.
                    std::str::from_utf8(bytes).map_err(|_| self.error(Code::Unicode))?;
                    return Ok(result);
                }
                Some(b'\\') => {
                    self.scratch
                        .extend_from_slice(&self.input[start..self.index])?;
                    self.index += 1;
                    self.escape()?;
                    start = self.index;
                }
                Some(_) => {
                    self.index += 1;
                    return Err(self.error(Code::Control));
                }
            }
        }
    }
    fn string_next(&mut self) -> Result<u8> {
        self.next().ok_or_else(|| self.error(Code::EofString))
    }
    fn string_peek(&self) -> Result<u8> {
        self.peek().ok_or_else(|| self.error(Code::EofString))
    }
    fn escape(&mut self) -> Result<()> {
        let byte = match self.string_next()? {
            b'"' => b'"',
            b'\\' => b'\\',
            b'/' => b'/',
            b'b' => 8,
            b'f' => 12,
            b'n' => b'\n',
            b'r' => b'\r',
            b't' => b'\t',
            b'u' => return self.unicode(),
            _ => return Err(self.error(Code::Escape)),
        };
        self.scratch.try_push(byte)?;
        Ok(())
    }
    fn hex(&mut self) -> Result<u16> {
        // SliceRead consumes all four bytes before reporting a non-hex error. For short
        // input it moves straight to EOF, independently of invalid bytes in that prefix.
        if self.input.len() - self.index < 4 {
            self.index = self.input.len();
            return Err(self.error(Code::EofString));
        }
        let start = self.index;
        self.index += 4;
        let mut value = 0;
        for &byte in &self.input[start..self.index] {
            let digit = match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                b'A'..=b'F' => byte - b'A' + 10,
                _ => return Err(self.error(Code::Escape)),
            };
            value = value * 16 + u16::from(digit);
        }
        Ok(value)
    }
    fn unicode(&mut self) -> Result<()> {
        let n = self.hex()?;
        if (0xDC00..=0xDFFF).contains(&n) {
            return Err(self.error(Code::Surrogate));
        }
        let codepoint = if (0xD800..=0xDBFF).contains(&n) {
            if self.string_peek()? != b'\\' {
                self.index += 1;
                return Err(self.error(Code::HexEnd));
            }
            self.index += 1;
            if self.string_peek()? != b'u' {
                self.index += 1;
                return Err(self.error(Code::HexEnd));
            }
            self.index += 1;
            let n2 = self.hex()?;
            if !(0xDC00..=0xDFFF).contains(&n2) {
                return Err(self.error(Code::Surrogate));
            }
            ((((n - 0xD800) as u32) << 10) | (n2 - 0xDC00) as u32) + 0x1_0000
        } else {
            u32::from(n)
        };
        // Upstream push_wtf8_codepoint reserves 4 bytes for non-ASCII, although UTF8
        // may need only 2/3; preserve that requested growth before writing any bytes.
        if codepoint < 0x80 {
            self.scratch.try_push(codepoint as u8)?;
        } else {
            self.scratch.try_reserve(4)?;
            let mut bytes = [0; 4];
            let encoded = char::from_u32(codepoint)
                .expect("paired validated surrogate")
                .encode_utf8(&mut bytes);
            self.scratch.extend_from_slice(encoded.as_bytes())?;
        }
        Ok(())
    }
}
