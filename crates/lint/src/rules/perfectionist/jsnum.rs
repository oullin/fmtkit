//! JavaScript's `Number(string)` and `Number#toString()`, and the int32
//! arithmetic of its bitwise operators.

use super::source::js_trim;

/// `Number(text)`: `None` where JavaScript gives `NaN`.
pub fn parse(text: &str) -> Option<f64> {
    let text = js_trim(text);

    if text.is_empty() {
        return Some(0.0);
    }

    let lower = text.get(..2).map(str::to_ascii_lowercase);
    let radix = match lower.as_deref() {
        Some("0x") => Some(16),
        Some("0o") => Some(8),
        Some("0b") => Some(2),
        _ => None,
    };

    if let Some(radix) = radix {
        return integer(&text[2..], radix);
    }

    let (sign, unsigned) = match text.as_bytes()[0] {
        b'+' => (1.0, &text[1..]),
        b'-' => (-1.0, &text[1..]),
        _ => (1.0, text),
    };

    if unsigned == "Infinity" {
        return Some(sign * f64::INFINITY);
    }

    decimal(unsigned).then(|| unsigned.parse::<f64>().ok()).flatten().map(|value| sign * value)
}

fn integer(digits: &str, radix: u32) -> Option<f64> {
    if digits.is_empty() {
        return None;
    }

    let mut value = 0.0_f64;
    let mut exact: Option<u128> = Some(0);

    for c in digits.chars() {
        let digit = c.to_digit(radix)?;

        exact = exact.and_then(|v| v.checked_mul(u128::from(radix))).and_then(|v| v.checked_add(u128::from(digit)));
        value = value * f64::from(radix) + f64::from(digit);
    }

    #[expect(clippy::cast_precision_loss, reason = "JavaScript rounds the exact value to the nearest double")]
    Some(exact.map_or(value, |exact| exact as f64))
}

/// `StrUnsignedDecimalLiteral` without `Infinity`.
fn decimal(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut i = 0;
    let int = count_digits(bytes, &mut i);
    let mut frac = 0;

    if bytes.get(i) == Some(&b'.') {
        i += 1;
        frac = count_digits(bytes, &mut i);
    }

    if int == 0 && frac == 0 {
        return false;
    }

    if matches!(bytes.get(i), Some(b'e' | b'E')) {
        i += 1;

        if matches!(bytes.get(i), Some(b'+' | b'-')) {
            i += 1;
        }

        if count_digits(bytes, &mut i) == 0 {
            return false;
        }
    }

    i == bytes.len()
}

fn count_digits(bytes: &[u8], i: &mut usize) -> usize {
    let start = *i;

    while bytes.get(*i).is_some_and(u8::is_ascii_digit) {
        *i += 1;
    }

    *i - start
}

/// `Number#toString()`.
pub fn format(value: f64) -> String {
    if value.is_nan() {
        return "NaN".into();
    }

    if value == 0.0 {
        return "0".into();
    }

    if value.is_infinite() {
        return if value > 0.0 { "Infinity".into() } else { "-Infinity".into() };
    }

    let sign = if value < 0.0 { "-" } else { "" };
    let scientific = format!("{:e}", value.abs());
    let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let exponent: i64 = exponent.parse().unwrap_or(0);
    let k = i64::try_from(digits.len()).unwrap_or(i64::MAX);
    let n = exponent + 1;
    let body = if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat(usize::try_from(n - k).unwrap_or(0)))
    } else if 0 < n && n <= 21 {
        let (int, frac) = digits.split_at(usize::try_from(n).unwrap_or(0));

        format!("{int}.{frac}")
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat(usize::try_from(-n).unwrap_or(0)))
    } else {
        let sign = if n - 1 < 0 { '-' } else { '+' };
        let (first, rest) = digits.split_at(1);
        let rest = if rest.is_empty() { String::new() } else { format!(".{rest}") };

        format!("{first}{rest}e{sign}{}", (n - 1).abs())
    };

    format!("{sign}{body}")
}

/// `ToInt32`.
pub fn int32(value: f64) -> i32 {
    if !value.is_finite() {
        return 0;
    }

    let modulo = value.trunc().rem_euclid(4_294_967_296.0);

    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "the value is an integer in [0, 2^32)")]
    let unsigned = modulo as u32;

    unsigned.cast_signed()
}

/// `ToUint32`.
pub fn uint32(value: f64) -> u32 {
    int32(value).cast_unsigned()
}

/// `**`, with JavaScript's `NaN` cases.
#[allow(clippy::float_cmp, reason = "JavaScript compares exactly")]
pub fn pow(base: f64, exponent: f64) -> f64 {
    if exponent.is_nan() || (base.abs() == 1.0 && exponent.is_infinite()) {
        return f64::NAN;
    }

    base.powf(exponent)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_like_javascript() {
        let cases = [
            (1.0, "1"),
            (-1.5, "-1.5"),
            (1e21, "1e+21"),
            (1e20, "100000000000000000000"),
            (0.000_001, "0.000001"),
            (1e-7, "1e-7"),
            (123.456, "123.456"),
            (0.1 + 0.2, "0.30000000000000004"),
            (1.5e-10, "1.5e-10"),
        ];

        for (value, expected) in cases {
            assert_eq!(format(value), expected);
        }
    }

    #[test]
    fn parses_like_javascript() {
        let cases = [
            ("", Some(0.0)),
            (" 12 ", Some(12.0)),
            ("0x1F", Some(31.0)),
            ("1e3", Some(1000.0)),
            (".5", Some(0.5)),
            ("5.", Some(5.0)),
            ("-Infinity", Some(f64::NEG_INFINITY)),
        ];

        for (text, expected) in cases {
            assert_eq!(parse(text), expected, "{text}");
        }

        for text in ["inf", "nan", "1_000", "-0x1", "abc", "."] {
            assert_eq!(parse(text), None, "{text}");
        }

        assert_eq!(int32(4_294_967_297.0), 1);
        assert_eq!(int32(-1.0), -1);
    }
}
