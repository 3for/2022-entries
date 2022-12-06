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

use snarkvm_curves::{
    bls12_377::{Bls12_377, Bls12_377Parameters, Fq12, G1Affine, G1Projective as G1, G2Affine, G2Projective as G2},
    templates::bls12::{G1Prepared, G2Prepared},
    traits::{PairingCurve, PairingEngine},
};
use snarkvm_utilities::rand::Uniform;

use criterion::Criterion;
use rand::SeedableRng;
use rand_xorshift::XorShiftRng;

use std::iter;

pub fn bench_pairing_miller_loop(c: &mut Criterion) {
    const SAMPLES: usize = 1000;

    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    let v: Vec<(G1Prepared<Bls12_377Parameters>, G2Prepared<Bls12_377Parameters>)> = (0..SAMPLES)
        .map(|_| (G1Affine::from(G1::rand(&mut rng)).prepare(), G2Affine::from(G2::rand(&mut rng)).prepare()))
        .collect();

    let mut count = 0;
    c.bench_function("bls12_377: pairing_miller_loop", |c| {
        c.iter(|| {
            let tmp = Bls12_377::miller_loop(iter::once((&v[count].0, &v[count].1)));
            count = (count + 1) % SAMPLES;
            tmp
        })
    });
}

pub fn bench_pairing_final_exponentiation(c: &mut Criterion) {
    const SAMPLES: usize = 1000;

    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    let v: Vec<Fq12> = (0..SAMPLES)
        .map(|_| (G1Affine::from(G1::rand(&mut rng)).prepare(), G2Affine::from(G2::rand(&mut rng)).prepare()))
        .map(|(ref p, ref q)| Bls12_377::miller_loop([(p, q)].iter().copied()))
        .collect();

    let mut count = 0;
    c.bench_function("bls12_377: pairing_final_exponentiation", |c| {
        c.iter(|| {
            let tmp = Bls12_377::final_exponentiation(&v[count]);
            count = (count + 1) % SAMPLES;
            tmp
        })
    });
}

pub fn bench_pairing_full(c: &mut Criterion) {
    const SAMPLES: usize = 1000;

    let mut rng = XorShiftRng::seed_from_u64(1231275789u64);

    let v: Vec<(G1, G2)> = (0..SAMPLES).map(|_| (G1::rand(&mut rng), G2::rand(&mut rng))).collect();

    let mut count = 0;
    c.bench_function("bls12_377: pairing_full", |c| {
        c.iter(|| {
            let tmp = Bls12_377::pairing(v[count].0, v[count].1);
            count = (count + 1) % SAMPLES;
            tmp
        })
    });
}
