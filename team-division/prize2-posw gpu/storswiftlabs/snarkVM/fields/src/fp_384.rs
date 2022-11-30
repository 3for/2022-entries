// Copyright (C) 2019-2022 Aleo Systems Inc.
// This file is part of the snarkVM library.

// The snarkVM library is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The snarkVM library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with the snarkVM library. If not, see <https://www.gnu.org/licenses/>.

use crate::{
    impl_add_sub_from_field_ref,
    impl_mul_div_from_field_ref,
    FftField,
    Field,
    FieldError,
    FieldParameters,
    LegendreSymbol,
    One,
    PoseidonDefaultField,
    PoseidonDefaultParameters,
    PrimeField,
    SquareRootField,
    Zero,
};
use snarkvm_utilities::{
    biginteger::{arithmetic as fa, BigInteger as _BigInteger, BigInteger384 as BigInteger},
    serialize::CanonicalDeserialize,
    FromBytes,
    ToBits,
    ToBytes,
};

use std::{
    cmp::{Ord, Ordering, PartialOrd},
    fmt::{Debug, Display, Formatter, Result as FmtResult},
    io::{Read, Result as IoResult, Write},
    marker::PhantomData,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
    str::FromStr,
};

pub trait Fp384Parameters: FieldParameters<BigInteger = BigInteger> {}

#[derive(Derivative)]
#[derivative(
    Default(bound = "P: Fp384Parameters"),
    Hash(bound = "P: Fp384Parameters"),
    Clone(bound = "P: Fp384Parameters"),
    Copy(bound = "P: Fp384Parameters"),
    PartialEq(bound = "P: Fp384Parameters"),
    Eq(bound = "P: Fp384Parameters")
)]
pub struct Fp384<P: Fp384Parameters>(
    pub BigInteger,
    #[derivative(Debug = "ignore")]
    #[doc(hidden)]
    pub PhantomData<P>,
);

impl<P: Fp384Parameters> Fp384<P> {
    #[inline]
    pub fn new(element: BigInteger) -> Self {
        Fp384::<P>(element, PhantomData)
    }

    #[inline(always)]
    pub fn is_valid(&self) -> bool {
        self.0 < P::MODULUS
    }

    #[inline(always)]
    fn reduce(&mut self) {
        if !self.is_valid() {
            self.0.sub_noborrow(&P::MODULUS);
        }
    }
    
    #[allow(unused)]
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    fn mont_reduce(
        &mut self,
        r0: u64,
        mut r1: u64,
        mut r2: u64,
        mut r3: u64,
        mut r4: u64,
        mut r5: u64,
        mut r6: u64,
        mut r7: u64,
        mut r8: u64,
        mut r9: u64,
        mut r10: u64,
        mut r11: u64,
    ) {
        // The Montgomery reduction here is based on Algorithm 14.32 in
        // Handbook of Applied Cryptography
        // <http://cacr.uwaterloo.ca/hac/about/chap14.pdf>.

        let k = r0.wrapping_mul(P::INV);
        let mut carry = 0;
        fa::mac_with_carry(r0, k, P::MODULUS.0[0], &mut carry);
        r1 = fa::mac_with_carry(r1, k, P::MODULUS.0[1], &mut carry);
        r2 = fa::mac_with_carry(r2, k, P::MODULUS.0[2], &mut carry);
        r3 = fa::mac_with_carry(r3, k, P::MODULUS.0[3], &mut carry);
        r4 = fa::mac_with_carry(r4, k, P::MODULUS.0[4], &mut carry);
        r5 = fa::mac_with_carry(r5, k, P::MODULUS.0[5], &mut carry);
        carry = fa::adc(&mut r6, 0, carry);
        let carry2 = carry;
        let k = r1.wrapping_mul(P::INV);
        let mut carry = 0;
        fa::mac_with_carry(r1, k, P::MODULUS.0[0], &mut carry);
        r2 = fa::mac_with_carry(r2, k, P::MODULUS.0[1], &mut carry);
        r3 = fa::mac_with_carry(r3, k, P::MODULUS.0[2], &mut carry);
        r4 = fa::mac_with_carry(r4, k, P::MODULUS.0[3], &mut carry);
        r5 = fa::mac_with_carry(r5, k, P::MODULUS.0[4], &mut carry);
        r6 = fa::mac_with_carry(r6, k, P::MODULUS.0[5], &mut carry);
        carry = fa::adc(&mut r7, carry2, carry);
        let carry2 = carry;
        let k = r2.wrapping_mul(P::INV);
        let mut carry = 0;
        fa::mac_with_carry(r2, k, P::MODULUS.0[0], &mut carry);
        r3 = fa::mac_with_carry(r3, k, P::MODULUS.0[1], &mut carry);
        r4 = fa::mac_with_carry(r4, k, P::MODULUS.0[2], &mut carry);
        r5 = fa::mac_with_carry(r5, k, P::MODULUS.0[3], &mut carry);
        r6 = fa::mac_with_carry(r6, k, P::MODULUS.0[4], &mut carry);
        r7 = fa::mac_with_carry(r7, k, P::MODULUS.0[5], &mut carry);
        carry = fa::adc(&mut r8, carry2, carry);
        let carry2 = carry;
        let k = r3.wrapping_mul(P::INV);
        let mut carry = 0;
        fa::mac_with_carry(r3, k, P::MODULUS.0[0], &mut carry);
        r4 = fa::mac_with_carry(r4, k, P::MODULUS.0[1], &mut carry);
        r5 = fa::mac_with_carry(r5, k, P::MODULUS.0[2], &mut carry);
        r6 = fa::mac_with_carry(r6, k, P::MODULUS.0[3], &mut carry);
        r7 = fa::mac_with_carry(r7, k, P::MODULUS.0[4], &mut carry);
        r8 = fa::mac_with_carry(r8, k, P::MODULUS.0[5], &mut carry);
        carry = fa::adc(&mut r9, carry2, carry);
        let carry2 = carry;
        let k = r4.wrapping_mul(P::INV);
        let mut carry = 0;
        fa::mac_with_carry(r4, k, P::MODULUS.0[0], &mut carry);
        r5 = fa::mac_with_carry(r5, k, P::MODULUS.0[1], &mut carry);
        r6 = fa::mac_with_carry(r6, k, P::MODULUS.0[2], &mut carry);
        r7 = fa::mac_with_carry(r7, k, P::MODULUS.0[3], &mut carry);
        r8 = fa::mac_with_carry(r8, k, P::MODULUS.0[4], &mut carry);
        r9 = fa::mac_with_carry(r9, k, P::MODULUS.0[5], &mut carry);
        carry = fa::adc(&mut r10, carry2, carry);
        let carry2 = carry;
        let k = r5.wrapping_mul(P::INV);
        let mut carry = 0;
        fa::mac_with_carry(r5, k, P::MODULUS.0[0], &mut carry);
        r6 = fa::mac_with_carry(r6, k, P::MODULUS.0[1], &mut carry);
        r7 = fa::mac_with_carry(r7, k, P::MODULUS.0[2], &mut carry);
        r8 = fa::mac_with_carry(r8, k, P::MODULUS.0[3], &mut carry);
        r9 = fa::mac_with_carry(r9, k, P::MODULUS.0[4], &mut carry);
        r10 = fa::mac_with_carry(r10, k, P::MODULUS.0[5], &mut carry);
        fa::adc(&mut r11, carry2, carry);
        (self.0).0[0] = r6;
        (self.0).0[1] = r7;
        (self.0).0[2] = r8;
        (self.0).0[3] = r9;
        (self.0).0[4] = r10;
        (self.0).0[5] = r11;
        self.reduce();
    }
}

impl<P: Fp384Parameters> Zero for Fp384<P> {
    #[inline]
    fn zero() -> Self {
        Fp384::<P>(BigInteger::from(0), PhantomData)
    }

    #[inline]
    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}

impl<P: Fp384Parameters> One for Fp384<P> {
    #[inline]
    fn one() -> Self {
        Fp384::<P>(P::R, PhantomData)
    }

    #[inline]
    fn is_one(&self) -> bool {
        self.0 == P::R
    }
}

impl<P: Fp384Parameters> Field for Fp384<P> {
    type BasePrimeField = Self;

    // 384/64 = 6 limbs.
    impl_field_from_random_bytes_with_flags!(6);

    fn from_base_prime_field(other: Self::BasePrimeField) -> Self {
        other
    }

    fn half() -> Self {
        // Compute 1/2 `(p+1)/2` as `1/2`.
        // This is cheaper than `Self::one().double().inverse()`
        let mut two_inv = P::MODULUS;
        two_inv.add_nocarry(&1u64.into());
        two_inv.div2();
        Self::from_repr(two_inv).unwrap() // Guaranteed to be valid.
    }

    #[inline(always)]
    fn double(&self) -> Self {
        let mut temp = *self;
        temp.double_in_place();
        temp
    }

    #[inline(always)]
    fn double_in_place(&mut self) {
        // This cannot exceed the backing capacity.
        self.0.mul2();
        // However, it may need to be reduced.
        self.reduce();
    }

    #[inline]
    fn characteristic<'a>() -> &'a [u64] {
        P::MODULUS.as_ref()
    }

    #[inline(always)]
    fn square(&self) -> Self {
        let mut temp = *self;
        temp.square_in_place();
        temp
    }

