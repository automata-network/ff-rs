#![allow(unused_imports)]
#![cfg_attr(feature = "tstd", no_std)]

#[cfg(feature = "tstd")]
#[macro_use]
extern crate sgxlib as std;

use std::prelude::v1::*;

extern crate byteorder;
extern crate rand;
extern crate serde;
extern crate hex as hex_ext;
pub mod hex {
    pub use hex_ext::*;
}

#[cfg(feature = "derive")]
#[macro_use]
extern crate ff_derive_ce;

#[cfg(feature = "derive")]
pub use ff_derive_ce::*;

use std::error::Error;
use std::fmt;
use std::hash;
use std::io::{self, Read, Write};

/// This trait represents an element of a field.
pub trait Field: Sized 
    + Eq 
    + Copy 
    + Clone 
    + Send 
    + Sync 
    + fmt::Debug 
    + fmt::Display 
    + 'static 
    + rand::Rand 
    + hash::Hash 
    + Default 
    + serde::Serialize
    + serde::de::DeserializeOwned
{
    /// Returns the zero element of the field, the additive identity.
    fn zero() -> Self;

    /// Returns the one element of the field, the multiplicative identity.
    fn one() -> Self;

    /// Returns true iff this element is zero.
    fn is_zero(&self) -> bool;

    /// Squares this element.
    fn square(&mut self);

    /// Doubles this element.
    fn double(&mut self);

    /// Negates this element.
    fn negate(&mut self);

    /// Adds another element to this element.
    fn add_assign(&mut self, other: &Self);

    /// Subtracts another element from this element.
    fn sub_assign(&mut self, other: &Self);

    /// Multiplies another element by this element.
    fn mul_assign(&mut self, other: &Self);

    /// Computes the multiplicative inverse of this element, if nonzero.
    fn inverse(&self) -> Option<Self>;

    /// Exponentiates this element by a power of the base prime modulus via
    /// the Frobenius automorphism.
    fn frobenius_map(&mut self, power: usize);

    /// Exponentiates this element by a number represented with `u64` limbs,
    /// least significant digit first.
    fn pow<S: AsRef<[u64]>>(&self, exp: S) -> Self {
        let mut res = Self::one();

        let mut found_one = false;

        for i in BitIterator::new(exp) {
            if found_one {
                res.square();
            } else {
                found_one = i;
            }

            if i {
                res.mul_assign(self);
            }
        }

        res
    }
}

/// This trait represents an element of a field that has a square root operation described for it.
pub trait SqrtField: Field {
    /// Returns the Legendre symbol of the field element.
    fn legendre(&self) -> LegendreSymbol;

    /// Returns the square root of the field element, if it is
    /// quadratic residue.
    fn sqrt(&self) -> Option<Self>;
}

