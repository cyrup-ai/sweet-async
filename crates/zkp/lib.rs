use std::fmt;
use super::data::{KeyId, EncryptedData};
use serde::{Serialize, de::DeserializeOwned};
use zeroize::{Zeroize, ZeroizeOnDrop};
use std::time::{Duration, SystemTime};
use sweet_async_api::task::{AsyncTask, AsyncTaskError};

/// Core trait for defining a zero-knowledge provable circuit
pub trait ZkProvable: Send + Sync + 'static {
    /// The type of public inputs for the proof
    type PublicInputs: Clone + Send + 'static;
    
    /// The type of private witness data
    type Witness: Clone + Send + 'static + Zeroize + ZeroizeOnDrop;
    
    /// The proof type produced by this circuit
    type Proof: Clone + Send + 'static + Serialize + DeserializeOwned;
    
    /// Create a circuit instance from witness data
    fn from_witness(witness: Self::Witness) -> Self where Self: Sized;
    
    /// Extract public inputs from the circuit
    fn public_inputs(&self) -> Self::PublicInputs;
}

/// Trait for generating zero-knowledge proofs
#[cfg(feature = "multi-node")]
pub trait ZkProver: Send + Sync {
    /// The circuit type used for proving
    type Circuit: ZkProvable;
    
    /// Generate a proof for the given circuit
    fn prove(&self, circuit: &Self::Circuit) -> AsyncTask<
        <Self::Circuit as ZkProvable>::Proof, 
        AsyncTaskError
    >;
    
    /// Generate proving parameters (typically used in setup)
    fn generate_parameters(&self) -> AsyncTask<Vec<u8>, AsyncTaskError>;
}

/// Trait for verifying zero-knowledge proofs
#[cfg(feature = "multi-node")]
pub trait ZkVerifier: Send + Sync {
    /// The circuit type associated with the proofs being verified
    type Circuit: ZkProvable;
    
    /// Verify a proof with the given public inputs
    fn verify(
        &self, 
        proof: &<Self::Circuit as ZkProvable>::Proof,
        public_inputs: &<Self::Circuit as ZkProvable>::PublicInputs
    ) -> AsyncTask<bool, AsyncTaskError>;
}

/// Trait for encrypted data that can have ZK proofs attached
#[cfg(feature = "multi-node")]
pub trait WithZkProof: Sized {
    /// Add a ZK proof to this data
    fn add_zk_proof<P: ZkProver>(&self, 
        prover: &P, 
        witness: &P::Circuit::Witness
    ) -> AsyncTask<Self, AsyncTaskError>;
    
    /// Verify a ZK proof attached to this data
    fn verify_zk_proof<V: ZkVerifier>(&self, 
        verifier: &V
    ) -> AsyncTask<bool, AsyncTaskError>;
}

/// Placeholder implementation that generates a dummy proof
/// This will be replaced with a real implementation when the clustered feature is used
#[cfg(not(feature = "clustered"))]
pub trait ZkProofData {
    fn new(data: Vec<u8>, public_inputs: Vec<u8>, proof_type: ZkProofType) -> Self;
    fn verify(&self) -> impl std::future::Future<Output = Result<bool, AsyncTaskError>> + Send;
}

#[cfg(not(feature = "clustered"))]
#[derive(Debug, Clone, Copy)]
pub enum ZkProofType {
    Knowledge,
    Range,
    Shuffle,
    Equality,
}