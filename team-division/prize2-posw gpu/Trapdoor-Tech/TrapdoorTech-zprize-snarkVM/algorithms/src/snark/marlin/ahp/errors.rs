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

/// Describes the failure modes of the AHP scheme.
#[derive(Debug)]
pub enum AHPError {
    /// An error occurred during constraint generation.
    ConstraintSystemError(snarkvm_r1cs::errors::SynthesisError),
    /// An error occurred during Fiat-Shamir.
    FiatShamirError(crate::snark::marlin::fiat_shamir::FiatShamirError),
    /// The instance generated during proving does not match that in the index.
    InstanceDoesNotMatchIndex,
    /// The number of public inputs is incorrect.
    InvalidPublicInputLength,
    /// During verification, a required evaluation is missing
    MissingEval(String),
    /// Currently we only support square constraint matrices.
    NonSquareMatrix,
    /// During synthesis, our polynomials ended up being too high of degree
    PolynomialDegreeTooLarge,
    /// GPU related error 
    GpuError(ec_gpu_common::GPUError),
}

impl From<crate::snark::marlin::fiat_shamir::FiatShamirError> for AHPError {
    fn from(other: crate::snark::marlin::fiat_shamir::FiatShamirError) -> Self {
        AHPError::FiatShamirError(other)
    }
}

impl From<snarkvm_r1cs::errors::SynthesisError> for AHPError {
    fn from(other: snarkvm_r1cs::errors::SynthesisError) -> Self {
        AHPError::ConstraintSystemError(other)
    }
}

impl From<ec_gpu_common::GPUError> for AHPError {
    fn from(e: ec_gpu_common::GPUError) -> AHPError {
        AHPError::GpuError(e)
    }
}
