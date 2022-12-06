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

use crate::{Circuit, LinearCombination, Variable, R1CS};
use snarkvm_curves::edwards_bls12::Fq;
use snarkvm_fields::PrimeField;

use indexmap::IndexMap;

/// A struct for tracking the mapping of variables from the virtual machine (first) to the gadget constraint system (second).
struct Converter {
    public: IndexMap<u64, snarkvm_r1cs::Variable>,
    private: IndexMap<u64, snarkvm_r1cs::Variable>,
}

impl snarkvm_r1cs::ConstraintSynthesizer<Fq> for Circuit {
    /// Synthesizes the constraints from the environment into a `snarkvm_r1cs`-compliant constraint system.
    fn generate_constraints<CS: snarkvm_r1cs::ConstraintSystem<Fq>>(
        &self,
        cs: &mut CS,
    ) -> Result<(), snarkvm_r1cs::SynthesisError> {
        crate::circuit::CIRCUIT.with(|circuit| (*(**circuit).borrow()).generate_constraints(cs))
    }
}

impl<F: PrimeField> R1CS<F> {
    /// Synthesizes the constraints from the environment into a `snarkvm_r1cs`-compliant constraint system.
    fn generate_constraints<CS: snarkvm_r1cs::ConstraintSystem<F>>(
        &self,
        cs: &mut CS,
    ) -> Result<(), snarkvm_r1cs::SynthesisError> {
        let mut converter = Converter { public: Default::default(), private: Default::default() };

        // Ensure the given `cs` is starting off clean.
        assert_eq!(1, cs.num_public_variables());
        assert_eq!(0, cs.num_private_variables());
        assert_eq!(0, cs.num_constraints());

        // Allocate the public variables.
        for (i, public) in self.to_public_variables().iter().enumerate() {
            match public {
                Variable::Public(index, value) => {
                    assert_eq!(
                        i as u64, *index,
                        "Public variables in first system must be processed in lexicographic order"
                    );

                    let gadget = cs.alloc_input(|| format!("Public {i}"), || Ok(**value))?;

                    assert_eq!(
                        snarkvm_r1cs::Index::Public((index + 1) as usize),
                        gadget.get_unchecked(),
                        "Public variables in the second system must match the first system (with an off-by-1 for the public case)"
                    );

                    let result = converter.public.insert(*index, gadget);

                    assert!(result.is_none(), "Overwrote an existing public variable in the converter");
                }
                _ => unreachable!("Public variables in the first system are not well-formed"),
            }
        }

        // Allocate the private variables.
        for (i, private) in self.to_private_variables().iter().enumerate() {
            match private {
                Variable::Private(index, value) => {
                    assert_eq!(
                        i as u64, *index,
                        "Private variables in first system must be processed in lexicographic order"
                    );

                    let gadget = cs.alloc(|| format!("Private {i}"), || Ok(**value))?;

                    assert_eq!(
                        snarkvm_r1cs::Index::Private(i),
                        gadget.get_unchecked(),
                        "Private variables in the second system must match the first system"
                    );

                    let result = converter.private.insert(*index, gadget);

                    assert!(result.is_none(), "Overwrote an existing private variable in the converter");
                }
                _ => unreachable!("Private variables in the first system are not well-formed"),
            }
        }

        // Enforce all of the constraints.
        for (i, constraint) in self.to_constraints().iter().enumerate() {
            // Converts terms from one linear combination in the first system to the second system.
            let convert_linear_combination = |lc: &LinearCombination<F>| -> snarkvm_r1cs::LinearCombination<F> {
                // Initialize a linear combination for the second system.
                let mut linear_combination = snarkvm_r1cs::LinearCombination::<F>::zero();

                // Keep an accumulator for constant values in the linear combination.
                let mut constant_accumulator = lc.to_constant();
                // Process every term in the linear combination.
                for (variable, coefficient) in lc.to_terms() {
                    match variable {
                        Variable::Constant(value) => {
                            constant_accumulator += **value;
                        }
                        Variable::Public(index, _) => {
                            let gadget = converter.public.get(index).unwrap();
                            assert_eq!(
                                snarkvm_r1cs::Index::Public((index + 1) as usize),
                                gadget.get_unchecked(),
                                "Failed during constraint translation. The public variable in the second system must match the first system (with an off-by-1 for the public case)"
                            );
                            linear_combination += (*coefficient, *gadget);
                        }
                        Variable::Private(index, _) => {
                            let gadget = converter.private.get(index).unwrap();
                            assert_eq!(
                                snarkvm_r1cs::Index::Private(*index as usize),
                                gadget.get_unchecked(),
                                "Failed during constraint translation. The private variable in the second system must match the first system"
                            );
                            linear_combination += (*coefficient, *gadget);
                        }
                    }
                }

                // Finally, add the accumulated constant value to the linear combination.
                linear_combination +=
                    (constant_accumulator, snarkvm_r1cs::Variable::new_unchecked(snarkvm_r1cs::Index::Public(0)));

                // Return the linear combination of the second system.
                linear_combination
            };

            let (a, b, c) = constraint.to_terms();

            cs.enforce(
                || format!("Constraint {i}"),
                |lc| lc + convert_linear_combination(a),
                |lc| lc + convert_linear_combination(b),
                |lc| lc + convert_linear_combination(c),
            );
        }

        // Ensure the given `cs` matches in size with the first system.
        assert_eq!(self.num_public() + 1, cs.num_public_variables() as u64);
        assert_eq!(self.num_private(), cs.num_private_variables() as u64);
        assert_eq!(self.num_constraints(), cs.num_constraints() as u64);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use snarkvm_algorithms::SNARK;
    use snarkvm_circuit::prelude::*;
    use snarkvm_curves::bls12_377::Fr;
    use snarkvm_r1cs::ConstraintSynthesizer;

    /// Compute 2^EXPONENT - 1, in a purposefully constraint-inefficient manner for testing.
    fn create_example_circuit<E: Environment>() -> Field<E> {
        let one = snarkvm_console_types::Field::<E::Network>::one();
        let two = one + one;

        const EXPONENT: u64 = 64;

        // Compute 2^EXPONENT - 1, in a purposefully constraint-inefficient manner for testing.
        let mut candidate = Field::<E>::new(Mode::Public, one);
        let mut accumulator = Field::new(Mode::Private, two);
        for _ in 0..EXPONENT {
            candidate += &accumulator;
            accumulator *= Field::new(Mode::Private, two);
        }

        assert_eq!((accumulator - Field::one()).eject_value(), candidate.eject_value());
        assert_eq!(2, E::num_public());
        assert_eq!(2 * EXPONENT + 1, E::num_private());
        assert_eq!(EXPONENT, E::num_constraints());
        assert!(E::is_satisfied());

        candidate
    }

    #[test]
    fn test_constraint_converter() {
        let _candidate_output = create_example_circuit::<Circuit>();

        let mut cs = snarkvm_r1cs::TestConstraintSystem::new();
        Circuit.generate_constraints(&mut cs).unwrap();
        {
            use snarkvm_r1cs::ConstraintSystem;
            assert_eq!(Circuit::num_public() + 1, cs.num_public_variables() as u64);
            assert_eq!(Circuit::num_private(), cs.num_private_variables() as u64);
            assert_eq!(Circuit::num_constraints(), cs.num_constraints() as u64);
            assert!(cs.is_satisfied());
        }
    }

    #[test]
    fn test_marlin() {
        let _candidate_output = create_example_circuit::<Circuit>();
        let one = snarkvm_console_types::Field::<<Circuit as Environment>::Network>::one();

        // Marlin setup, prove, and verify.

        use snarkvm_algorithms::{
            crypto_hash::PoseidonSponge,
            snark::marlin::{
                ahp::AHPForR1CS,
                fiat_shamir::FiatShamirAlgebraicSpongeRng,
                MarlinHidingMode,
                MarlinSNARK,
            },
        };
        use snarkvm_curves::bls12_377::{Bls12_377, Fq};
        use snarkvm_utilities::rand::test_crypto_rng;

        type MarlinInst = MarlinSNARK<
            Bls12_377,
            FiatShamirAlgebraicSpongeRng<Fr, Fq, PoseidonSponge<Fq, 6, 1>>,
            MarlinHidingMode,
            [Fr],
        >;

        let rng = &mut test_crypto_rng();

        let max_degree = AHPForR1CS::<Fr, MarlinHidingMode>::max_degree(200, 200, 300).unwrap();
        let universal_srs = MarlinInst::universal_setup(&max_degree, rng).unwrap();

        let (index_pk, index_vk) = MarlinInst::circuit_setup(&universal_srs, &Circuit).unwrap();
        println!("Called circuit setup");

        let proof = MarlinInst::prove(&index_pk, &Circuit, rng).unwrap();
        println!("Called prover");

        assert!(MarlinInst::verify(&index_vk, [*one, *one], &proof).unwrap());
        println!("Called verifier");
        println!("\nShould not verify (i.e. verifier messages should print below):");
        assert!(!MarlinInst::verify(&index_vk, [*one, *one + *one], &proof).unwrap());
    }
}
