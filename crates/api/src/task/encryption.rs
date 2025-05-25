use crate::task::AsyncTaskError;

/// Trait for encryption providers
pub trait EncryptionProvider: Send + Sync {
    /// Encrypt data using the specified method
    fn encrypt(&self, data: &[u8], key_id: Option<&str>) -> Result<EncryptedData, AsyncTaskError>;
    
    /// Decrypt data
    fn decrypt(&self, encrypted: &EncryptedData) -> Result<Vec<u8>, AsyncTaskError>;
    
    /// Generate a new encryption key
    fn generate_key(&self, key_id: &str) -> Result<(), AsyncTaskError>;
    
    /// Rotate encryption keys
    fn rotate_keys(&self) -> Result<(), AsyncTaskError>;
}

/// Encrypted data with metadata
#[derive(Clone, Debug)]
pub struct EncryptedData {
    /// The encrypted payload
    pub ciphertext: Vec<u8>,
    /// Nonce/IV used for encryption
    pub nonce: Vec<u8>,
    /// Algorithm used
    pub algorithm: EncryptionAlgorithm,
    /// Key identifier
    pub key_id: Option<String>,
    /// Additional authenticated data (AAD)
    pub aad: Option<Vec<u8>>,
}

/// Supported encryption algorithms
#[derive(Clone, Debug, PartialEq)]
pub enum EncryptionAlgorithm {
    /// AES-256-GCM
    Aes256Gcm,
    /// ChaCha20-Poly1305
    ChaCha20Poly1305,
}

