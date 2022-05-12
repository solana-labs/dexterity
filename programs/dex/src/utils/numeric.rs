use num::Num;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    fmt::Display,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
    str::FromStr,
};

use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};

use crate::error::{DomainOrProgramError, DomainOrProgramResult, UtilError};

pub const DIVISION_PRECISION: i64 = 10;
pub const SQRT_PRECISION: i64 = 4; // Should always be even
pub const FLOATING_PRECISION: i64 = 10;
pub const I64_MAX: i128 = i64::MAX as i128;
pub const EXP_UPPER_LIMIT: u64 = 15;

pub fn num_in_i64(num: i128) -> bool {
    !(num > (i64::MAX as i128) || num < (i64::MIN as i128))
}

const POW10: [i64; 19] = [
    1,
    10,
    100,
    1_000,
    10_000,
    100_000,
    1_000_000,
    10_000_000,
    100_000_000,
    1_000_000_000,
    10_000_000_000,
    100_000_000_000,
    1_000_000_000_000,
    10_000_000_000_000,
    100_000_000_000_000,
    1_000_000_000_000_000,
    10_000_000_000_000_000,
    100_000_000_000_000_000,
    1_000_000_000_000_000_000,
];

// /// a is fp0, b is fp32 and std::result::Result is a*b fp0
pub fn fp32_mul(a: u64, b_fp32: u64) -> u64 {
    (((a as u128) * (b_fp32 as u128)) >> 32) as u64
}
pub fn int_sqrt(m: i128) -> std::result::Result<i128, UtilError> {
    let mut start = 0_i128;
    let mut sq_root = 0_i128;
    if m < 0 {
        Err(UtilError::SqrtRootError)
    } else if m == 0 {
        Ok(0)
    } else if m > 1 {
        let mut end = 2;

        // safe for big numbers
        while end * end <= m {
            end *= 2;
        }
        end += 1;

        // outer loop for [n, n+1]
        while start <= end {
            let mid = (start + end) / 2;

            if mid * mid == m {
                sq_root = mid;
                break;
            }
            if mid * mid < m {
                sq_root = start;
                start = mid + 1;
            } else {
                end = mid - 1;
            }
        }
        Ok(sq_root)
    } else {
        Ok(1)
    }
}

pub fn int_div(m: u128, other: u128) -> std::result::Result<u128, UtilError> {
    if other == 0 {
        Err(UtilError::DivisionbyZero)
    } else {
        Ok(m / other)
    }
}
pub fn u64_to_quote(a: u64) -> std::result::Result<Fractional, UtilError> {
    if a > (i64::MAX) as u64 {
        Err(UtilError::NumericalOverflow)
    } else {
        Ok(Fractional {
            m: (a as i64),
            exp: 0,
        })
    }
}

/// Fractional Operations
#[repr(C)]
#[derive(
    Debug,
    Default,
    AnchorSerialize,
    AnchorDeserialize,
    Clone,
    Copy,
    Zeroable,
    Pod,
    Deserialize,
    Serialize,
)]
pub struct Fractional {
    pub m: i64,
    pub exp: u64,
}

impl Display for Fractional {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let base = POW10[self.exp as usize];
        if base == 0 {
            return write!(f, "0");
        }
        let lhs = self.m / base;
        let rhs = format!(
            "{:0width$}",
            (self.m % base).abs(),
            width = self.exp as usize
        );
        write!(f, "{}.{}", lhs, rhs)
    }
}

pub const ZERO_FRAC: Fractional = Fractional { m: 0, exp: 0 };

impl Neg for Fractional {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            m: -self.m,
            exp: self.exp,
        }
    }
}

impl Add for Fractional {
    type Output = Self;
    // Can overflow
    fn add(self, other: Self) -> Self {
        let (m, exp) = if self.exp > other.exp {
            (self.m + other.round_up(self.exp as u32).unwrap(), self.exp)
        } else if self.exp < other.exp {
            (
                self.round_up(other.exp as u32).unwrap() + other.m,
                other.exp,
            )
        } else {
            (self.m + other.m, self.exp)
        };
        Self { m, exp }
    }
}

impl AddAssign for Fractional {
    fn add_assign(&mut self, other: Self) {
        *self = self.add(other);
    }
}

