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

use crate::{AleoAmount, BlockError, DecryptionKey, Network, Record, Transaction};
use snarkvm_algorithms::merkle_tree::*;
use snarkvm_utilities::{has_duplicates, FromBytes, FromBytesDeserializer, ToBytes, ToBytesSerializer};

use anyhow::{anyhow, Result};
use rayon::prelude::*;
use serde::{
    de,
    ser::{self, SerializeStruct},
    Deserialize,
    Deserializer,
    Serialize,
    Serializer,
};
use std::{
    fmt,
    io::{Read, Result as IoResult, Write},
    ops::Deref,
    str::FromStr,
    sync::Arc,
};

#[derive(Clone, Derivative)]
#[derivative(Debug(bound = "N: Network"), PartialEq(bound = "N: Network"), Eq(bound = "N: Network"))]
pub struct Transactions<N: Network> {
    /// The list of transactions included in a block.
    transactions: Vec<Transaction<N>>,
    /// A Merkle tree composed of transaction IDs at the leaves.
    #[derivative(Debug = "ignore", PartialEq = "ignore")]
    tree: Arc<MerkleTree<N::TransactionsRootParameters>>,
}

impl<N: Network> Transactions<N> {
    /// Initializes from a given transactions list.
    pub fn from(transactions: &[Transaction<N>]) -> Result<Self, BlockError> {
        // Compute the transactions tree.
        let tree = MerkleTree::<N::TransactionsRootParameters>::new(
            Arc::new(N::transactions_root_parameters().clone()),
            &transactions.iter().map(Transaction::transaction_id).collect::<Vec<_>>(),
        )?;

        // Construct the transactions struct.
        let transactions = Self { transactions: transactions.to_vec(), tree: Arc::new(tree) };

        // Ensure the list of transactions are valid.
        match transactions.is_valid() {
            true => Ok(transactions),
            false => Err(anyhow!("Failed to initialize the transactions list").into()),
        }
    }

    /// Returns `true` if the transactions are well-formed.
    pub fn is_valid(&self) -> bool {
        // Ensure the transactions list is not empty.
        if self.transactions.is_empty() {
            eprintln!("Cannot process validity checks on an empty transactions list");
            return false;
        }

        // Ensure each transaction is well-formed.
        if !self.transactions.as_parallel_slice().par_iter().all(Transaction::is_valid) {
            eprintln!("Invalid transaction found in the transactions list");
            return false;
        }

        // Ensure there are no duplicate serial numbers.
        if has_duplicates(self.transactions.iter().flat_map(Transaction::serial_numbers)) {
            eprintln!("Found duplicate serial numbers in the transactions list");
            return false;
        }

        // Ensure there are no duplicate commitments.
        if has_duplicates(self.transactions.iter().flat_map(Transaction::commitments)) {
            eprintln!("Found duplicate commitments in the transactions list");
            return false;
        }

        // Ensure there is 1 coinbase transaction.
        let num_coinbase = self.transactions.iter().filter(|t| t.value_balance().is_negative()).count();
        if num_coinbase != 1 {
            eprintln!("Block must have exactly 1 coinbase transaction, found {}", num_coinbase);
            return false;
        }

        true
    }