    // #[cfg(feature = "unadx")]
    #[cfg(not(feature = "unadx"))]
    #[inline(always)]
    fn square_in_place(&mut self) -> &mut Self {
        let mut carry = 0;
        let r1 = fa::mac_with_carry(0, (self.0).0[0], (self.0).0[1], &mut carry);
        let r2 = fa::mac_with_carry(0, (self.0).0[0], (self.0).0[2], &mut carry);
        let r3 = fa::mac_with_carry(0, (self.0).0[0], (self.0).0[3], &mut carry);
        let r4 = fa::mac_with_carry(0, (self.0).0[0], (self.0).0[4], &mut carry);
        let r5 = fa::mac_with_carry(0, (self.0).0[0], (self.0).0[5], &mut carry);
        let r6 = carry;
        let mut carry = 0;
        let r3 = fa::mac_with_carry(r3, (self.0).0[1], (self.0).0[2], &mut carry);
        let r4 = fa::mac_with_carry(r4, (self.0).0[1], (self.0).0[3], &mut carry);
        let r5 = fa::mac_with_carry(r5, (self.0).0[1], (self.0).0[4], &mut carry);
        let r6 = fa::mac_with_carry(r6, (self.0).0[1], (self.0).0[5], &mut carry);
        let r7 = carry;
        let mut carry = 0;
        let r5 = fa::mac_with_carry(r5, (self.0).0[2], (self.0).0[3], &mut carry);
        let r6 = fa::mac_with_carry(r6, (self.0).0[2], (self.0).0[4], &mut carry);
        let r7 = fa::mac_with_carry(r7, (self.0).0[2], (self.0).0[5], &mut carry);
        let r8 = carry;
        let mut carry = 0;
        let r7 = fa::mac_with_carry(r7, (self.0).0[3], (self.0).0[4], &mut carry);
        let r8 = fa::mac_with_carry(r8, (self.0).0[3], (self.0).0[5], &mut carry);
        let r9 = carry;
        let mut carry = 0;
        let r9 = fa::mac_with_carry(r9, (self.0).0[4], (self.0).0[5], &mut carry);
        let r10 = carry;

        let mut r11 = r10 >> 63;
        let r10 = (r10 << 1) | (r9 >> 63);
        let mut r9 = (r9 << 1) | (r8 >> 63);
        let r8 = (r8 << 1) | (r7 >> 63);
        let mut r7 = (r7 << 1) | (r6 >> 63);
        let r6 = (r6 << 1) | (r5 >> 63);
        let mut r5 = (r5 << 1) | (r4 >> 63);
        let r4 = (r4 << 1) | (r3 >> 63);
        let mut r3 = (r3 << 1) | (r2 >> 63);
        let r2 = (r2 << 1) | (r1 >> 63);
        let mut r1 = r1 << 1;

        let mut carry = 0;
        let r0 = fa::mac_with_carry(0, (self.0).0[0], (self.0).0[0], &mut carry);
        carry = fa::adc(&mut r1, 0, carry);
        let r2 = fa::mac_with_carry(r2, (self.0).0[1], (self.0).0[1], &mut carry);
        carry = fa::adc(&mut r3, 0, carry);
        let r4 = fa::mac_with_carry(r4, (self.0).0[2], (self.0).0[2], &mut carry);
        carry = fa::adc(&mut r5, 0, carry);
        let r6 = fa::mac_with_carry(r6, (self.0).0[3], (self.0).0[3], &mut carry);
        carry = fa::adc(&mut r7, 0, carry);
        let r8 = fa::mac_with_carry(r8, (self.0).0[4], (self.0).0[4], &mut carry);
        carry = fa::adc(&mut r9, 0, carry);
        let r10 = fa::mac_with_carry(r10, (self.0).0[5], (self.0).0[5], &mut carry);
        fa::adc(&mut r11, 0, carry);
        self.mont_reduce(r0, r1, r2, r3, r4, r5, r6, r7, r8, r9, r10, r11);
        self
    }

