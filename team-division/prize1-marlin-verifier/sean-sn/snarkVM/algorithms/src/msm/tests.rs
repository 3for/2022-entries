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

use crate::msm::*;
use snarkvm_curves::{
    bls12_377::{Fr, G1Projective},
    traits::{AffineCurve, ProjectiveCurve},
};
use snarkvm_fields::{PrimeField, Zero};
use snarkvm_utilities::{
    rand::{test_rng, Uniform},
    BitIteratorBE,
};

fn naive_variable_base_msm<G: AffineCurve>(
    bases: &[G],
    scalars: &[<G::ScalarField as PrimeField>::BigInteger],
) -> G::Projective {
    let mut acc = G::Projective::zero();

    for (base, scalar) in bases.iter().zip(scalars.iter()) {
        acc += base.mul_bits(BitIteratorBE::new(*scalar));
    }
    acc
}

#[test]
fn variable_base_test_with_bls12() {
    const SAMPLES: usize = 1 << 10;

    let mut rng = test_rng();

    let v = (0..SAMPLES).map(|_| Fr::rand(&mut rng).to_repr()).collect::<Vec<_>>();
    let g = (0..SAMPLES).map(|_| G1Projective::rand(&mut rng).to_affine()).collect::<Vec<_>>();

    let naive = naive_variable_base_msm(g.as_slice(), v.as_slice());
    let fast = VariableBase::msm(g.as_slice(), v.as_slice());

    assert_eq!(naive.to_affine(), fast.to_affine());
}

#[test]
fn variable_base_test_with_bls12_unequal_numbers() {
    const SAMPLES: usize = 1 << 10;

    let mut rng = test_rng();

    let v = (0..SAMPLES - 1).map(|_| Fr::rand(&mut rng).to_repr()).collect::<Vec<_>>();
    let g = (0..SAMPLES).map(|_| G1Projective::rand(&mut rng).to_affine()).collect::<Vec<_>>();

    let naive = naive_variable_base_msm(g.as_slice(), v.as_slice());
    let fast = VariableBase::msm(g.as_slice(), v.as_slice());

    assert_eq!(naive.to_affine(), fast.to_affine());
}
