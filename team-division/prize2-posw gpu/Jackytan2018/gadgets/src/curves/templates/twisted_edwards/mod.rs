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
    bits::{Boolean, ToBitsBEGadget, ToBytesGadget},
    integers::uint::UInt8,
    traits::{
        alloc::AllocGadget,
        curves::{CompressedGroupGadget, GroupGadget},
        eq::{ConditionalEqGadget, EqGadget, NEqGadget},
        fields::FieldGadget,
        select::CondSelectGadget,
    },
    FpGadget,
    ToConstraintFieldGadget,
};
use snarkvm_curves::{
    templates::twisted_edwards_extended::{Affine as TEAffine, Projective as TEProjective},
    traits::{AffineCurve, MontgomeryParameters, TwistedEdwardsParameters},
    ProjectiveCurve,
};
use snarkvm_fields::{Field, One, PrimeField, Zero};
use snarkvm_r1cs::{errors::SynthesisError, Assignment, ConstraintSystem, Namespace};
use snarkvm_utilities::bititerator::BitIteratorBE;

use std::{
    borrow::Borrow,
    marker::PhantomData,
    ops::{AddAssign, MulAssign, Neg, SubAssign},
};

use parking_lot::{ RwLock};
use snarkvm_curves::templates::twisted_edwards_extended::Projective;
use std::any::Any;

lazy_static::lazy_static! {
    pub static ref GLOBAL_SM_TABLE_CACHE: RwLock<Vec<[snarkvm_curves::templates::twisted_edwards_extended::projective::Projective<snarkvm_curves::edwards_bls12::parameters::EdwardsParameters>;4]>> =RwLock::new(vec![]);
    pub static ref GLOBAL_SSM_TABLE_CACHE: RwLock<Vec<[snarkvm_curves::templates::twisted_edwards_extended::projective::Projective<snarkvm_curves::edwards_bls12::parameters::EdwardsParameters>;4]>> =RwLock::new(vec![]);
    pub static ref GLOBAL_MSM_TABLE_CACHE: RwLock<Vec<[snarkvm_curves::templates::twisted_edwards_extended::projective::Projective<snarkvm_curves::edwards_bls12::parameters::EdwardsParameters>;4]>> =RwLock::new(vec![]);
}

#[cfg(test)]
pub mod test;

#[derive(Derivative)]
#[derivative(Debug, Clone)]
#[derivative(Debug(bound = "P: TwistedEdwardsParameters, F: Field"))]
#[must_use]
pub struct MontgomeryAffineGadget<P: TwistedEdwardsParameters, F: Field, FG: FieldGadget<P::BaseField, F>> {
    pub x: FG,
    pub y: FG,
    #[derivative(Debug = "ignore")]
    _params: PhantomData<P>,
    #[derivative(Debug = "ignore")]
    _engine: PhantomData<F>,
}

impl<P: TwistedEdwardsParameters, F: Field, FG: FieldGadget<P::BaseField, F>> MontgomeryAffineGadget<P, F, FG> {
    pub fn new(x: FG, y: FG) -> Self {
        Self { x, y, _params: PhantomData, _engine: PhantomData }
    }

    pub fn from_edwards_to_coords(p: &TEAffine<P>) -> Result<(P::BaseField, P::BaseField), SynthesisError> {
        let montgomery_point: TEAffine<P> = if p.y == P::BaseField::one() {
            TEAffine::zero()
        } else if p.x == P::BaseField::zero() {
            TEAffine::new(P::BaseField::zero(), P::BaseField::zero())
        } else {
            let u = (P::BaseField::one() + p.y) * (P::BaseField::one() - p.y).inverse().unwrap();
            let v = u * p.x.inverse().unwrap();
            TEAffine::new(u, v)
        };

        Ok((montgomery_point.x, montgomery_point.y))
    }

    pub fn from_edwards<CS: ConstraintSystem<F>>(mut cs: CS, p: &TEAffine<P>) -> Result<Self, SynthesisError> {
        let montgomery_coords = Self::from_edwards_to_coords(p)?;

        let u = FG::alloc(cs.ns(|| "u"), || Ok(montgomery_coords.0))?;

        let v = FG::alloc(cs.ns(|| "v"), || Ok(montgomery_coords.1))?;

        Ok(Self::new(u, v))
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn into_edwards<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<AffineGadget<P, F, FG>, SynthesisError> {
        // Compute u = x / y
        let u = FG::alloc(cs.ns(|| "u"), || {
            let mut t0 = self.x.get_value().get()?;

            match self.y.get_value().get()?.inverse() {
                Some(invy) => {
                    t0.mul_assign(&invy);

                    Ok(t0)
                }
                None => Err(SynthesisError::DivisionByZero),
            }
        })?;

        u.mul_equals(cs.ns(|| "u equals"), &self.y, &self.x)?;

        let v = FG::alloc(cs.ns(|| "v"), || {
            let mut t0 = self.x.get_value().get()?;
            let mut t1 = t0;
            t0.sub_assign(&P::BaseField::one());
            t1.add_assign(P::BaseField::one());

            match t1.inverse() {
                Some(t1) => {
                    t0.mul_assign(&t1);

                    Ok(t0)
                }
                None => Err(SynthesisError::DivisionByZero),
            }
        })?;

        let xplusone = self.x.add_constant(cs.ns(|| "x plus one"), &P::BaseField::one())?;
        let xminusone = self.x.sub_constant(cs.ns(|| "x minus one"), &P::BaseField::one())?;
        v.mul_equals(cs.ns(|| "v equals"), &xplusone, &xminusone)?;

        Ok(AffineGadget::new(u, v))
    }

    pub fn add<CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &Self) -> Result<Self, SynthesisError> {
        let lambda = FG::alloc(cs.ns(|| "lambda"), || {
            let mut n = other.y.get_value().get()?;
            n.sub_assign(&self.y.get_value().get()?);

            let mut d = other.x.get_value().get()?;
            d.sub_assign(&self.x.get_value().get()?);

            match d.inverse() {
                Some(d) => {
                    n.mul_assign(&d);
                    Ok(n)
                }
                None => Err(SynthesisError::DivisionByZero),
            }
        })?;
        let lambda_n = other.y.sub(cs.ns(|| "other.y - self.y"), &self.y)?;
        let lambda_d = other.x.sub(cs.ns(|| "other.x - self.x"), &self.x)?;
        lambda_d.mul_equals(cs.ns(|| "lambda equals"), &lambda, &lambda_n)?;

        // Compute x'' = B*lambda^2 - A - x - x'
        let xprime = FG::alloc(cs.ns(|| "xprime"), || {
            Ok(lambda.get_value().get()?.square() * P::MontgomeryParameters::COEFF_B
                - P::MontgomeryParameters::COEFF_A
                - self.x.get_value().get()?
                - other.x.get_value().get()?)
        })?;

        let xprime_lc = self
            .x
            .add(cs.ns(|| "self.x + other.x"), &other.x)?
            .add(cs.ns(|| "+ xprime"), &xprime)?
            .add_constant(cs.ns(|| "+ A"), &P::MontgomeryParameters::COEFF_A)?;
        // (lambda) * (lambda) = (A + x + x' + x'')
        let lambda_b = lambda.mul_by_constant(cs.ns(|| "lambda * b"), &P::MontgomeryParameters::COEFF_B)?;
        lambda_b.mul_equals(cs.ns(|| "xprime equals"), &lambda, &xprime_lc)?;

        let yprime = FG::alloc(cs.ns(|| "yprime"), || {
            Ok(-(self.y.get_value().get()?
                + (lambda.get_value().get()? * (xprime.get_value().get()? - self.x.get_value().get()?))))
        })?;

        let xres = self.x.sub(cs.ns(|| "xres"), &xprime)?;
        let yres = self.y.add(cs.ns(|| "yres"), &yprime)?;
        lambda.mul_equals(cs.ns(|| "yprime equals"), &xres, &yres)?;
        Ok(MontgomeryAffineGadget::new(xprime, yprime))
    }
}

