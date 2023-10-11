use tokio::sync::mpsc::Sender;
use tokio_stream::wrappers::ReceiverStream;

use crate::uuid::Uuid;

use crate::buffer::res::expire::svc::ExpireService;

use crate::rpc::perf::helper;
use helper::proto::buffer::v1::res_buffer_service_server::ResBufferService;

use core::time::Duration;
use std::time::{Instant, SystemTime};

use tokio::time::Interval;

use tonic::{Code, Request, Response, Status};

use crate::retry::Retry;

use crate::buffer::res::cmd::del::DelReq;
use crate::buffer::res::cmd::get::GetReq;
use crate::buffer::res::cmd::set::SetReq;

use helper::proto::buffer::v1::res_buf::{DelRequest, DelResponse};
use helper::proto::buffer::v1::res_buf::{GetRequest, GetResponse};
use helper::proto::buffer::v1::res_buf::{LenRequest, LenResponse};
use helper::proto::buffer::v1::res_buf::{SetRequest, SetResponse};

pub struct AutoExpireSvc<B, E> {
    buf: B,
    expire: E,
}

#[tonic::async_trait]
impl<B, E> ResBufferService for AutoExpireSvc<B, E>
where
    B: Send + Sync + 'static + ResBufferService,
    E: Send + Sync + 'static + ExpireService,
{
    type GetStream = B::GetStream;

    async fn get(&self, req: Request<GetRequest>) -> Result<Response<Self::GetStream>, Status> {
        self.buf.get(req).await
    }

    async fn set(&self, req: Request<SetRequest>) -> Result<Response<SetResponse>, Status> {
        let id: Uuid = req
            .get_ref()
            .reply_id
            .as_ref()
            .map(|u| u.into())
            .ok_or_else(|| Status::invalid_argument("reply id missing"))?;
        self.expire.register_key(id).await?;
        self.buf.set(req).await
    }

    async fn del(&self, req: Request<DelRequest>) -> Result<Response<DelResponse>, Status> {
        self.buf.del(req).await
    }

    async fn len(&self, req: Request<LenRequest>) -> Result<Response<LenResponse>, Status> {
        self.buf.len(req).await
    }
}
