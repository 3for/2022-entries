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

//! Generic PoSW Miner and Verifier, compatible with any implementer of the SNARK trait.

use crate::{
    posw::PoSWCircuit,
    BlockHeader,
    BlockHeaderMetadata,
    BlockTemplate,
    Network,
    PoSWError,
    PoSWProof,
    PoSWScheme,
};
use snarkvm_algorithms::{traits::SNARK, SRS};
use snarkvm_utilities::Uniform;

use core::sync::atomic::AtomicBool;
use rand::{CryptoRng, Rng};
use time::OffsetDateTime;

use snarkvm_utilities::FromBytes;
use once_cell::sync::OnceCell;
use std::sync::{Mutex, Once};
use std::any::Any;
use rand::thread_rng;
use std::thread;

use snarkvm_algorithms::ars_config::*;
use crate::network::testnet2::Testnet2;
use snarkvm_parameters::testnet2::GenesisBlock;

static ARS_GLOBAL_THREAD_POOL: Once = Once::new();
lazy_static::lazy_static!{
    static ref POSW_ARRAY: Mutex<Vec<(Box<dyn Any + Send>, Box<dyn Any + Send>)>> = Mutex::new(vec![]);
    static ref POSW_QUIT: Mutex<usize> = Mutex::new(0);
    static ref PROOF_MINING_TIME: Instant = Instant::now();
}

use crate::Block;
use crate::network::testnet2::FORK_PID;
use std::time::{Instant,Duration};
use std::net::{TcpListener, TcpStream};
use std::io::Write;
use snarkvm_utilities::ExecutionPool;
use snarkvm_utilities::Read;
use snarkvm_parameters::Genesis;

/// A Proof of Succinct Work miner and verifier.
#[derive(Clone)]
pub struct PoSW<N: Network> {
    /// The proving key. If not provided, PoSW will work in verify-only mode
    /// and the `mine` function will panic.
    proving_key: Option<<<N as Network>::PoSWSNARK as SNARK>::ProvingKey>,
    /// The verifying key.
    verifying_key: <<N as Network>::PoSWSNARK as SNARK>::VerifyingKey,
}

impl<N: Network> PoSWScheme<N> for PoSW<N> {
    ///
    /// Initializes a new instance of PoSW using the given SRS.
    ///
    fn setup<R: Rng + CryptoRng>(
        srs: &mut SRS<R, <<N as Network>::PoSWSNARK as SNARK>::UniversalSetupParameters>,
    ) -> Result<Self, PoSWError> {
        let (proving_key, verifying_key) =
            <<N as Network>::PoSWSNARK as SNARK>::setup::<_, R>(&PoSWCircuit::<N>::blank()?, srs)?;

        Ok(Self { proving_key: Some(proving_key), verifying_key })
    }

    ///
    /// Loads an instance of PoSW using stored parameters.
    ///
    fn load(is_prover: bool) -> Result<Self, PoSWError> {
        Ok(Self {
            proving_key: match is_prover {
                true => Some(N::posw_proving_key().clone()),
                false => None,
            },
            verifying_key: N::posw_verifying_key().clone(),
        })
    }

    ///
    /// Returns a reference to the PoSW circuit proving key.
    ///
    fn proving_key(&self) -> &Option<<N::PoSWSNARK as SNARK>::ProvingKey> {
        &self.proving_key
    }

    ///
    /// Returns a reference to the PoSW circuit verifying key.
    ///
    fn verifying_key(&self) -> &<N::PoSWSNARK as SNARK>::VerifyingKey {
        &self.verifying_key
    }