/// This trait represents a wrapper around a biginteger which can encode any element of a particular
/// prime field. It is a smart wrapper around a sequence of `u64` limbs, least-significant digit
/// first.
pub trait PrimeFieldRepr:
    Sized
    + Copy
    + Clone
    + Eq
    + Ord
    + Send
    + Sync
    + Default
    + fmt::Debug
    + fmt::Display
    + 'static
    + rand::Rand
    + AsRef<[u64]>
    + AsMut<[u64]>
    + From<u64>
    + hash::Hash
    + serde::Serialize
    + serde::de::DeserializeOwned
{
    /// Subtract another represetation from this one.
    fn sub_noborrow(&mut self, other: &Self);

    /// Add another representation to this one.
    fn add_nocarry(&mut self, other: &Self);

    /// Compute the number of bits needed to encode this number. Always a
    /// multiple of 64.
    fn num_bits(&self) -> u32;

    /// Returns true iff this number is zero.
    fn is_zero(&self) -> bool;

    /// Returns true iff this number is odd.
    fn is_odd(&self) -> bool;

    /// Returns true iff this number is even.
    fn is_even(&self) -> bool;

    /// Performs a rightwise bitshift of this number, effectively dividing
    /// it by 2.
    fn div2(&mut self);

    /// Performs a rightwise bitshift of this number by some amount.
    fn shr(&mut self, amt: u32);

    /// Performs a leftwise bitshift of this number, effectively multiplying
    /// it by 2. Overflow is ignored.
    fn mul2(&mut self);

    /// Performs a leftwise bitshift of this number by some amount.
    fn shl(&mut self, amt: u32);

    /// Writes this `PrimeFieldRepr` as a big endian integer.
    fn write_be<W: Write>(&self, mut writer: W) -> io::Result<()> {
        use byteorder::{BigEndian, WriteBytesExt};

        for digit in self.as_ref().iter().rev() {
            writer.write_u64::<BigEndian>(*digit)?;
        }

        Ok(())
    }

    /// Reads a big endian integer into this representation.
    fn read_be<R: Read>(&mut self, mut reader: R) -> io::Result<()> {
        use byteorder::{BigEndian, ReadBytesExt};

        for digit in self.as_mut().iter_mut().rev() {
            *digit = reader.read_u64::<BigEndian>()?;
        }

        Ok(())
    }

    /// Writes this `PrimeFieldRepr` as a little endian integer.
    fn write_le<W: Write>(&self, mut writer: W) -> io::Result<()> {
        use byteorder::{LittleEndian, WriteBytesExt};

        for digit in self.as_ref().iter() {
            writer.write_u64::<LittleEndian>(*digit)?;
        }

        Ok(())
    }

    /// Reads a little endian integer into this representation.
    fn read_le<R: Read>(&mut self, mut reader: R) -> io::Result<()> {
        use byteorder::{LittleEndian, ReadBytesExt};

        for digit in self.as_mut().iter_mut() {
            *digit = reader.read_u64::<LittleEndian>()?;
        }

        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub enum LegendreSymbol {
    Zero = 0,
    QuadraticResidue = 1,
    QuadraticNonResidue = -1,
}

/// An error that may occur when trying to interpret a `PrimeFieldRepr` as a
/// `PrimeField` element.
#[derive(Debug)]
pub enum PrimeFieldDecodingError {
    /// The encoded value is not in the field
    NotInField(String),
}

impl Error for PrimeFieldDecodingError {
    fn description(&self) -> &str {
        match *self {
            PrimeFieldDecodingError::NotInField(..) => "not an element of the field",
        }
    }
}

impl fmt::Display for PrimeFieldDecodingError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            PrimeFieldDecodingError::NotInField(ref repr) => {
                write!(f, "{} is not an element of the field", repr)
            }
        }
    }
}

/// This represents an element of a prime field.
pub trait PrimeField: Field {
    /// The prime field can be converted back and forth into this biginteger
    /// representation.
    type Repr: PrimeFieldRepr + From<Self>;

    /// Interpret a string of numbers as a (congruent) prime field element.
    /// Does not accept unnecessary leading zeroes or a blank string.
    fn from_str(s: &str) -> Option<Self> {
        if s.is_empty() {
            return None;
        }

        if s == "0" {
            return Some(Self::zero());
        }

        let mut res = Self::zero();

        let ten = Self::from_repr(Self::Repr::from(10)).unwrap();

        let mut first_digit = true;

        for c in s.chars() {
            match c.to_digit(10) {
                Some(c) => {
                    if first_digit {
                        if c == 0 {
                            return None;
                        }

                        first_digit = false;
                    }

                    res.mul_assign(&ten);
                    res.add_assign(&Self::from_repr(Self::Repr::from(u64::from(c))).unwrap());
                }
                None => {
                    return None;
                }
            }
        }

        Some(res)
    }

    /// Convert this prime field element into a biginteger representation.
    fn from_repr(repr: Self::Repr) -> Result<Self, PrimeFieldDecodingError>;

    /// Creates an element from raw representation in Montgommery form.
    fn from_raw_repr(repr: Self::Repr) -> Result<Self, PrimeFieldDecodingError>;

    /// Convert a biginteger representation into a prime field element, if
    /// the number is an element of the field.
    fn into_repr(&self) -> Self::Repr;

    /// Expose Montgommery represendation.
    fn into_raw_repr(&self) -> Self::Repr;

