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

use snarkvm_fields::{traits::FftParameters, FftField, Field, LegendreSymbol, PrimeField, SquareRootField};
use snarkvm_utilities::{
    io::Cursor,
    rand::test_rng,
    serialize::{CanonicalDeserialize, Flags, SWFlags},
};

use rand::Rng;

pub const ITERATIONS: u32 = 10;

fn random_negation_tests<F: Field, R: Rng>(rng: &mut R) {
    for _ in 0..ITERATIONS {
        let a = F::rand(rng);
        let mut b = -a;
        b += &a;

        assert!(b.is_zero());
    }
}

fn random_addition_tests<F: Field, R: Rng>(rng: &mut R) {
    for _ in 0..ITERATIONS {
        let a = F::rand(rng);
        let b = F::rand(rng);
        let c = F::rand(rng);

        let t0 = (a + b) + c; // (a + b) + c

        let t1 = (a + c) + b; // (a + c) + b

        let t2 = (b + c) + a; // (b + c) + a

        assert_eq!(t0, t1);
        assert_eq!(t1, t2);
    }
}

fn random_subtraction_tests<F: Field, R: Rng>(rng: &mut R) {
    for _ in 0..ITERATIONS {
        let a = F::rand(rng);
        let b = F::rand(rng);

        let t0 = a - b; // (a - b)

        let mut t1 = b; // (b - a)
        t1 -= &a;

        let mut t2 = t0; // (a - b) + (b - a) = 0
        t2 += &t1;

        assert!(t2.is_zero());
    }
}

fn random_multiplication_tests<F: Field, R: Rng>(rng: &mut R) {
    for _ in 0..ITERATIONS {
        let a = F::rand(rng);
        let b = F::rand(rng);
        let c = F::rand(rng);

        let mut t0 = a; // (a * b) * c
        t0 *= &b;
        t0 *= &c;

        let mut t1 = a; // (a * c) * b
        t1 *= &c;
        t1 *= &b;

        let mut t2 = b; // (b * c) * a
        t2 *= &c;
        t2 *= &a;

        assert_eq!(t0, t1);
        assert_eq!(t1, t2);
    }
}

fn random_inversion_tests<F: Field, R: Rng>(rng: &mut R) {
    assert!(F::zero().inverse().is_none());

    for _ in 0..ITERATIONS {
        let mut a = F::rand(rng);
        let b = a.inverse().unwrap(); // probablistically nonzero
        a *= &b;

        assert_eq!(a, F::one());
    }
}

fn random_doubling_tests<F: Field, R: Rng>(rng: &mut R) {
    for _ in 0..ITERATIONS {
        let mut a = F::rand(rng);
        let mut b = a;
        a += &b;
        b.double_in_place();

        assert_eq!(a, b);
    }
}

fn random_squaring_tests<F: Field, R: Rng>(rng: &mut R) {
    for _ in 0..ITERATIONS {
        let mut a = F::rand(rng);
        let mut b = a;
        a *= &b;
        b.square_in_place();

        assert_eq!(a, b);
    }
}

fn random_expansion_tests<F: Field, R: Rng>(rng: &mut R) {
    for _ in 0..ITERATIONS {
        // Compare (a + b)(c + d) and (a*c + b*c + a*d + b*d)

        let a = F::rand(rng);
        let b = F::rand(rng);
        let c = F::rand(rng);
        let d = F::rand(rng);

        let mut t0 = a;
        t0 += &b;
        let mut t1 = c;
        t1 += &d;
        t0 *= &t1;

        let mut t2 = a;
        t2 *= &c;
        let mut t3 = b;
        t3 *= &c;
        let mut t4 = a;
        t4 *= &d;
        let mut t5 = b;
        t5 *= &d;

        t2 += &t3;
        t2 += &t4;
        t2 += &t5;

        assert_eq!(t0, t2);
    }

    for _ in 0..ITERATIONS {
        // Compare (a + b)c and (a*c + b*c)

        let a = F::rand(rng);
        let b = F::rand(rng);
        let c = F::rand(rng);

        let t0 = (a + b) * c;
        let t2 = a * c + (b * c);

        assert_eq!(t0, t2);
    }
}