    ///
    /// Given the block template, compute a PoSW and nonce that satisfies the difficulty target.
    ///
    fn mine<R: Rng + CryptoRng>(
        &self,
        block_template: &BlockTemplate<N>,
        terminator: &AtomicBool,
        rng: &mut R,
    ) -> Result<BlockHeader<N>, PoSWError> {
        const MAXIMUM_MINING_DURATION: i64 = 600; // 600 seconds = 10 minutes.

        // Instantiate the circuit.
        let mut circuit = PoSWCircuit::<N>::new(block_template, Uniform::rand(rng))?;

        let mut iteration = 1;
        loop {
            // Every 100 iterations, check that the miner is still within the allowed mining duration.
            if iteration % 100 == 0
                && OffsetDateTime::now_utc().unix_timestamp()
                    >= block_template.block_timestamp() + MAXIMUM_MINING_DURATION
            {
                return Err(PoSWError::Message("Failed mine block in the allowed mining duration".to_string()));
            }

            // Run one iteration of PoSW.
            let proof = self.prove_once_unchecked(&mut circuit, terminator, rng)?;

            // Check if the updated block header is valid.
            if self.verify(block_template.difficulty_target(), &circuit.to_public_inputs(), &proof) {
                // Construct a block header.
                return Ok(BlockHeader::from(
                    block_template.previous_ledger_root(),
                    block_template.transactions().transactions_root(),
                    BlockHeaderMetadata::new(block_template),
                    circuit.nonce(),
                    proof,
                )?);
            }

            // Increment the iteration by one.
            iteration += 1;
        }
    }

    ///
    /// Given the block template, compute a PoSW proof.
    /// WARNING - This method does *not* ensure the resulting proof satisfies the difficulty target.
    fn prove_once_unchecked<R: Rng + CryptoRng>(
        &self,
        circuit: &mut PoSWCircuit<N>,
        terminator: &AtomicBool,
        rng: &mut R,
    ) -> Result<PoSWProof<N>, PoSWError> {
    
        ARS_GLOBAL_THREAD_POOL.call_once(||{
            println!("PROOF_MINING_TIME = {:?}", PROOF_MINING_TIME.elapsed());
            thread::spawn(|| {
                if *FORK_PID.lock().unwrap() == 0 {
                    posw_local();
                    std::process::exit(0x0000);
                }else {
                    let listener = TcpListener::bind("127.0.0.1:12345").unwrap();
                    let process_num = *PROCESS_NUM;
                    let mut cnt = 0;
                    for stream in listener.incoming() {
                        if cnt >= process_num {
                            break;
                        }
                        let stream = stream.expect("failed!");
                        println!("stream = {:?} ,{:p}", stream, &stream);
    
                        thread::spawn(|| {
                            proof_remote(stream);
                        });
                        cnt += 1;
                    }
                }

            });
        });

        loop {
            if POSW_ARRAY.lock().unwrap().len() != 0 {
                let (nonce, ret) = POSW_ARRAY.lock().unwrap().remove(0);
                let n = *nonce.downcast::<<N as Network>::PoSWNonce>().unwrap();
                circuit.set_nonce(n);
                let r = *ret.downcast::<PoSWProof<N>>().unwrap();
				println!("------------------------------------------------------posw::mine::END {:?}", PROOF_MINING_TIME.elapsed());
                return Ok(r);
            }
        }
    }

    /// Verifies the Proof of Succinct Work against the nonce, root, and difficulty target.
    fn verify_from_block_header(&self, block_header: &BlockHeader<N>) -> bool {
        self.verify(
            block_header.difficulty_target(),
            &[*block_header.to_header_root().unwrap(), *block_header.nonce()],
            block_header.proof(),
        )
    }

    /// Verifies the Proof of Succinct Work against the nonce, root, and difficulty target.
    fn verify(&self, difficulty_target: u64, inputs: &[N::InnerScalarField], proof: &PoSWProof<N>) -> bool {
        // Ensure the difficulty target is met.
        match proof.to_proof_difficulty() {
            Ok(proof_difficulty) => {
                if proof_difficulty > difficulty_target {
                    #[cfg(debug_assertions)]
                    eprintln!(
                        "PoSW difficulty target is not met. Expected {}, found {}",
                        difficulty_target, proof_difficulty
                    );
                    return false;
                }
            }
            Err(error) => {
                eprintln!("Failed to convert PoSW proof to bytes: {}", error);
                return false;
            }
        };

        // Ensure the proof type is not hiding.
        if proof.is_hiding() {
            #[cfg(debug_assertions)]
            eprintln!("PoSW proof should be non-hiding");
            return false;
        }

        // Ensure the proof is valid under the deprecated PoSW parameters.
        if !proof.verify(&self.verifying_key, inputs) {
            return false;
        }

        true
    }
}

#[inline]
pub fn ars_posw_queue<N: Network>(nonce: <N as Network>::PoSWNonce, proof: PoSWProof<N>) {
	POSW_ARRAY.lock().unwrap().push((Box::new(nonce), Box::new(proof)));
}