    /// Returns the field characteristic; the modulus.
    fn char() -> Self::Repr;

    /// How many bits are needed to represent an element of this field.
    const NUM_BITS: u32;

    /// How many bits of information can be reliably stored in the field element.
    const CAPACITY: u32;

    /// Returns the multiplicative generator of `char()` - 1 order. This element
    /// must also be quadratic nonresidue.
    fn multiplicative_generator() -> Self;

    /// 2^s * t = `char()` - 1 with t odd.
    const S: u32;

    /// Returns the 2^s root of unity computed by exponentiating the `multiplicative_generator()`
    /// by t.
    fn root_of_unity() -> Self;
}

/// An "engine" is a collection of types (fields, elliptic curve groups, etc.)
/// with well-defined relationships. Specific relationships (for example, a
/// pairing-friendly curve) can be defined in a subtrait.
pub trait ScalarEngine: Sized + 'static + Clone + Copy + Send + Sync + fmt::Debug {
    /// This is the scalar field of the engine's groups.
    type Fr: PrimeField + SqrtField;
}

#[derive(Debug)]
pub struct BitIterator<E> {
    t: E,
    n: usize,
}

impl<E: AsRef<[u64]>> BitIterator<E> {
    pub fn new(t: E) -> Self {
        let n = t.as_ref().len() * 64;

        BitIterator { t, n }
    }
}

impl<E: AsRef<[u64]>> Iterator for BitIterator<E> {
    type Item = bool;

    fn next(&mut self) -> Option<bool> {
        if self.n == 0 {
            None
        } else {
            self.n -= 1;
            let part = self.n / 64;
            let bit = self.n - (64 * part);

            Some(self.t.as_ref()[part] & (1 << bit) > 0)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.n, Some(self.n))
    }
}

impl<E: AsRef<[u64]>> ExactSizeIterator for BitIterator<E> {
    fn len(&self) -> usize {
        self.n
    }
}

#[test]
fn test_bit_iterator() {
    let mut a = BitIterator::new([0xa953d79b83f6ab59, 0x6dea2059e200bd39]);
    let expected = "01101101111010100010000001011001111000100000000010111101001110011010100101010011110101111001101110000011111101101010101101011001";

    for e in expected.chars() {
        assert!(a.next().unwrap() == (e == '1'));
    }

    assert!(a.next().is_none());

    let expected = "1010010101111110101010000101101011101000011101110101001000011001100100100011011010001011011011010001011011101100110100111011010010110001000011110100110001100110011101101000101100011100100100100100001010011101010111110011101011000011101000111011011101011001";

    let mut a = BitIterator::new([
        0x429d5f3ac3a3b759,
        0xb10f4c66768b1c92,
        0x92368b6d16ecd3b4,
        0xa57ea85ae8775219,
    ]);

    for e in expected.chars() {
        assert!(a.next().unwrap() == (e == '1'));
    }

    assert!(a.next().is_none());
}


#[test]
fn test_bit_iterator_length() {
    let a = BitIterator::new([0xa953d79b83f6ab59, 0x6dea2059e200bd39]);
    let trusted_len = a.len();
    let (lower, some_upper) = a.size_hint();
    let upper = some_upper.unwrap();
    assert_eq!(trusted_len, 128);
    assert_eq!(lower, 128);
    assert_eq!(upper, 128);

    let mut i = 0;
    for _ in a {
        i += 1;
    }

    assert_eq!(trusted_len, i);
}

pub use self::arith_impl::*;

mod arith_impl {
    /// Calculate a - b - borrow, returning the result and modifying
    /// the borrow value.
    #[inline(always)]
    pub fn sbb(a: u64, b: u64, borrow: &mut u64) -> u64 {
        use std::num::Wrapping;

        let tmp = (1u128 << 64).wrapping_add(u128::from(a)).wrapping_sub(u128::from(b)).wrapping_sub(u128::from(*borrow));

        *borrow = if tmp >> 64 == 0 { 1 } else { 0 };

        tmp as u64
    }

