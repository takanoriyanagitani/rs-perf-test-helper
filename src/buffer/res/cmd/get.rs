use tonic::Status;

use crate::retry::Retry;
use crate::uuid::Uuid;

use crate::rpc::perf::helper;
use helper::proto::buffer::v1::res_buf::GetRequest;

pub struct GetReq {
    request_id: Uuid,
    reply_id: Uuid,
    retry: Retry,
}

impl GetReq {
    pub fn as_request_id(&self) -> Uuid {
        self.request_id
    }

    pub fn as_reply_id(&self) -> Uuid {
        self.reply_id
    }

    pub fn as_retry(&self) -> &Retry {
        &self.retry
    }
}

impl TryFrom<GetRequest> for GetReq {
    type Error = Status;
    fn try_from(g: GetRequest) -> Result<Self, Self::Error> {
        let request_id: Uuid = g
            .request_id
            .as_ref()
            .try_into()
            .map_err(|_| Status::invalid_argument("request id missing"))?;
        let reply_id: Uuid = g.reply_id.as_ref().try_into().map_err(|_| {
            Status::invalid_argument(format!("reply id missing. request id: {request_id}"))
        })?;
        let retry: Retry = g.retry.as_ref().try_into().map_err(|_| {
            Status::invalid_argument(format!("retry missing. request id: {request_id}"))
        })?;
        Ok(Self {
            request_id,
            reply_id,
            retry,
        })
    }
}
