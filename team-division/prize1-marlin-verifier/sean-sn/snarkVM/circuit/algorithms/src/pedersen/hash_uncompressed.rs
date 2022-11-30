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

use super::*;

use std::borrow::Cow;

impl<E: Environment, const NUM_BITS: u8> HashUncompressed for Pedersen<E, NUM_BITS> {
    type Input = Boolean<E>;
    type Output = Group<E>;

    /// Returns the Pedersen hash of the given input as an affine group element.
    fn hash_uncompressed(&self, input: &[Self::Input]) -> Self::Output {
        // Ensure the input is within the size bounds.
        let mut input = Cow::Borrowed(input);
        match input.len() <= NUM_BITS as usize {
            // Pad the input if it is under the required parameter size.
            true => input.to_mut().resize(NUM_BITS as usize, Boolean::constant(false)),
            // Ensure the input size is within the parameter size.
            false => E::halt(format!("The Pedersen hash input cannot exceed {NUM_BITS} bits.")),
        }

        // Compute the sum of base_i^{input_i} for all i.
        input
            .iter()
            .zip_eq(&self.base_window)
            .map(|(bit, base)| Group::ternary(bit, base, &Group::zero()))
            .fold(Group::<E>::zero(), |acc, x| acc + x)
    }
}

impl<E: Environment, const NUM_BITS: u8> Metrics<dyn HashUncompressed<Input = Boolean<E>, Output = Group<E>>>
    for Pedersen<E, NUM_BITS>
{
    type Case = Vec<Mode>;

    #[inline]
    fn count(case: &Self::Case) -> Count {
        // Calculate the counts for constructing each of the individual group elements from the bits of the input.
        let group_initialization_counts = case
            .iter()
            .map(|mode| {
                count!(
                    Group<E>,
                    Ternary<Boolean = Boolean<E>, Output = Group<E>>,
                    &(*mode, Mode::Constant, Mode::Constant)
                )
            })
            .fold(Count::zero(), |cumulative, count| cumulative + count);

        // Determine the modes of each of the group elements.
        let modes = case
            .iter()
            .map(|mode| {
                // The `first` and `second` inputs to `Group::ternary` are always constant so we can directly determine the mode instead of
                // using the `output_mode` macro. This avoids the need to use `CircuitType` as a parameter, simplifying the logic of this function.
                match mode.is_constant() {
                    true => Mode::Constant,
                    false => Mode::Private,
                }
            })
            .collect::<Vec<_>>();

        // Calculate the cost of summing the group elements.
        let sum_counts = match modes.split_first() {
            Some((start_mode, modes)) => {
                modes
                    .iter()
                    .fold((*start_mode, Count::zero()), |(prev_mode, cumulative), curr_mode| {
                        let mode = output_mode!(Group<E>, Add<Group<E>, Output = Group<E>>, &(prev_mode, *curr_mode));
                        let sum_count = count!(Group<E>, Add<Group<E>, Output = Group<E>>, &(prev_mode, *curr_mode));
                        (mode, cumulative + sum_count)
                    })
                    .1
            }
            None => Count::zero(),
        };

        group_initialization_counts + sum_counts
    }
}

impl<E: Environment, const NUM_BITS: u8> OutputMode<dyn HashUncompressed<Input = Boolean<E>, Output = Group<E>>>
    for Pedersen<E, NUM_BITS>
{
    type Case = Vec<Mode>;

    #[inline]
    fn output_mode(parameter: &Self::Case) -> Mode {
        match parameter.iter().all(|mode| mode.is_constant()) {
            true => Mode::Constant,
            false => Mode::Private,
        }
    }
}

#[cfg(all(test, console))]
mod tests {
    use super::*;
    use snarkvm_circuit_types::environment::Circuit;
    use snarkvm_utilities::{test_rng, Uniform};

    const ITERATIONS: u64 = 10;
    const MESSAGE: &str = "PedersenCircuit0";
    const NUM_BITS_MULTIPLIER: u8 = 8;