/// Based on 2 input bits, output on a group element from a 4 element table.
/// 00 => table[0]
/// 10 => table[1]
/// 01 => table[2]
/// 11 => table[3]
fn two_bit_lookup_helper<P: TwistedEdwardsParameters, F: Field, FG: FieldGadget<P::BaseField, F>, CS>(
    mut cs: CS,
    bits: [Boolean; 2],
    mut table: [TEProjective<P>; 4],
) -> Result<AffineGadget<P, F, FG>, SynthesisError>
where
    CS: ConstraintSystem<F>,
{
    TEProjective::batch_normalization(&mut table);
    let x_s = [table[0].x, table[1].x, table[2].x, table[3].x];
    let y_s = [table[0].y, table[1].y, table[2].y, table[3].y];

    let x: FG = FG::two_bit_lookup(cs.ns(|| "Lookup x"), &bits[..], &x_s)?;
    let y: FG = FG::two_bit_lookup(cs.ns(|| "Lookup y"), &bits[..], &y_s)?;

    Ok(AffineGadget::new(x, y))
}

#[derive(Derivative)]
#[derivative(Debug, Clone)]
#[derivative(Debug(bound = "P: TwistedEdwardsParameters, F: Field"))]
#[must_use]
pub struct AffineGadget<P: TwistedEdwardsParameters, F: Field, FG: FieldGadget<P::BaseField, F>> {
    pub x: FG,
    pub y: FG,
    #[derivative(Debug = "ignore")]
    _params: PhantomData<P>,
    #[derivative(Debug = "ignore")]
    _engine: PhantomData<F>,
}

impl<P: TwistedEdwardsParameters, F: Field, FG: FieldGadget<P::BaseField, F>> AffineGadget<P, F, FG> {
    pub fn new(x: FG, y: FG) -> Self {
        Self { x, y, _params: PhantomData, _engine: PhantomData }
    }
}

impl<P: TwistedEdwardsParameters, F: Field, FG: FieldGadget<P::BaseField, F>> PartialEq for AffineGadget<P, F, FG> {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl<P: TwistedEdwardsParameters, F: Field, FG: FieldGadget<P::BaseField, F>> Eq for AffineGadget<P, F, FG> {}

impl<P: TwistedEdwardsParameters, F: Field, FG: FieldGadget<P::BaseField, F>> CompressedGroupGadget<TEAffine<P>, F>
    for AffineGadget<P, F, FG>
{
    type BaseFieldGadget = FG;

    fn to_x_coordinate(&self) -> Self::BaseFieldGadget {
        self.x.clone()
    }
}

impl<P: TwistedEdwardsParameters, F: Field, FG: FieldGadget<P::BaseField, F>> GroupGadget<TEAffine<P>, F>
    for AffineGadget<P, F, FG>
{
    type Value = TEAffine<P>;
    type Variable = (FG::Variable, FG::Variable);

    #[inline]
    fn get_value(&self) -> Option<Self::Value> {
        match (self.x.get_value(), self.y.get_value()) {
            (Some(x), Some(y)) => Some(TEAffine::new(x, y)),
            (..) => None,
        }
    }

    #[inline]
    fn get_variable(&self) -> Self::Variable {
        (self.x.get_variable(), self.y.get_variable())
    }

    #[inline]
    fn zero<CS: ConstraintSystem<F>>(mut cs: CS) -> Result<Self, SynthesisError> {
        Ok(Self::new(FG::zero(cs.ns(|| "zero"))?, FG::one(cs.ns(|| "one"))?))
    }

    /// Optimized constraints for checking Edwards point addition from ZCash
    /// developers Daira Hopwood and Sean Bowe. Requires only 6 constraints
    /// compared to 7 for the straightforward version we had earlier.
    fn add<CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &Self) -> Result<Self, SynthesisError> {
        let a = P::COEFF_A;
        let d = P::COEFF_D;

        // Compute U = (x1 + y1) * (x2 + y2)
        let u1 = self.x.mul_by_constant(cs.ns(|| "-A * x1"), &a.neg())?.add(cs.ns(|| "-A * x1 + y1"), &self.y)?;
        let u2 = other.x.add(cs.ns(|| "x2 + y2"), &other.y)?;

        let u = u1.mul(cs.ns(|| "(-A * x1 + y1) * (x2 + y2)"), &u2)?;

        // Compute v0 = x1 * y2
        let v0 = other.y.mul(&mut cs.ns(|| "v0"), &self.x)?;

        // Compute v1 = x2 * y1
        let v1 = other.x.mul(&mut cs.ns(|| "v1"), &self.y)?;

        // Compute C = d*v0*v1
        let v2 = v0.mul(cs.ns(|| "v0 * v1"), &v1)?.mul_by_constant(cs.ns(|| "D * v0 * v1"), &d)?;

        // Compute x3 = (v0 + v1) / (1 + v2)
        let x3 = FG::alloc(&mut cs.ns(|| "x3"), || {
            let t0 = v0.get_value().get()? + v1.get_value().get()?;
            let t1 = P::BaseField::one() + v2.get_value().get()?;
            Ok(t0 * t1.inverse().get()?)
        })?;

        let one = P::BaseField::one();
        let v2_plus_one = v2.add_constant(cs.ns(|| "v2 + 1"), &one)?;
        let v0_plus_v1 = v0.add(cs.ns(|| "v0 + v1"), &v1)?;
        x3.mul_equals(cs.ns(|| "check x3"), &v2_plus_one, &v0_plus_v1)?;

        // Compute y3 = (U + a * v0 - v1) / (1 - v2)
        let y3 = FG::alloc(&mut cs.ns(|| "y3"), || {
            let t0 = u.get_value().get()? + (a * v0.get_value().get()?) - v1.get_value().get()?;
            let t1 = P::BaseField::one() - v2.get_value().get()?;
            Ok(t0 * t1.inverse().get()?)
        })?;

        let one_minus_v2 = v2.add_constant(cs.ns(|| "v2 - 1"), &(-one))?.negate(cs.ns(|| "1 - v2"))?;
        let a_v0 = v0.mul_by_constant(cs.ns(|| "a * v0"), &a)?;
        let u_plus_a_v0_minus_v1 = u.add(cs.ns(|| "u + a * v0"), &a_v0)?.sub(cs.ns(|| "u + a * v0 - v1"), &v1)?;

        y3.mul_equals(cs.ns(|| "check y3"), &one_minus_v2, &u_plus_a_v0_minus_v1)?;

        Ok(Self::new(x3, y3))
    }

