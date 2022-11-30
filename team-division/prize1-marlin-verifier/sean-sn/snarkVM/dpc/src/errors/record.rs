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

use snarkvm_algorithms::CRHError;

#[derive(Debug, Error)]
pub enum RecordError {
    #[error("{}", _0)]
    AccountError(#[from] crate::AccountError),

    #[error("{}", _0)]
    AnyhowError(#[from] anyhow::Error),

    #[error("{}: {}", _0, _1)]
    Crate(&'static str, String),

    #[error("{}", _0)]
    CRHError(#[from] CRHError),

    #[error("{}", _0)]
    FromHexError(#[from] hex::FromHexError),

    #[error("Given compute key does not correspond to the record owner")]
    IncorrectComputeKey,

    #[error("Invalid commitment. Expected {}, found {}", _0, _1)]
    InvalidCommitment(String, String),
}

impl From<serde_json::Error> for RecordError {
    fn from(error: serde_json::Error) -> Self {
        RecordError::Crate("serde_json::Error", format!("{:?}", error))
    }
}

impl From<std::io::Error> for RecordError {
    fn from(error: std::io::Error) -> Self {
        RecordError::Crate("std::io", format!("{:?}", error))
    }
}

impl From<RecordError> for std::io::Error {
    fn from(error: RecordError) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", error))
    }
}