    fn check_hash_uncompressed<const NUM_BITS: u8>(mode: Mode) {
        use console::HashUncompressed as H;

        // Initialize the Pedersen hash.
        let native = console::Pedersen::<<Circuit as Environment>::Network, NUM_BITS>::setup(MESSAGE);
        let circuit = Pedersen::<Circuit, NUM_BITS>::constant(native.clone());

        for i in 0..ITERATIONS {
            // Sample a random input.
            let input = (0..NUM_BITS).map(|_| bool::rand(&mut test_rng())).collect::<Vec<bool>>();
            // Compute the expected hash.
            let expected = native.hash_uncompressed(&input).expect("Failed to hash native input");
            // Prepare the circuit input.
            let circuit_input: Vec<Boolean<_>> = Inject::new(mode, input);

            Circuit::scope(format!("Pedersen {mode} {i}"), || {
                // Perform the hash operation.
                let candidate = circuit.hash_uncompressed(&circuit_input);
                assert_eq!(expected, candidate.eject_value());

                // Check constraint counts and output mode.
                let modes = circuit_input.iter().map(|b| b.eject_mode()).collect::<Vec<_>>();
                assert_count!(
                    Pedersen<Circuit, NUM_BITS>,
                    HashUncompressed<Input = Boolean<Circuit>, Output = Group<Circuit>>,
                    &modes
                );
                assert_output_mode!(
                    Pedersen<Circuit, NUM_BITS>,
                    HashUncompressed<Input = Boolean<Circuit>, Output = Group<Circuit>>,
                    &modes,
                    candidate
                );
            });
        }
    }

    fn check_homomorphic_addition<C: Display + Eject + Add<Output = C> + ToBits<Boolean = Boolean<Circuit>>>(
        pedersen: &impl HashUncompressed<Input = Boolean<Circuit>, Output = Group<Circuit>>,
        first: C,
        second: C,
    ) {
        println!("Checking homomorphic addition on {} + {}", first, second);

        // Compute the expected hash, by hashing them individually and summing their results.
        let a = pedersen.hash_uncompressed(&first.to_bits_le());
        let b = pedersen.hash_uncompressed(&second.to_bits_le());
        let expected = a + b;

        // Sum the two integers, and then hash the sum.
        let candidate = pedersen.hash_uncompressed(&(first + second).to_bits_le());
        assert_eq!(expected.eject(), candidate.eject());
        assert!(Circuit::is_satisfied());
    }

    #[test]
    fn test_hash_uncompressed_constant() {
        // Set the number of windows, and modulate the window size.
        check_hash_uncompressed::<NUM_BITS_MULTIPLIER>(Mode::Constant);
        check_hash_uncompressed::<{ 2 * NUM_BITS_MULTIPLIER }>(Mode::Constant);
        check_hash_uncompressed::<{ 3 * NUM_BITS_MULTIPLIER }>(Mode::Constant);
        check_hash_uncompressed::<{ 4 * NUM_BITS_MULTIPLIER }>(Mode::Constant);
        check_hash_uncompressed::<{ 5 * NUM_BITS_MULTIPLIER }>(Mode::Constant);
    }

    #[test]
    fn test_hash_uncompressed_public() {
        // Set the number of windows, and modulate the window size.
        check_hash_uncompressed::<NUM_BITS_MULTIPLIER>(Mode::Public);
        check_hash_uncompressed::<{ 2 * NUM_BITS_MULTIPLIER }>(Mode::Public);
        check_hash_uncompressed::<{ 3 * NUM_BITS_MULTIPLIER }>(Mode::Public);
        check_hash_uncompressed::<{ 4 * NUM_BITS_MULTIPLIER }>(Mode::Public);
        check_hash_uncompressed::<{ 5 * NUM_BITS_MULTIPLIER }>(Mode::Public);
    }

    #[test]
    fn test_hash_uncompressed_private() {
        // Set the number of windows, and modulate the window size.
        check_hash_uncompressed::<NUM_BITS_MULTIPLIER>(Mode::Private);
        check_hash_uncompressed::<{ 2 * NUM_BITS_MULTIPLIER }>(Mode::Private);
        check_hash_uncompressed::<{ 3 * NUM_BITS_MULTIPLIER }>(Mode::Private);
        check_hash_uncompressed::<{ 4 * NUM_BITS_MULTIPLIER }>(Mode::Private);
        check_hash_uncompressed::<{ 5 * NUM_BITS_MULTIPLIER }>(Mode::Private);
    }

