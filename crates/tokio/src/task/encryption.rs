use sweet_async_api::task::{AsyncTaskError, EncryptionProvider, EncryptedData, EncryptionAlgorithm};

#[cfg(feature = "encryption")]
use std::collections::HashMap;
#[cfg(feature = "encryption")]
use std::sync::RwLock;
#[cfg(feature = "encryption")]
use aead::{Aead, AeadCore, KeyInit, OsRng};
#[cfg(feature = "encryption")]
use aes_gcm::{Aes256Gcm, Key as AesKey, Nonce as AesNonce};
#[cfg(feature = "encryption")]
use chacha20poly1305::{ChaCha20Poly1305, Key as ChaChaKey, Nonce as ChaChaNoce};
#[cfg(feature = "encryption")]
use rand::RngCore;

/// Secure key storage with automatic zeroing
#[cfg(feature = "encryption")]
struct SecureKey {
    key_material: Vec<u8>,
    algorithm: EncryptionAlgorithm,
}

#[cfg(feature = "encryption")]
impl Drop for SecureKey {
    fn drop(&mut self) {
        // Manually zero out the key material
        self.key_material.iter_mut().for_each(|b| *b = 0);
    }
}

/// Basic encryption provider with in-memory key storage
#[cfg(feature = "encryption")]
pub struct BasicEncryptionProvider {
    keys: RwLock<HashMap<String, SecureKey>>,
    default_algorithm: EncryptionAlgorithm,
}

#[cfg(feature = "encryption")]
impl BasicEncryptionProvider {
    /// Create a new basic encryption provider
    pub fn new(algorithm: EncryptionAlgorithm) -> Self {
        let provider = Self {
            keys: RwLock::new(HashMap::new()),
            default_algorithm: algorithm,
        };
        
        // Generate default key
        let _ = provider.generate_key("default");
        provider
    }

    /// Generate a random key for the specified algorithm
    fn generate_key_material(algorithm: &EncryptionAlgorithm) -> Vec<u8> {
        let key_size = match algorithm {
            EncryptionAlgorithm::Aes256Gcm => 32,
            EncryptionAlgorithm::ChaCha20Poly1305 => 32,
        };
        
        let mut key = vec![0u8; key_size];
        OsRng.fill_bytes(&mut key);
        key
    }
}

#[cfg(feature = "encryption")]
impl EncryptionProvider for BasicEncryptionProvider {
    fn encrypt(&self, data: &[u8], key_id: Option<&str>) -> Result<EncryptedData, AsyncTaskError> {
        let key_id = key_id.unwrap_or("default");
        let keys = self.keys.read().map_err(|_| AsyncTaskError::Unknown("Lock poisoned".into()))?;
        
        let secure_key = keys.get(key_id)
            .ok_or_else(|| AsyncTaskError::Unknown(format!("Key '{}' not found", key_id)))?;
        
        match secure_key.algorithm {
            EncryptionAlgorithm::Aes256Gcm => {
                let key = AesKey::<Aes256Gcm>::from_slice(&secure_key.key_material);
                let cipher = Aes256Gcm::new(key);
                let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
                
                let ciphertext = cipher.encrypt(&nonce, data)
                    .map_err(|e| AsyncTaskError::Unknown(format!("Encryption failed: {}", e)))?;
                
                Ok(EncryptedData {
                    ciphertext,
                    nonce: nonce.to_vec(),
                    algorithm: EncryptionAlgorithm::Aes256Gcm,
                    key_id: Some(key_id.to_string()),
                    aad: None,
                })
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                let key = ChaChaKey::from_slice(&secure_key.key_material);
                let cipher = ChaCha20Poly1305::new(key);
                let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
                
                let ciphertext = cipher.encrypt(&nonce, data)
                    .map_err(|e| AsyncTaskError::Unknown(format!("Encryption failed: {}", e)))?;
                
                Ok(EncryptedData {
                    ciphertext,
                    nonce: nonce.to_vec(),
                    algorithm: EncryptionAlgorithm::ChaCha20Poly1305,
                    key_id: Some(key_id.to_string()),
                    aad: None,
                })
            }
        }
    }
    
