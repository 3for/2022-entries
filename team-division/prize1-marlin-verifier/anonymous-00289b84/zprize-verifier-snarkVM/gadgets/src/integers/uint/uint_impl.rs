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
    bits::boolean::{AllocatedBit, Boolean},
    fields::FpGadget,
    traits::{
        alloc::AllocGadget,
        eq::{ConditionalEqGadget, EqGadget},
        integers::Integer,
        select::CondSelectGadget,
    },
    ToBitsLEGadget,
    ToConstraintFieldGadget,
    UnsignedIntegerError,
};
use snarkvm_fields::{Field, FieldParameters, PrimeField, ToConstraintField};
use snarkvm_r1cs::{errors::SynthesisError, Assignment, ConstraintSystem, LinearCombination};

use core::fmt::Debug;

uint_impl!(UInt8, u8, 8);

pub trait UInt: Integer {
    /// Returns the inverse `UInt`
    fn negate(&self) -> Self;

    /// Rotate self bits by size
    fn rotr(&self, by: usize) -> Self;

    /// Perform modular addition of several `UInt` objects.
    fn addmany<F: PrimeField, CS: ConstraintSystem<F>>(cs: CS, operands: &[Self]) -> Result<Self, SynthesisError>;

    /// Perform Bitwise multiplication of two `UInt` objects.
    /// Reference: https://en.wikipedia.org/wiki/Binary_multiplier
    fn mul<F: PrimeField, CS: ConstraintSystem<F>>(&self, cs: CS, other: &Self) -> Result<Self, UnsignedIntegerError>;
}

// These methods are used throughout snarkvm-gadgets exclusively by UInt8
impl UInt8 {
    /// Construct a constant vector of `UInt8` from a vector of `u8`
    pub fn constant_vec(values: &[u8]) -> Vec<Self> {
        values.iter().copied().map(UInt8::constant).collect()
    }

    pub fn alloc_vec<F, CS, T>(mut cs: CS, values: &[T]) -> Result<Vec<Self>, SynthesisError>
    where
        F: Field,
        CS: ConstraintSystem<F>,
        T: Into<Option<u8>> + Copy,
    {
        let mut output_vec = Vec::with_capacity(values.len());
        for (i, value) in values.iter().enumerate() {
            let byte: Option<u8> = Into::into(*value);
            let alloc_byte = Self::alloc(&mut cs.ns(|| format!("byte_{}", i)), || byte.get())?;
            output_vec.push(alloc_byte);
        }
        Ok(output_vec)
    }

    /// Allocates a vector of `u8`'s by first converting them to `F` elements,
    /// (thus reducing the number of input allocations), and then converts
    /// this list of `F` gadgets back into bytes.
    pub fn alloc_input_vec_le<F, CS>(mut cs: CS, values: &[u8]) -> Result<Vec<Self>, SynthesisError>
    where
        F: PrimeField,
        CS: ConstraintSystem<F>,
    {
        // Allocates a vector of `u8`'s by first converting them to `F` elements,
        // thus reducing the number of input allocations.
        let mut bits_le = Vec::with_capacity(8 * values.len());
        for (i, field_element) in values.to_field_elements()?.iter().enumerate() {
            let fe = FpGadget::alloc_input(&mut cs.ns(|| format!("Field element {}", i)), || Ok(field_element))?;
            let fe_bits_le = fe.to_bits_le(cs.ns(|| format!("Convert fe to bits le {}", i)))?;

            // Save the byte-aligned bits.
            bits_le.extend_from_slice(&fe_bits_le[0..(8 * (F::size_in_data_bits() / 8))]);
        }

        // Chunk up slices of 8 bit into bytes.
        Ok(bits_le[0..8 * values.len()].chunks(8).map(Self::from_bits_le).collect())
    }
}

/// Parses the `Vec<UInt8>` in fixed-sized `ConstraintF::Params::CAPACITY` chunks and
/// converts each chunk, which is assumed to be little-endian, to its `FpGadget<ConstraintF>`
/// representation.
impl<ConstraintF: PrimeField> ToConstraintFieldGadget<ConstraintF> for [UInt8] {
    fn to_constraint_field<CS: ConstraintSystem<ConstraintF>>(
        &self,
        mut cs: CS,
    ) -> Result<Vec<FpGadget<ConstraintF>>, SynthesisError> {
        let max_size = (ConstraintF::Parameters::CAPACITY / 8) as usize;
        let mut res = Vec::new();
        for (i, chunk) in self.chunks(max_size).enumerate() {
            let bits = chunk.to_bits_le(cs.ns(|| format!("chunk to bits {}", i)))?;
            res.push(Boolean::le_bits_to_fp_var(cs.ns(|| format!("combine {}", i)), &bits)?);
        }
        Ok(res)
    }
}

impl<ConstraintF: PrimeField> ToConstraintFieldGadget<ConstraintF> for Vec<UInt8> {
    fn to_constraint_field<CS: ConstraintSystem<ConstraintF>>(
        &self,
        cs: CS,
    ) -> Result<Vec<FpGadget<ConstraintF>>, SynthesisError> {
        self.as_slice().to_constraint_field(cs)
    }
}
