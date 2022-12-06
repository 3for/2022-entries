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

use crate::{bits::ToBytesGadget, integers::uint::UInt8, traits::algorithms::MaskedCRHGadget};
use snarkvm_algorithms::traits::CRH;
use snarkvm_fields::PrimeField;
use snarkvm_r1cs::{errors::SynthesisError, ConstraintSystem};

/// Computes a root given `leaves`. Uses a nonce to mask the computation,
/// to ensure amortization resistance. Assumes the number of leaves is
/// for a full tree, so it hashes the leaves until there is only one element.
pub fn compute_masked_root<
    H: CRH,
    HG: MaskedCRHGadget<H, F>,
    F: PrimeField,
    TB: ToBytesGadget<F>,
    CS: ConstraintSystem<F>,
>(
    mut cs: CS,
    parameters: &HG,
    mask_parameters: &HG::MaskParametersGadget,
    mask: &TB,
    leaves: &[HG::OutputGadget],
) -> Result<HG::OutputGadget, SynthesisError> {
    // Mask is assumed to be derived from the nonce and the root, which will be checked by the
    // verifier.
    let mask_bytes = mask.to_bytes(cs.ns(|| "mask to bytes"))?;

    // Assume the leaves are already hashed.
    let mut current_leaves = leaves.to_vec();
    let mut level = 0;
    // Keep hashing pairs until there is only one element - the root.
    while current_leaves.len() != 1 {
        current_leaves = current_leaves
            .chunks(2)
            .enumerate()
            .map(|(i, left_right)| {
                let inner_hash = hash_inner_node_gadget::<H, HG, F, _, _>(
                    cs.ns(|| format!("hash left right {} on level {}", i, level)),
                    parameters,
                    &left_right[0],
                    &left_right[1],
                    mask_parameters,
                    mask_bytes.clone(),
                );
                inner_hash
            })
            .collect::<Result<Vec<_>, _>>()?;
        level += 1;
    }

    let computed_root = current_leaves[0].clone();

    Ok(computed_root)
}

pub(crate) fn hash_inner_node_gadget<H, HG, F, TB, CS>(
    mut cs: CS,
    parameters: &HG,
    left_child: &TB,
    right_child: &TB,
    mask_parameters: &HG::MaskParametersGadget,
    mask: Vec<UInt8>,
) -> Result<HG::OutputGadget, SynthesisError>
where
    F: PrimeField,
    CS: ConstraintSystem<F>,
    H: CRH,
    HG: MaskedCRHGadget<H, F>,
    TB: ToBytesGadget<F>,
{
    let left_bytes = left_child.to_bytes(&mut cs.ns(|| "left_to_bytes"))?;
    let right_bytes = right_child.to_bytes(&mut cs.ns(|| "right_to_bytes"))?;
    let bytes = [left_bytes, right_bytes].concat();

    parameters.check_evaluation_gadget_masked(cs, bytes, mask_parameters, mask)
}