    /// Calculate a + b + carry, returning the sum and modifying the
    /// carry value.
    #[inline(always)]
    pub fn adc(a: u64, b: u64, carry: &mut u64) -> u64 {
        use std::num::Wrapping;

        let tmp = u128::from(a).wrapping_add(u128::from(b)).wrapping_add(u128::from(*carry));

        *carry = (tmp >> 64) as u64;

        tmp as u64
    }

    /// Calculate a + (b * c) + carry, returning the least significant digit
    /// and setting carry to the most significant digit.
    #[inline(always)]
    pub fn mac_with_carry(a: u64, b: u64, c: u64, carry: &mut u64) -> u64 {
        use std::num::Wrapping;

        let tmp = (u128::from(a)).wrapping_add(u128::from(b).wrapping_mul(u128::from(c))).wrapping_add(u128::from(*carry));

        *carry = (tmp >> 64) as u64;

        tmp as u64
    }

    #[inline(always)]
    pub fn full_width_mul(a: u64, b: u64) -> (u64, u64) {
        let tmp = (a as u128) * (b as u128);

        return (tmp as u64, (tmp >> 64) as u64);
    }

    #[inline(always)]
    pub fn mac_by_value(a: u64, b: u64, c: u64) -> (u64, u64) {
        let tmp = ((b as u128) * (c as u128)) + (a as u128);

        (tmp as u64, (tmp >> 64) as u64)
    }

    #[inline(always)]
    pub fn mac_by_value_return_carry_only(a: u64, b: u64, c: u64) -> u64 {
        let tmp = ((b as u128) * (c as u128)) + (a as u128);

        (tmp >> 64) as u64
    }

    #[inline(always)]
    pub fn mac_with_carry_by_value(a: u64, b: u64, c: u64, carry: u64) -> (u64, u64) {
        let tmp = ((b as u128) * (c as u128)) + (a as u128) + (carry as u128);

        (tmp as u64, (tmp >> 64) as u64)
    }

    #[inline(always)]
    pub fn mul_double_add_by_value(a: u64, b: u64, c: u64) -> (u64, u64, u64) {
        // multiply
        let tmp = (b as u128) * (c as u128);
        // doulbe
        let lo = tmp as u64;
        let hi = (tmp >> 64) as u64;
        let superhi = hi >> 63;
        let hi = hi << 1 | lo >> 63;
        let lo = lo << 1;
        // add
        let tmp = (lo as u128) + ((hi as u128) << 64) + (a as u128);

        (tmp as u64, (tmp >> 64) as u64, superhi)
    }

    #[inline(always)]
    pub fn mul_double_add_add_carry_by_value(a: u64, b: u64, c: u64, carry: u64) -> (u64, u64, u64) {
        // multiply
        let tmp = (b as u128) * (c as u128);
        // doulbe
        let lo = tmp as u64;
        let hi = (tmp >> 64) as u64;
        let superhi = hi >> 63;
        let hi = hi << 1 | lo >> 63;
        let lo = lo << 1;
        // add
        let tmp = (lo as u128) + ((hi as u128) << 64) + (a as u128) + (carry as u128);

        (tmp as u64, (tmp >> 64) as u64, superhi)
    }

    #[inline(always)]
    pub fn mul_double_add_add_carry_by_value_ignore_superhi(a: u64, b: u64, c: u64, carry: u64) -> (u64, u64) {
        // multiply
        let tmp = (b as u128) * (c as u128);
        // doulbe
        let lo = tmp as u64;
        let hi = (tmp >> 64) as u64;
        let hi = hi << 1 | lo >> 63;
        let lo = lo << 1;
        // add
        let tmp = (lo as u128) + ((hi as u128) << 64) + (a as u128) + (carry as u128);

        (tmp as u64, (tmp >> 64) as u64)
    }

    #[inline(always)]
    pub fn mul_double_add_low_and_high_carry_by_value(b: u64, c: u64, lo_carry: u64, hi_carry: u64) -> (u64, u64, u64) {
        // multiply
        let tmp = (b as u128) * (c as u128);
        // doulbe
        let lo = tmp as u64;
        let hi = (tmp >> 64) as u64;
        let superhi = hi >> 63;
        let hi = hi << 1 | lo >> 63;
        let lo = lo << 1;
        // add
        let tmp = (lo as u128) + ((hi as u128) << 64) + (lo_carry as u128) + ((hi_carry as u128) << 64);

        (tmp as u64, (tmp >> 64) as u64, superhi)
    }

