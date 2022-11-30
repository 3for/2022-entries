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

mod batched;
mod standard;

#[cfg(target_arch = "x86_64")]
pub mod prefetch;

use snarkvm_curves::{bls12_377::G1Affine, traits::AffineCurve};
use snarkvm_fields::PrimeField;

use core::any::TypeId;

pub struct VariableBase;

impl VariableBase {
    pub fn msm<G: AffineCurve>(bases: &[G], scalars: &[<G::ScalarField as PrimeField>::BigInteger]) -> G::Projective {
        // For BLS12-377, we perform variable base MSM using a batched addition technique.
        if TypeId::of::<G>() == TypeId::of::<G1Affine>() {
            #[cfg(all(feature = "cuda", target_arch = "x86_64"))]
            if scalars.len() > 1024 {
                let cuda = snarkvm_cuda::msm::
                <G, G::Projective, <G::ScalarField as PrimeField>::BigInteger>(bases, scalars);
                return cuda;
            }
            batched::msm(bases, scalars)
        }
        // For all other curves, we perform variable base MSM using Pippenger's algorithm.
        else {
            standard::msm(bases, scalars)
        }
    }

    #[cfg(test)]
    fn msm_naive<G: AffineCurve>(bases: &[G], scalars: &[<G::ScalarField as PrimeField>::BigInteger]) -> G::Projective {
        use itertools::Itertools;
        use snarkvm_utilities::BitIteratorBE;

        bases.iter().zip_eq(scalars).map(|(base, scalar)| base.mul_bits(BitIteratorBE::new(*scalar))).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snarkvm_curves::bls12_377::{Fr, G1Affine};
    use snarkvm_curves::ProjectiveCurve;
    use snarkvm_fields::PrimeField;
    use snarkvm_utilities::rand::test_rng;

    use rand_xorshift::XorShiftRng;

    fn create_scalar_bases<G: AffineCurve<ScalarField = F>, F: PrimeField>(
        rng: &mut XorShiftRng,
        size: usize,
    ) -> (Vec<G>, Vec<F::BigInteger>) {
        let bases = (0..size).map(|_| G::rand(rng)).collect::<Vec<_>>();
        let scalars = (0..size).map(|_| F::rand(rng).to_repr()).collect::<Vec<_>>();
        (bases, scalars)
    }

    #[test]
    fn test_msm() {
        let mut rng = test_rng();
        let (bases, scalars) = create_scalar_bases::<G1Affine, Fr>(&mut rng, 1000);

        let naive_a = VariableBase::msm_naive(bases.as_slice(), scalars.as_slice());
        //let naive_b = VariableBase::msm_naive_parallel(bases.as_slice(), scalars.as_slice());
        //assert_eq!(naive_a, naive_b);

        let candidate = standard::msm(bases.as_slice(), scalars.as_slice());
        assert_eq!(naive_a, candidate);

        let candidate = batched::msm(bases.as_slice(), scalars.as_slice());
        assert_eq!(naive_a, candidate);
    }

    #[cfg(all(feature = "cuda", target_arch = "x86_64"))]
    #[test]
    fn test_msm_cuda() {
        snarkvm_cuda::init_gpu();
        
        let size = 32764;
        
        let mut rng = test_rng();
        for _ in 0..1 {
            println!("Testing {}", size);
            let (bases, scalars) = create_scalar_bases::<G1Affine, Fr>(&mut rng, size);
            let rust = standard::msm(bases.as_slice(), scalars.as_slice());

            let cuda = VariableBase::msm::<G1Affine>(bases.as_slice(), scalars.as_slice());
            assert_eq!(rust.to_affine(), cuda.to_affine());
       }
    }
}