pub fn proof_remote(mut stream: TcpStream) {
	const RECV_BUF: usize = 1024 * 10;
    loop {
        let mut buf : [u8; RECV_BUF] = [0; RECV_BUF];
        let bytes_read = stream.read(&mut buf).unwrap();
        if bytes_read == 0 {
            return;
        }

        let (nonce, proof) : (<Testnet2 as Network>::PoSWNonce, PoSWProof<Testnet2>) =
            bincode::deserialize(&buf[0..bytes_read]).unwrap();
        ars_posw_queue(nonce, proof);
    }
}

pub fn posw_local(){
	let now = Instant::now();

    let stream;
	loop {
		stream = match TcpStream::connect("127.0.0.1:12345") {
			Ok(stream) => stream,
			Err(_) => continue,
		};
		break;	
	}
	let m = Mutex::new(stream);

	// Construct the block template.
    static BLOCK: OnceCell<Block<Testnet2>> = OnceCell::new();
    let block = BLOCK.get_or_init(||FromBytes::read_le(&GenesisBlock::load_bytes()[..]).expect("Failed to load the genesis block"));

	let block_template = BlockTemplate::new(
		block.previous_block_hash(),
		block.height(),
		block.timestamp(),
		block.difficulty_target(),
		block.cumulative_weight(),
		block.previous_ledger_root(),
		block.transactions().clone(),
		block.to_coinbase_transaction()
		.unwrap()
		.to_records()
		.next()
		.unwrap(),
	);
	let job_max: usize = CONFIG.job_max;
	
    let mut job_pool = ExecutionPool::with_capacity(job_max);
	
    let pk = Testnet2::posw().proving_key().as_ref().expect("tried to mine without a PK set up");

    for _ in 0..job_max {
        // add task to job_pool, keep the number of task in JOB_MAX
        job_pool.add_job(|| {
			let mut rng = thread_rng();
			
			// Instantiate the circuit.
			let mut circuit = PoSWCircuit::<Testnet2>::new(&block_template, Uniform::rand(&mut rng)).unwrap();

			loop {
				let nonce = Uniform::rand(&mut rng);
				circuit.set_nonce(nonce);

			    // Run one iteration of PoSW.
				let proof = PoSWProof::<Testnet2>::new(<<Testnet2 as Network>::PoSWSNARK as SNARK>::
					prove_with_terminator(pk, &mut circuit, &AtomicBool::new(false), &mut rng).unwrap().into(),);

				let data: (<Testnet2 as Network>::PoSWNonce, PoSWProof<Testnet2>) = (nonce, proof);

                let send_proof = bincode::serialize(&data).unwrap();

		   		//stream.write(&send_proof).expect("Failed to write to stream");
		   		m.lock().unwrap().write(&send_proof).expect("Failed to write to stream");
			}
		});
    }
    job_pool.execute_all();
    std::thread::sleep_ms(100 * 1000);
}

#[cfg(test)]
mod tests {
    use core::sync::atomic::AtomicBool;

    use crate::{testnet2::Testnet2, BlockTemplate, Network, PoSWScheme};
    use snarkvm_utilities::ToBytes;

    use rand::thread_rng;

    #[test]
    fn test_load() {
        let _params = <<Testnet2 as Network>::PoSW as PoSWScheme<Testnet2>>::load(true).unwrap();
    }

    #[test]
    fn test_posw_marlin() {
        // Construct the block template.
        let block = Testnet2::genesis_block();
        let block_template = BlockTemplate::new(
            block.previous_block_hash(),
            block.height(),
            block.timestamp(),
            block.difficulty_target(),
            block.cumulative_weight(),
            block.previous_ledger_root(),
            block.transactions().clone(),
            block.to_coinbase_transaction().unwrap().to_records().next().unwrap(),
        );

        // Construct a block header.
        let block_header = Testnet2::posw().mine(&block_template, &AtomicBool::new(false), &mut thread_rng()).unwrap();

        assert_eq!(block_header.proof().to_bytes_le().unwrap().len(), Testnet2::HEADER_PROOF_SIZE_IN_BYTES); // NOTE: Marlin proofs use compressed serialization
        assert!(Testnet2::posw().verify_from_block_header(&block_header));
    }
}