    // #[cfg(not(feature = "unadx"))]
    #[cfg(feature = "unadx")]
    #[inline(always)]
    fn square_in_place(&mut self) -> &mut Self {
        let mut r = [0u64;4]; // r8 ~ r11

        unsafe {
            std::arch::asm!(
                // Square Ops
                "mov (%rax), %rdx               \n\t",
                "mulx 0x8(%rax), {R1}, {C1}     \n\t",
                "mulx 0x10(%rax), {R2}, {C2}    \n\t",
                "add {C1}, {R2}                 \n\t",
                "mulx 0x18(%rax), {R3}, {C1}    \n\t",
                "adc {C2}, {R3}                 \n\t",
                "mulx 0x20(%rax), {R4}, {C2}    \n\t",
                "adc {C1}, {R4}                 \n\t",
                "mulx 0x28(%rax), {R5}, {R6}    \n\t",
                "adc {C2}, {R5}                 \n\t",
                "adc $0, {R6}                   \n\t",

                "xor %rdx, %rdx                 \n\t",
                "mov 0x8(%rax), %rdx            \n\t",
                "mulx 0x10(%rax), {C2}, {C1}    \n\t",
                "adcx {C2}, {R3}                \n\t",
                "mulx 0x18(%rax), {C2}, {C3}    \n\t",
                "adcx {C1}, {R4}                \n\t",
                "adox {C2}, {R4}                \n\t",
                "mulx 0x20(%rax), {C2}, {C1}    \n\t",
                "adcx {C2}, {R5}                \n\t",
                "adox {C3}, {R5}                \n\t",
                "mulx 0x28(%rax), {C2}, {R7}    \n\t",
                "adcx {C1}, {R6}                \n\t",
                "adox {C2}, {R6}                \n\t",
                "mov $0, {C3}                   \n\t",
                "adox {C3}, {R7}                \n\t",
                "adcx {C3}, {R7}                \n\t",

                "xor %rdx, %rdx                 \n\t",
                "mov 0x10(%rax), %rdx           \n\t",
                "mulx 0x18(%rax), {C2}, {C1}    \n\t",
                "adcx {C2}, {R5}                \n\t",
                "adcx {C1}, {R6}                \n\t",
                "mulx 0x20(%rax), {C2}, {C1}    \n\t",
                "adox {C2}, {R6}                \n\t",
                "adcx {C1}, {R7}                \n\t",
                "mulx 0x28(%rax), {C2}, {C3}    \n\t", // R8 - C3
                "adox {C2}, {R7}                \n\t",
                "mov $0, {C1}                   \n\t",
                "adox {C1}, {C3}                \n\t",
                "adcx {C1}, {C3}                \n\t",

                "xor %rdx, %rdx                 \n\t",
                "mov 0x18(%rax), %rdx           \n\t",
                "mulx 0x20(%rax), {C2}, {C1}    \n\t",
                "adcx {C2}, {R7}                \n\t",
                "adcx {C1}, {C3}                \n\t", // R8 - C3
                "mulx 0x28(%rax), {C1}, {C2}    \n\t", // R9 - C2
                "adox {C1}, {C3}                \n\t",
                "mov $0, {C1}                   \n\t", // R9 - C2
                "adox {C1}, {C2}                \n\t",
                "adcx {C1}, {C2}                \n\t",

                "mov 0x20(%rax), %rdx           \n\t",
                "mulx 0x28(%rax), {C1}, %rdx    \n\t", // R10 - rdx
                "add {C1}, {C2}                 \n\t",
                "adc $0, %rdx                   \n\t",

                // Shift Ops
                "mov %rdx, {C1}                 \n\t", // r11 - C1
                "shld $0x1, {C2}, %rdx          \n\t", // r10 - rdx
                "shld $0x1, {C3}, {C2}          \n\t", // R9  - C2
                "shld $0x1, {R7}, {C3}          \n\t", // R8  - C3
                "shld $0x1, {R6}, {R7}          \n\t",
                "shld $0x1, {R5}, {R6}          \n\t",
                "shld $0x1, {R4}, {R5}          \n\t",
                "shld $0x1, {R3}, {R4}          \n\t",
                "shld $0x1, {R2}, {R3}          \n\t",
                "shld $0x1, {R1}, {R2}          \n\t",
                "shr $0x3f, {C1}                \n\t", // r11 - C1
                "add {R1}, {R1}                 \n\t",
                "mov %rdx, 0x10(%rdi)           \n\t", // r10 - store
                "mov {C1}, 0x18(%rdi)           \n\t", // r11 - store

                // MAC Ops
                "mov (%rax), %rdx               \n\t",
                "mulx (%rax), {R0}, {C1}        \n\t",
                "add {C1}, {R1}                 \n\t",
                "mov 0x8(%rax), %rdx            \n\t",
                "mulx 0x8(%rax), %rdx, {C1}     \n\t",
                "adc %rdx, {R2}                 \n\t",
                "adc {C1}, {R3}                 \n\t",
                "mov 0x10(%rax), %rdx           \n\t",
                "mulx 0x10(%rax), %rdx, {C1}    \n\t",
                "adc %rdx, {R4}                 \n\t",
                "adc {C1}, {R5}                 \n\t",
                "mov 0x18(%rax), %rdx           \n\t",
                "mulx 0x18(%rax), %rdx, {C1}    \n\t",
                "adc %rdx, {R6}                 \n\t",
                "adc {C1}, {R7}                 \n\t",
                "mov 0x20(%rax), %rdx           \n\t",
                "mulx 0x20(%rax), %rdx, {C1}    \n\t",
                "adc %rdx, {C3}                 \n\t", // R8 - C3
                "adc {C1}, {C2}                 \n\t", // R9 - C2
                "mov 0x28(%rax), %rdx           \n\t",
                "mulx 0x28(%rax), %rdx, {C1}    \n\t",
                "adc %rdx, 0x10(%rdi)           \n\t", // r10
                "adc {C1}, 0x18(%rdi)           \n\t", // r11
                "mov {C3}, (%rdi)               \n\t", // r8 - store
                "mov {C2}, 0x8(%rdi)            \n\t", // r9 - store


                // Montgomery Reduction
                // Iteration 0
                "xor %rdx, %rdx                 \n\t",
                "movabs $0x8508bfffffffffff, %rdx       \n\t",
                //"mov ${INV}, %rdx               \n\t",
                "mulx {R0}, %rdx, {C1}          \n\t",  // wrapping_mul
                "mulx (%rsi), {C2}, {C1}        \n\t",
                "adcx {R0}, {C2}                \n\t",
                "mulx 0x8(%rsi), {R0}, {C2}     \n\t",  // r1 - R0
                "adcx {R1}, {R0}                \n\t",
                "adox {C1}, {R0}                \n\t",
                "mulx 0x10(%rsi), {R1}, {C1}    \n\t",  // r2 - R1
                "adcx {R2}, {R1}                \n\t",
                "adox {C2}, {R1}                \n\t",
                "mulx 0x18(%rsi), {R2}, {C2}    \n\t",  // r3 - R2
                "adcx {R3}, {R2}                \n\t",
                "adox {C1}, {R2}                \n\t",
                "mulx 0x20(%rsi), {R3}, {C1}    \n\t",  // r4 - R3
                "adcx {R4}, {R3}                \n\t",
                "adox {C2}, {R3}                \n\t",
                "adcx {R5}, {C1}                \n\t",
                "mulx 0x28(%rsi), {R4}, {R5}    \n\t",  // r5 - R4
                "adox {C1}, {R4}                \n\t",
                "adcx {R6}, {R5}                \n\t",  // r6 - R5
                "mov $0, {C3}                   \n\t",  // carry2 - C3
                "adox {C3}, {R5}                \n\t",
                "adox {C3}, {C3}                \n\t",
                "adc $0, {C3}                   \n\t",

                // Iteration 1
                "xor %rdx, %rdx                 \n\t",
                "movabs $0x8508bfffffffffff, %rdx       \n\t",
                //"mov ${INV}, %rdx               \n\t",
                "mulx {R0}, %rdx, {C1}          \n\t",  // wrapping_mul
                "mulx (%rsi), {C2}, {C1}        \n\t",
                "adcx {C2}, {R0}                \n\t",
                "mulx 0x8(%rsi), {R0}, {C2}     \n\t",  // r2 - R0
                "adcx {R1}, {R0}                \n\t",
                "adox {C1}, {R0}                \n\t",
                "mulx 0x10(%rsi), {R1}, {C1}    \n\t",  // r3 - R1
                "adcx {R2}, {R1}                \n\t",
                "adox {C2}, {R1}                \n\t",
                "mulx 0x18(%rsi), {R2}, {C2}    \n\t",  // r4 - R2
                "adcx {R3}, {R2}                \n\t",
                "adox {C1}, {R2}                \n\t",
                "mulx 0x20(%rsi), {R3}, {C1}    \n\t",  // r5 - R3
                "adcx {R4}, {R3}                \n\t",
                "adox {C2}, {R3}                \n\t",
                "adcx {R5}, {C1}                \n\t",
                "mulx 0x28(%rsi), {R4}, {R5}    \n\t",  // r6 - R4
                "adox {C1}, {R4}                \n\t",
                "adcx {R7}, {R5}                \n\t",  // r7 - R5
                "adox {C3}, {R5}                \n\t",
                "mov $0, {C3}                   \n\t",  // carry2 - C3
                "adox {C3}, {C3}                \n\t",
                "adc $0, {C3}                   \n\t",  // Flags Affected: The OF, SF, ZF, AF, CF, and PF flags are set according to the result.

                // Iteration 2
                //"xor %rdx, %rdx                 \n\t",
                "movabs $0x8508bfffffffffff, %rdx       \n\t",
                //"mov ${INV}, %rdx               \n\t",
                "mulx {R0}, %rdx, {C1}          \n\t",  // wrapping_mul
                "mulx (%rsi), {C2}, {C1}        \n\t",
                "adcx {C2}, {R0}                \n\t",
                "mulx 0x8(%rsi), {R0}, {C2}     \n\t",  // r3 - R0
                "adcx {R1}, {R0}                \n\t",
                "adox {C1}, {R0}                \n\t",
                "mulx 0x10(%rsi), {R1}, {C1}    \n\t",  // r4 - R1
                "adcx {R2}, {R1}                \n\t",
                "adox {C2}, {R1}                \n\t",
                "mulx 0x18(%rsi), {R2}, {C2}    \n\t",  // r5 - R2
                "adcx {R3}, {R2}                \n\t",
                "adox {C1}, {R2}                \n\t",
                "mulx 0x20(%rsi), {R3}, {C1}    \n\t",  // r6 - R3
                "adcx {R4}, {R3}                \n\t",
                "adox {C2}, {R3}                \n\t",
                "adcx {R5}, {C1}                \n\t",
                "mulx 0x28(%rsi), {R4}, {R5}    \n\t",  // r7 - R4
                "adox {C1}, {R4}                \n\t",
                "adcx (%rdi), {R5}              \n\t",  // r8 - R5
                "adox {C3}, {R5}                \n\t",
                "mov $0, {C3}                   \n\t",  // carry2 - C3
                "adox {C3}, {C3}                \n\t",
                "adc $0, {C3}                   \n\t",

                // Iteration 3
                //"xor %rdx, %rdx                 \n\t",
                "movabs $0x8508bfffffffffff, %rdx       \n\t",
                //"mov ${INV}, %rdx               \n\t",
                "mulx {R0}, %rdx, {C1}          \n\t",  // wrapping_mul
                "mulx (%rsi), {C2}, {C1}        \n\t",
                "adcx {C2}, {R0}                \n\t",
                "mulx 0x8(%rsi), {R0}, {C2}     \n\t",  // r4 - R0
                "adcx {R1}, {R0}                \n\t",
                "adox {C1}, {R0}                \n\t",
                "mulx 0x10(%rsi), {R1}, {C1}    \n\t",  // r5 - R1
                "adcx {R2}, {R1}                \n\t",
                "adox {C2}, {R1}                \n\t",
                "mulx 0x18(%rsi), {R2}, {C2}    \n\t",  // r6 - R2
                "adcx {R3}, {R2}                \n\t",
                "adox {C1}, {R2}                \n\t",
                "mulx 0x20(%rsi), {R3}, {C1}    \n\t",  // r7 - R3
                "adcx {R4}, {R3}                \n\t",
                "adox {C2}, {R3}                \n\t",
                "adcx {R5}, {C1}                \n\t",
                "mulx 0x28(%rsi), {R4}, {R5}    \n\t",  // r8 - R4
                "adox {C1}, {R4}                \n\t",
                "adcx 0x8(%rdi), {R5}           \n\t",  // r9 - R5
                "adox {C3}, {R5}                \n\t",
                "mov $0, {C3}                   \n\t",  // carry2 - C3
                "adox {C3}, {C3}                \n\t",
                "adc $0, {C3}                   \n\t",

                // Iteration 4
                //"xor %rdx, %rdx                 \n\t",
                "movabs $0x8508bfffffffffff, %rdx       \n\t",
                //"mov ${INV}, %rdx               \n\t",
                "mulx {R0}, %rdx, {C1}          \n\t",  // wrapping_mul
                "mulx (%rsi), {C2}, {C1}        \n\t",
                "adcx {C2}, {R0}                \n\t",
                "mulx 0x8(%rsi), {R0}, {C2}     \n\t",  // r5 - R0
                "adcx {R1}, {R0}                \n\t",
                "adox {C1}, {R0}                \n\t",
                "mulx 0x10(%rsi), {R1}, {C1}    \n\t",  // r6 - R1
                "adcx {R2}, {R1}                \n\t",
                "adox {C2}, {R1}                \n\t",
                "mulx 0x18(%rsi), {R2}, {C2}    \n\t",  // r7 - R2
                "adcx {R3}, {R2}                \n\t",
                "adox {C1}, {R2}                \n\t",
                "mulx 0x20(%rsi), {R3}, {C1}    \n\t",  // r8 - R3
                "adcx {R4}, {R3}                \n\t",
                "adox {C2}, {R3}                \n\t",
                "adcx {R5}, {C1}                \n\t",
                "mulx 0x28(%rsi), {R4}, {R5}    \n\t",  // r9 - R4
                "adox {C1}, {R4}                \n\t",
                "adcx 0x10(%rdi), {R5}          \n\t",  // r10 - R5
                "adox {C3}, {R5}                \n\t",
                "mov $0, {C3}                   \n\t",  // carry2 - C3
                "adox {C3}, {C3}                \n\t",
                "adc $0, {C3}                   \n\t",

                // Iteration 5
                //"xor %rdx, %rdx                 \n\t",
                "movabs $0x8508bfffffffffff, %rdx       \n\t",
                //"mov ${INV}, %rdx               \n\t",
                "mulx {R0}, %rdx, {C1}          \n\t",  // wrapping_mul
                "mulx (%rsi), {C2}, {C1}        \n\t",
                "adcx {C2}, {R0}                \n\t",
                "mulx 0x8(%rsi), {R0}, {C2}     \n\t",  // r6 - R0
                "adcx {R1}, {R0}                \n\t",
                "adox {C1}, {R0}                \n\t",
                "mulx 0x10(%rsi), {R1}, {C1}    \n\t",  // r7 - R1
                "adcx {R2}, {R1}                \n\t",
                "adox {C2}, {R1}                \n\t",
                "mulx 0x18(%rsi), {R2}, {C2}    \n\t",  // r8 - R2
                "adcx {R3}, {R2}                \n\t",
                "adox {C1}, {R2}                \n\t",
                "mulx 0x20(%rsi), {R3}, {C1}    \n\t",  // r9 - R3
                "adcx {R4}, {R3}                \n\t",
                "adox {C2}, {R3}                \n\t",
                "adcx {R5}, {C1}                \n\t",
                "mulx 0x28(%rsi), {R4}, {R5}    \n\t",  // r10 - R4
                "adox {C1}, {R4}                \n\t",
                "adcx 0x18(%rdi), {R5}          \n\t",  // r11 - R5
                "adox {C3}, {R5}                \n\t",

                "mov {R0}, (%rax)               \n\t",
                "mov {R1}, 0x8(%rax)            \n\t",
                "mov {R2}, 0x10(%rax)           \n\t",
                "mov {R3}, 0x18(%rax)           \n\t",
                "mov {R4}, 0x20(%rax)           \n\t",
                "mov {R5}, 0x28(%rax)           \n\t",

                R0 = out(reg) _,
                R1 = out(reg) _,
                R2 = out(reg) _,
                R3 = out(reg) _,
                R4 = out(reg) _,
                R5 = out(reg) _,
                R6 = out(reg) _,
                R7 = out(reg) _,
                C1 = out(reg) _,    // C1 - R8
                C2 = out(reg) _,    // C2 - R9
                C3 = out(reg) _,    // C3 - R10
                //INV = const P::INV,
                out("rdx") _,
                inout("rsi") P::MODULUS.0.as_ptr() => _,    // Modulus.0
                inout("rdi") r.as_mut_ptr() => _,           // Params: r10 ~ r11
                inout("rax") (self.0).0.as_mut_ptr() => _,  // Result
                options(nostack, att_syntax)
            );
        }

        self.reduce();
        self
    }