impl Sub for Fractional {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        let (m, exp) = if self.exp > other.exp {
            (self.m - other.round_up(self.exp as u32).unwrap(), self.exp)
        } else if self.exp < other.exp {
            (
                self.round_up(other.exp as u32).unwrap() - other.m,
                other.exp,
            )
        } else {
            (self.m - other.m, self.exp)
        };
        Self { m, exp }
    }
}

impl SubAssign for Fractional {
    fn sub_assign(&mut self, other: Self) {
        *self = self.sub(other);
    }
}

impl Mul for Fractional {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        let self_reduced = self.get_reduced_form();
        let other_reduced = other.get_reduced_form();

        let m = self_reduced.m as i128 * other_reduced.m as i128;
        let exp = self_reduced.exp + other_reduced.exp;

        match Fractional::reduce_from_i128_unchecked(m, exp) {
            Ok(v) => v,
            Err(_) => ZERO_FRAC,
        }
    }
}

impl MulAssign for Fractional {
    fn mul_assign(&mut self, other: Self) {
        *self = self.mul(other);
    }
}

impl Div for Fractional {
    type Output = Self;

    fn div(self, other: Self) -> Self {
        let sign = self.sign() * other.sign();
        let self_reduced = self.get_reduced_form();
        let other_reduced = other.get_reduced_form();

        let mut dividend: u128 = self_reduced.m.abs() as u128;
        let divisor: u128 = other_reduced.m.abs() as u128;
        let exp = (self_reduced.exp as i64) - (other_reduced.exp as i64);
        dividend *= POW10[(DIVISION_PRECISION - exp.min(0)) as usize] as u128;

        let quotient: u128 = dividend / divisor;
        let mut divided_val = Fractional::new(
            quotient as i64,
            (exp - exp.min(0) + DIVISION_PRECISION) as u64,
        )
        .round_sf_unchecked(FLOATING_PRECISION as u32);

        if sign < 0 {
            divided_val.m *= -1;
        }
        divided_val
    }
}
impl DivAssign for Fractional {
    fn div_assign(&mut self, other: Self) {
        *self = self.div(other);
    }
}

impl PartialOrd for Fractional {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self.is_negative(), other.is_negative()) {
            (false, true) => return Some(Ordering::Greater),
            (true, false) => return Some(Ordering::Less),
            _ => {}
        }
        if self.m == 0 {
            return 0.partial_cmp(&other.m);
        } else if other.m == 0 {
            return self.m.partial_cmp(&0);
        }
        (self.m as i128 * POW10[other.exp as usize] as i128)
            .partial_cmp(&(other.m as i128 * POW10[self.exp as usize] as i128))
    }
}

impl PartialEq for Fractional {
    fn eq(&self, other: &Self) -> bool {
        if self.m == other.m && self.exp == other.exp {
            return true;
        }
        if self.m == 0 {
            return other.m == 0;
        } else if other.m == 0 {
            return self.m == 0;
        }
        match self.partial_cmp(other) {
            Some(Ordering::Equal) => true,
            _ => false,
        }
    }
}

impl From<i64> for Fractional {
    fn from(x: i64) -> Self {
        Fractional::new(x, 0)
    }
}

impl Mul<i64> for Fractional {
    type Output = Fractional;

    fn mul(self, rhs: i64) -> Self::Output {
        self * Fractional::from(rhs)
    }
}

impl Add<i64> for Fractional {
    type Output = Fractional;

    fn add(self, rhs: i64) -> Self::Output {
        self + Fractional::from(rhs)
    }
}

impl Add<Fractional> for i64 {
    type Output = Fractional;

    fn add(self, rhs: Fractional) -> Self::Output {
        rhs + self
    }
}

impl Mul<Fractional> for i64 {
    type Output = Fractional;

    fn mul(self, rhs: Fractional) -> Self::Output {
        Fractional::from(self) * rhs
    }
}

pub fn bps(x: i64) -> Fractional {
    Fractional::new(x, 4)
}

impl Eq for Fractional {}