    #[inline(always)]
    pub fn mul_double_add_low_and_high_carry_by_value_ignore_superhi(b: u64, c: u64, lo_carry: u64, hi_carry: u64) -> (u64, u64) {
        // multiply
        let tmp = (b as u128) * (c as u128);
        // doulbe
        let lo = tmp as u64;
        let hi = (tmp >> 64) as u64;
        let hi = hi << 1 | lo >> 63;
        let lo = lo << 1;
        // add
        let tmp = (lo as u128) + ((hi as u128) << 64) + (lo_carry as u128) + ((hi_carry as u128) << 64);

        (tmp as u64, (tmp >> 64) as u64)
    }

    #[inline(always)]
    pub fn mul_double_add_add_low_and_high_carry_by_value(a: u64, b: u64, c: u64, lo_carry: u64, hi_carry: u64) -> (u64, u64, u64) {
        // multiply
        let tmp = (b as u128) * (c as u128);
        // doulbe
        let lo = tmp as u64;
        let hi = (tmp >> 64) as u64;
        let superhi = hi >> 63;
        let hi = hi << 1 | lo >> 63;
        let lo = lo << 1;
        // add
        let tmp = (lo as u128) + ((hi as u128) << 64) + (a as u128) + (lo_carry as u128) + ((hi_carry as u128) << 64);

        (tmp as u64, (tmp >> 64) as u64, superhi)
    }

    #[inline(always)]
    pub fn mul_double_add_add_low_and_high_carry_by_value_ignore_superhi(a: u64, b: u64, c: u64, lo_carry: u64, hi_carry: u64) -> (u64, u64) {
        // multiply
        let tmp = (b as u128) * (c as u128);
        // doulbe
        let lo = tmp as u64;
        let hi = (tmp >> 64) as u64;
        let hi = hi << 1 | lo >> 63;
        let lo = lo << 1;
        // add
        let tmp = (lo as u128) + ((hi as u128) << 64) + (a as u128) + (lo_carry as u128) + ((hi_carry as u128) << 64);

        (tmp as u64, (tmp >> 64) as u64)
    }

    #[inline(always)]
    pub fn mac_with_low_and_high_carry_by_value(a: u64, b: u64, c: u64, lo_carry: u64, hi_carry: u64) -> (u64, u64) {
        let tmp = ((b as u128) * (c as u128)) + (a as u128) + (lo_carry as u128) + ((hi_carry as u128) << 64);

        (tmp as u64, (tmp >> 64) as u64)
    }
}

pub use to_hex::{to_hex, from_hex};

mod to_hex {
    use super::{PrimeField, PrimeFieldRepr, hex_ext};
    use std::prelude::v1::*;

    pub fn to_hex<F: PrimeField>(el: &F) -> String {
        let repr = el.into_repr();
        let required_length = repr.as_ref().len() * 8;
        let mut buf: Vec<u8> = Vec::with_capacity(required_length);
        repr.write_be(&mut buf).unwrap();

        hex_ext::encode(&buf)
    }

    pub fn from_hex<F: PrimeField>(value: &str) -> Result<F, String> {
        let value = if value.starts_with("0x") { &value[2..] } else { value };
        if value.len() % 2 != 0 {return Err(format!("hex length must be even for full byte encoding: {}", value))}
        let mut buf = hex_ext::decode(&value).map_err(|_| format!("could not decode hex: {}", value))?;
        let mut repr = F::Repr::default();
        let required_length = repr.as_ref().len() * 8;
        buf.reverse();
        buf.resize(required_length, 0);

        repr.read_le(&buf[..]).map_err(|e| format!("could not read {}: {}", value, &e))?;

        F::from_repr(repr).map_err(|e| format!("could not convert into prime field: {}: {}", value, &e))
    }
}