    #[inline(always)]
    fn inverse(&self) -> Option<Self> {
        if self.is_zero() {
            None
        } else {
            // Guajardo Kumar Paar Pelzl
            // Efficient Software-Implementation of Finite Fields with Applications to
            // Cryptography
            // Algorithm 16 (BEA for Inversion in Fp)

            let one = BigInteger::from(1);

            let mut u = self.0;
            let mut v = P::MODULUS;
            let mut b = Fp384::<P>(P::R2, PhantomData); // Avoids unnecessary reduction step.
            let mut c = Self::zero();

            while u != one && v != one {
                while u.is_even() {
                    u.div2();

                    if b.0.is_even() {
                        b.0.div2();
                    } else {
                        b.0.add_nocarry(&P::MODULUS);
                        b.0.div2();
                    }
                }

                while v.is_even() {
                    v.div2();

                    if c.0.is_even() {
                        c.0.div2();
                    } else {
                        c.0.add_nocarry(&P::MODULUS);
                        c.0.div2();
                    }
                }

                if v < u {
                    u.sub_noborrow(&v);
                    b.sub_assign(&c);
                } else {
                    v.sub_noborrow(&u);
                    c.sub_assign(&b);
                }
            }

            if u == one { Some(b) } else { Some(c) }
        }
    }

    fn inverse_in_place(&mut self) -> Option<&mut Self> {
        if let Some(inverse) = self.inverse() {
            *self = inverse;
            Some(self)
        } else {
            None
        }
    }

    #[inline]
    fn frobenius_map(&mut self, _: usize) {
        // No-op: No effect in a prime field.
    }
}

impl<P: Fp384Parameters> PrimeField for Fp384<P> {
    type BigInteger = BigInteger;
    type Parameters = P;

    #[inline]
    fn from_repr(r: BigInteger) -> Option<Self> {
        let mut r = Fp384(r, PhantomData);
        if r.is_zero() {
            Some(r)
        } else if r.is_valid() {
            r *= &Fp384(P::R2, PhantomData);
            Some(r)
        } else {
            None
        }
    }

    #[cfg(feature = "unadx")]
    #[inline(always)]
    fn to_repr(&self) -> BigInteger {
        let mut tmp = self.0;
        let mut r = tmp.0;
        // Montgomery Reduction
        // Iteration 0
        let k = r[0].wrapping_mul(P::INV);
        let mut carry = 0;
        fa::mac_with_carry(r[0], k, P::MODULUS.0[0], &mut carry);
        r[1] = fa::mac_with_carry(r[1], k, P::MODULUS.0[1], &mut carry);
        r[2] = fa::mac_with_carry(r[2], k, P::MODULUS.0[2], &mut carry);
        r[3] = fa::mac_with_carry(r[3], k, P::MODULUS.0[3], &mut carry);
        r[4] = fa::mac_with_carry(r[4], k, P::MODULUS.0[4], &mut carry);
        r[5] = fa::mac_with_carry(r[5], k, P::MODULUS.0[5], &mut carry);
        r[0] = carry;

        let k = r[1].wrapping_mul(P::INV);
        let mut carry = 0;
        fa::mac_with_carry(r[1], k, P::MODULUS.0[0], &mut carry);
        r[2] = fa::mac_with_carry(r[2], k, P::MODULUS.0[1], &mut carry);
        r[3] = fa::mac_with_carry(r[3], k, P::MODULUS.0[2], &mut carry);
        r[4] = fa::mac_with_carry(r[4], k, P::MODULUS.0[3], &mut carry);
        r[5] = fa::mac_with_carry(r[5], k, P::MODULUS.0[4], &mut carry);
        r[0] = fa::mac_with_carry(r[0], k, P::MODULUS.0[5], &mut carry);
        r[1] = carry;

        let k = r[2].wrapping_mul(P::INV);
        let mut carry = 0;
        fa::mac_with_carry(r[2], k, P::MODULUS.0[0], &mut carry);
        r[3] = fa::mac_with_carry(r[3], k, P::MODULUS.0[1], &mut carry);
        r[4] = fa::mac_with_carry(r[4], k, P::MODULUS.0[2], &mut carry);
        r[5] = fa::mac_with_carry(r[5], k, P::MODULUS.0[3], &mut carry);
        r[0] = fa::mac_with_carry(r[0], k, P::MODULUS.0[4], &mut carry);
        r[1] = fa::mac_with_carry(r[1], k, P::MODULUS.0[5], &mut carry);
        r[2] = carry;

        let k = r[3].wrapping_mul(P::INV);
        let mut carry = 0;
        fa::mac_with_carry(r[3], k, P::MODULUS.0[0], &mut carry);
        r[4] = fa::mac_with_carry(r[4], k, P::MODULUS.0[1], &mut carry);
        r[5] = fa::mac_with_carry(r[5], k, P::MODULUS.0[2], &mut carry);
        r[0] = fa::mac_with_carry(r[0], k, P::MODULUS.0[3], &mut carry);
        r[1] = fa::mac_with_carry(r[1], k, P::MODULUS.0[4], &mut carry);
        r[2] = fa::mac_with_carry(r[2], k, P::MODULUS.0[5], &mut carry);
        r[3] = carry;

        let k = r[4].wrapping_mul(P::INV);
        let mut carry = 0;
        fa::mac_with_carry(r[4], k, P::MODULUS.0[0], &mut carry);
        r[5] = fa::mac_with_carry(r[5], k, P::MODULUS.0[1], &mut carry);
        r[0] = fa::mac_with_carry(r[0], k, P::MODULUS.0[2], &mut carry);
        r[1] = fa::mac_with_carry(r[1], k, P::MODULUS.0[3], &mut carry);
        r[2] = fa::mac_with_carry(r[2], k, P::MODULUS.0[4], &mut carry);
        r[3] = fa::mac_with_carry(r[3], k, P::MODULUS.0[5], &mut carry);
        r[4] = carry;

        let k = r[5].wrapping_mul(P::INV);
        let mut carry = 0;
        fa::mac_with_carry(r[5], k, P::MODULUS.0[0], &mut carry);
        r[0] = fa::mac_with_carry(r[0], k, P::MODULUS.0[1], &mut carry);
        r[1] = fa::mac_with_carry(r[1], k, P::MODULUS.0[2], &mut carry);
        r[2] = fa::mac_with_carry(r[2], k, P::MODULUS.0[3], &mut carry);
        r[3] = fa::mac_with_carry(r[3], k, P::MODULUS.0[4], &mut carry);
        r[4] = fa::mac_with_carry(r[4], k, P::MODULUS.0[5], &mut carry);
        r[5] = carry;

        tmp.0 = r;
        tmp
    }

