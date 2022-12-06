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

use crate::traits::Genesis;

pub struct GenesisBlock;

impl Genesis for GenesisBlock {
    const CHECKSUM: &'static str = "";
    const SIZE: u64 = 2519;

    fn load_bytes() -> Vec<u8> {
        include_bytes!("./resources/block.genesis").to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genesis_block() {
        let block = GenesisBlock::load_bytes();
        assert_eq!(GenesisBlock::SIZE, block.len() as u64);
    }
}
