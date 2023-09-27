use tonic::Status;

use crate::retry::Retry;
use crate::uuid::Uuid;

use crate::rpc::perf::helper;
use helper::proto::buffer::v1::req_buf::LoadRequest;

pub struct LoadReq {
    request_id: Uuid,
    retry: Retry,
}

impl LoadReq {
    pub fn as_request_id(&self) -> Uuid {
        self.request_id
    }

    pub fn as_retry(&self) -> &Retry {
        &self.retry
    }
}

impl TryFrom<LoadRequest> for LoadReq {
    type Error = Status;
    fn try_from(g: LoadRequest) -> Result<Self, Self::Error> {
        let request_id: Uuid = g
            .request_id
            .as_ref()
            .try_into()
            .map_err(|_| Status::invalid_argument("request id missing"))?;
        let retry: Retry = g.retry.as_ref().try_into().map_err(|_| {
            Status::invalid_argument(format!("retry missing. request id: {request_id}"))
        })?;
        Ok(Self { request_id, retry })
    }
}
