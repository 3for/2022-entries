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
    traits::{
        alloc::AllocGadget,
        eq::{EqGadget, NEqGadget},
        select::CondSelectGadget,
    },
};
use snarkvm_curves::traits::{AffineCurve, ProjectiveCurve};
use snarkvm_fields::Field;
use snarkvm_r1cs::{errors::SynthesisError, ConstraintSystem};

use itertools::Itertools;
use std::{borrow::Borrow, fmt::Debug};

pub trait GroupGadget<G: AffineCurve, F: Field>:
    Sized
    + ToBytesGadget<F>
    + NEqGadget<F>
    + EqGadget<F>
    + ToBitsBEGadget<F>
    + CondSelectGadget<F>
    + AllocGadget<G, F>
    + Clone
    + Debug
{
    type Value: Debug;
    type Variable;

    fn get_value(&self) -> Option<Self::Value>;

    fn get_variable(&self) -> Self::Variable;

    fn zero<CS: ConstraintSystem<F>>(cs: CS) -> Result<Self, SynthesisError>;

    fn add<CS: ConstraintSystem<F>>(&self, cs: CS, other: &Self) -> Result<Self, SynthesisError>;

    fn sub<CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &Self) -> Result<Self, SynthesisError> {
        let neg_other = other.negate(cs.ns(|| "Negate other"))?;
        self.add(cs.ns(|| "Self - other"), &neg_other)
    }

    fn add_constant<CS: ConstraintSystem<F>>(&self, cs: CS, other: &G) -> Result<Self, SynthesisError>;

    fn sub_constant<CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &G) -> Result<Self, SynthesisError> {
        let neg_other = -(*other);
        self.add_constant(cs.ns(|| "Self - other"), &neg_other)
    }

    fn double_in_place<CS: ConstraintSystem<F>>(&mut self, cs: CS) -> Result<(), SynthesisError>;

    fn negate<CS: ConstraintSystem<F>>(&self, cs: CS) -> Result<Self, SynthesisError>;

    /// Inputs must be specified in *little-endian* form.
    /// If the addition law is incomplete for the identity element,
    /// `result` must not be the identity element.
    fn mul_bits<CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        result: &Self,
        bits: impl Iterator<Item = Boolean>,
    ) -> Result<Self, SynthesisError> {
        let mut base = self.clone();
        let mut result = result.clone();
        for (i, bit) in bits.enumerate() {
            let new_encoded = result.add(&mut cs.ns(|| format!("Add {}-th power", i)), &base)?;
            result = Self::conditionally_select(
                &mut cs.ns(|| format!("Select {}", i)),
                bit.borrow(),
                &new_encoded,
                &result,
            )?;
            base.double_in_place(&mut cs.ns(|| format!("{}-th Doubling", i)))?;
        }
        Ok(result)
    }

    fn scalar_multiplication<'a, CS, I, B>(
        &mut self,
        mut cs: CS,
        scalar_bits_with_base_powers: I,
    ) -> Result<(), SynthesisError>
    where
        CS: ConstraintSystem<F>,
        I: Iterator<Item = (B, &'a G::Projective)>,
        B: Borrow<Boolean>,
        G: 'a,
    {
        for (i, (bit, base_power)) in scalar_bits_with_base_powers.enumerate() {
            let new_encoded =
                self.add_constant(&mut cs.ns(|| format!("Add {}-th base power", i)), &base_power.to_affine())?;
            *self = Self::conditionally_select(
                &mut cs.ns(|| format!("Conditional Select {}", i)),
                bit.borrow(),
                &new_encoded,
                self,
            )?;
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
        I: Iterator<Item = (B, &'a G::Projective)>,
        B: Borrow<Boolean>,
        G: 'a,
    {
        for (i, (bit, base_power)) in scalar_bits_with_base_powers.enumerate() {
            let base_power = base_power.to_affine();
            let new_encoded_plus =
                self.add_constant(&mut cs.ns(|| format!("Add {}-th base power plus", i)), &base_power)?;
            let new_encoded_minus =
                self.add_constant(&mut cs.ns(|| format!("Add {}-th base power minus", i)), &base_power.neg())?;
            *self = Self::conditionally_select(
                &mut cs.ns(|| format!("Conditional Select {}", i)),
                bit.borrow(),
                &new_encoded_plus,
                &new_encoded_minus,
            )?;
        }
        Ok(())
    }

    fn masked_scalar_multiplication<'a, CS, I, B>(&mut self, _: CS, _: I, _: I) -> Result<(), SynthesisError>
    where
        CS: ConstraintSystem<F>,
        I: Iterator<Item = (B, &'a G::Projective)>,
        B: Borrow<Boolean>,
        G: 'a,
    {
        Err(SynthesisError::AssignmentMissing)
    }

    fn three_bit_signed_digit_scalar_multiplication<CS, I, J, K, B>(
        _: CS,
        _: &[B],
        _: K,
    ) -> Result<Self, SynthesisError>
    where
        CS: ConstraintSystem<F>,
        I: Borrow<[Boolean]>,
        J: Iterator<Item = I>,
        K: Iterator<Item = J>,
        B: Borrow<[G::Projective]>,
    {
        Err(SynthesisError::AssignmentMissing)
    }

    /// Computes `Σⱼ(scalarⱼ * baseⱼ)` for all j,
    /// where `scalarⱼ` is a `Boolean` representation of the j-th scalar.
    fn multi_scalar_multiplication<'a, CS, T, I, B>(mut cs: CS, bases: &[B], scalars: I) -> Result<Self, SynthesisError>
    where
        CS: ConstraintSystem<F>,
        T: 'a + ToBitsBEGadget<F> + ?Sized,
        I: Iterator<Item = &'a T>,
        B: Borrow<[G::Projective]>,
    {
        let mut result = Self::zero(&mut cs.ns(|| "Declare Result"))?;
        // Compute Σᵢ(bitᵢ * baseᵢ) for all i.
        for (i, (bits, base_powers)) in scalars.zip_eq(bases).enumerate() {
            let base_powers = base_powers.borrow();
            let bits = bits.to_bits_be(&mut cs.ns(|| format!("Convert Scalar {} to bits", i)))?;            
            
            result.ars_scalar_multiplication(cs.ns(|| format!("Chunk {}", i)), bits.iter().zip_eq(base_powers), i)?;
        }
        Ok(result)
    }

    fn symmetric_multi_scalar_multiplication<'a, CS, T, I, B>(
        mut cs: CS,
        bases: &[B],
        scalars: I,
    ) -> Result<Self, SynthesisError>
    where
        CS: ConstraintSystem<F>,
        T: 'a + ToBitsBEGadget<F> + ?Sized,
        I: Iterator<Item = &'a T>,
        B: Borrow<[G::Projective]>,
    {
        let mut result = Self::zero(&mut cs.ns(|| "Declare Result"))?;
        // Compute ∏(h_i^{1  - 2*m_i}) for all i.
        for (i, (bits, base_powers)) in scalars.zip_eq(bases).enumerate() {
            let base_powers = base_powers.borrow();
            let bits = bits.to_bits_be(&mut cs.ns(|| format!("Convert Scalar {} to bits", i)))?;            
            result.ars_symmetric_scalar_multiplication(cs.ns(|| format!("Chunk {}", i)), bits.iter().zip_eq(base_powers), i)?;
        }
        Ok(result)
    }

    /// Compute ∏((h_i^{-1} * 1[p_i = 0] + h_i * 1[p_i = 1])^{1 - m_i \xor p_i})((g_i h_i^{-1} *
    /// 1[p_i = 0] + g_i^{-1} h_i * 1[p_i = 1])^{m_i \xor p_i}) for all i, m_i
    /// being the scalars, p_i being the masks, h_i being the symmetric Pedersen bases and g_i the
    /// Pedersen bases.
    fn masked_multi_scalar_multiplication<'a, CS, T, I, B>(
        mut cs: CS,
        bases: &[B],
        scalars: I,
        mask_bases: &[B],
        masks: I,
    ) -> Result<Self, SynthesisError>
    where
        CS: ConstraintSystem<F>,
        T: 'a + ToBitsBEGadget<F> + ?Sized,
        I: Iterator<Item = &'a T>,
        B: Borrow<[G::Projective]>,
    {
        let mut result = Self::zero(&mut cs.ns(|| "Declare Result"))?;
        for (i, (((scalar, mask), base_powers), mask_powers)) in
            scalars.zip_eq(masks).zip_eq(bases).zip_eq(mask_bases).enumerate()
        {
            let base_powers = base_powers.borrow();
            let mask_powers = mask_powers.borrow();
            let scalar_bits = scalar.to_bits_be(&mut cs.ns(|| format!("Convert scalar {} to bits", i)))?;
            let mask_bits = mask.to_bits_be(&mut cs.ns(|| format!("Convert mask {} to bits", i)))?;

            let scalar_bits_with_base_powers = scalar_bits.into_iter().zip_eq(base_powers);
            let mask_bits_with_mask_powers = mask_bits.into_iter().zip_eq(mask_powers);
            
            result.ars_masked_scalar_multiplication(
                cs.ns(|| format!("Chunk {}", i)),
                scalar_bits_with_base_powers,
                mask_bits_with_mask_powers,
                i,
            )?;
        }
        Ok(result)
    }

    fn cost_of_add() -> usize;

    fn cost_of_double() -> usize;

    fn ars_scalar_multiplication<'a, CS, I, B>(
        &mut self,
        mut cs: CS,
        scalar_bits_with_base_powers: I,
        _idx: usize,
    ) -> Result<(), SynthesisError>
        where
            CS: ConstraintSystem<F>,
            I: Iterator<Item = (B, &'a G::Projective)>,
            B: Borrow<Boolean>,
            G: 'a,
    {
        for (i, (bit, base_power)) in scalar_bits_with_base_powers.enumerate() {
            let new_encoded =
                self.add_constant(&mut cs.ns(|| format!("Add {}-th base power", i)), &base_power.to_affine())?;
            *self = Self::conditionally_select(
                &mut cs.ns(|| format!("Conditional Select {}", i)),
                bit.borrow(),
                &new_encoded,
                self,
            )?;
        }
        Ok(())
    }

    fn ars_symmetric_scalar_multiplication<'a, CS, I, B>(
        &mut self,
        mut cs: CS,
        scalar_bits_with_base_powers: I,
        _idx: usize,
    ) -> Result<(), SynthesisError>
        where
            CS: ConstraintSystem<F>,
            I: Iterator<Item = (B, &'a G::Projective)>,
            B: Borrow<Boolean>,
            G: 'a,
    {
        for (i, (bit, base_power)) in scalar_bits_with_base_powers.enumerate() {
            let base_power = base_power.to_affine();
            let new_encoded_plus =
                self.add_constant(&mut cs.ns(|| format!("Add {}-th base power plus", i)), &base_power)?;
            let new_encoded_minus =
                self.add_constant(&mut cs.ns(|| format!("Add {}-th base power minus", i)), &base_power.neg())?;
            *self = Self::conditionally_select(
                &mut cs.ns(|| format!("Conditional Select {}", i)),
                bit.borrow(),
                &new_encoded_plus,
                &new_encoded_minus,
            )?;
        }
        Ok(())
    }

    fn ars_masked_scalar_multiplication<'a, CS, I, B>(&mut self, _: CS, _: I, _: I, _:usize) -> Result<(), SynthesisError>
        where
            CS: ConstraintSystem<F>,
            I: Iterator<Item = (B, &'a G::Projective)>,
            B: Borrow<Boolean>,
            G: 'a,
    {
        Err(SynthesisError::AssignmentMissing)
    }
}

pub trait CompressedGroupGadget<G: AffineCurve, F: Field>: GroupGadget<G, F> {
    type BaseFieldGadget: ToBytesGadget<F>
        + ToBitsBEGadget<F>
        + EqGadget<F>
        + CondSelectGadget<F>
        + AllocGadget<G::BaseField, F>
        + Clone
        + Debug;

    fn to_x_coordinate(&self) -> Self::BaseFieldGadget;
}
