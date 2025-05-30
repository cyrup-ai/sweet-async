//! Cryptographic utilities for sweet_async
//! 
//! This crate provides cryptographic primitives and utilities for secure
//! communication, data encryption, and zero-knowledge proofs in distributed systems.

#![warn(missing_docs)]
#![forbid(unsafe_code)]

pub mod crypt;

// Re-export main types and traits from the crypt module
pub use crypt::{
    cipher::{Cipher, CipherError, CipherProvider},
    data::{EncryptedData, DataError, DataProvider},
    provider::{Provider, ProviderError, ProviderConfig},
    zk::{ZeroKnowledgeProof, ZkProof, ZkError, ZkProvider},
};

// Common traits
pub use crypt::{
    Encrypt, Decrypt, Sign, Verify,
    KeyDerivation, SecureRandom,
};

// Prelude for convenient imports
pub mod prelude {
    pub use crate::{
        Cipher, CipherProvider,
        EncryptedData, DataProvider,
        Provider, ProviderConfig,
        ZeroKnowledgeProof, ZkProof,
        Encrypt, Decrypt, Sign, Verify,
    };
}