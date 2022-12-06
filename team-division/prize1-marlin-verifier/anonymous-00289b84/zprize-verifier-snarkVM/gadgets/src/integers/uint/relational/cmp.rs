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

use std::cmp::Ordering;

use snarkvm_fields::{Field, PrimeField};
use snarkvm_r1cs::{errors::SynthesisError, ConstraintSystem};

use crate::{
    bits::Boolean,
    integers::uint::UInt8,
    traits::{
        bits::{ComparatorGadget, EvaluateLtGadget, Xor},
        select::CondSelectGadget,
    },
};

macro_rules! uint_cmp_impl {
    ($($gadget: ident),*) => ($(
        /*  Bitwise less than comparison of two unsigned integers */
        impl<F: Field + PrimeField> EvaluateLtGadget<F> for $gadget {
            fn less_than<CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &Self) -> Result<Boolean, SynthesisError> {

                let mut result = Boolean::constant(true);
                let mut all_equal = Boolean::constant(true);

                // msb -> lsb
                for (i, (a, b)) in self
                    .bits
                    .iter()
                    .rev()
                    .zip(other.bits.iter().rev())
                    .enumerate()
                {
                    // a == 0 & b == 1
                    let less = Boolean::and(cs.ns(|| format!("not a and b [{}]", i)), &a.not(), b)?;

                    // a == b = !(a ^ b)
                    let not_equal = a.xor(cs.ns(|| format!("a XOR b [{}]", i)), b)?;
                    let equal = not_equal.not();

                    // evaluate a <= b
                    let less_or_equal = Boolean::or(cs.ns(|| format!("less or equal [{}]", i)), &less, &equal)?;

                    // select the current result if it is the first bit difference
                    result = Boolean::conditionally_select(cs.ns(|| format!("select bit [{}]", i)), &all_equal, &less_or_equal, &result)?;

                    // keep track of equal bits
                    all_equal = Boolean::and(cs.ns(|| format!("accumulate equal [{}]", i)), &all_equal, &equal)?;
                }

                result = Boolean::and(cs.ns(|| format!("false if all equal")), &result, &all_equal.not())?;

                Ok(result)
            }
        }

        /* Bitwise comparison of two unsigned integers */
        impl<F: Field + PrimeField> ComparatorGadget<F> for $gadget {}

        impl PartialOrd for $gadget {
            fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
                Option::from(self.value.cmp(&other.value))
            }
        }
    )*)
}

uint_cmp_impl!(UInt8);