    #[cfg(not(feature = "unadx"))]
    #[inline(always)]
    fn to_repr(&self) -> BigInteger {
        let mut tmp = self.0;

        // Montgomery Reduction
        unsafe {
            std::arch::asm!(
                // Iteration 0
                "xor {C1}, {C1}                 \n\t",
                "mov {INV}, %rdx               \n\t",
                "mulx (%rdi), %rdx, {C1}        \n\t",  // wrapping_mul
                "mulx (%rsi), {R0}, {C1}        \n\t",
                "adcx (%rdi), {R0}              \n\t",
                "mulx 0x8(%rsi), {R1}, {R0}     \n\t",
                "adcx 0x8(%rdi), {R1}           \n\t",
                "adox {C1}, {R1}                \n\t",
                "mulx 0x10(%rsi), {R2}, {C1}    \n\t",
                "adcx 0x10(%rdi), {R2}          \n\t",
                "adox {R0}, {R2}                \n\t",
                "mulx 0x18(%rsi), {R3}, {R0}    \n\t",
                "adcx 0x18(%rdi), {R3}          \n\t",
                "adox {C1}, {R3}                \n\t",
                "mulx 0x20(%rsi), {R4}, {C1}    \n\t",
                "adcx 0x20(%rdi), {R4}          \n\t",
                "adox {R0}, {R4}                \n\t",
                "mulx 0x28(%rsi), {R5}, {R0}    \n\t",
                "adcx 0x28(%rdi), {R5}          \n\t",
                "adox {C1}, {R5}                \n\t",
                "mov $0, {C1}                   \n\t",
                "adox {C1}, {R0}                \n\t",
                "adc $0, {R0}                   \n\t",

                // Iteration 1
                "xor {C1}, {C1}                 \n\t",
                "mov {INV}, %rdx               \n\t",
                "mulx {R1}, %rdx, {C1}          \n\t",  // wrapping_mul
                "mulx (%rsi), {C2}, {C1}        \n\t",
                "adcx {R1}, {C2}                \n\t",
                "mulx 0x8(%rsi), {R1}, {C2}     \n\t",
                "adcx {R1}, {R2}                \n\t",
                "adox {C1}, {R2}                \n\t",
                "mulx 0x10(%rsi), {R1}, {C1}    \n\t",
                "adcx {R1}, {R3}                \n\t",
                "adox {C2}, {R3}                \n\t",
                "mulx 0x18(%rsi), {R1}, {C2}    \n\t",
                "adcx {R1}, {R4}                \n\t",
                "adox {C1}, {R4}                \n\t",
                "mulx 0x20(%rsi), {R1}, {C1}    \n\t",
                "adcx {R1}, {R5}                \n\t",
                "adox {C2}, {R5}                \n\t",
                "mulx 0x28(%rsi), {C2}, {R1}    \n\t",
                "adcx {C2}, {R0}                \n\t",
                "adox {C1}, {R0}                \n\t",
                "mov $0, {C1}                   \n\t",
                "adox {C1}, {R1}                \n\t",
                "adc $0, {R1}                   \n\t",

                // Iteration 2
                "xor {C1}, {C1}                 \n\t",
                "mov {INV}, %rdx               \n\t",
                "mulx {R2}, %rdx, {C1}          \n\t",  // wrapping_mul
                "mulx (%rsi), {C2}, {C1}        \n\t",
                "adcx {R2}, {C2}                \n\t",
                "mulx 0x8(%rsi), {R2}, {C2}     \n\t",
                "adcx {R2}, {R3}                \n\t",
                "adox {C1}, {R3}                \n\t",
                "mulx 0x10(%rsi), {R2}, {C1}    \n\t",
                "adcx {R2}, {R4}                \n\t",
                "adox {C2}, {R4}                \n\t",
                "mulx 0x18(%rsi), {R2}, {C2}    \n\t",
                "adcx {R2}, {R5}                \n\t",
                "adox {C1}, {R5}                \n\t",
                "mulx 0x20(%rsi), {R2}, {C1}    \n\t",
                "adcx {R2}, {R0}                \n\t",
                "adox {C2}, {R0}                \n\t",
                "mulx 0x28(%rsi), {C2}, {R2}    \n\t",
                "adcx {C2}, {R1}                \n\t",
                "adox {C1}, {R1}                \n\t",
                "mov $0, {C1}                   \n\t",
                "adox {C1}, {R2}                \n\t",
                "adc $0, {R2}                   \n\t",

                // Iteration 3
                "xor {C1}, {C1}                 \n\t",
                "mov {INV}, %rdx               \n\t",
                "mulx {R3}, %rdx, {C1}          \n\t",  // wrapping_mul
                "mulx (%rsi), {C2}, {C1}        \n\t",
                "adcx {R3}, {C2}                \n\t",
                "mulx 0x8(%rsi), {R3}, {C2}     \n\t",
                "adcx {R3}, {R4}                \n\t",
                "adox {C1}, {R4}                \n\t",
                "mulx 0x10(%rsi), {R3}, {C1}    \n\t",
                "adcx {R3}, {R5}                \n\t",
                "adox {C2}, {R5}                \n\t",
                "mulx 0x18(%rsi), {R3}, {C2}    \n\t",
                "adcx {R3}, {R0}                \n\t",
                "adox {C1}, {R0}                \n\t",
                "mulx 0x20(%rsi), {R3}, {C1}    \n\t",
                "adcx {R3}, {R1}                \n\t",
                "adox {C2}, {R1}                \n\t",
                "mulx 0x28(%rsi), {C2}, {R3}    \n\t",
                "adcx {C2}, {R2}                \n\t",
                "adox {C1}, {R2}                \n\t",
                "mov $0, {C1}                   \n\t",
                "adox {C1}, {R3}                \n\t",
                "adc $0, {R3}                   \n\t",

                // Iteration 4
                "xor {C1}, {C1}                 \n\t",
                "mov {INV}, %rdx               \n\t",
                "mulx {R4}, %rdx, {C1}          \n\t",  // wrapping_mul
                "mulx (%rsi), {C2}, {C1}        \n\t",
                "adcx {R4}, {C2}                \n\t",
                "mulx 0x8(%rsi), {R4}, {C2}     \n\t",
                "adcx {R4}, {R5}                \n\t",
                "adox {C1}, {R5}                \n\t",
                "mulx 0x10(%rsi), {R4}, {C1}    \n\t",
                "adcx {R4}, {R0}                \n\t",
                "adox {C2}, {R0}                \n\t",
                "mulx 0x18(%rsi), {R4}, {C2}    \n\t",
                "adcx {R4}, {R1}                \n\t",
                "adox {C1}, {R1}                \n\t",
                "mulx 0x20(%rsi), {R4}, {C1}    \n\t",
                "adcx {R4}, {R2}                \n\t",
                "adox {C2}, {R2}                \n\t",
                "mulx 0x28(%rsi), {C2}, {R4}    \n\t",
                "adcx {C2}, {R3}                \n\t",
                "adox {C1}, {R3}                \n\t",
                "mov $0, {C1}                   \n\t",
                "adox {C1}, {R4}                \n\t",
                "adc $0, {R4}                   \n\t",

                // Iteration 5
                "xor {C1}, {C1}                 \n\t",
                "mov {INV}, %rdx               \n\t",
                "mulx {R5}, %rdx, {C1}          \n\t",  // wrapping_mul
                "mulx (%rsi), {C2}, {C1}        \n\t",
                "adcx {R5}, {C2}                \n\t",
                "mulx 0x8(%rsi), {R5}, {C2}     \n\t",
                "adcx {R5}, {R0}                \n\t",
                "adox {C1}, {R0}                \n\t",
                "mulx 0x10(%rsi), {R5}, {C1}    \n\t",
                "adcx {R5}, {R1}                \n\t",
                "adox {C2}, {R1}                \n\t",
                "mulx 0x18(%rsi), {R5}, {C2}    \n\t",
                "adcx {R5}, {R2}                \n\t",
                "adox {C1}, {R2}                \n\t",
                "mulx 0x20(%rsi), {R5}, {C1}    \n\t",
                "adcx {R5}, {R3}                \n\t",
                "adox {C2}, {R3}                \n\t",
                "mulx 0x28(%rsi), {C2}, {R5}    \n\t",
                "adcx {C2}, {R4}                \n\t",
                "adox {C1}, {R4}                \n\t",
                "mov $0, {C1}                   \n\t",
                "adox {C1}, {R5}                \n\t",
                "adc $0, {R5}                   \n\t",

                "mov {R0}, (%rdi)               \n\t",
                "mov {R1}, 0x8(%rdi)            \n\t",
                "mov {R2}, 0x10(%rdi)           \n\t",
                "mov {R3}, 0x18(%rdi)           \n\t",
                "mov {R4}, 0x20(%rdi)           \n\t",
                "mov {R5}, 0x28(%rdi)           \n\t",

                R0 = out(reg) _,
                R1 = out(reg) _,
                R2 = out(reg) _,
                R3 = out(reg) _,
                R4 = out(reg) _,
                R5 = out(reg) _,
                C1 = out(reg) _,    // C1
                C2 = out(reg) _,    // C2
                INV = in(reg) P::INV,
                out("rdx") _,
                inout("rsi") P::MODULUS.0.as_ptr() => _,    // Modulus.0
                inout("rdi") tmp.0.as_mut_ptr() => _,       // Result
                options(nostack, att_syntax)
            );
        }

        tmp
    }

    #[inline]
    fn to_repr_unchecked(&self) -> BigInteger {
        self.0
    }
}

impl<P: Fp384Parameters> FftField for Fp384<P> {
    type FftParameters = P;

    #[inline]
    fn two_adic_root_of_unity() -> Self {
        Self(P::TWO_ADIC_ROOT_OF_UNITY, PhantomData)
    }

    #[inline]
    fn large_subgroup_root_of_unity() -> Option<Self> {
        Some(Self(P::LARGE_SUBGROUP_ROOT_OF_UNITY?, PhantomData))
    }

    #[inline]
    fn multiplicative_generator() -> Self {
        Self(P::GENERATOR, PhantomData)
    }
}

impl<P: Fp384Parameters> SquareRootField for Fp384<P> {
    #[inline]
    fn legendre(&self) -> LegendreSymbol {
        use crate::LegendreSymbol::*;

        // s = self^((MODULUS - 1) // 2)
        let s = self.pow(P::MODULUS_MINUS_ONE_DIV_TWO);
        if s.is_zero() {
            Zero
        } else if s.is_one() {
            QuadraticResidue
        } else {
            QuadraticNonResidue
        }
    }

    #[inline]
    fn sqrt(&self) -> Option<Self> {
        sqrt_impl!(Self, P, self)
    }

    #[inline(always)]
    fn sqrt_in_place(&mut self) -> Option<&mut Self> {
        (*self).sqrt().map(|sqrt| {
            *self = sqrt;
            self
        })
    }
}

impl<P: Fp384Parameters> Ord for Fp384<P> {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        self.to_repr().cmp(&other.to_repr())
    }
}

impl<P: Fp384Parameters> PartialOrd for Fp384<P> {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<P: Fp384Parameters + PoseidonDefaultParameters> PoseidonDefaultField for Fp384<P> {}

impl_primefield_from_int!(Fp384, u128, Fp384Parameters);
impl_primefield_from_int!(Fp384, u64, Fp384Parameters);
impl_primefield_from_int!(Fp384, u32, Fp384Parameters);
impl_primefield_from_int!(Fp384, u16, Fp384Parameters);
impl_primefield_from_int!(Fp384, u8, Fp384Parameters);

impl_primefield_standard_sample!(Fp384, Fp384Parameters);

impl_add_sub_from_field_ref!(Fp384, Fp384Parameters);
impl_mul_div_from_field_ref!(Fp384, Fp384Parameters);

impl<P: Fp384Parameters> ToBits for Fp384<P> {
    fn to_bits_le(&self) -> Vec<bool> {
        let mut bits_vec = self.to_repr().to_bits_le();
        bits_vec.truncate(P::MODULUS_BITS as usize);

        bits_vec
    }

    fn to_bits_be(&self) -> Vec<bool> {
        let mut bits_vec = self.to_bits_le();
        bits_vec.reverse();

        bits_vec
    }
}

impl<P: Fp384Parameters> ToBytes for Fp384<P> {
    #[inline]
    fn write_le<W: Write>(&self, writer: W) -> IoResult<()> {
        self.to_repr().write_le(writer)
    }
}

impl<P: Fp384Parameters> FromBytes for Fp384<P> {
    #[inline]
    fn read_le<R: Read>(reader: R) -> IoResult<Self> {
        BigInteger::read_le(reader).and_then(|b| match Self::from_repr(b) {
            Some(f) => Ok(f),
            None => Err(FieldError::InvalidFieldElement.into()),
        })
    }
}

impl<P: Fp384Parameters> FromStr for Fp384<P> {
    type Err = FieldError;

