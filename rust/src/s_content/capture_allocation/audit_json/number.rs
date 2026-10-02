//! serde_json 1.0.151 src/de.rs 的 std-only/non-float_roundtrip 数值算法。
//! 锁定 Cargo.lock c841b55e，cargo tree -e features -i serde_json 仅 default/std。
//! 只替换读取面、错误载体；保留 mantissa 截断、指数、-0 和 POW10 的原运算顺序。
//! 原源 https://github.com/serde-rs/json/blob/v1.0.151/src/de.rs
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
use super::{invalid, CodecError};
use serde_json::Number;
type Result<T> = std::result::Result<T, CodecError>;
macro_rules! tri {
    ($e:expr $(,)?) => {
        $e?
    };
}
macro_rules! overflow {
    ($a:ident * 10 + $b:ident, $c:expr) => {
        match $c {
            c => $a >= c / 10 && ($a > c / 10 || $b > c % 10),
        }
    };
}
enum ParserNumber {
    F64(f64),
    U64(u64),
    I64(i64),
}
enum ErrorCode {
    InvalidNumber,
    EofWhileParsingValue,
    NumberOutOfRange,
}
struct Numeric<'a> {
    input: &'a [u8],
    pos: usize,
}
pub(super) fn parse(input: &[u8]) -> Result<(Number, usize)> {
    let mut p = Numeric { input, pos: 0 };
    let positive = if input.first() == Some(&b'-') {
        p.pos += 1;
        false
    } else {
        true
    };
    let n = match p.parse_integer(positive)? {
        ParserNumber::U64(n) => Number::from(n),
        ParserNumber::I64(n) => Number::from(n),
        ParserNumber::F64(n) => {
            Number::from_f64(n).ok_or_else(|| invalid("number out of range"))?
        }
    };
    Ok((n, p.pos))
}
impl Numeric<'_> {
    fn peek(&self) -> Result<Option<u8>> {
        Ok(self.input.get(self.pos).copied())
    }
    fn peek_or_null(&self) -> Result<u8> {
        Ok(self.input.get(self.pos).copied().unwrap_or(0))
    }
    fn next_char(&mut self) -> Result<Option<u8>> {
        let v = self.input.get(self.pos).copied();
        if v.is_some() {
            self.pos += 1;
        }
        Ok(v)
    }
    fn eat_char(&mut self) {
        self.pos += 1;
    }
    fn error(&self, e: ErrorCode) -> CodecError {
        match e {
            ErrorCode::InvalidNumber => invalid("invalid number"),
            ErrorCode::EofWhileParsingValue => invalid("EOF while parsing value"),
            ErrorCode::NumberOutOfRange => invalid("number out of range"),
        }
    }
    fn peek_error(&self, e: ErrorCode) -> CodecError {
        self.error(e)
    }
    fn parse_integer(&mut self, positive: bool) -> Result<ParserNumber> {
        let next = match tri!(self.next_char()) {
            Some(b) => b,
            None => {
                return Err(self.error(ErrorCode::EofWhileParsingValue));
            }
        };

        match next {
            b'0' => {
                // There can be only one leading '0'.
                match tri!(self.peek_or_null()) {
                    b'0'..=b'9' => Err(self.peek_error(ErrorCode::InvalidNumber)),
                    _ => self.parse_number(positive, 0),
                }
            }
            c @ b'1'..=b'9' => {
                let mut significand = (c - b'0') as u64;

                loop {
                    match tri!(self.peek_or_null()) {
                        c @ b'0'..=b'9' => {
                            let digit = (c - b'0') as u64;

                            // We need to be careful with overflow. If we can,
                            // try to keep the number as a `u64` until we grow
                            // too large. At that point, switch to parsing the
                            // value as a `f64`.
                            if overflow!(significand * 10 + digit, u64::MAX) {
                                return Ok(ParserNumber::F64(tri!(
                                    self.parse_long_integer(positive, significand),
                                )));
                            }

                            self.eat_char();
                            significand = significand * 10 + digit;
                        }
                        _ => {
                            return self.parse_number(positive, significand);
                        }
                    }
                }
            }
            _ => Err(self.error(ErrorCode::InvalidNumber)),
        }
    }

    fn parse_number(&mut self, positive: bool, significand: u64) -> Result<ParserNumber> {
        Ok(match tri!(self.peek_or_null()) {
            b'.' => ParserNumber::F64(tri!(self.parse_decimal(positive, significand, 0))),
            b'e' | b'E' => ParserNumber::F64(tri!(self.parse_exponent(positive, significand, 0))),
            _ => {
                if positive {
                    ParserNumber::U64(significand)
                } else {
                    let neg = (significand as i64).wrapping_neg();

                    // Convert into a float if we underflow, or on `-0`.
                    if neg >= 0 {
                        ParserNumber::F64(-(significand as f64))
                    } else {
                        ParserNumber::I64(neg)
                    }
                }
            }
        })
    }

    fn parse_decimal(
        &mut self,
        positive: bool,
        mut significand: u64,
        exponent_before_decimal_point: i32,
    ) -> Result<f64> {
        self.eat_char();

        let mut exponent_after_decimal_point = 0;
        while let c @ b'0'..=b'9' = tri!(self.peek_or_null()) {
            let digit = (c - b'0') as u64;

            if overflow!(significand * 10 + digit, u64::MAX) {
                let exponent = exponent_before_decimal_point + exponent_after_decimal_point;
                return self.parse_decimal_overflow(positive, significand, exponent);
            }

            self.eat_char();
            significand = significand * 10 + digit;
            exponent_after_decimal_point -= 1;
        }

        // Error if there is not at least one digit after the decimal point.
        if exponent_after_decimal_point == 0 {
            match tri!(self.peek()) {
                Some(_) => return Err(self.peek_error(ErrorCode::InvalidNumber)),
                None => return Err(self.peek_error(ErrorCode::EofWhileParsingValue)),
            }
        }

        let exponent = exponent_before_decimal_point + exponent_after_decimal_point;
        match tri!(self.peek_or_null()) {
            b'e' | b'E' => self.parse_exponent(positive, significand, exponent),
            _ => self.f64_from_parts(positive, significand, exponent),
        }
    }

    fn parse_exponent(
        &mut self,
        positive: bool,
        significand: u64,
        starting_exp: i32,
    ) -> Result<f64> {
        self.eat_char();

        let positive_exp = match tri!(self.peek_or_null()) {
            b'+' => {
                self.eat_char();
                true
            }
            b'-' => {
                self.eat_char();
                false
            }
            _ => true,
        };

        let next = match tri!(self.next_char()) {
            Some(b) => b,
            None => {
                return Err(self.error(ErrorCode::EofWhileParsingValue));
            }
        };

        // Make sure a digit follows the exponent place.
        let mut exp = match next {
            c @ b'0'..=b'9' => (c - b'0') as i32,
            _ => {
                return Err(self.error(ErrorCode::InvalidNumber));
            }
        };

        while let c @ b'0'..=b'9' = tri!(self.peek_or_null()) {
            self.eat_char();
            let digit = (c - b'0') as i32;

            if overflow!(exp * 10 + digit, i32::MAX) {
                let zero_significand = significand == 0;
                return self.parse_exponent_overflow(positive, zero_significand, positive_exp);
            }

            exp = exp * 10 + digit;
        }

        let final_exp = if positive_exp {
            starting_exp.saturating_add(exp)
        } else {
            starting_exp.saturating_sub(exp)
        };

        self.f64_from_parts(positive, significand, final_exp)
    }

    fn f64_from_parts(
        &mut self,
        positive: bool,
        significand: u64,
        mut exponent: i32,
    ) -> Result<f64> {
        let mut f = significand as f64;
        loop {
            match POW10.get(exponent.wrapping_abs() as usize) {
                Some(&pow) => {
                    if exponent >= 0 {
                        f *= pow;
                        if f.is_infinite() {
                            return Err(self.error(ErrorCode::NumberOutOfRange));
                        }
                    } else {
                        f /= pow;
                    }
                    break;
                }
                None => {
                    if f == 0.0 {
                        break;
                    }
                    if exponent >= 0 {
                        return Err(self.error(ErrorCode::NumberOutOfRange));
                    }
                    f /= 1e308;
                    exponent += 308;
                }
            }
        }
        Ok(if positive { f } else { -f })
    }

    fn parse_long_integer(&mut self, positive: bool, significand: u64) -> Result<f64> {
        let mut exponent = 0;
        loop {
            match tri!(self.peek_or_null()) {
                b'0'..=b'9' => {
                    self.eat_char();
                    // This could overflow... if your integer is gigabytes long.
                    // Ignore that possibility.
                    exponent += 1;
                }
                b'.' => {
                    return self.parse_decimal(positive, significand, exponent);
                }
                b'e' | b'E' => {
                    return self.parse_exponent(positive, significand, exponent);
                }
                _ => {
                    return self.f64_from_parts(positive, significand, exponent);
                }
            }
        }
    }

    fn parse_decimal_overflow(
        &mut self,
        positive: bool,
        significand: u64,
        exponent: i32,
    ) -> Result<f64> {
        // The next multiply/add would overflow, so just ignore all further
        // digits.
        while let b'0'..=b'9' = tri!(self.peek_or_null()) {
            self.eat_char();
        }

        match tri!(self.peek_or_null()) {
            b'e' | b'E' => self.parse_exponent(positive, significand, exponent),
            _ => self.f64_from_parts(positive, significand, exponent),
        }
    }

    fn parse_exponent_overflow(
        &mut self,
        positive: bool,
        zero_significand: bool,
        positive_exp: bool,
    ) -> Result<f64> {
        // Error instead of +/- infinity.
        if !zero_significand && positive_exp {
            return Err(self.error(ErrorCode::NumberOutOfRange));
        }

        while let b'0'..=b'9' = tri!(self.peek_or_null()) {
            self.eat_char();
        }
        Ok(if positive { 0.0 } else { -0.0 })
    }
}
static POW10: [f64; 309] = [
    1e000, 1e001, 1e002, 1e003, 1e004, 1e005, 1e006, 1e007, 1e008, 1e009, //
    1e010, 1e011, 1e012, 1e013, 1e014, 1e015, 1e016, 1e017, 1e018, 1e019, //
    1e020, 1e021, 1e022, 1e023, 1e024, 1e025, 1e026, 1e027, 1e028, 1e029, //
    1e030, 1e031, 1e032, 1e033, 1e034, 1e035, 1e036, 1e037, 1e038, 1e039, //
    1e040, 1e041, 1e042, 1e043, 1e044, 1e045, 1e046, 1e047, 1e048, 1e049, //
    1e050, 1e051, 1e052, 1e053, 1e054, 1e055, 1e056, 1e057, 1e058, 1e059, //
    1e060, 1e061, 1e062, 1e063, 1e064, 1e065, 1e066, 1e067, 1e068, 1e069, //
    1e070, 1e071, 1e072, 1e073, 1e074, 1e075, 1e076, 1e077, 1e078, 1e079, //
    1e080, 1e081, 1e082, 1e083, 1e084, 1e085, 1e086, 1e087, 1e088, 1e089, //
    1e090, 1e091, 1e092, 1e093, 1e094, 1e095, 1e096, 1e097, 1e098, 1e099, //
    1e100, 1e101, 1e102, 1e103, 1e104, 1e105, 1e106, 1e107, 1e108, 1e109, //
    1e110, 1e111, 1e112, 1e113, 1e114, 1e115, 1e116, 1e117, 1e118, 1e119, //
    1e120, 1e121, 1e122, 1e123, 1e124, 1e125, 1e126, 1e127, 1e128, 1e129, //
    1e130, 1e131, 1e132, 1e133, 1e134, 1e135, 1e136, 1e137, 1e138, 1e139, //
    1e140, 1e141, 1e142, 1e143, 1e144, 1e145, 1e146, 1e147, 1e148, 1e149, //
    1e150, 1e151, 1e152, 1e153, 1e154, 1e155, 1e156, 1e157, 1e158, 1e159, //
    1e160, 1e161, 1e162, 1e163, 1e164, 1e165, 1e166, 1e167, 1e168, 1e169, //
    1e170, 1e171, 1e172, 1e173, 1e174, 1e175, 1e176, 1e177, 1e178, 1e179, //
    1e180, 1e181, 1e182, 1e183, 1e184, 1e185, 1e186, 1e187, 1e188, 1e189, //
    1e190, 1e191, 1e192, 1e193, 1e194, 1e195, 1e196, 1e197, 1e198, 1e199, //
    1e200, 1e201, 1e202, 1e203, 1e204, 1e205, 1e206, 1e207, 1e208, 1e209, //
    1e210, 1e211, 1e212, 1e213, 1e214, 1e215, 1e216, 1e217, 1e218, 1e219, //
    1e220, 1e221, 1e222, 1e223, 1e224, 1e225, 1e226, 1e227, 1e228, 1e229, //
    1e230, 1e231, 1e232, 1e233, 1e234, 1e235, 1e236, 1e237, 1e238, 1e239, //
    1e240, 1e241, 1e242, 1e243, 1e244, 1e245, 1e246, 1e247, 1e248, 1e249, //
    1e250, 1e251, 1e252, 1e253, 1e254, 1e255, 1e256, 1e257, 1e258, 1e259, //
    1e260, 1e261, 1e262, 1e263, 1e264, 1e265, 1e266, 1e267, 1e268, 1e269, //
    1e270, 1e271, 1e272, 1e273, 1e274, 1e275, 1e276, 1e277, 1e278, 1e279, //
    1e280, 1e281, 1e282, 1e283, 1e284, 1e285, 1e286, 1e287, 1e288, 1e289, //
    1e290, 1e291, 1e292, 1e293, 1e294, 1e295, 1e296, 1e297, 1e298, 1e299, //
    1e300, 1e301, 1e302, 1e303, 1e304, 1e305, 1e306, 1e307, 1e308,
];