    /// Returns the transaction IDs, by constructing a flattened list of transaction IDs from all transactions.
    pub fn transaction_ids(&self) -> impl Iterator<Item = N::TransactionID> + '_ {
        self.transactions.iter().map(Transaction::transaction_id)
    }

    /// Returns the transition IDs, by constructing a flattened list of transition IDs from all transactions.
    pub fn transition_ids(&self) -> impl Iterator<Item = N::TransitionID> + '_ {
        self.transactions.iter().flat_map(Transaction::transition_ids)
    }

    /// Returns the ledger roots, by constructing a flattened list of ledger roots from all transactions.
    pub fn ledger_roots(&self) -> impl Iterator<Item = N::LedgerRoot> + '_ {
        self.transactions.iter().map(Transaction::ledger_root)
    }

    /// Returns the serial numbers, by constructing a flattened list of serial numbers from all transactions.
    pub fn serial_numbers(&self) -> impl Iterator<Item = &N::SerialNumber> + '_ {
        self.transactions.iter().flat_map(Transaction::serial_numbers)
    }

    /// Returns the commitments, by constructing a flattened list of commitments from all transactions.
    pub fn commitments(&self) -> impl Iterator<Item = &N::Commitment> + '_ {
        self.transactions.iter().flat_map(Transaction::commitments)
    }

    /// Returns the net value balance, by summing the value balance from all transactions.
    pub fn net_value_balance(&self) -> AleoAmount {
        self.transactions.iter().map(Transaction::value_balance).fold(AleoAmount::ZERO, |a, b| a.add(b))
    }

    /// Returns the total transaction fees, by summing the value balance from all positive transactions.
    /// Note - this amount does *not* include the block reward.
    pub fn transaction_fees(&self) -> AleoAmount {
        self.transactions
            .iter()
            .filter_map(|t| match t.value_balance().is_negative() {
                true => None,
                false => Some(t.value_balance()),
            })
            .fold(AleoAmount::ZERO, |a, b| a.add(b))
    }

    /// Returns the coinbase transaction for the block.
    pub fn to_coinbase_transaction(&self) -> Result<Transaction<N>> {
        // Filter out all transactions with a positive value balance.
        let coinbase_transaction: Vec<_> = self.iter().filter(|t| t.value_balance().is_negative()).collect();

        // Ensure there is exactly 1 coinbase transaction.
        let num_coinbase = coinbase_transaction.len();
        match num_coinbase == 1 {
            true => Ok(coinbase_transaction[0].clone()),
            false => Err(anyhow!("Block must have 1 coinbase transaction, found {}", num_coinbase)),
        }
    }

    /// Returns the transactions root, by computing the root for a Merkle tree of the transaction IDs.
    pub fn transactions_root(&self) -> N::TransactionsRoot {
        (*self.tree.root()).into()
    }

    /// Returns an inclusion proof for the transactions tree.
    pub fn to_transactions_inclusion_proof(
        &self,
        index: usize,
        leaf: impl ToBytes,
    ) -> Result<MerklePath<N::TransactionsRootParameters>> {
        Ok(self.tree.generate_proof(index, &leaf)?)
    }

    /// Returns records from the transactions belonging to the given account view key.
    pub fn to_decrypted_records<'a>(
        &'a self,
        decryption_key: &'a DecryptionKey<N>,
    ) -> impl Iterator<Item = Record<N>> + 'a {
        self.transactions.iter().flat_map(move |transaction| transaction.to_decrypted_records(decryption_key))
    }
}

impl<N: Network> FromBytes for Transactions<N> {
    #[inline]
    fn read_le<R: Read>(mut reader: R) -> IoResult<Self> {
        let num_transactions: u16 = FromBytes::read_le(&mut reader)?;
        let mut transactions = Vec::with_capacity(num_transactions as usize);
        for _ in 0..num_transactions {
            transactions.push(FromBytes::read_le(&mut reader)?);
        }
        Ok(Self::from(&transactions)?)
    }
}

impl<N: Network> ToBytes for Transactions<N> {
    #[inline]
    fn write_le<W: Write>(&self, mut writer: W) -> IoResult<()> {
        match self.is_valid() {
            true => {
                (self.transactions.len() as u16).write_le(&mut writer)?;
                for transaction in &self.transactions {
                    transaction.write_le(&mut writer)?;
                }
                Ok(())
            }
            false => Err(BlockError::Message("Invalid transactions list".to_string()).into()),
        }
    }
}

impl<N: Network> FromStr for Transactions<N> {
    type Err = anyhow::Error;

    fn from_str(transactions: &str) -> Result<Self, Self::Err> {
        Ok(serde_json::from_str(transactions)?)
    }
}

impl<N: Network> fmt::Display for Transactions<N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", serde_json::to_string(self).map_err::<fmt::Error, _>(ser::Error::custom)?)
    }
}

