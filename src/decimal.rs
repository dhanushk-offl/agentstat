use crate::{AgentStatError, Result};
use num_bigint::BigInt;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};

/// Exact finite decimal, serialized as a decimal string. No binary floating-point money.
/// Input is bounded to 512 digits and an exponent magnitude of 512.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Decimal(String);
impl Decimal {
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn to_f64(&self) -> Option<f64> {
        self.0.parse::<f64>().ok().filter(|v| v.is_finite())
    }
    pub fn is_negative(&self) -> bool {
        self.0.starts_with('-')
    }
    pub fn is_zero(&self) -> bool {
        self.0 == "0"
    }
    fn parts(&self) -> (BigInt, usize) {
        let scale = self.0.split_once('.').map_or(0, |(_, f)| f.len());
        (
            self.0.replace('.', "").parse().expect("validated decimal"),
            scale,
        )
    }
    pub fn checked_sub(&self, other: &Self) -> Option<Self> {
        let (a, sa) = self.parts();
        let (b, sb) = other.parts();
        let scale = sa.max(sb);
        let coefficient = a * BigInt::from(10u8).pow((scale - sa) as u32)
            - b * BigInt::from(10u8).pow((scale - sb) as u32);
        Self::from_str(&format!("{coefficient}e-{scale}")).ok()
    }
    pub fn checked_add(&self, other: &Self) -> Option<Self> {
        let (a, sa) = self.parts();
        let (b, sb) = other.parts();
        let scale = sa.max(sb);
        let coefficient = a * BigInt::from(10u8).pow((scale - sa) as u32)
            + b * BigInt::from(10u8).pow((scale - sb) as u32);
        Self::from_str(&format!("{coefficient}e-{scale}")).ok()
    }
    pub fn divided_by_power_of_ten(&self, power: u16) -> Option<Self> {
        let (n, scale) = self.parts();
        Self::from_str(&format!("{n}e-{}", scale + usize::from(power))).ok()
    }
}
impl FromStr for Decimal {
    type Err = AgentStatError;
    fn from_str(raw: &str) -> Result<Self> {
        let invalid = || AgentStatError::InvalidResponse("invalid decimal");
        if raw.is_empty() || raw.len() > 1024 || raw.trim() != raw {
            return Err(invalid());
        }
        let (mantissa, exponent) = raw.find(['e', 'E']).map_or(Ok((raw, 0i32)), |i| {
            raw[i + 1..]
                .parse::<i32>()
                .map(|e| (&raw[..i], e))
                .map_err(|_| invalid())
        })?;
        if exponent.unsigned_abs() > 512 {
            return Err(invalid());
        }
        let negative = mantissa.starts_with('-');
        let mantissa = mantissa.strip_prefix('-').unwrap_or(mantissa);
        let mut split = mantissa.split('.');
        let whole = split.next().unwrap_or("");
        let fraction = split.next().unwrap_or("");
        if split.next().is_some()
            || whole.is_empty()
            || !whole.bytes().all(|b| b.is_ascii_digit())
            || !fraction.bytes().all(|b| b.is_ascii_digit())
            || (mantissa.contains('.') && fraction.is_empty())
            || whole.len() + fraction.len() > 512
        {
            return Err(invalid());
        }
        let mut digits = format!("{whole}{fraction}");
        let scale = fraction.len() as i32 - exponent;
        let mut value = if scale <= 0 {
            digits.push_str(&"0".repeat((-scale) as usize));
            digits
        } else {
            let scale = scale as usize;
            if digits.len() <= scale {
                digits = format!("{}{}", "0".repeat(scale + 1 - digits.len()), digits);
            }
            digits.insert(digits.len() - scale, '.');
            digits
        };
        if value.contains('.') {
            value = value.trim_end_matches('0').trim_end_matches('.').to_owned();
        }
        value = value.trim_start_matches('0').to_owned();
        if value.starts_with('.') {
            value.insert(0, '0');
        }
        if value.is_empty() {
            value = "0".into();
        }
        if negative && value != "0" {
            value.insert(0, '-');
        }
        Ok(Self(value))
    }
}
impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl Serialize for Decimal {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.0)
    }
}
impl<'de> Deserialize<'de> for Decimal {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let v = serde_json::Value::deserialize(d)?;
        let s = match v {
            serde_json::Value::String(s) => s,
            serde_json::Value::Number(n) => n.to_string(),
            _ => return Err(serde::de::Error::custom("expected decimal")),
        };
        s.parse()
            .map_err(|_| serde::de::Error::custom("invalid decimal"))
    }
}
impl From<u64> for Decimal {
    fn from(v: u64) -> Self {
        Self(v.to_string())
    }
}