    fn add_constant<CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &TEAffine<P>) -> Result<Self, SynthesisError> {
        let a = P::COEFF_A;
        let d = P::COEFF_D;
        let other_x = other.x;
        let other_y = other.y;

        // Compute U = (x1 + y1) * (x2 + y2)
        let u1 = self.x.mul_by_constant(cs.ns(|| "-A * x1"), &a.neg())?.add(cs.ns(|| "-A * x1 + y1"), &self.y)?;
        let u2 = other_x + other_y;

        let u = u1.mul_by_constant(cs.ns(|| "(-A * x1 + y1) * (x2 + y2)"), &u2)?;

        // Compute v0 = x1 * y2
        let v0 = self.x.mul_by_constant(&mut cs.ns(|| "v0"), &other_y)?;

        // Compute v1 = x2 * y1
        let v1 = self.y.mul_by_constant(&mut cs.ns(|| "v1"), &other.x)?;

        // Compute C = d*v0*v1
        let v2 = v0.mul(cs.ns(|| "v0 * v1"), &v1)?.mul_by_constant(cs.ns(|| "D * v0 * v1"), &d)?;

        // Compute x3 = (v0 + v1) / (1 + v2)
        let x3 = FG::alloc(&mut cs.ns(|| "x3"), || {
            let t0 = v0.get_value().get()? + v1.get_value().get()?;
            let t1 = P::BaseField::one() + v2.get_value().get()?;
            Ok(t0 * t1.inverse().get()?)
        })?;

        let one = P::BaseField::one();
        let v2_plus_one = v2.add_constant(cs.ns(|| "v2 + 1"), &one)?;
        let v0_plus_v1 = v0.add(cs.ns(|| "v0 + v1"), &v1)?;
        x3.mul_equals(cs.ns(|| "check x3"), &v2_plus_one, &v0_plus_v1)?;

        // Compute y3 = (U + a * v0 - v1) / (1 - v2)
        let y3 = FG::alloc(&mut cs.ns(|| "y3"), || {
            let t0 = u.get_value().get()? + (a * v0.get_value().get()?) - v1.get_value().get()?;
            let t1 = P::BaseField::one() - v2.get_value().get()?;
            Ok(t0 * t1.inverse().get()?)
        })?;

        let one_minus_v2 = v2.add_constant(cs.ns(|| "v2 - 1"), &(-one))?.negate(cs.ns(|| "1 - v2"))?;
        let a_v0 = v0.mul_by_constant(cs.ns(|| "a * v0"), &a)?;
        let u_plus_a_v0_minus_v1 = u.add(cs.ns(|| "u + a * v0"), &a_v0)?.sub(cs.ns(|| "u + a * v0 - v1"), &v1)?;

        y3.mul_equals(cs.ns(|| "check y3"), &one_minus_v2, &u_plus_a_v0_minus_v1)?;

        Ok(Self::new(x3, y3))
    }

    fn double_in_place<CS: ConstraintSystem<F>>(&mut self, mut cs: CS) -> Result<(), SynthesisError> {
        let a = P::COEFF_A;

        // xy
        let xy = self.x.mul(cs.ns(|| "x * y"), &self.y)?;
        let x2 = self.x.square(cs.ns(|| "x * x"))?;
        let y2 = self.y.square(cs.ns(|| "y * y"))?;

        let a_x2 = x2.mul_by_constant(cs.ns(|| "a * x^2"), &a)?;

        // Compute x3 = (2xy) / (ax^2 + y^2)
        let x3 = FG::alloc(&mut cs.ns(|| "x3"), || {
            let t0 = xy.get_value().get()?.double();
            let t1 = a * x2.get_value().get()? + y2.get_value().get()?;
            Ok(t0 * t1.inverse().get()?)
        })?;

        let a_x2_plus_y2 = a_x2.add(cs.ns(|| "a * x^2 + y^2"), &y2)?;
        let two_xy = xy.double(cs.ns(|| "2xy"))?;
        x3.mul_equals(cs.ns(|| "check x3"), &a_x2_plus_y2, &two_xy)?;

        // Compute y3 = (y^2 - ax^2) / (2 - ax^2 - y^2)
        let two = P::BaseField::one().double();
        let y3 = FG::alloc(&mut cs.ns(|| "y3"), || {
            let a_x2 = a * x2.get_value().get()?;
            let t0 = y2.get_value().get()? - a_x2;
            let t1 = two - a_x2 - y2.get_value().get()?;
            Ok(t0 * t1.inverse().get()?)
        })?;

        let y2_minus_a_x2 = y2.sub(cs.ns(|| "y^2 - ax^2"), &a_x2)?;
        let two_minus_ax2_minus_y2 = a_x2
            .add(cs.ns(|| "ax2 + y2"), &y2)?
            .negate(cs.ns(|| "-ax2 - y2"))?
            .add_constant(cs.ns(|| "2 -ax2 - y2"), &two)?;

        y3.mul_equals(cs.ns(|| "check y3"), &two_minus_ax2_minus_y2, &y2_minus_a_x2)?;
        self.x = x3;
        self.y = y3;

        Ok(())
    }

    fn negate<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Self, SynthesisError> {
        Ok(Self::new(self.x.negate(cs.ns(|| "negate x"))?, self.y.clone()))
    }

    fn scalar_multiplication<'a, CS, I, B>(
        &mut self,
        mut cs: CS,
        scalar_bits_with_base_powers: I,
    ) -> Result<(), SynthesisError>
    where
        CS: ConstraintSystem<F>,
        I: Iterator<Item = (B, &'a TEProjective<P>)>,
        B: Borrow<Boolean>,
    {
        let scalar_bits_with_base_powers: Vec<_> =
            scalar_bits_with_base_powers.map(|(bit, base)| (*bit.borrow(), *base)).collect();
        let zero = TEProjective::zero();
        for (i, bits_base_powers) in scalar_bits_with_base_powers.chunks(2).enumerate() {
            let mut cs = cs.ns(|| format!("Chunk {}", i));
            if bits_base_powers.len() == 2 {
                let bits = [bits_base_powers[0].0, bits_base_powers[1].0];
                let base_powers = [bits_base_powers[0].1, bits_base_powers[1].1];
                let table = [zero, base_powers[0], base_powers[1], base_powers[0] + base_powers[1]];
                let adder: Self = two_bit_lookup_helper(cs.ns(|| "two bit lookup"), bits, table)?;
                *self = <Self as GroupGadget<TEAffine<P>, F>>::add(self, &mut cs.ns(|| "Add"), &adder)?;
            } else if bits_base_powers.len() == 1 {
                let bit = bits_base_powers[0].0;
                let base_power = bits_base_powers[0].1.to_affine();
                let new_encoded = self.add_constant(&mut cs.ns(|| "Add base power"), &base_power)?;
                *self = Self::conditionally_select(&mut cs.ns(|| "Conditional Select"), &bit, &new_encoded, self)?;
            }
        }

        Ok(())
    }

    fn symmetric_scalar_multiplication<'a, CS, I, B>(
        &mut self,
        mut cs: CS,
        scalar_bits_with_base_powers: I,
    ) -> Result<(), SynthesisError>
    where
        CS: ConstraintSystem<F>,
        I: Iterator<Item = (B, &'a TEProjective<P>)>,
        B: Borrow<Boolean>,
    {
        let scalar_bits_with_base_powers: Vec<_> =
            scalar_bits_with_base_powers.map(|(bit, base)| (*bit.borrow(), *base)).collect();
        for (i, bits_base_powers) in scalar_bits_with_base_powers.chunks(2).enumerate() {
            let mut cs = cs.ns(|| format!("Chunk {}", i));
            if bits_base_powers.len() == 2 {
                // The symmetric hash with two-bit lookup has the result incremented by:
                //   1. g_i*g_{i+1} if the first input bit is 0 and the second input bit is 0.
                //   2. g_i^-1*g_{i+1} if the first input bit is 1 and the second input bit is 0.
                //   3. g_i*g_{i+1}^-1 if the first input bit is 0 and the second input bit is 1.
                //   4. g_i^-1*g_{i+1}^-1 if the first input bit is 1 and the second input bit is 1.
                let bits = [bits_base_powers[0].0, bits_base_powers[1].0];
                let base_powers = [bits_base_powers[0].1, bits_base_powers[1].1];
                let table = [
                    base_powers[0] + base_powers[1],
                    base_powers[0].neg() + base_powers[1],
                    base_powers[0] + base_powers[1].neg(),
                    base_powers[0].neg() + base_powers[1].neg(),
                ];
                let adder: Self = two_bit_lookup_helper(cs.ns(|| "two bit lookup"), bits, table)?;
                *self = <Self as GroupGadget<TEAffine<P>, F>>::add(self, &mut cs.ns(|| "Add"), &adder)?;
            } else if bits_base_powers.len() == 1 {
                let bit = bits_base_powers[0].0;
                let base_power = bits_base_powers[0].1.to_affine();
                let new_encoded_plus = self.add_constant(&mut cs.ns(|| "Add base power plus"), &base_power)?;
                let new_encoded_minus = self.add_constant(&mut cs.ns(|| "Add base power minus"), &base_power.neg())?;
                *self = Self::conditionally_select(
                    &mut cs.ns(|| "Conditional Select"),
                    &bit,
                    &new_encoded_plus,
                    &new_encoded_minus,
                )?;
            }
        }

        Ok(())
    }

    fn masked_scalar_multiplication<'a, CS, I, B>(
        &mut self,
        mut cs: CS,
        scalar_bits_with_base_powers: I,
        mask_bits_with_mask_powers: I,
    ) -> Result<(), SynthesisError>
    where
        CS: ConstraintSystem<F>,
        I: Iterator<Item = (B, &'a TEProjective<P>)>,
        B: Borrow<Boolean>,
    {
        for (i, ((bit, base), (mask, mask_base))) in
            scalar_bits_with_base_powers.zip(mask_bits_with_mask_powers).enumerate()
        {
            let mut cs = cs.ns(|| format!("Bit {}", i));
            // The masking algorithm specifies that the result must be incremented by:
            //   1. h_i^-1 if the input bit is 0 and the mask is 0.
            //   2. g_i*h_i^-1 if the input bit is 1 and the mask is 0.
            //   3. g_i^-1*h_i if the input bit is 0 and the mask is 1.
            //   4. h_i if the input bit is 1 and the mask is 1.
            let bits = [*bit.borrow(), *mask.borrow()];
            let table = [mask_base.neg(), mask_base.neg() + base, base.neg() + mask_base, *mask_base];
            let adder: Self = two_bit_lookup_helper(cs.ns(|| "two bit lookup"), bits, table)?;
            *self = <Self as GroupGadget<TEAffine<P>, F>>::add(self, &mut cs.ns(|| "Add"), &adder)?;
        }

        Ok(())
    }

    fn three_bit_signed_digit_scalar_multiplication<'a, CS, I, J, K, B>(
        mut cs: CS,
        bases: &[B],
        scalars: K,
    ) -> Result<Self, SynthesisError>
    where
        CS: ConstraintSystem<F>,
        I: Borrow<[Boolean]>,
        J: Iterator<Item = I>,
        K: Iterator<Item = J>,
        B: Borrow<[TEProjective<P>]>,
    {
        const CHUNK_SIZE: usize = 3;
        let mut edwards_result: Option<AffineGadget<P, F, FG>> = None;
        let mut result: Option<MontgomeryAffineGadget<P, F, FG>> = None;

        let mut process_segment_result =
            |mut cs: Namespace<_, _>, result: &MontgomeryAffineGadget<P, F, FG>| -> Result<(), SynthesisError> {
                let segment_result = result.into_edwards(cs.ns(|| "segment result"))?;
                match edwards_result {
                    None => {
                        edwards_result = Some(segment_result);
                    }
                    Some(ref mut edwards_result) => {
                        *edwards_result = GroupGadget::<TEAffine<P>, F>::add(
                            &segment_result,
                            cs.ns(|| "edwards addition"),
                            edwards_result,
                        )?;
                    }
                }

                Ok(())
            };

        // Compute ∏(h_i^{m_i}) for all i.
        let mut x_coeffs = Vec::with_capacity(4);
        let mut y_coeffs = Vec::with_capacity(4);
        for (segment_i, (segment_bits_chunks, segment_powers)) in scalars.zip(bases.iter()).enumerate() {
            for (i, (bits, base_power)) in segment_bits_chunks.zip(segment_powers.borrow().iter()).enumerate() {
                let base_power = base_power.borrow();

                let bits =
                    bits.borrow().to_bits_be(&mut cs.ns(|| format!("Convert Scalar {}, {} to bits", segment_i, i)))?;
                if bits.len() != CHUNK_SIZE {
                    return Err(SynthesisError::Unsatisfiable);
                }

                x_coeffs.clear();
                y_coeffs.clear();
                let mut acc_power = *base_power;
                for _ in 0..4 {
                    let p = acc_power;
                    let (x, y) = MontgomeryAffineGadget::<P, F, FG>::from_edwards_to_coords(&p.to_affine()).unwrap();
                    x_coeffs.push(x);
                    y_coeffs.push(y);
                    acc_power += base_power;
                }

                let precomp =
                    Boolean::and(cs.ns(|| format!("precomp in window {}, {}", segment_i, i)), &bits[0], &bits[1])?;

                let x = FG::zero(cs.ns(|| format!("x in window {}, {}", segment_i, i)))?
                    .conditionally_add_constant(
                        cs.ns(|| format!("add bool 00 in window {}, {}", segment_i, i)),
                        &Boolean::constant(true),
                        x_coeffs[0],
                    )?
                    .conditionally_add_constant(
                        cs.ns(|| format!("add bool 01 in window {}, {}", segment_i, i)),
                        &bits[0],
                        x_coeffs[1] - x_coeffs[0],
                    )?
                    .conditionally_add_constant(
                        cs.ns(|| format!("add bool 10 in window {}, {}", segment_i, i)),
                        &bits[1],
                        x_coeffs[2] - x_coeffs[0],
                    )?
                    .conditionally_add_constant(
                        cs.ns(|| format!("add bool 11 in window {}, {}", segment_i, i)),
                        &precomp,
                        x_coeffs[3] - x_coeffs[2] - x_coeffs[1] + x_coeffs[0],
                    )?;

                let y = FG::three_bit_cond_neg_lookup(
                    cs.ns(|| format!("y lookup in window {}, {}", segment_i, i)),
                    &bits,
                    &precomp,
                    &y_coeffs,
                )?;

                let tmp = MontgomeryAffineGadget::new(x, y);

                match result {
                    None => {
                        result = Some(tmp);
                    }
                    Some(ref mut result) => {
                        *result = tmp.add(cs.ns(|| format!("addition of window {}, {}", segment_i, i)), result)?;
                    }
                }
            }

            process_segment_result(cs.ns(|| format!("window {}", segment_i)), &result.unwrap())?;
            result = None;
        }
        if result.is_some() {
            process_segment_result(cs.ns(|| "leftover"), &result.unwrap())?;
        }
        Ok(edwards_result.unwrap())
    }

    fn cost_of_add() -> usize {
        4 + 2 * FG::cost_of_mul()
    }

    fn cost_of_double() -> usize {
        4 + FG::cost_of_mul()
    }

    fn ars_scalar_multiplication<'a, CS, I, B>(
        &mut self,
        mut cs: CS,
        scalar_bits_with_base_powers: I,
        idx: usize,
    ) -> Result<(), SynthesisError>
        where
            CS: ConstraintSystem<F>,
            I: Iterator<Item = (B, &'a TEProjective<P>)>,
            B: Borrow<Boolean>,
    {
        let scalar_bits_with_base_powers: Vec<_> =
            scalar_bits_with_base_powers.map(|(bit, base)| (*bit.borrow(), *base)).collect();
        let zero = TEProjective::zero();

        for (i, bits_base_powers) in scalar_bits_with_base_powers.chunks(2).enumerate() {
            let mut cs = cs.ns(|| format!("Chunk {}", i));
            if bits_base_powers.len() == 2 {
                let bits = [bits_base_powers[0].0, bits_base_powers[1].0];
                let base_powers = [bits_base_powers[0].1, bits_base_powers[1].1];
                let index = idx * 64 + i;              

                let mut table_lock = GLOBAL_SM_TABLE_CACHE.write();
                let table =  match table_lock.get(index) {
                    Some(val) => {                       
                        let global_h_read_any = val as &dyn Any;
                        let res = global_h_read_any.downcast_ref::<[Projective<P>;4]>().unwrap();
                        res.clone()
                    },
                    None => {                      
                        let mut table_t = [zero, base_powers[0], base_powers[1], base_powers[0] + base_powers[1]];
                        TEProjective::batch_normalization(&mut table_t);
                        let table_t_any = &table_t as &dyn Any;
                        let table_t_res = table_t_any.downcast_ref::<[snarkvm_curves::templates::twisted_edwards_extended::projective::Projective<snarkvm_curves::edwards_bls12::parameters::EdwardsParameters>;4]>().unwrap();
                        table_lock.push(table_t_res.clone());
                        table_t
                    },
                };
                drop(table_lock);
               
                let x_s = [table[0].x, table[1].x, table[2].x, table[3].x];
                let y_s = [table[0].y, table[1].y, table[2].y, table[3].y];
                let x: FG = FG::two_bit_lookup(cs.ns(|| "Lookup x"), &bits[..], &x_s)?;
                let y: FG = FG::two_bit_lookup(cs.ns(|| "Lookup y"), &bits[..], &y_s)?;                
                let adder: Self = AffineGadget::new(x, y);
                
                *self = <Self as GroupGadget<TEAffine<P>, F>>::add(self, &mut cs.ns(|| "Add"), &adder)?;               
            } else if bits_base_powers.len() == 1 {                
                let bit = bits_base_powers[0].0;
                let base_power = bits_base_powers[0].1.to_affine();
                let new_encoded = self.add_constant(&mut cs.ns(|| "Add base power"), &base_power)?;
                *self = Self::conditionally_select(&mut cs.ns(|| "Conditional Select"), &bit, &new_encoded, self)?;
            }
        }

        Ok(())
    }
    
    fn ars_symmetric_scalar_multiplication<'a, CS, I, B>(
        &mut self,
        mut cs: CS,
        scalar_bits_with_base_powers: I,
        idx: usize,
    ) -> Result<(), SynthesisError>
        where
            CS: ConstraintSystem<F>,
            I: Iterator<Item = (B, &'a TEProjective<P>)>,
            B: Borrow<Boolean>,
    {
        let scalar_bits_with_base_powers: Vec<_> =
            scalar_bits_with_base_powers.map(|(bit, base)| (*bit.borrow(), *base)).collect();
        for (i, bits_base_powers) in scalar_bits_with_base_powers.chunks(2).enumerate() {
            let mut cs = cs.ns(|| format!("Chunk {}", i));
            if bits_base_powers.len() == 2 {
                // The symmetric hash with two-bit lookup has the result incremented by:
                //   1. g_i*g_{i+1} if the first input bit is 0 and the second input bit is 0.
                //   2. g_i^-1*g_{i+1} if the first input bit is 1 and the second input bit is 0.
                //   3. g_i*g_{i+1}^-1 if the first input bit is 0 and the second input bit is 1.
                //   4. g_i^-1*g_{i+1}^-1 if the first input bit is 1 and the second input bit is 1.
                let bits = [bits_base_powers[0].0, bits_base_powers[1].0];
                let base_powers = [bits_base_powers[0].1, bits_base_powers[1].1];
                //let t_time = start_timer!(|| "======6=symmetric_scalar_multiplication :table");
                let index = idx * 64 + i;


                let mut table_lock = GLOBAL_SSM_TABLE_CACHE.write();
                let table =  match table_lock.get(index) {
                    Some(val) => {                        
                        let global_h_read_any = val as &dyn Any;
                        let res = global_h_read_any.downcast_ref::<[Projective<P>;4]>().unwrap();
                        res.clone()
                    },
                    None => {                        
                        let mut table_t = [
                            base_powers[0] + base_powers[1],
                            base_powers[0].neg() + base_powers[1],
                            base_powers[0] + base_powers[1].neg(),
                            base_powers[0].neg() + base_powers[1].neg(),
                        ];
                        TEProjective::batch_normalization(&mut table_t);
                        let table_t_any = &table_t as &dyn Any;
                        let table_t_res = table_t_any.downcast_ref::<[snarkvm_curves::templates::twisted_edwards_extended::projective::Projective<snarkvm_curves::edwards_bls12::parameters::EdwardsParameters>;4]>().unwrap();
                        table_lock.push(table_t_res.clone());
                        table_t
                    },
                };
                drop(table_lock);                

                let x_s = [table[0].x, table[1].x, table[2].x, table[3].x];
                let y_s = [table[0].y, table[1].y, table[2].y, table[3].y];
                let x: FG = FG::two_bit_lookup(cs.ns(|| "Lookup x"), &bits[..], &x_s)?;
                let y: FG = FG::two_bit_lookup(cs.ns(|| "Lookup y"), &bits[..], &y_s)?;                
                let adder: Self = AffineGadget::new(x, y);                
                *self = <Self as GroupGadget<TEAffine<P>, F>>::add(self, &mut cs.ns(|| "Add"), &adder)?;               
            } else if bits_base_powers.len() == 1 {
                let bit = bits_base_powers[0].0;
                let base_power = bits_base_powers[0].1.to_affine();
                let new_encoded_plus = self.add_constant(&mut cs.ns(|| "Add base power plus"), &base_power)?;
                let new_encoded_minus = self.add_constant(&mut cs.ns(|| "Add base power minus"), &base_power.neg())?;
                *self = Self::conditionally_select(
                    &mut cs.ns(|| "Conditional Select"),
                    &bit,
                    &new_encoded_plus,
                    &new_encoded_minus,
                )?;
            }
        }

        Ok(())
    }

    fn ars_masked_scalar_multiplication<'a, CS, I, B>(
        &mut self,
        mut cs: CS,
        scalar_bits_with_base_powers: I,
        mask_bits_with_mask_powers: I,
        idx: usize,
    ) -> Result<(), SynthesisError>
        where
            CS: ConstraintSystem<F>,
            I: Iterator<Item = (B, &'a TEProjective<P>)>,
            B: Borrow<Boolean>,
    {
        for (i, ((bit, base), (mask, mask_base))) in
        scalar_bits_with_base_powers.zip(mask_bits_with_mask_powers).enumerate()
        {
            let mut cs = cs.ns(|| format!("Bit {}", i));
            // The masking algorithm specifies that the result must be incremented by:
            //   1. h_i^-1 if the input bit is 0 and the mask is 0.
            //   2. g_i*h_i^-1 if the input bit is 1 and the mask is 0.
            //   3. g_i^-1*h_i if the input bit is 0 and the mask is 1.
            //   4. h_i if the input bit is 1 and the mask is 1.
            let bits = [*bit.borrow(), *mask.borrow()];

            let index = idx * 128 + i;

            let mut table_lock = GLOBAL_MSM_TABLE_CACHE.write();
            let table =  match table_lock.get(index) {
                Some(val) => {
                    let global_h_read_any = val as &dyn Any;
                    let res = global_h_read_any.downcast_ref::<[Projective<P>;4]>().unwrap();
                    res.clone()
                },
                None => {
                    let mut table_t = [mask_base.neg(), mask_base.neg() + base, base.neg() + mask_base, *mask_base];
                    TEProjective::batch_normalization(&mut table_t);
                    let table_t_any = &table_t as &dyn Any;
                    let table_t_res = table_t_any.downcast_ref::<[snarkvm_curves::templates::twisted_edwards_extended::projective::Projective<snarkvm_curves::edwards_bls12::parameters::EdwardsParameters>;4]>().unwrap();
                    table_lock.push(table_t_res.clone());
                    table_t
                },
            };
            drop(table_lock);            

            let x_s = [table[0].x, table[1].x, table[2].x, table[3].x];
            let y_s = [table[0].y, table[1].y, table[2].y, table[3].y];
            let x: FG = FG::two_bit_lookup(cs.ns(|| "Lookup x"), &bits[..], &x_s)?;
            let y: FG = FG::two_bit_lookup(cs.ns(|| "Lookup y"), &bits[..], &y_s)?;           
            let adder: Self = AffineGadget::new(x, y);

            *self = <Self as GroupGadget<TEAffine<P>, F>>::add(self, &mut cs.ns(|| "Add"), &adder)?;           
        }

        Ok(())
    }
}