impl Fractional {
    #[must_use]
    pub fn new(m: i64, e: u64) -> Fractional {
        if e > EXP_UPPER_LIMIT {
            panic!("Exponent cannot exceed {}", EXP_UPPER_LIMIT)
        }
        Fractional { m, exp: e }
    }

    pub fn to_int(&self) -> i64 {
        self.to_int_with_remainder().0
    }

    pub fn to_int_with_remainder(&self) -> (i64, Fractional) {
        let reduced = self.get_reduced_form();
        let int = reduced.m / POW10[reduced.exp as usize];
        (int, *self + (-int))
    }

    pub fn from_str(s: &str) -> std::result::Result<Fractional, DomainOrProgramError> {
        match s.split_once(".") {
            Some((lhs, rhs)) => {
                let m = format!("{}{}", lhs, rhs)
                    .parse::<i64>()
                    .map_err(|_| UtilError::DeserializeError)?;
                Ok(Fractional::new(m, rhs.len() as u64))
            }
            None => {
                let m = s.parse::<i64>().map_err(|_| UtilError::DeserializeError)?;
                Ok(Fractional::new(m, 0))
            }
        }
    }

    pub fn is_negative(&self) -> bool {
        self.m < 0
    }

    pub fn sign(&self) -> i32 {
        -2 * (self.is_negative() as i32) + 1
    }

    pub fn min(&self, other: Fractional) -> Fractional {
        match *self > other {
            true => other,
            false => *self,
        }
    }

    pub fn max(&self, other: Fractional) -> Fractional {
        match *self > other {
            true => *self,
            false => other,
        }
    }

    pub fn abs(&self) -> Fractional {
        Fractional {
            m: self.m.abs(),
            exp: self.exp,
        }
    }

    pub fn reduce_mut(&mut self) {
        if self.m == 0 {
            self.exp = 0;
            return;
        }
        while self.m % 10 == 0 {
            self.m /= 10;
            self.exp -= 1;
        }
    }

    pub fn get_reduced_form(&self) -> Self {
        let mut reduced = Fractional::new(self.m, self.exp);
        if reduced.m == 0 {
            reduced.exp = 0;
            return reduced;
        }
        while reduced.m % 10 == 0 && reduced.exp > 0 {
            reduced.m /= 10;
            reduced.exp -= 1;
        }
        reduced
    }

    pub fn reduce_from_i128(m: &mut i128, exp: &mut u64) -> std::result::Result<Self, UtilError> {
        if *m == 0 {
            *exp = 0;
        }
        if *m % POW10[16] as i128 == 0 && *exp >= 16 {
            *m /= POW10[16] as i128;
            *exp -= 16;
        }
        if *m % POW10[8] as i128 == 0 && *exp >= 8 {
            *m /= POW10[8] as i128;
            *exp -= 8;
        }
        if *m % POW10[4] as i128 == 0 && *exp >= 4 {
            *m /= POW10[4] as i128;
            *exp -= 4;
        }
        if *m % POW10[2] as i128 == 0 && *exp >= 2 {
            *m /= POW10[2] as i128;
            *exp -= 2;
        }
        while *m % 10 == 0 && *exp > 0 {
            *m /= 10;
            *exp -= 1;
        }

        if !num_in_i64(*m) || *exp > EXP_UPPER_LIMIT {
            return Err(UtilError::NumericalOverflow);
        }

        Ok(Fractional::new(*m as i64, *exp))
    }

    pub fn reduce_from_i128_unchecked(
        mut m: i128,
        mut exp: u64,
    ) -> std::result::Result<Self, UtilError> {
        if m == 0 {
            exp = 0;
        }

        while (exp > FLOATING_PRECISION as u64) || (!num_in_i64(m) && exp > 0) {
            m /= 10;
            exp -= 1;
        }

        if !num_in_i64(m) {
            return Err(UtilError::NumericalOverflow);
        }
        Ok(Fractional::new(m as i64, exp))
    }

    pub fn reduce_unchecked(m: &mut i128, exp: &mut u64, precision: u64) -> Self {
        if *m == 0 {
            return Fractional::new(0, 0);
        }
        while *exp > precision {
            *m /= 10;
            *exp -= 1;
        }
        Fractional::new(*m as i64, *exp)
    }

