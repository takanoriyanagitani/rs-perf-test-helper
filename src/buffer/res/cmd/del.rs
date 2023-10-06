use tonic::Status;

use crate::uuid::Uuid;

use crate::rpc::perf::helper;
use helper::proto::buffer::v1::res_buf::DelRequest;

pub struct DelReq {
    request_id: Uuid,
    reply_id: Uuid,
}

impl DelReq {
    pub fn as_request_id(&self) -> Uuid {
        self.request_id
    }

    pub fn as_reply_id(&self) -> Uuid {
        self.reply_id
    }
}

impl TryFrom<DelRequest> for DelReq {
    type Error = Status;
    fn try_from(g: DelRequest) -> Result<Self, Self::Error> {
        let request_id: Uuid = g
            .request_id
            .as_ref()
            .try_into()
            .map_err(|_| Status::invalid_argument("request id missing"))?;
        let reply_id: Uuid = g.reply_id.as_ref().try_into().map_err(|_| {
            Status::invalid_argument(format!("reply id missing. request id: {request_id}"))
        })?;
        Ok(Self {
            request_id,
            reply_id,
        })
    }
}