impl<P: TwistedEdwardsParameters, F: Field, FG: FieldGadget<P::BaseField, F>> AllocGadget<TEAffine<P>, F>
    for AffineGadget<P, F, FG>
where
    Self: GroupGadget<TEAffine<P>, F>,
{
    fn alloc_constant<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<TEAffine<P>>, CS: ConstraintSystem<F>>(
        mut cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        let (x, y) = match value_gen() {
            Ok(ge) => {
                let ge = *ge.borrow();
                (Ok(ge.x), Ok(ge.y))
            }
            _ => (Err(SynthesisError::AssignmentMissing), Err(SynthesisError::AssignmentMissing)),
        };

        let d = P::COEFF_D;
        let a = P::COEFF_A;

        let x = FG::alloc_constant(&mut cs.ns(|| "x"), || x)?;
        let y = FG::alloc_constant(&mut cs.ns(|| "y"), || y)?;

        // Check that ax^2 + y^2 = 1 + dx^2y^2
        // We do this by checking that ax^2 - 1 = y^2 * (dx^2 - 1)
        let x2 = x.square(&mut cs.ns(|| "x^2"))?;
        let y2 = y.square(&mut cs.ns(|| "y^2"))?;

        let one = P::BaseField::one();
        let d_x2_minus_one =
            x2.mul_by_constant(cs.ns(|| "d * x^2"), &d)?.add_constant(cs.ns(|| "d * x^2 - 1"), &one.neg())?;

        let a_x2_minus_one =
            x2.mul_by_constant(cs.ns(|| "a * x^2"), &a)?.add_constant(cs.ns(|| "a * x^2 - 1"), &one.neg())?;

        d_x2_minus_one.mul_equals(cs.ns(|| "on curve check"), &y2, &a_x2_minus_one)?;

        Ok(Self::new(x, y))
    }

    fn alloc<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<TEAffine<P>>, CS: ConstraintSystem<F>>(
        mut cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        let (x, y) = match value_gen() {
            Ok(ge) => {
                let ge = *ge.borrow();
                (Ok(ge.x), Ok(ge.y))
            }
            _ => (Err(SynthesisError::AssignmentMissing), Err(SynthesisError::AssignmentMissing)),
        };

        let d = P::COEFF_D;
        let a = P::COEFF_A;

        let x = FG::alloc(&mut cs.ns(|| "x"), || x)?;
        let y = FG::alloc(&mut cs.ns(|| "y"), || y)?;

        // Check that ax^2 + y^2 = 1 + dx^2y^2
        // We do this by checking that ax^2 - 1 = y^2 * (dx^2 - 1)
        let x2 = x.square(&mut cs.ns(|| "x^2"))?;
        let y2 = y.square(&mut cs.ns(|| "y^2"))?;

        let one = P::BaseField::one();
        let d_x2_minus_one =
            x2.mul_by_constant(cs.ns(|| "d * x^2"), &d)?.add_constant(cs.ns(|| "d * x^2 - 1"), &one.neg())?;

        let a_x2_minus_one =
            x2.mul_by_constant(cs.ns(|| "a * x^2"), &a)?.add_constant(cs.ns(|| "a * x^2 - 1"), &one.neg())?;

        d_x2_minus_one.mul_equals(cs.ns(|| "on curve check"), &y2, &a_x2_minus_one)?;
        Ok(Self::new(x, y))
    }

    fn alloc_checked<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<TEAffine<P>>, CS: ConstraintSystem<F>>(
        mut cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        let cofactor_weight = BitIteratorBE::new(P::COFACTOR).filter(|b| *b).count();
        // If we multiply by r, we actually multiply by r - 2.
        let r_minus_1 = (-P::ScalarField::one()).to_repr();
        let r_weight = BitIteratorBE::new(&r_minus_1).filter(|b| *b).count();

        // We pick the most efficient method of performing the prime order check:
        // If the cofactor has lower hamming weight than the scalar field's modulus,
        // we first multiply by the inverse of the cofactor, and then, after allocating,
        // multiply by the cofactor. This ensures the resulting point has no cofactors
        //
        // Else, we multiply by the scalar field's modulus and ensure that the result
        // is zero.
        if cofactor_weight < r_weight {
            let ge =
                Self::alloc(cs.ns(|| "Alloc checked"), || value_gen().map(|ge| ge.borrow().mul_by_cofactor_inv()))?;
            let mut seen_one = false;
            let mut result = Self::zero(cs.ns(|| "result"))?;
            for (i, b) in BitIteratorBE::new(P::COFACTOR).enumerate() {
                let mut cs = cs.ns(|| format!("Iteration {}", i));

                let old_seen_one = seen_one;
                if seen_one {
                    result.double_in_place(cs.ns(|| "Double"))?;
                } else {
                    seen_one = b;
                }

                if b {
                    result = if old_seen_one { result.add(cs.ns(|| "Add"), &ge)? } else { ge.clone() };
                }
            }
            Ok(result)
        } else {
            let ge = Self::alloc(cs.ns(|| "Alloc checked"), value_gen)?;
            let mut seen_one = false;
            let mut result = Self::zero(cs.ns(|| "result"))?;
            // Returns bits in big-endian order
            for (i, b) in BitIteratorBE::new(r_minus_1).enumerate() {
                let mut cs = cs.ns(|| format!("Iteration {}", i));

                let old_seen_one = seen_one;
                if seen_one {
                    result.double_in_place(cs.ns(|| "Double"))?;
                } else {
                    seen_one = b;
                }

                if b {
                    result = if old_seen_one { result.add(cs.ns(|| "Add"), &ge)? } else { ge.clone() };
                }
            }
            let neg_ge = ge.negate(cs.ns(|| "Negate ge"))?;
            neg_ge.enforce_equal(cs.ns(|| "Check equals"), &result)?;
            Ok(ge)
        }
    }

    fn alloc_input<Fn: FnOnce() -> Result<T, SynthesisError>, T: Borrow<TEAffine<P>>, CS: ConstraintSystem<F>>(
        mut cs: CS,
        value_gen: Fn,
    ) -> Result<Self, SynthesisError> {
        let (x, y) = match value_gen() {
            Ok(ge) => {
                let ge = *ge.borrow();
                (Ok(ge.x), Ok(ge.y))
            }
            _ => (Err(SynthesisError::AssignmentMissing), Err(SynthesisError::AssignmentMissing)),
        };

        let d = P::COEFF_D;
        let a = P::COEFF_A;

        let x = FG::alloc_input(&mut cs.ns(|| "x"), || x)?;
        let y = FG::alloc_input(&mut cs.ns(|| "y"), || y)?;

        // Check that ax^2 + y^2 = 1 + dx^2y^2
        // We do this by checking that ax^2 - 1 = y^2 * (dx^2 - 1)
        let x2 = x.square(&mut cs.ns(|| "x^2"))?;
        let y2 = y.square(&mut cs.ns(|| "y^2"))?;

        let one = P::BaseField::one();
        let d_x2_minus_one =
            x2.mul_by_constant(cs.ns(|| "d * x^2"), &d)?.add_constant(cs.ns(|| "d * x^2 - 1"), &one.neg())?;

        let a_x2_minus_one =
            x2.mul_by_constant(cs.ns(|| "a * x^2"), &a)?.add_constant(cs.ns(|| "a * x^2 - 1"), &one.neg())?;

        d_x2_minus_one.mul_equals(cs.ns(|| "on curve check"), &y2, &a_x2_minus_one)?;
        Ok(Self::new(x, y))
    }
}

