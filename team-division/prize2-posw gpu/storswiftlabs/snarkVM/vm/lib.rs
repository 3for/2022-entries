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

#![forbid(unsafe_code)]
#![allow(clippy::module_inception)]
#![allow(clippy::single_element_loop)]

#[macro_use]
extern crate tracing;

#[cfg(feature = "cli")]
#[macro_use]
extern crate thiserror;

#[cfg(feature = "cli")]
pub mod cli;
pub mod file;
pub mod package;

mod ledger;
pub use ledger::*;

#[cfg(feature = "algorithms")]
pub use snarkvm_algorithms as algorithms;
#[cfg(feature = "circuit")]
pub use snarkvm_circuit as circuit;
#[cfg(feature = "console")]
pub use snarkvm_console as console;
#[cfg(feature = "curves")]
pub use snarkvm_curves as curves;
#[cfg(feature = "dpc")]
pub use snarkvm_dpc as dpc;
#[cfg(feature = "fields")]
pub use snarkvm_fields as fields;
#[cfg(feature = "gadgets")]
pub use snarkvm_gadgets as gadgets;
#[cfg(feature = "parameters")]
pub use snarkvm_parameters as parameters;
#[cfg(feature = "r1cs")]
pub use snarkvm_r1cs as r1cs;
#[cfg(feature = "utilities")]
pub use snarkvm_utilities as utilities;

pub use snarkvm_compiler as compiler;

pub mod errors {
    #[cfg(feature = "algorithms")]
    pub use crate::algorithms::errors::*;

    #[cfg(feature = "curves")]
    pub use crate::curves::errors::*;

    #[cfg(feature = "fields")]
    pub use crate::fields::errors::*;

    #[cfg(feature = "parameters")]
    pub use crate::parameters::errors::*;

    #[cfg(feature = "r1cs")]
    pub use crate::r1cs::errors::*;
}

pub mod traits {
    #[cfg(feature = "algorithms")]
    pub use crate::algorithms::traits::*;

    #[cfg(feature = "curves")]
    pub use crate::curves::traits::*;

    #[cfg(feature = "fields")]
    pub use crate::fields::traits::*;

    #[cfg(feature = "gadgets")]
    pub use crate::gadgets::traits::*;

    #[cfg(feature = "parameters")]
    pub use crate::parameters::traits::*;
}

pub mod prelude {
    pub use crate::{errors::*, traits::*};

    #[cfg(feature = "algorithms")]
    pub use crate::algorithms::prelude::*;

    #[cfg(feature = "console")]
    pub use crate::console::{account::*, network::*, prelude::*, program::*};

    #[cfg(feature = "parameters")]
    pub use crate::parameters::prelude::*;

    #[cfg(feature = "utilities")]
    pub use crate::utilities::*;
}
