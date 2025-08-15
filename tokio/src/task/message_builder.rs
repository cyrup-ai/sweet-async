//! Tokio implementation of message builder types

use std::time::{SystemTime, UNIX_EPOCH};
use sweet_async_api::task::TaskId;
use sweet_async_api::task::message_builder::TaskMessageBuilder;
use sweet_async_api::task::task_communication::{TaskEnvelope, TaskMessage};
use sweet_async_api::task::{AsyncTaskError, TaskStatus};

#[cfg(feature = "cryypt")]
use cryypt_cipher::Cipher;

/// Zero-allocation Tokio implementation of TaskMessageBuilder trait
#[derive(Debug)]
pub struct TokioTaskMessageBuilder<T, I> {
    sender_id: I,
    hostname: Option<String>,
    correlation_id: Option<String>,
    is_encrypted: bool,
    #[cfg(feature = "cryypt")]
    cipher: Option<std::sync::Arc<Cipher>>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T, I> TokioTaskMessageBuilder<T, I>
where
    T: Clone + Send + 'static,
    I: TaskId + Clone,
{
    #[inline]
    pub fn new(sender_id: I) -> Self {
        Self {
            sender_id,
            hostname: None,
            correlation_id: None,
            is_encrypted: false,
            #[cfg(feature = "cryypt")]
            cipher: None,
            _phantom: std::marker::PhantomData,
        }
    }

    #[inline]
    fn build_envelope(&self, message: TaskMessage<T>) -> TaskEnvelope<T, I> {
        let hostname = self.hostname.clone().unwrap_or_else(|| {
            std::env::var("HOSTNAME")
                .or_else(|_| std::env::var("COMPUTERNAME"))
                .unwrap_or_else(|_| "localhost".to_string())
        });

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);

        TaskEnvelope {
            sender_id: self.sender_id.clone(),
            sender_hostname: hostname,
            timestamp,
            message,
            is_encrypted: self.is_encrypted,
            correlation_id: self.correlation_id.clone(),
        }
    }
}

impl<T, I> Default for TokioTaskMessageBuilder<T, I>
where
    T: Clone + Send + 'static,
    I: TaskId + Clone + Default,
{
    #[inline]
    fn default() -> Self {
        Self::new(I::default())
    }
}

impl<T, I> TaskMessageBuilder<T, I> for TokioTaskMessageBuilder<T, I>
where
    T: Clone + Send + 'static,
    I: TaskId + Clone,
{
    #[inline]
    fn hostname(mut self, hostname: impl Into<String>) -> Self {
        self.hostname = Some(hostname.into());
        self
    }

    #[inline]
    fn correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }

    #[cfg(feature = "cryypt")]
    #[inline]
    fn cipher(mut self, cipher: std::sync::Arc<Cipher>) -> Self {
        self.cipher = Some(cipher);
        self
    }

    #[inline]
    fn encrypt(mut self) -> Self {
        self.is_encrypted = true;
        self
    }

    #[inline]
    fn data(self, data: T) -> TaskEnvelope<T, I> {
        self.build_envelope(TaskMessage::Data(data))
    }

    #[inline]
    fn status(self, status: TaskStatus) -> TaskEnvelope<T, I> {
        self.build_envelope(TaskMessage::StatusUpdate(status))
    }

    #[inline]
    fn error(self, error: AsyncTaskError) -> TaskEnvelope<T, I> {
        self.build_envelope(TaskMessage::Error(error))
    }

    #[inline]
    fn cancel_request(self) -> TaskEnvelope<T, I> {
        self.build_envelope(TaskMessage::CancelRequest)
    }

    #[inline]
    fn cancel_ack(self) -> TaskEnvelope<T, I> {
        self.build_envelope(TaskMessage::CancelAck)
    }

    #[inline]
    fn completed(self) -> TaskEnvelope<T, I> {
        self.build_envelope(TaskMessage::Completed)
    }

    #[inline]
    fn heartbeat(self) -> TaskEnvelope<T, I> {
        self.build_envelope(TaskMessage::Heartbeat)
    }

    #[inline]
    fn custom(self, tag: impl Into<String>, data: Vec<u8>) -> TaskEnvelope<T, I> {
        self.build_envelope(TaskMessage::Custom(tag.into(), data))
    }

    #[inline]
    fn encrypted_data(self, data: Vec<u8>) -> TaskEnvelope<T, I> {
        self.build_envelope(TaskMessage::EncryptedData(data))
    }
}