    /// Interpret a string of numbers as a (congruent) prime field element.
    /// Does not accept unnecessary leading zeroes or a blank string.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(FieldError::ParsingEmptyString);
        }

        if s == "0" {
            return Ok(Self::zero());
        }

        let mut res = Self::zero();

        let ten = Self::from_repr(<Self as PrimeField>::BigInteger::from(10)).ok_or(FieldError::InvalidFieldElement)?;

        let mut first_digit = true;

        for c in s.chars() {
            match c.to_digit(10) {
                Some(c) => {
                    if first_digit {
                        if c == 0 {
                            return Err(FieldError::InvalidString);
                        }

                        first_digit = false;
                    }

                    res.mul_assign(&ten);
                    res.add_assign(
                        &Self::from_repr(<Self as PrimeField>::BigInteger::from(u64::from(c)))
                            .ok_or(FieldError::InvalidFieldElement)?,
                    );
                }
                None => {
                    return Err(FieldError::ParsingNonDigitCharacter);
                }
            }
        }

        if !res.is_valid() { Err(FieldError::InvalidFieldElement) } else { Ok(res) }
    }
}

impl<P: Fp384Parameters> Debug for Fp384<P> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.to_repr())
    }
}

impl<P: Fp384Parameters> Display for Fp384<P> {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.to_repr())
    }
}

impl<P: Fp384Parameters> Neg for Fp384<P> {
    type Output = Self;

    #[inline]
    #[must_use]
    fn neg(self) -> Self {
        if !self.is_zero() {
            let mut tmp = P::MODULUS;
            tmp.sub_noborrow(&self.0);
            Fp384::<P>(tmp, PhantomData)
        } else {
            self
        }
    }
}

impl<'a, P: Fp384Parameters> Add<&'a Fp384<P>> for Fp384<P> {
    type Output = Self;

    #[inline]
    fn add(self, other: &Self) -> Self {
        let mut result = self;
        result.add_assign(other);
        result
    }
}

impl<'a, P: Fp384Parameters> Sub<&'a Fp384<P>> for Fp384<P> {
    type Output = Self;

    #[inline]
    fn sub(self, other: &Self) -> Self {
        let mut result = self;
        result.sub_assign(other);
        result
    }
}

impl<'a, P: Fp384Parameters> Mul<&'a Fp384<P>> for Fp384<P> {
    type Output = Self;

    #[inline(always)]
    fn mul(self, other: &Self) -> Self {
        let mut result = self;
        result.mul_assign(other);
        result
    }
}

impl<'a, P: Fp384Parameters> Div<&'a Fp384<P>> for Fp384<P> {
    type Output = Self;

    #[inline]
    fn div(self, other: &Self) -> Self {
        let mut result = self;
        result.mul_assign(&other.inverse().unwrap());
        result
    }
}

impl<'a, P: Fp384Parameters> AddAssign<&'a Self> for Fp384<P> {
    #[inline]
    fn add_assign(&mut self, other: &Self) {
        // This cannot exceed the backing capacity.
        self.0.add_nocarry(&other.0);
        // However, it may need to be reduced
        self.reduce();
    }
}

impl<'a, P: Fp384Parameters> SubAssign<&'a Self> for Fp384<P> {
    #[inline]
    fn sub_assign(&mut self, other: &Self) {
        // If `other` is larger than `self`, add the modulus to self first.
        if other.0 > self.0 {
            self.0.add_nocarry(&P::MODULUS);
        }

        self.0.sub_noborrow(&other.0);
    }
}

