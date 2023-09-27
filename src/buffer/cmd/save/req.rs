use std::time::SystemTime;

use prost_types::Timestamp;

use tonic::Status;

use crate::uuid::Uuid;

use crate::rpc::perf::helper;
use helper::proto::buffer::v1::req_buf::SaveRequest;
use helper::proto::direct::v1::conv_svc::ConvertRequest;

pub struct SaveReq {
    request_id: Uuid,
    reply_id: Uuid,
    request: ConvertRequest,
    received: Timestamp,
}

impl SaveReq {
    pub fn as_request_id(&self) -> Uuid {
        self.request_id
    }

    pub fn as_reply_id(&self) -> Uuid {
        self.reply_id
    }

    pub fn as_received(&self) -> &Timestamp {
        &self.received
    }

    pub fn into_request(self) -> ConvertRequest {
        self.request
    }
}

impl TryFrom<SaveRequest> for SaveReq {
    type Error = Status;
    fn try_from(g: SaveRequest) -> Result<Self, Self::Error> {
        let request_id: Uuid = g
            .request_id
            .as_ref()
            .try_into()
            .map_err(|_| Status::invalid_argument("request id missing"))?;
        let reply_id: Uuid = g.reply_id.as_ref().try_into().map_err(|_| {
            Status::invalid_argument(format!("reply id missing. request id: {request_id}"))
        })?;
        let request: ConvertRequest = g.req.ok_or_else(|| {
            Status::invalid_argument(format!("request missing. request id: {request_id}"))
        })?;
        let received: Timestamp = g.received.ok_or_else(|| {
            Status::invalid_argument(format!("received time missing. request id: {request_id}"))
        })?;
        Ok(Self {
            request_id,
            reply_id,
            request,
            received,
        })
    }
}

pub struct SaveInfo {
    req: SaveReq,
    saved: SystemTime,
}

impl SaveInfo {
    pub fn new(req: SaveReq, saved: SystemTime) -> Self {
        Self { req, saved }
    }

    pub fn as_saved(&self) -> SystemTime {
        self.saved
    }
    pub fn into_req(self) -> SaveReq {
        self.req
    }
}
