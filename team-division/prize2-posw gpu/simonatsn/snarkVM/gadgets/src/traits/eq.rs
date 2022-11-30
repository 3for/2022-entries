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

use snarkvm_fields::Field;
use snarkvm_r1cs::{errors::SynthesisError, ConstraintSystem};

use crate::{bits::Boolean, traits::select::CondSelectGadget};

pub trait EvaluateEqGadget<F: Field> {
    fn evaluate_equal<CS: ConstraintSystem<F>>(&self, cs: CS, other: &Self) -> Result<Boolean, SynthesisError>;
}

/// If `condition == 1`, then enforces that `self` and `other` are equal;
/// otherwise, it doesn't enforce anything.
pub trait ConditionalEqGadget<F: Field>: Eq {
    fn conditional_enforce_equal<CS: ConstraintSystem<F>>(
        &self,
        cs: CS,
        other: &Self,
        condition: &Boolean,
    ) -> Result<(), SynthesisError>;

    fn cost() -> usize;
}
impl<T: ConditionalEqGadget<F>, F: Field> ConditionalEqGadget<F> for [T] {
    fn conditional_enforce_equal<CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        other: &Self,
        condition: &Boolean,
    ) -> Result<(), SynthesisError> {
        assert_eq!(self.len(), other.len());
        for (i, (a, b)) in self.iter().zip(other.iter()).enumerate() {
            let mut cs = cs.ns(|| format!("Iteration {}", i));
            a.conditional_enforce_equal(&mut cs, b, condition)?;
        }
        Ok(())
    }

    fn cost() -> usize {
        unimplemented!()
    }
}
impl<T: ConditionalEqGadget<F>, F: Field> ConditionalEqGadget<F> for Vec<T> {
    fn conditional_enforce_equal<CS: ConstraintSystem<F>>(
        &self,
        mut cs: CS,
        other: &Self,
        condition: &Boolean,
    ) -> Result<(), SynthesisError> {
        assert_eq!(self.len(), other.len());
        for (i, (a, b)) in self.iter().zip(other.iter()).enumerate() {
            let mut cs = cs.ns(|| format!("Iteration {}", i));
            a.conditional_enforce_equal(&mut cs, b, condition)?;
        }
        Ok(())
    }

    fn cost() -> usize {
        unimplemented!()
    }
}

pub trait EqGadget<F: Field>: Eq
where
    Self: ConditionalEqGadget<F>,
{
    /// Output a `Boolean` value representing whether `self.value() ==
    /// other.value()`.
    fn is_eq<CS: ConstraintSystem<F>>(&self, _cs: CS, _other: &Self) -> Result<Boolean, SynthesisError> {
        unimplemented!()
    }

    fn enforce_equal<CS: ConstraintSystem<F>>(&self, cs: CS, other: &Self) -> Result<(), SynthesisError> {
        self.conditional_enforce_equal(cs, other, &Boolean::constant(true))
    }

    fn cost() -> usize {
        <Self as ConditionalEqGadget<F>>::cost()
    }
}

impl<T: EqGadget<F>, F: Field> EqGadget<F> for [T] {
    fn is_eq<CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &Self) -> Result<Boolean, SynthesisError> {
        assert_eq!(self.len(), other.len());
        assert!(!self.is_empty());
        let mut results = Vec::with_capacity(self.len());
        for (i, (a, b)) in self.iter().zip(other).enumerate() {
            results.push(a.is_eq(cs.ns(|| format!("is_eq_{}", i)), b)?);
        }
        Boolean::kary_and(cs.ns(|| "is_eq_kary_and"), &results)
    }
}

impl<T: EqGadget<F>, F: Field> EqGadget<F> for Vec<T> {
    fn is_eq<CS: ConstraintSystem<F>>(&self, mut cs: CS, other: &Self) -> Result<Boolean, SynthesisError> {
        assert_eq!(self.len(), other.len());
        assert!(!self.is_empty());
        let mut results = Vec::with_capacity(self.len());
        for (i, (a, b)) in self.iter().zip(other).enumerate() {
            results.push(a.is_eq(cs.ns(|| format!("is_eq_{}", i)), b)?);
        }
        Boolean::kary_and(cs.ns(|| "is_eq_kary_and"), &results)
    }
}

pub trait NEqGadget<F: Field>: Eq {
    fn enforce_not_equal<CS: ConstraintSystem<F>>(&self, cs: CS, other: &Self) -> Result<(), SynthesisError>;

    fn cost() -> usize;
}

pub trait OrEqualsGadget<F: Field>
where
    Self: Sized,
{
    fn enforce_equal_or<CS: ConstraintSystem<F>>(
        cs: CS,
        cond: &Boolean,
        var: &Self,
        first: &Self,
        second: &Self,
    ) -> Result<(), SynthesisError>;

    fn cost() -> usize;
}

impl<F: Field, T: Sized + ConditionalOrEqualsGadget<F>> OrEqualsGadget<F> for T {
    fn enforce_equal_or<CS: ConstraintSystem<F>>(
        cs: CS,
        cond: &Boolean,
        var: &Self,
        first: &Self,
        second: &Self,
    ) -> Result<(), SynthesisError> {
        Self::conditional_enforce_equal_or(cs, cond, var, first, second, &Boolean::Constant(true))
    }

    fn cost() -> usize {
        <Self as ConditionalOrEqualsGadget<F>>::cost()
    }
}

pub trait ConditionalOrEqualsGadget<F: Field>
where
    Self: Sized,
{
    fn conditional_enforce_equal_or<CS: ConstraintSystem<F>>(
        cs: CS,
        cond: &Boolean,
        var: &Self,
        first: &Self,
        second: &Self,
        should_enforce: &Boolean,
    ) -> Result<(), SynthesisError>;

    fn cost() -> usize;
}

impl<F: Field, T: Sized + ConditionalEqGadget<F> + CondSelectGadget<F>> ConditionalOrEqualsGadget<F> for T {
    fn conditional_enforce_equal_or<CS: ConstraintSystem<F>>(
        mut cs: CS,
        cond: &Boolean,
        var: &Self,
        first: &Self,
        second: &Self,
        should_enforce: &Boolean,
    ) -> Result<(), SynthesisError> {
        let match_opt = Self::conditionally_select(&mut cs.ns(|| "conditional_select_in_or"), cond, first, second)?;
        var.conditional_enforce_equal(&mut cs.ns(|| "equals_in_or"), &match_opt, should_enforce)
    }

    fn cost() -> usize {
        <Self as ConditionalEqGadget<F>>::cost() + <Self as CondSelectGadget<F>>::cost()
    }
}
