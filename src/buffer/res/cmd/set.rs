use std::time::SystemTime;

use prost_types::Timestamp;

use tonic::Status;

use crate::uuid::Uuid;

use crate::rpc::perf::helper;
use helper::proto::buffer::v1::res_buf::GetResponse;
use helper::proto::buffer::v1::res_buf::SetRequest;

use helper::proto::direct::v1::conv_svc::ConvertResponse;

pub struct SetReq {
    request_id: Uuid,
    reply_id: Uuid,
    response: ConvertResponse,
    received: Timestamp,
    saved: Timestamp,
    converted: Timestamp,
}

impl SetReq {
    pub fn as_request_id(&self) -> Uuid {
        self.request_id
    }

    pub fn as_reply_id(&self) -> Uuid {
        self.reply_id
    }

    pub fn as_received(&self) -> &Timestamp {
        &self.received
    }
    pub fn as_saved(&self) -> &Timestamp {
        &self.saved
    }
    pub fn as_converted(&self) -> &Timestamp {
        &self.converted
    }

    pub fn into_response(self) -> ConvertResponse {
        self.response
    }
}

impl TryFrom<SetRequest> for SetReq {
    type Error = Status;
    fn try_from(g: SetRequest) -> Result<Self, Self::Error> {
        let request_id: Uuid = g
            .request_id
            .as_ref()
            .try_into()
            .map_err(|_| Status::invalid_argument("request id missing"))?;
        let reply_id: Uuid = g.reply_id.as_ref().try_into().map_err(|_| {
            Status::invalid_argument(format!("reply id missing. request id: {request_id}"))
        })?;
        let response: ConvertResponse = g.res.ok_or_else(|| {
            Status::invalid_argument(format!("response missing. request id: {request_id}"))
        })?;
        let received: Timestamp = g.received.ok_or_else(|| {
            Status::invalid_argument(format!("received missing. request id: {request_id}"))
        })?;

        let saved: Timestamp = g.saved.ok_or_else(|| {
            Status::invalid_argument(format!("saved missing. request id: {request_id}"))
        })?;

        let converted: Timestamp = g.converted.ok_or_else(|| {
            Status::invalid_argument(format!("converted missing. request id: {request_id}"))
        })?;

        Ok(Self {
            request_id,
            reply_id,
            response,
            received,
            saved,
            converted,
        })
    }
}

impl From<SetReq> for GetResponse {
    fn from(d: SetReq) -> Self {
        Self {
            res: Some(d.response),
            received: Some(d.received),
            saved: Some(d.saved),
            converted: Some(d.converted),
            set: Some(SystemTime::now().into()),
        }
    }
}
