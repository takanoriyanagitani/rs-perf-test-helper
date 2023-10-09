use futures_core::stream::Stream;

use tonic::Status;

use crate::uuid::Uuid;

#[tonic::async_trait]
pub trait ExpireService {
    type ExpiredKeysStream: Stream<Item = Result<Uuid, Status>> + Send + 'static;
    async fn expired_keys(&self) -> Result<Self::ExpiredKeysStream, Status>;

    async fn register_key(&self, key: Uuid) -> Result<(), Status>;

    /// forget_key forgets the key registered by register_key.
    /// This "provided method" can not be used(actual implementation required)
    async fn forget_key(&self, _key: Uuid) -> Result<(), Status> {
        Err(Status::unimplemented("not implemented"))
    }
}