    #[test]
    fn test_pedersen64_homomorphism_private() {
        // Initialize Pedersen64.
        let pedersen = Pedersen64::constant(console::Pedersen64::setup("Pedersen64HomomorphismTest"));

        for _ in 0..ITERATIONS {
            // Sample two random unsigned integers, with the MSB set to 0.
            let first = U8::<Circuit>::new(Mode::Private, console::U8::new(u8::rand(&mut test_rng()) >> 1));
            let second = U8::new(Mode::Private, console::U8::new(u8::rand(&mut test_rng()) >> 1));
            check_homomorphic_addition(&pedersen, first, second);

            // Sample two random unsigned integers, with the MSB set to 0.
            let first = U16::<Circuit>::new(Mode::Private, console::U16::new(u16::rand(&mut test_rng()) >> 1));
            let second = U16::new(Mode::Private, console::U16::new(u16::rand(&mut test_rng()) >> 1));
            check_homomorphic_addition(&pedersen, first, second);

            // Sample two random unsigned integers, with the MSB set to 0.
            let first = U32::<Circuit>::new(Mode::Private, console::U32::new(u32::rand(&mut test_rng()) >> 1));
            let second = U32::new(Mode::Private, console::U32::new(u32::rand(&mut test_rng()) >> 1));
            check_homomorphic_addition(&pedersen, first, second);

            // Sample two random unsigned integers, with the MSB set to 0.
            let first = U64::<Circuit>::new(Mode::Private, console::U64::new(u64::rand(&mut test_rng()) >> 1));
            let second = U64::new(Mode::Private, console::U64::new(u64::rand(&mut test_rng()) >> 1));
            check_homomorphic_addition(&pedersen, first, second);
        }
    }

    #[test]
    fn test_pedersen_homomorphism_private() {
        fn check_pedersen_homomorphism(
            pedersen: &impl HashUncompressed<Input = Boolean<Circuit>, Output = Group<Circuit>>,
        ) {
            for _ in 0..ITERATIONS {
                // Sample two random unsigned integers, with the MSB set to 0.
                let first = U8::<Circuit>::new(Mode::Private, console::U8::new(u8::rand(&mut test_rng()) >> 1));
                let second = U8::new(Mode::Private, console::U8::new(u8::rand(&mut test_rng()) >> 1));
                check_homomorphic_addition(pedersen, first, second);

                // Sample two random unsigned integers, with the MSB set to 0.
                let first = U16::<Circuit>::new(Mode::Private, console::U16::new(u16::rand(&mut test_rng()) >> 1));
                let second = U16::new(Mode::Private, console::U16::new(u16::rand(&mut test_rng()) >> 1));
                check_homomorphic_addition(pedersen, first, second);

                // Sample two random unsigned integers, with the MSB set to 0.
                let first = U32::<Circuit>::new(Mode::Private, console::U32::new(u32::rand(&mut test_rng()) >> 1));
                let second = U32::new(Mode::Private, console::U32::new(u32::rand(&mut test_rng()) >> 1));
                check_homomorphic_addition(pedersen, first, second);

                // Sample two random unsigned integers, with the MSB set to 0.
                let first = U64::<Circuit>::new(Mode::Private, console::U64::new(u64::rand(&mut test_rng()) >> 1));
                let second = U64::new(Mode::Private, console::U64::new(u64::rand(&mut test_rng()) >> 1));
                check_homomorphic_addition(pedersen, first, second);

                // Sample two random unsigned integers, with the MSB set to 0.
                let first = U128::<Circuit>::new(Mode::Private, console::U128::new(u128::rand(&mut test_rng()) >> 1));
                let second = U128::new(Mode::Private, console::U128::new(u128::rand(&mut test_rng()) >> 1));
                check_homomorphic_addition(pedersen, first, second);
            }
        }

        // Check Pedersen128.
        let pedersen128 = Pedersen128::constant(console::Pedersen128::setup("Pedersen128HomomorphismTest"));
        check_pedersen_homomorphism(&pedersen128);
    }
}
