use tonic::Status;

use crate::uuid::Uuid;

use crate::rpc::perf::helper;
use helper::proto::indirect::v1::conv_req::GetRequest;

pub struct GetReq {
    request_id: Uuid,
}

impl GetReq {
    pub fn as_request_id(&self) -> Uuid {
        self.request_id
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
        Ok(Self { request_id })
    }
}