    pub fn reduce(
        m: &mut i128,
        exp: &mut u64,
        precision: u64,
    ) -> std::result::Result<Self, DomainOrProgramError> {
        if *m == 0 {
            return Ok(Fractional::new(0, 0));
        }
        while *exp > precision {
            if *m % 10 != 0 {
                return Err(UtilError::RoundError.into());
            }
            *m /= 10;
            *exp -= 1;
        }
        if !num_in_i64(*m) {
            return Err(UtilError::NumericalOverflow.into());
        }
        Ok(Fractional::new(*m as i64, *exp))
    }

    pub fn round_unchecked(
        &self,
        digits: u32,
    ) -> std::result::Result<Fractional, DomainOrProgramError> {
        let diff = digits as i32 - self.exp as i32;
        if diff >= 0 {
            Ok(Fractional::new(
                (self.m)
                    .checked_mul(POW10[diff as usize])
                    .ok_or(UtilError::NumericalOverflow)?,
                digits as u64,
            ))
        } else {
            Ok(Fractional::new(
                self.m / POW10[diff.abs() as usize],
                digits as u64,
            ))
        }
    }

    pub fn round(&self, digits: u32) -> DomainOrProgramResult<Fractional> {
        let num = self.round_unchecked(digits)?;
        if &num != self {
            return Err(UtilError::RoundError.into());
        }
        Ok(num)
    }

    fn round_up(&self, digits: u32) -> std::result::Result<i64, UtilError> {
        let diff = digits as usize - self.exp as usize;
        (self.m)
            .checked_mul(POW10[diff])
            .ok_or(UtilError::NumericalOverflow)
    }

    pub fn round_sf(&self, digits: u32) -> Self {
        if digits >= self.exp as u32 {
            Fractional::new(self.m, self.exp)
        } else {
            let m = self.m / POW10[self.exp as usize - digits as usize];
            Fractional::new(m, digits as u64)
        }
    }

    pub fn round_sf_unchecked(&self, digits: u32) -> Self {
        if digits >= self.exp as u32 {
            Fractional::new(self.m, self.exp)
        } else {
            let m = self.m / POW10[self.exp as usize - digits as usize];
            Fractional::new(m, digits as u64)
        }
    }

    pub fn checked_add(
        &self,
        other: impl Into<Fractional>,
    ) -> std::result::Result<Fractional, UtilError> {
        let other = other.into();
        let (mut m, mut exp) = if self.exp > other.exp {
            (
                self.m as i128 + other.round_up(self.exp as u32)? as i128,
                self.exp,
            )
        } else if self.exp < other.exp {
            (
                self.round_up(other.exp as u32)? as i128 + other.m as i128,
                other.exp,
            )
        } else {
            (self.m as i128 + other.m as i128, self.exp)
        };

        if i128::abs(m) > i64::max_value() as i128 {
            Fractional::reduce_from_i128(&mut m, &mut exp)
        } else {
            Ok(Self { m: m as i64, exp })
        }
    }

    pub fn checked_sub(
        &self,
        other: impl Into<Fractional>,
    ) -> std::result::Result<Fractional, UtilError> {
        let other = other.into();
        let (mut m, mut exp) = if self.exp > other.exp {
            (
                self.m as i128 - other.round_up(self.exp as u32)? as i128,
                self.exp,
            )
        } else if self.exp < other.exp {
            (
                self.round_up(other.exp as u32)? as i128 - other.m as i128,
                other.exp,
            )
        } else {
            (self.m as i128 - other.m as i128, other.exp)
        };

        if i128::abs(m) > i64::max_value() as i128 {
            Fractional::reduce_from_i128(&mut m, &mut exp)
        } else {
            Ok(Self { m: m as i64, exp })
        }
    }

    pub fn checked_mul(
        &self,
        other: impl Into<Fractional>,
    ) -> std::result::Result<Fractional, DomainOrProgramError> {
        let other = other.into();
        match self.m == 0 || other.m == 0 {
            true => Ok(ZERO_FRAC),
            false => {
                let mut m = (self.m as i128) * (other.m as i128);
                let mut exp = self.exp + other.exp;
                Ok(Fractional::reduce_from_i128(&mut m, &mut exp)?)
            }
        }
    }