impl<P: TwistedEdwardsParameters, F: Field, FG: FieldGadget<P::BaseField, F>> CondSelectGadget<F>
    for AffineGadget<P, F, FG>
{
    #[inline]
    fn conditionally_select<CS: ConstraintSystem<F>>(
        mut cs: CS,
        cond: &Boolean,
        first: &Self,
        second: &Self,
    ) -> Result<Self, SynthesisError> {
        let x = FG::conditionally_select(&mut cs.ns(|| "x"), cond, &first.x, &second.x)?;
        let y = FG::conditionally_select(&mut cs.ns(|| "y"), cond, &first.y, &second.y)?;

        Ok(Self::new(x, y))
    }

    fn cost() -> usize {
        2 * <FG as CondSelectGadget<F>>::cost()
    }
}

impl<P: TwistedEdwardsParameters, F: Field, FG: FieldGadget<P::BaseField, F>> EqGadget<F> for AffineGadget<P, F, FG> {
    fn is_eq<CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &Self) -> Result<Boolean, SynthesisError> {
        let x = self.x.is_eq(cs.ns(|| "x_is_eq"), &other.x)?;
        let y = self.y.is_eq(cs.ns(|| "y_is_eq"), &other.y)?;
        Boolean::and(cs.ns(|| "x_and_y"), &x, &y)
    }
}