fn random_string_tests<F: PrimeField>() {
    let mut rng = test_rng();

    {
        let a = "84395729384759238745923745892374598234705297301958723458712394587103249587213984572934750213947582345792304758273458972349582734958273495872304598234";
        let b = "38495729084572938457298347502349857029384609283450692834058293405982304598230458230495820394850293845098234059823049582309485203948502938452093482039";
        let c = "3248875134290623212325429203829831876024364170316860259933542844758450336418538569901990710701240661702808867062612075657861768196242274635305077449545396068598317421057721935408562373834079015873933065667961469731886739181625866970316226171512545167081793907058686908697431878454091011239990119126";

        let mut a = F::from_str(a).map_err(|_| ()).unwrap();
        let b = F::from_str(b).map_err(|_| ()).unwrap();
        let c = F::from_str(c).map_err(|_| ()).unwrap();

        a *= &b;

        assert_eq!(a, c);
    }

    assert!(F::from_str("").is_err());
    assert!(F::from_str("0").map_err(|_| ()).unwrap().is_zero());
    assert!(F::from_str("00").is_err());
    assert!(F::from_str("00000000000").is_err());

    for _ in 0..ITERATIONS {
        let n: u64 = rng.gen();

        let a = F::from_str(&format!("{}", n)).map_err(|_| ()).unwrap();
        let b = F::from_repr(n.into()).unwrap();

        assert_eq!(a, b);
    }

    for _ in 0..ITERATIONS {
        let a = F::rand(&mut rng);

        let reference = a.to_string();
        let candidate = F::from_str(&reference).map_err(|_| panic!()).unwrap().to_string();
        assert_eq!(reference, candidate);
    }
}

fn random_sqrt_tests<F: SquareRootField>() {
    let mut rng = test_rng();

    for _ in 0..ITERATIONS {
        let a = F::rand(&mut rng);
        let b = a.square();
        assert_eq!(b.legendre(), LegendreSymbol::QuadraticResidue);

        let b = b.sqrt().unwrap();
        assert!(a == b || a == -b);
    }

    let mut c = F::one();
    for _ in 0..ITERATIONS {
        let mut b = c.square();
        assert_eq!(b.legendre(), LegendreSymbol::QuadraticResidue);

        b = b.sqrt().unwrap();

        if b != c {
            b = -b;
        }

        assert_eq!(b, c);

        c += &F::one();
    }
}

#[allow(clippy::eq_op)]
pub fn field_test<F: Field>(a: F, b: F) {
    let zero = F::zero();
    assert!(zero == zero);
    assert!(zero.is_zero()); // true
    assert!(!zero.is_one()); // false

    let one = F::one();
    assert!(one == one);
    assert!(!one.is_zero()); // false
    assert!(one.is_one()); // true
    assert_eq!(zero + one, one);

    let two = one + one;
    assert!(two == two);
    assert_ne!(zero, two);
    assert_ne!(one, two);

    // a == a
    assert!(a == a);
    // a + 0 = a
    assert_eq!(a + zero, a);
    // a - 0 = a
    assert_eq!(a - zero, a);
    // a - a = 0
    assert_eq!(a - a, zero);
    // 0 - a = -a
    assert_eq!(zero - a, -a);
    // a.double() = a + a
    assert_eq!(a.double(), a + a);
    // b.double() = b + b
    assert_eq!(b.double(), b + b);
    // a + b = b + a
    assert_eq!(a + b, b + a);
    // a - b = -(b - a)
    assert_eq!(a - b, -(b - a));
    // (a + b) + a = a + (b + a)
    assert_eq!((a + b) + a, a + (b + a));
    // (a + b).double() = (a + b) + (b + a)
    assert_eq!((a + b).double(), (a + b) + (b + a));
    assert_eq!(F::half(), F::one().double().inverse().unwrap());

    // a * 0 = 0
    assert_eq!(a * zero, zero);
    // a * 1 = a
    assert_eq!(a * one, a);
    // a * 2 = a.double()
    assert_eq!(a * two, a.double());
    // a * a^-1 = 1
    assert_eq!(a * a.inverse().unwrap(), one);
    // a * a = a^2
    assert_eq!(a * a, a.square());
    // a * a * a = a^3
    assert_eq!(a * (a * a), a.pow([0x3, 0x0, 0x0, 0x0]));
    // a * b = b * a
    assert_eq!(a * b, b * a);
    // (a * b) * a = a * (b * a)
    assert_eq!((a * b) * a, a * (b * a));
    // (a + b)^2 = a^2 + 2ab + b^2
    assert_eq!((a + b).square(), a.square() + ((a * b) + (a * b)) + b.square());
    // (a - b)^2 = (-(b - a))^2
    assert_eq!((a - b).square(), (-(b - a)).square());

    let mut rng = test_rng();
    random_negation_tests::<F, _>(&mut rng);
    random_addition_tests::<F, _>(&mut rng);
    random_subtraction_tests::<F, _>(&mut rng);
    random_multiplication_tests::<F, _>(&mut rng);
    random_inversion_tests::<F, _>(&mut rng);
    random_doubling_tests::<F, _>(&mut rng);
    random_squaring_tests::<F, _>(&mut rng);
    random_expansion_tests::<F, _>(&mut rng);

    assert!(F::zero().is_zero());
    {
        let z = -F::zero();
        assert!(z.is_zero());
    }

    assert!(F::zero().inverse().is_none());

    // Multiplication by zero
    {
        let a = F::rand(&mut rng) * F::zero();
        assert!(a.is_zero());
    }

    // Addition by zero
    {
        let mut a = F::rand(&mut rng);
        let copy = a;
        a += &F::zero();
        assert_eq!(a, copy);
    }
}