    pub fn saturating_mul(&self, other: impl Into<Fractional>) -> Fractional {
        match self.checked_mul(other) {
            Ok(f) => f,
            _ => Fractional::new(i64::MAX, 0),
        }
    }

    pub fn saturating_add(&self, other: impl Into<Fractional>) -> Fractional {
        match self.checked_add(other) {
            Ok(f) => f,
            _ => Fractional::new(i64::MAX, 0),
        }
    }

    pub fn checked_div(
        &self,
        other: impl Into<Fractional>,
    ) -> std::result::Result<Fractional, UtilError> {
        let other = other.into();
        let sign = self.sign() * other.sign();
        let mut dividend: u128 = self.m.abs() as u128;
        let divisor: u128 = other.m.abs() as u128;
        let mut exp = (self.exp as i64) - (other.exp as i64);
        dividend = dividend
            .checked_mul(POW10[(DIVISION_PRECISION - exp.min(0)) as usize] as u128)
            .ok_or(UtilError::NumericalOverflow)?;

        let quotient: u128 = dividend / divisor;
        exp = exp - exp.min(0) + DIVISION_PRECISION;

        let divided = Fractional::reduce_from_i128(&mut (quotient as i128), &mut (exp as u64))?;
        Ok(if sign >= 0 {
            divided
        } else {
            Fractional::new(-1 * divided.m, divided.exp)
        })
    }

    pub fn sqrt(&self) -> std::result::Result<Fractional, UtilError> {
        let mut exp = self.exp;
        let mut m = self.m as i128;

        if exp % 2 != 0 {
            if m < I64_MAX {
                m *= 10;
                exp += 1;
            } else {
                m /= 10; // huge number does not matter if we lose precision!!
                exp -= 1;
            }
        }
        let mut add_exp = 2;

        for _ in 0..SQRT_PRECISION / 2 {
            let pre_m = m * POW10[2] as i128;
            if pre_m > I64_MAX {
                break;
            }
            m = pre_m;
            add_exp += 2;
        }

        exp += (add_exp - 2) as u64;

        let int_sqrt_m = int_sqrt(m)?;

        if !num_in_i64(int_sqrt_m) {
            return Err(UtilError::NumericalOverflow);
        }
        Ok(Fractional::new(int_sqrt_m as i64, exp / 2))
    }

    pub fn exp(&self) -> std::result::Result<Fractional, UtilError> {
        let x = *self;
        let e_x = if x > Fractional::new(-1, 0) {
            Fractional::new(1, 0)
                .checked_add(x)?
                .checked_add(x * x * Fractional::new(5, 1))?
        } else if x > Fractional::new(-15, 1) {
            Fractional::new(22, 2)
        } else if x > Fractional::new(-2, 0) {
            Fractional::new(13, 2)
        } else if x > Fractional::new(-25, 1) {
            Fractional::new(8, 2)
        } else if x > Fractional::new(-3, 0) {
            Fractional::new(5, 2)
        } else {
            ZERO_FRAC
        };

        Ok(e_x)
    }

    pub fn has_precision(&self, precision: i64) -> bool {
        if precision > 0 {
            match self.checked_div(Fractional {
                m: POW10[precision as usize],
                exp: 0,
            }) {
                Err(_) => false,
                Ok(_) => true,
            }
        } else {
            match self.round((-precision) as u32) {
                Err(_) => false,
                Ok(_) => true,
            }
        }
    }
}

impl FromStr for Fractional {
    type Err = DomainOrProgramError;

    #[inline]
    fn from_str(s: &str) -> std::result::Result<Fractional, DomainOrProgramError> {
        match s.split_once(".") {
            Some((lhs, rhs)) => {
                let m = format!("{}{}", lhs, rhs)
                    .parse::<i64>()
                    .map_err(|_| UtilError::DeserializeError)?;
                Ok(Fractional::new(m, rhs.len() as u64))
            }
            None => {
                let m = s.parse::<i64>().map_err(|_| UtilError::DeserializeError)?;
                Ok(Fractional::new(m, 0))
            }
        }
    }
}

