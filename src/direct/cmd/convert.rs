use tonic::Status;

use crate::uuid::Uuid;

use crate::rpc::perf::helper;
use helper::proto::direct::v1::conv_svc::ConvertRequest;

pub struct ConvertReq {
    request_id: Uuid,
    seed: Vec<u8>,
}

impl ConvertReq {
    pub fn as_request(&self) -> Uuid {
        self.request_id
    }
    pub fn into_seed(self) -> Vec<u8> {
        self.seed
    }
}

impl TryFrom<ConvertRequest> for ConvertReq {
    type Error = Status;
    fn try_from(g: ConvertRequest) -> Result<Self, Self::Error> {
        let request_id: Uuid = g
            .request_id
            .as_ref()
            .try_into()
            .map_err(|_| Status::invalid_argument("request id missing"))?;
        let seed: Vec<u8> = g.seed;
        Ok(Self { request_id, seed })
    }
}

impl From<ConvertReq> for ConvertRequest {
    fn from(d: ConvertReq) -> Self {
        Self {
            request_id: Some(d.request_id.into()),
            seed: d.seed,
        }
    }
}