pub fn fft_field_test<F: PrimeField + FftField>() {
    let modulus_minus_one_div_two = F::from_repr(F::modulus_minus_one_div_two()).unwrap();
    assert!(!modulus_minus_one_div_two.is_zero());

    // modulus - 1 == 2^s * t
    // => t == (modulus - 1) / 2^s
    // => t == [(modulus - 1) / 2] * [1 / 2^(s-1)]
    let two_adicity = F::FftParameters::TWO_ADICITY;
    assert!(two_adicity > 0);
    let two_s_minus_one = F::from(2_u32).pow(&[(two_adicity - 1) as u64]);
    let trace = modulus_minus_one_div_two * two_s_minus_one.inverse().unwrap();
    assert_eq!(trace, F::from_repr(F::trace()).unwrap());

    // (trace - 1) / 2 == trace_minus_one_div_two
    let trace_minus_one_div_two = F::from_repr(F::trace_minus_one_div_two()).unwrap();
    assert!(!trace_minus_one_div_two.is_zero());
    assert_eq!((trace - F::one()) / F::one().double(), trace_minus_one_div_two);

    // multiplicative_generator^trace == root of unity
    let generator = F::multiplicative_generator();
    assert!(!generator.is_zero());
    let two_adic_root_of_unity = F::two_adic_root_of_unity();
    assert!(!two_adic_root_of_unity.is_zero());
    assert_eq!(two_adic_root_of_unity.pow([1 << two_adicity]), F::one());
    assert_eq!(generator.pow(trace.to_repr().as_ref()), two_adic_root_of_unity);
}

pub fn primefield_test<F: PrimeField>() {
    let one = F::one();
    assert_eq!(F::from_repr(one.to_repr()).unwrap(), one);
    assert_eq!(F::from_str("1").ok().unwrap(), one);
    assert_eq!(F::from_str(&one.to_string()).ok().unwrap(), one);

    let two = F::one().double();
    assert_eq!(F::from_repr(two.to_repr()).unwrap(), two);
    assert_eq!(F::from_str("2").ok().unwrap(), two);
    assert_eq!(F::from_str(&two.to_string()).ok().unwrap(), two);

    random_string_tests::<F>();
    fft_field_test::<F>();
}

pub fn sqrt_field_test<F: SquareRootField>(elem: F) {
    let square = elem.square();
    let sqrt = square.sqrt().unwrap();
    assert!(sqrt == elem || sqrt == -elem);
    if let Some(sqrt) = elem.sqrt() {
        assert!(sqrt.square() == elem || sqrt.square() == -elem);
    }
    random_sqrt_tests::<F>();
}