impl<P: TwistedEdwardsParameters, F: Field, FG: FieldGadget<P::BaseField, F>> ConditionalEqGadget<F>
    for AffineGadget<P, F, FG>
{
    #[inline]
    fn conditional_enforce_equal<CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        other: &Self,
        condition: &Boolean,
    ) -> Result<(), SynthesisError> {
        self.x.conditional_enforce_equal(&mut cs.ns(|| "X Coordinate Conditional Equality"), &other.x, condition)?;
        self.y.conditional_enforce_equal(&mut cs.ns(|| "Y Coordinate Conditional Equality"), &other.y, condition)?;
        Ok(())
    }

    fn cost() -> usize {
        2 * <FG as ConditionalEqGadget<F>>::cost()
    }
}

impl<P: TwistedEdwardsParameters, F: Field, FG: FieldGadget<P::BaseField, F>> NEqGadget<F> for AffineGadget<P, F, FG> {
    #[inline]
    fn enforce_not_equal<CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &Self) -> Result<(), SynthesisError> {
        self.x.enforce_not_equal(&mut cs.ns(|| "X Coordinate Inequality"), &other.x)?;
        self.y.enforce_not_equal(&mut cs.ns(|| "Y Coordinate Inequality"), &other.y)?;
        Ok(())
    }

    fn cost() -> usize {
        2 * <FG as NEqGadget<F>>::cost()
    }
}