impl<'a, P: Fp384Parameters> MulAssign<&'a Self> for Fp384<P> {
    #[cfg(not(feature = "unadx"))]
    #[inline(always)]
    fn mul_assign(&mut self, other: &Self) {
        unsafe {
            std::arch::asm!(
                // Iteration 0.
                "mov (%rdi), %rdx                \n\t", // other[0]
                "mulx (%rcx), {R0}, %r8         \n\t", // b*c+carry, C1
                "mulx 0x8(%rcx), {R1}, %r9      \n\t",
                "add %r8, {R1}                 \n\t",
                "mulx 0x10(%rcx), {R2}, %r8     \n\t",
                "adc %r9, {R2}                 \n\t",
                "mulx 0x18(%rcx), {R3}, %r9     \n\t",
                "adc %r8, {R3}                 \n\t",
                "mulx 0x20(%rcx), {R4}, %r8     \n\t",
                "adc %r9, {R4}                 \n\t",
                "mulx 0x28(%rcx), {R5}, %r9     \n\t",
                "adc %r8, {R5}                 \n\t",
                "adc $0, %r9                   \n\t", // C2 - carry1


                //"mov ${INV}, %rdx               \n\t",
                "movabs $0x8508bfffffffffff, %rdx    \n\t",
                "mulx {R0}, %rdx, {C3}               \n\t",
                "mulx (%rsi), {C3}, %r8         \n\t",
                "adcx {R0}, {C3}                \n\t", // gen carray2
                "mulx 0x08(%rsi), {R0}, {C3}    \n\t",
                "adcx %r8, {R0}                 \n\n",
                "adox {R1}, {R0}                \n\n",
                "mulx 0x10(%rsi), {R1}, %r8     \n\t",
                "adcx {C3}, {R1}                \n\n",
                "adox {R2}, {R1}                \n\n",
                "mulx 0x18(%rsi), {R2}, {C3}    \n\t",
                "adcx %r8, {R2}                 \n\n",
                "adox {R3}, {R2}                \n\n",
                "mulx 0x20(%rsi), {R3}, %r8     \n\t",
                "adcx {C3}, {R3}                \n\n",
                "adox {R4}, {R3}                \n\n",
                "mulx 0x28(%rsi), {R4}, {C3}    \n\t", // C3 - carry2
                "adcx %r8, {R4}                 \n\n",
                "adox {R5}, {R4}                \n\n",

                "adox %r9, {C3}                \n\n", // C3 - R5
                "adc $0, {C3}                   \n\n", // C3 - R5

                // Iteration 1.
                "xor %rdx, %rdx                 \n\t",
                "mov 0x8(%rdi), %rdx             \n\t", // other[1]
                "mulx (%rcx), {R5}, %r8         \n\t", // a+b*c+carry, C1
                "adcx {R5}, {R0}                 \n\t",
                "mulx 0x8(%rcx), {R5}, %r9      \n\t",
                "adcx %r8, {R5}                \n\t",
                "adox {R5}, {R1}                \n\t",
                "mulx 0x10(%rcx), {R5}, %r8     \n\t",
                "adcx %r9, {R5}                \n\t",
                "adox {R5}, {R2}                \n\t",
                "mulx 0x18(%rcx), {R5}, %r9     \n\t",
                "adcx %r8, {R5}                \n\t",
                "adox {R5}, {R3}                \n\t",
                "mulx 0x20(%rcx), {R5}, %r8     \n\t",
                "adcx %r9, {R5}                \n\t",
                "adox {R5}, {R4}                \n\t",
                "mulx 0x28(%rcx), {R5}, %r9     \n\t", // C2 - carry1
                "adcx %r8, {R5}                \n\t",
                "adox {C3}, {R5}                \n\t",

                "mov $0, {C3}                   \n\t",
                "adox {C3}, %r9                \n\t",
                "adc $0, %r9                   \n\t",


                //"mov ${INV}, %rdx               \n\t",
                "movabs $0x8508bfffffffffff, %rdx    \n\t",
                "mulx {R0}, %rdx, {C3}               \n\t",
                "mulx (%rsi), {C3}, %r8         \n\t",
                "adcx {R0}, {C3}                \n\t",
                "mulx 0x8(%rsi), {R0}, {C3}     \n\t",
                "adcx %r8, {R0}                 \n\t",
                "adox {R1}, {R0}                \n\t",
                "mulx 0x10(%rsi), {R1}, %r8     \n\t",
                "adcx {C3}, {R1}                \n\t",
                "adox {R2}, {R1}                \n\t",
                "mulx 0x18(%rsi), {R2}, {C3}    \n\t",
                "adcx %r8, {R2}                 \n\t",
                "adox {R3}, {R2}                \n\t",
                "mulx 0x20(%rsi), {R3}, %r8     \n\t",
                "adcx {C3}, {R3}                \n\t",
                "adox {R4}, {R3}                \n\t",
                "mulx 0x28(%rsi), {R4}, {C3}     \n\t",
                "adcx %r8, {R4}                \n\t",
                "adox {R5}, {R4}                \n\t",

                "adox %r9, {C3}                \n\n", // C3 - R5
                "adc $0, {C3}                   \n\n", // C3 - R5

                // Iteration 2.
                "xor %rdx, %rdx                 \n\t",
                "mov 0x10(%rdi), %rdx           \n\t", // other[2]
                "mulx (%rcx), {R5}, %r8         \n\t", // a+b*c+carry, C1
                "adcx {R5}, {R0}                \n\t",
                "mulx 0x8(%rcx), {R5}, %r9      \n\t",
                "adcx %r8, {R5}                 \n\t",
                "adox {R5}, {R1}                \n\t",
                "mulx 0x10(%rcx), {R5}, %r8     \n\t",
                "adcx %r9, {R5}                 \n\t",
                "adox {R5}, {R2}                \n\t",
                "mulx 0x18(%rcx), {R5}, %r9     \n\t",
                "adcx %r8, {R5}                 \n\t",
                "adox {R5}, {R3}                \n\t",
                "mulx 0x20(%rcx), {R5}, %r8     \n\t",
                "adcx %r9, {R5}                 \n\t",
                "adox {R5}, {R4}                \n\t",
                "mulx 0x28(%rcx), {R5}, %r9     \n\t", // C2 - carry1
                "adcx %r8, {R5}                 \n\t",
                "adox {C3}, {R5}                \n\t",

                "mov $0, {C3}                   \n\t",
                "adox {C3}, %r9                 \n\t",
                "adc $0, %r9                    \n\t",


                //"mov ${INV}, %rdx               \n\t",
                "movabs $0x8508bfffffffffff, %rdx    \n\t",
                "mulx {R0}, %rdx, {C3}          \n\t",
                "mulx (%rsi), {C3}, %r8         \n\t",
                "adcx {R0}, {C3}                \n\t",
                "mulx 0x8(%rsi), {R0}, {C3}     \n\t",
                "adcx %r8, {R0}                 \n\t",
                "adox {R1}, {R0}                \n\t",
                "mulx 0x10(%rsi), {R1}, %r8     \n\t",
                "adcx {C3}, {R1}                \n\t",
                "adox {R2}, {R1}                \n\t",
                "mulx 0x18(%rsi), {R2}, {C3}    \n\t",
                "adcx %r8, {R2}                 \n\t",
                "adox {R3}, {R2}                \n\t",
                "mulx 0x20(%rsi), {R3}, %r8     \n\t",
                "adcx {C3}, {R3}                \n\t",
                "adox {R4}, {R3}                \n\t",
                "mulx 0x28(%rsi), {R4}, {C3}    \n\t",
                "adcx %r8, {R4}                 \n\t",
                "adox {R5}, {R4}                \n\t",

                "adox %r9, {C3}                 \n\n", // C3 - R5
                "adc $0, {C3}                   \n\n", // C3 - R5

                // Iteration 3.
                "xor %rdx, %rdx                \n\t",
                "mov 0x18(%rdi), %rdx          \n\t", // other[2]
                "mulx (%rcx), {R5}, %r8        \n\t", // a+b*c+carry, C1
                "adcx {R5}, {R0}               \n\t",
                "mulx 0x8(%rcx), {R5}, %r9     \n\t",
                "adcx %r8, {R5}                \n\t",
                "adox {R5}, {R1}               \n\t",
                "mulx 0x10(%rcx), {R5}, %r8    \n\t",
                "adcx %r9, {R5}                \n\t",
                "adox {R5}, {R2}               \n\t",
                "mulx 0x18(%rcx), {R5}, %r9    \n\t",
                "adcx %r8, {R5}                \n\t",
                "adox {R5}, {R3}               \n\t",
                "mulx 0x20(%rcx), {R5}, %r8    \n\t",
                "adcx %r9, {R5}                \n\t",
                "adox {R5}, {R4}               \n\t",
                "mulx 0x28(%rcx), {R5}, %r9    \n\t", // C2 - carry1
                "adcx %r8, {R5}                \n\t",
                "adox {C3}, {R5}               \n\t",

                "mov $0, {C3}                  \n\t",
                "adox {C3}, %r9                \n\t",
                "adc $0, %r9                   \n\t",


                //"mov ${INV}, %rdx            \n\t",
                "movabs $0x8508bfffffffffff, %rdx    \n\t",
                "mulx {R0}, %rdx, {C3}          \n\t",
                "mulx (%rsi), {C3}, %r8         \n\t",
                "adcx {R0}, {C3}                \n\t",
                "mulx 0x8(%rsi), {R0}, {C3}     \n\t",
                "adcx %r8, {R0}                 \n\t",
                "adox {R1}, {R0}                \n\t",
                "mulx 0x10(%rsi), {R1}, %r8     \n\t",
                "adcx {C3}, {R1}                \n\t",
                "adox {R2}, {R1}                \n\t",
                "mulx 0x18(%rsi), {R2}, {C3}    \n\t",
                "adcx %r8, {R2}                 \n\t",
                "adox {R3}, {R2}                \n\t",
                "mulx 0x20(%rsi), {R3}, %r8     \n\t",
                "adcx {C3}, {R3}                \n\t",
                "adox {R4}, {R3}                \n\t",
                "mulx 0x28(%rsi), {R4}, {C3}    \n\t",
                "adcx %r8, {R4}                 \n\t",
                "adox {R5}, {R4}                \n\t",

                "adox %r9, {C3}                 \n\n", // C3 - R5
                "adc $0, {C3}                   \n\n", // C3 - R5

                // Iteration 4.
                "xor %rdx, %rdx                     \n\t",
                "mov 0x20(%rdi), %rdx               \n\t", // other[2]
                "mulx (%rcx), {R5}, %r8             \n\t", // a+b*c+carry, C1
                "adcx {R5}, {R0}                    \n\t",
                "mulx 0x8(%rcx), {R5}, %r9          \n\t",
                "adcx %r8, {R5}                     \n\t",
                "adox {R5}, {R1}                    \n\t",
                "mulx 0x10(%rcx), {R5}, %r8         \n\t",
                "adcx %r9, {R5}                     \n\t",
                "adox {R5}, {R2}                    \n\t",
                "mulx 0x18(%rcx), {R5}, %r9         \n\t",
                "adcx %r8, {R5}                     \n\t",
                "adox {R5}, {R3}                    \n\t",
                "mulx 0x20(%rcx), {R5}, %r8         \n\t",
                "adcx %r9, {R5}                     \n\t",
                "adox {R5}, {R4}                    \n\t",
                "mulx 0x28(%rcx), {R5}, %r9         \n\t", // C2 - carry1
                "adcx %r8, {R5}                     \n\t",
                "adox {C3}, {R5}                    \n\t",

                "mov $0, {C3}                       \n\t",
                "adox {C3}, %r9                     \n\t",
                "adc $0, %r9                        \n\t",


                //"mov ${INV}, %rdx                   \n\t",
                "movabs $0x8508bfffffffffff, %rdx    \n\t",
                "mulx {R0}, %rdx, {C3}               \n\t",
                "mulx (%rsi), {C3}, %r8             \n\t",
                "adcx {R0}, {C3}                    \n\t",
                "mulx 0x8(%rsi), {R0}, {C3}         \n\t",
                "adcx %r8, {R0}                     \n\t",
                "adox {R1}, {R0}                    \n\t",
                "mulx 0x10(%rsi), {R1}, %r8         \n\t",
                "adcx {C3}, {R1}                    \n\t",
                "adox {R2}, {R1}                    \n\t",
                "mulx 0x18(%rsi), {R2}, {C3}        \n\t",
                "adcx %r8, {R2}                     \n\t",
                "adox {R3}, {R2}                    \n\t",
                "mulx 0x20(%rsi), {R3}, %r8         \n\t",
                "adcx {C3}, {R3}                    \n\t",
                "adox {R4}, {R3}                    \n\t",
                "mulx 0x28(%rsi), {R4}, {C3}        \n\t",
                "adcx %r8, {R4}                     \n\t",
                "adox {R5}, {R4}                    \n\t",

                "adox %r9, {C3}                     \n\n", // C3 - R5
                "adc $0, {C3}                       \n\n", // C3 - R5

                // Iteration 5.
                "xor %rdx, %rdx                     \n\t",
                "mov 0x28(%rdi), %rdx               \n\t", // other[2]
                "mulx (%rcx), {R5}, %r8             \n\t", // a+b*c+carry, C1
                "adcx {R5}, {R0}                    \n\t",
                "mulx 0x8(%rcx), {R5}, %r9          \n\t",
                "adcx %r8, {R5}                     \n\t",
                "adox {R5}, {R1}                    \n\t",
                "mulx 0x10(%rcx), {R5}, %r8         \n\t",
                "adcx %r9, {R5}                     \n\t",
                "adox {R5}, {R2}                    \n\t",
                "mulx 0x18(%rcx), {R5}, %r9         \n\t",
                "adcx %r8, {R5}                     \n\t",
                "adox {R5}, {R3}                    \n\t",
                "mulx 0x20(%rcx), {R5}, %r8         \n\t",
                "adcx %r9, {R5}                     \n\t",
                "adox {R5}, {R4}                    \n\t",
                "mulx 0x28(%rcx), {R5}, %r9         \n\t", // C2 - carry1
                "adcx %r8, {R5}                     \n\t",
                "adox {C3}, {R5}                    \n\t",

                "mov $0, {C3}                       \n\t",
                "adox {C3}, %r9                     \n\t",
                "adc $0, %r9                        \n\t",


                //"mov ${INV}, %rdx                   \n\t",
                "movabs $0x8508bfffffffffff, %rdx   \n\t",
                "mulx {R0}, %rdx, {C3}              \n\t",
                "mulx (%rsi), {C3}, %r8             \n\t",
                "adcx {R0}, {C3}                    \n\t",
                "mulx 0x8(%rsi), {R0}, {C3}         \n\t",
                "adcx %r8, {R0}                     \n\t",
                "adox {R1}, {R0}                    \n\t",
                "mulx 0x10(%rsi), {R1}, %r8         \n\t",
                "adcx {C3}, {R1}                    \n\t",
                "adox {R2}, {R1}                    \n\t",
                "mulx 0x18(%rsi), {R2}, {C3}        \n\t",
                "adcx %r8, {R2}                     \n\t",
                "adox {R3}, {R2}                    \n\t",
                "mulx 0x20(%rsi), {R3}, %r8         \n\t",
                "adcx {C3}, {R3}                    \n\t",
                "adox {R4}, {R3}                    \n\t",
                "mulx 0x28(%rsi), {R4}, {C3}        \n\t",
                "adcx %r8, {R4}                     \n\t",
                "adox {R5}, {R4}                    \n\t",

                "adox %r9, {C3}                     \n\n", // C3 - R5
                "adc $0, {C3}                       \n\n", // C3 - R5

                // Result
                "mov {R0}, 0x00(%rcx)               \n\t",
                "mov {R1}, 0x08(%rcx)               \n\t",
                "mov {R2}, 0x10(%rcx)               \n\t",
                "mov {R3}, 0x18(%rcx)               \n\t",
                "mov {R4}, 0x20(%rcx)               \n\t",
                "mov {C3}, 0x28(%rcx)               \n\t",

                C3 = lateout(reg) _,
                R0 = lateout(reg) _,
                R1 = lateout(reg) _,
                R2 = lateout(reg) _,
                R3 = lateout(reg) _,
                R4 = lateout(reg) _,
                R5 = lateout(reg) _,
                //INV = const P::INV,
                //INV = const 0x8508bfffffffffff,
                out("rdx") _,
                lateout("r8") _,    // C1
                lateout("r9") _,    // C2
                inout("rcx") (self.0).0.as_mut_ptr() => _,  // Self.0
                inout("rsi") P::MODULUS.0.as_ptr() => _,    // Modulus.0
                inout("rdi") (other.0).0.as_ptr() => _,     // Other.0
                options(nostack, att_syntax)
            );
        }
        self.reduce();
    }