    fn decrypt(&self, encrypted: &EncryptedData) -> Result<Vec<u8>, AsyncTaskError> {
        let key_id = encrypted.key_id.as_deref().unwrap_or("default");
        let keys = self.keys.read().map_err(|_| AsyncTaskError::Unknown("Lock poisoned".into()))?;
        
        let secure_key = keys.get(key_id)
            .ok_or_else(|| AsyncTaskError::Unknown(format!("Key '{}' not found", key_id)))?;
        
        match encrypted.algorithm {
            EncryptionAlgorithm::Aes256Gcm => {
                let key = AesKey::<Aes256Gcm>::from_slice(&secure_key.key_material);
                let cipher = Aes256Gcm::new(key);
                let nonce = AesNonce::from_slice(&encrypted.nonce);
                
                cipher.decrypt(nonce, encrypted.ciphertext.as_ref())
                    .map_err(|e| AsyncTaskError::Unknown(format!("Decryption failed: {}", e)))
            }
            EncryptionAlgorithm::ChaCha20Poly1305 => {
                let key = ChaChaKey::from_slice(&secure_key.key_material);
                let cipher = ChaCha20Poly1305::new(key);
                let nonce = ChaChaNoce::from_slice(&encrypted.nonce);
                
                cipher.decrypt(nonce, encrypted.ciphertext.as_ref())
                    .map_err(|e| AsyncTaskError::Unknown(format!("Decryption failed: {}", e)))
            }
        }
    }
    
    fn generate_key(&self, key_id: &str) -> Result<(), AsyncTaskError> {
        let key_material = Self::generate_key_material(&self.default_algorithm);
        
        let secure_key = SecureKey {
            key_material,
            algorithm: self.default_algorithm.clone(),
        };
        
        let mut keys = self.keys.write().map_err(|_| AsyncTaskError::Unknown("Lock poisoned".into()))?;
        keys.insert(key_id.to_string(), secure_key);
        
        Ok(())
    }
    
    fn rotate_keys(&self) -> Result<(), AsyncTaskError> {
        let mut keys = self.keys.write().map_err(|_| AsyncTaskError::Unknown("Lock poisoned".into()))?;
        
        // Generate new keys with "_new" suffix
        let existing_keys: Vec<_> = keys.keys().cloned().collect();
        for key_id in existing_keys {
            let new_key_material = Self::generate_key_material(&self.default_algorithm);
            let secure_key = SecureKey {
                key_material: new_key_material,
                algorithm: self.default_algorithm.clone(),
            };
            keys.insert(format!("{}_new", key_id), secure_key);
        }
        
        Ok(())
    }
}

/// Stub implementation when encryption feature is disabled
#[cfg(not(feature = "encryption"))]
pub struct BasicEncryptionProvider;

#[cfg(not(feature = "encryption"))]
impl BasicEncryptionProvider {
    pub fn new(_algorithm: EncryptionAlgorithm) -> Self {
        Self
    }
}

#[cfg(not(feature = "encryption"))]
impl EncryptionProvider for BasicEncryptionProvider {
    fn encrypt(&self, data: &[u8], _key_id: Option<&str>) -> Result<EncryptedData, AsyncTaskError> {
        // When encryption is disabled, just pass through
        Ok(EncryptedData {
            ciphertext: data.to_vec(),
            nonce: vec![],
            algorithm: EncryptionAlgorithm::Aes256Gcm,
            key_id: None,
            aad: None,
        })
    }
    
    fn decrypt(&self, encrypted: &EncryptedData) -> Result<Vec<u8>, AsyncTaskError> {
        Ok(encrypted.ciphertext.clone())
    }
    
    fn generate_key(&self, _key_id: &str) -> Result<(), AsyncTaskError> {
        Ok(())
    }
    
    fn rotate_keys(&self) -> Result<(), AsyncTaskError> {
        Ok(())
    }
}

/// No-op encryption provider for when encryption is disabled
pub struct NoOpEncryptionProvider;

impl EncryptionProvider for NoOpEncryptionProvider {
    fn encrypt(&self, data: &[u8], _key_id: Option<&str>) -> Result<EncryptedData, AsyncTaskError> {
        Ok(EncryptedData {
            ciphertext: data.to_vec(),
            nonce: vec![],
            algorithm: EncryptionAlgorithm::Aes256Gcm,
            key_id: None,
            aad: None,
        })
    }
    
    fn decrypt(&self, encrypted: &EncryptedData) -> Result<Vec<u8>, AsyncTaskError> {
        Ok(encrypted.ciphertext.clone())
    }
    
    fn generate_key(&self, _key_id: &str) -> Result<(), AsyncTaskError> {
        Ok(())
    }
    
    fn rotate_keys(&self) -> Result<(), AsyncTaskError> {
        Ok(())
    }
}