impl<P: TwistedEdwardsParameters, F: Field, FG: FieldGadget<P::BaseField, F>> ToBitsBEGadget<F>
    for AffineGadget<P, F, FG>
{
    fn to_bits_be<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Vec<Boolean>, SynthesisError> {
        let mut x_bits = self.x.to_bits_be(cs.ns(|| "X Coordinate To Bits"))?;
        let y_bits = self.y.to_bits_be(cs.ns(|| "Y Coordinate To Bits"))?;
        x_bits.extend_from_slice(&y_bits);
        Ok(x_bits)
    }

    fn to_bits_be_strict<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Vec<Boolean>, SynthesisError> {
        let mut x_bits = self.x.to_bits_be_strict(cs.ns(|| "X Coordinate To Bits"))?;
        let y_bits = self.y.to_bits_be_strict(cs.ns(|| "Y Coordinate To Bits"))?;
        x_bits.extend_from_slice(&y_bits);

        Ok(x_bits)
    }
}

impl<P: TwistedEdwardsParameters, F: Field, FG: FieldGadget<P::BaseField, F>> ToBytesGadget<F>
    for AffineGadget<P, F, FG>
{
    fn to_bytes<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        let mut x_bytes = self.x.to_bytes(cs.ns(|| "x"))?;
        let y_bytes = self.y.to_bytes(cs.ns(|| "y"))?;
        x_bytes.extend_from_slice(&y_bytes);
        Ok(x_bytes)
    }

    fn to_bytes_strict<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Vec<UInt8>, SynthesisError> {
        let mut x_bytes = self.x.to_bytes_strict(cs.ns(|| "x"))?;
        let y_bytes = self.y.to_bytes_strict(cs.ns(|| "y"))?;
        x_bytes.extend_from_slice(&y_bytes);

        Ok(x_bytes)
    }
}

impl<P: TwistedEdwardsParameters, F: PrimeField> ToConstraintFieldGadget<F> for AffineGadget<P, F, FpGadget<F>>
where
    P: TwistedEdwardsParameters<BaseField = F>,
{
    fn to_constraint_field<CS: ConstraintSystem<F>>(&self, mut cs: CS) -> Result<Vec<FpGadget<F>>, SynthesisError> {
        let mut res = Vec::new();
        res.extend_from_slice(&self.x.to_constraint_field(cs.ns(|| "x_to_constraint_field"))?);
        res.extend_from_slice(&self.y.to_constraint_field(cs.ns(|| "y_to_constraint_field"))?);
        Ok(res)
    }
}