impl<N: Network> Serialize for Transactions<N> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match serializer.is_human_readable() {
            true => {
                let mut transactions = serializer.serialize_struct("Transactions", 1)?;
                transactions.serialize_field("transactions", &self.transactions)?;
                transactions.end()
            }
            false => ToBytesSerializer::serialize_with_size_encoding(self, serializer),
        }
    }
}

impl<'de, N: Network> Deserialize<'de> for Transactions<N> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        match deserializer.is_human_readable() {
            true => {
                let transactions = serde_json::Value::deserialize(deserializer)?;
                let transactions: Vec<_> =
                    serde_json::from_value(transactions["transactions"].clone()).map_err(de::Error::custom)?;
                Ok(Self::from(&transactions).map_err(de::Error::custom)?)
            }
            false => FromBytesDeserializer::<Self>::deserialize_with_size_encoding(deserializer, "transactions"),
        }
    }
}

impl<N: Network> Deref for Transactions<N> {
    type Target = Vec<Transaction<N>>;

    fn deref(&self) -> &Self::Target {
        &self.transactions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{testnet2::Testnet2, Account};
    use rand::thread_rng;

    #[test]
    fn test_to_decrypted_records() {
        let rng = &mut thread_rng();
        let account = Account::<Testnet2>::new(rng);

        // Craft a transaction with 1 coinbase record.
        let (transaction, expected_record) =
            Transaction::new_coinbase(account.address(), AleoAmount(1234), true, rng).unwrap();

        // Craft a Transactions struct with 1 coinbase record.
        let transactions = Transactions::from(&[transaction]).unwrap();
        let decrypted_records =
            transactions.to_decrypted_records(&account.view_key().into()).collect::<Vec<Record<Testnet2>>>();
        assert_eq!(decrypted_records.len(), 1); // Excludes dummy records upon decryption.

        let candidate_record = decrypted_records.first().unwrap();
        assert_eq!(&expected_record, candidate_record);
        assert_eq!(expected_record.owner(), candidate_record.owner());
        assert_eq!(expected_record.value(), candidate_record.value());
        // TODO (howardwu): Reenable this after fixing how payloads are handled.
        // assert_eq!(expected_record.payload(), candidate_record.payload());
        assert_eq!(expected_record.program_id(), candidate_record.program_id());
    }

    #[test]
    fn test_duplicate_transactions() {
        // Fetch any transaction.
        let transaction = Testnet2::genesis_block().to_coinbase_transaction().unwrap();
        // Duplicate the transaction, and ensure it errors.
        assert!(Transactions::from(&[transaction.clone(), transaction]).is_err());
    }

    #[test]
    fn test_transactions_serde_json() {
        let expected_transactions = Testnet2::genesis_block().transactions().clone();

        // Serialize
        let expected_string = expected_transactions.to_string();
        let candidate_string = serde_json::to_string(&expected_transactions).unwrap();
        assert_eq!(3078, candidate_string.len(), "Update me if serialization has changed");
        assert_eq!(expected_string, candidate_string);

        // Deserialize
        assert_eq!(expected_transactions, Transactions::<Testnet2>::from_str(&candidate_string).unwrap());
        assert_eq!(expected_transactions, serde_json::from_str(&candidate_string).unwrap());
    }

    #[test]
    fn test_transactions_bincode() {
        let expected_transactions = Testnet2::genesis_block().transactions().clone();

        // Serialize
        let expected_bytes = expected_transactions.to_bytes_le().unwrap();
        let candidate_bytes = bincode::serialize(&expected_transactions).unwrap();
        assert_eq!(1519, expected_bytes.len(), "Update me if serialization has changed");
        // TODO (howardwu): Serialization - Handle the inconsistency between ToBytes and Serialize (off by a length encoding).
        assert_eq!(&expected_bytes[..], &candidate_bytes[8..]);

        // Deserialize
        assert_eq!(expected_transactions, Transactions::<Testnet2>::read_le(&expected_bytes[..]).unwrap());
        assert_eq!(expected_transactions, bincode::deserialize(&candidate_bytes[..]).unwrap());
    }
}