    #[cfg(feature = "unadx")]
    #[inline(always)]
    fn mul_assign(&mut self, other: &Self) {
        let mut r = [0u64; 6];
        let mut carry1 = 0u64;
        let mut carry2 = 0u64;

        r[0] = fa::mac(r[0], (self.0).0[0], (other.0).0[0], &mut carry1);
        let k = r[0].wrapping_mul(P::INV);
        fa::mac_discard(r[0], k, P::MODULUS.0[0], &mut carry2);
        r[1] = fa::mac_with_carry(r[1], (self.0).0[1], (other.0).0[0], &mut carry1);
        r[0] = fa::mac_with_carry(r[1], k, P::MODULUS.0[1], &mut carry2);

        r[2] = fa::mac_with_carry(r[2], (self.0).0[2], (other.0).0[0], &mut carry1);
        r[1] = fa::mac_with_carry(r[2], k, P::MODULUS.0[2], &mut carry2);

        r[3] = fa::mac_with_carry(r[3], (self.0).0[3], (other.0).0[0], &mut carry1);
        r[2] = fa::mac_with_carry(r[3], k, P::MODULUS.0[3], &mut carry2);

        r[4] = fa::mac_with_carry(r[4], (self.0).0[4], (other.0).0[0], &mut carry1);
        r[3] = fa::mac_with_carry(r[4], k, P::MODULUS.0[4], &mut carry2);

        r[5] = fa::mac_with_carry(r[5], (self.0).0[5], (other.0).0[0], &mut carry1);
        r[4] = fa::mac_with_carry(r[5], k, P::MODULUS.0[5], &mut carry2);
        r[5] = carry1 + carry2;

        // Iteration 1.
        r[0] = fa::mac(r[0], (self.0).0[0], (other.0).0[1], &mut carry1);
        let k = r[0].wrapping_mul(P::INV);
        fa::mac_discard(r[0], k, P::MODULUS.0[0], &mut carry2);
        r[1] = fa::mac_with_carry(r[1], (self.0).0[1], (other.0).0[1], &mut carry1);
        r[0] = fa::mac_with_carry(r[1], k, P::MODULUS.0[1], &mut carry2);

        r[2] = fa::mac_with_carry(r[2], (self.0).0[2], (other.0).0[1], &mut carry1);
        r[1] = fa::mac_with_carry(r[2], k, P::MODULUS.0[2], &mut carry2);

        r[3] = fa::mac_with_carry(r[3], (self.0).0[3], (other.0).0[1], &mut carry1);
        r[2] = fa::mac_with_carry(r[3], k, P::MODULUS.0[3], &mut carry2);

        r[4] = fa::mac_with_carry(r[4], (self.0).0[4], (other.0).0[1], &mut carry1);
        r[3] = fa::mac_with_carry(r[4], k, P::MODULUS.0[4], &mut carry2);

        r[5] = fa::mac_with_carry(r[5], (self.0).0[5], (other.0).0[1], &mut carry1);
        r[4] = fa::mac_with_carry(r[5], k, P::MODULUS.0[5], &mut carry2);
        r[5] = carry1 + carry2;

        // Iteration 2.
        r[0] = fa::mac(r[0], (self.0).0[0], (other.0).0[2], &mut carry1);
        let k = r[0].wrapping_mul(P::INV);
        fa::mac_discard(r[0], k, P::MODULUS.0[0], &mut carry2);
        r[1] = fa::mac_with_carry(r[1], (self.0).0[1], (other.0).0[2], &mut carry1);
        r[0] = fa::mac_with_carry(r[1], k, P::MODULUS.0[1], &mut carry2);

        r[2] = fa::mac_with_carry(r[2], (self.0).0[2], (other.0).0[2], &mut carry1);
        r[1] = fa::mac_with_carry(r[2], k, P::MODULUS.0[2], &mut carry2);

        r[3] = fa::mac_with_carry(r[3], (self.0).0[3], (other.0).0[2], &mut carry1);
        r[2] = fa::mac_with_carry(r[3], k, P::MODULUS.0[3], &mut carry2);

        r[4] = fa::mac_with_carry(r[4], (self.0).0[4], (other.0).0[2], &mut carry1);
        r[3] = fa::mac_with_carry(r[4], k, P::MODULUS.0[4], &mut carry2);

        r[5] = fa::mac_with_carry(r[5], (self.0).0[5], (other.0).0[2], &mut carry1);
        r[4] = fa::mac_with_carry(r[5], k, P::MODULUS.0[5], &mut carry2);
        r[5] = carry1 + carry2;

        // Iteration 3.
        r[0] = fa::mac(r[0], (self.0).0[0], (other.0).0[3], &mut carry1);
        let k = r[0].wrapping_mul(P::INV);
        fa::mac_discard(r[0], k, P::MODULUS.0[0], &mut carry2);
        r[1] = fa::mac_with_carry(r[1], (self.0).0[1], (other.0).0[3], &mut carry1);
        r[0] = fa::mac_with_carry(r[1], k, P::MODULUS.0[1], &mut carry2);

        r[2] = fa::mac_with_carry(r[2], (self.0).0[2], (other.0).0[3], &mut carry1);
        r[1] = fa::mac_with_carry(r[2], k, P::MODULUS.0[2], &mut carry2);

        r[3] = fa::mac_with_carry(r[3], (self.0).0[3], (other.0).0[3], &mut carry1);
        r[2] = fa::mac_with_carry(r[3], k, P::MODULUS.0[3], &mut carry2);

        r[4] = fa::mac_with_carry(r[4], (self.0).0[4], (other.0).0[3], &mut carry1);
        r[3] = fa::mac_with_carry(r[4], k, P::MODULUS.0[4], &mut carry2);

        r[5] = fa::mac_with_carry(r[5], (self.0).0[5], (other.0).0[3], &mut carry1);
        r[4] = fa::mac_with_carry(r[5], k, P::MODULUS.0[5], &mut carry2);
        r[5] = carry1 + carry2;

        // Iteration 4.
        r[0] = fa::mac(r[0], (self.0).0[0], (other.0).0[4], &mut carry1);
        let k = r[0].wrapping_mul(P::INV);
        fa::mac_discard(r[0], k, P::MODULUS.0[0], &mut carry2);
        r[1] = fa::mac_with_carry(r[1], (self.0).0[1], (other.0).0[4], &mut carry1);
        r[0] = fa::mac_with_carry(r[1], k, P::MODULUS.0[1], &mut carry2);

        r[2] = fa::mac_with_carry(r[2], (self.0).0[2], (other.0).0[4], &mut carry1);
        r[1] = fa::mac_with_carry(r[2], k, P::MODULUS.0[2], &mut carry2);

        r[3] = fa::mac_with_carry(r[3], (self.0).0[3], (other.0).0[4], &mut carry1);
        r[2] = fa::mac_with_carry(r[3], k, P::MODULUS.0[3], &mut carry2);

        r[4] = fa::mac_with_carry(r[4], (self.0).0[4], (other.0).0[4], &mut carry1);
        r[3] = fa::mac_with_carry(r[4], k, P::MODULUS.0[4], &mut carry2);

        r[5] = fa::mac_with_carry(r[5], (self.0).0[5], (other.0).0[4], &mut carry1);
        r[4] = fa::mac_with_carry(r[5], k, P::MODULUS.0[5], &mut carry2);
        r[5] = carry1 + carry2;

        // Iteration 5.
        r[0] = fa::mac(r[0], (self.0).0[0], (other.0).0[5], &mut carry1);
        let k = r[0].wrapping_mul(P::INV);
        fa::mac_discard(r[0], k, P::MODULUS.0[0], &mut carry2);
        r[1] = fa::mac_with_carry(r[1], (self.0).0[1], (other.0).0[5], &mut carry1);
        r[0] = fa::mac_with_carry(r[1], k, P::MODULUS.0[1], &mut carry2);

        r[2] = fa::mac_with_carry(r[2], (self.0).0[2], (other.0).0[5], &mut carry1);
        r[1] = fa::mac_with_carry(r[2], k, P::MODULUS.0[2], &mut carry2);

        r[3] = fa::mac_with_carry(r[3], (self.0).0[3], (other.0).0[5], &mut carry1);
        r[2] = fa::mac_with_carry(r[3], k, P::MODULUS.0[3], &mut carry2);

        r[4] = fa::mac_with_carry(r[4], (self.0).0[4], (other.0).0[5], &mut carry1);
        r[3] = fa::mac_with_carry(r[4], k, P::MODULUS.0[4], &mut carry2);

        r[5] = fa::mac_with_carry(r[5], (self.0).0[5], (other.0).0[5], &mut carry1);
        r[4] = fa::mac_with_carry(r[5], k, P::MODULUS.0[5], &mut carry2);
        r[5] = carry1 + carry2;

        (self.0).0 = r;
        self.reduce();
    }
}

impl<'a, P: Fp384Parameters> DivAssign<&'a Self> for Fp384<P> {
    #[inline]
    fn div_assign(&mut self, other: &Self) {
        self.mul_assign(&other.inverse().unwrap());
    }
}