pub fn frobenius_test<F: Field, C: AsRef<[u64]>>(characteristic: C, maxpower: usize) {
    let mut rng = test_rng();

    for _ in 0..ITERATIONS {
        let a = F::rand(&mut rng);

        let mut a_0 = a;
        a_0.frobenius_map(0);
        assert_eq!(a, a_0);

        let mut a_q = a.pow(&characteristic);
        for power in 1..maxpower {
            let mut a_qi = a;
            a_qi.frobenius_map(power);
            assert_eq!(a_qi, a_q);

            a_q = a_q.pow(&characteristic);
        }
    }
}
pub fn field_serialization_test<F: Field>() {
    let mut rng = &mut rand::thread_rng();
    use snarkvm_utilities::serialize::{Compress, Validate};
    let modes = [
        (Compress::No, Validate::No),
        (Compress::Yes, Validate::No),
        (Compress::Yes, Validate::Yes),
        (Compress::No, Validate::Yes),
    ];

    for _ in 0..ITERATIONS {
        let a = F::rand(&mut rng);
        for (compress, validate) in modes {
            let serialized_size = a.serialized_size(compress);
            let mut serialized = vec![0u8; serialized_size];
            let mut cursor = Cursor::new(&mut serialized);
            a.serialize_with_mode(&mut cursor, compress).unwrap();
            let serialized2 = bincode::serialize(&a).unwrap();
            assert_eq!(serialized, serialized2);

            let mut cursor = Cursor::new(&serialized[..]);
            let b = F::deserialize_with_mode(&mut cursor, compress, validate).unwrap();
            let c: F = bincode::deserialize(&serialized).unwrap();
            assert_eq!(a, b);
            assert_eq!(a, c);
        }
        {
            let mut serialized = vec![0u8; a.uncompressed_size()];
            let mut cursor = Cursor::new(&mut serialized[..]);
            a.serialize_uncompressed(&mut cursor).unwrap();
            let mut cursor = Cursor::new(&serialized[..]);
            let b = F::deserialize_uncompressed(&mut cursor).unwrap();
            assert_eq!(a, b);
        }

        {
            let mut serialized = vec![0u8; F::one().serialized_size_with_flags::<SWFlags>()];
            let mut cursor = Cursor::new(&mut serialized[..]);
            a.serialize_with_flags(&mut cursor, SWFlags::from_y_sign(true)).unwrap();
            let mut cursor = Cursor::new(&serialized[..]);
            let (b, flags) = F::deserialize_with_flags::<_, SWFlags>(&mut cursor).unwrap();
            assert_eq!(flags.is_positive(), Some(true));
            assert!(!flags.is_infinity());
            assert_eq!(a, b);
        }
        #[derive(Default, Clone, Copy, Debug)]
        struct DummyFlags;
        impl Flags for DummyFlags {
            const BIT_SIZE: usize = 200;

            fn u8_bitmask(&self) -> u8 {
                0
            }

            fn from_u8(_value: u8) -> Option<Self> {
                Some(DummyFlags)
            }

            fn from_u8_remove_flags(_value: &mut u8) -> Option<Self> {
                Some(DummyFlags)
            }
        }

        use snarkvm_utilities::serialize::SerializationError;
        {
            let mut serialized = vec![0; F::one().serialized_size_with_flags::<DummyFlags>()];
            assert!(matches!(
                a.serialize_with_flags(&mut serialized[..], DummyFlags).unwrap_err(),
                SerializationError::NotEnoughSpace
            ));
            assert!(matches!(
                F::deserialize_with_flags::<_, DummyFlags>(&mut &serialized[..]).unwrap_err(),
                SerializationError::NotEnoughSpace
            ));
        }

        {
            for (compress, validate) in modes {
                let mut serialized = vec![0; F::one().serialized_size(compress) - 1];
                let mut cursor = Cursor::new(&mut serialized[..]);
                a.serialize_with_mode(&mut cursor, compress).unwrap_err();
                let mut cursor = Cursor::new(&serialized[..]);
                <F as CanonicalDeserialize>::deserialize_with_mode(&mut cursor, compress, validate).unwrap_err();
            }
        }
    }
}