#[cfg(test)]
#[test]
fn test_numeric() {
    // Test square root
    let big_int_0 = 1 << 103_i128;
    let big_int_1 = 1 << 100_i128;

    let sq_int_0 = 1 << 51_i128;
    let sq_int_1 = 1 << 50_i128;

    let sqrt_m = int_sqrt(big_int_0 + big_int_1).unwrap_or(-1);
    assert_eq!(sqrt_m, sq_int_0 + sq_int_1);

    let big_int_0 = 1 << 126;
    let big_int_1 = 1 << 125;
    let quot = int_div(big_int_0, big_int_1).unwrap_or(0);

    assert_eq!(quot, 2);

    // Correct rounding
    let m_round = match Fractional::new(1256000000000000, 12).round(6) {
        Ok(v) => v,
        Err(_) => ZERO_FRAC,
    };
    assert_eq!(m_round.m, 1256000000);
    assert_eq!(m_round.exp, 6);

    // Incorrect rounding
    let m_round = match Fractional::new(1, 12).round(6) {
        Ok(v) => v,
        Err(_) => Fractional::new(-1, 0),
    };
    assert_eq!(m_round.m, -1);
    assert_eq!(m_round.exp, 0);

    // reduce from i128: success
    let mut m = i64::MAX as i128;
    let mut exp = 0_u64;

    let m_frac = match Fractional::reduce_from_i128(&mut m, &mut exp) {
        Ok(v) => v,
        Err(_) => ZERO_FRAC,
    };
    let match_value = i64::MAX as i128;
    assert_eq!(m_frac.m as i128, match_value);

    // failure
    let mut m = i64::MAX as i128 + 1;
    let mut exp = 0_u64;
    let m_frac = match Fractional::reduce_from_i128(&mut m, &mut exp) {
        Ok(v) => v,
        Err(_) => ZERO_FRAC,
    };
    assert_eq!(m_frac.m as i128, 0);

    //round_sf
    let m = Fractional::new(i64::MAX, 7);

    let m_round = m.round_sf(10);
    assert_eq!(m_round.m, i64::MAX);
    assert_eq!(m_round.exp, 7);

    let m_round = m.round_sf(4);

    assert_eq!(m_round.m, i64::MAX / 10_i128.pow(3) as i64);
    assert_eq!(m_round.exp, 4);

    // Big number comparisions:
    // `big_int` 2**31 (~10**9) can only be added to dust ~10**-9
    // This is because of the shifts 2**63-1 (~10**18)
    // Increasing `big_int` or increasing precision will cause failure
    let big_int = (1 << 31) as i64;
    let num = Fractional::new(big_int, 0);
    let dust = Fractional::new(1, 9);

    let big_add = match num.checked_add(dust) {
        Ok(v) => v,
        Err(_) => ZERO_FRAC,
    };

    let big_sub = match num.checked_sub(dust) {
        Ok(v) => v,
        Err(_) => ZERO_FRAC,
    };

    assert!(big_add > num);
    assert!(big_sub < num);

    // This fails
    let big_int = (1 << 31) as i64;
    let num = Fractional::new(big_int, 0);
    let dust = Fractional::new(1, 10);

    let big_add = match num.checked_add(dust) {
        Ok(v) => v,
        Err(_) => ZERO_FRAC,
    };

    assert!(big_add == ZERO_FRAC);

    // checked_mul on large m
    let v = match Fractional::new(1 << 62, 4).checked_div(Fractional::new(1 << 34, 0)) {
        Ok(n) => n,
        Err(_) => ZERO_FRAC,
    };
    assert_eq!(v, Fractional::new(1 << 28, 4));

    // This will fail in checked_mul
    let v = match Fractional::new(1 << 40, EXP_UPPER_LIMIT)
        .checked_mul(Fractional::new(1 << 35, EXP_UPPER_LIMIT))
    {
        Ok(_) => 0,
        Err(_) => 1,
    };
    assert_eq!(v, 1);

    // This will not fail on * but will round down to the best value
    let v = if Fractional::new(1 << 40, EXP_UPPER_LIMIT) * Fractional::new(1 << 35, EXP_UPPER_LIMIT)
        > ZERO_FRAC
    {
        0
    } else {
        1
    };
    assert_eq!(v, 0);
}
