use std::sync::Arc;
use std::time::SystemTime;

use futures::StreamExt;

use tonic::{Request, Response, Status};

use crate::uuid::Uuid;

use crate::rpc::perf::helper;

use helper::proto::common::v1::Retry;

use helper::proto::direct::v1::conv_svc::{ConvertRequest, ConvertResponse};
use helper::proto::direct::v1::convert_service_server::ConvertService;

use helper::proto::buffer::v1::req_buf::{SaveRequest, SaveResponse};
use helper::proto::buffer::v1::req_buffer_service_server::ReqBufferService;
use helper::proto::buffer::v1::res_buf::{GetRequest, GetResponse};
use helper::proto::buffer::v1::res_buffer_service_server::ResBufferService;

pub struct Buffered<Q, S> {
    req_svc: Arc<Q>,
    res_svc: Arc<S>,

    retry: Retry,
}

impl<Q, S> Buffered<Q, S>
where
    Q: ReqBufferService,
{
    async fn save(
        &self,
        received: SystemTime,
        req: ConvertRequest,
        reply: Uuid,
    ) -> Result<Response<SaveResponse>, Status> {
        let reqid: Uuid = Uuid::new_v4();

        let saveq = SaveRequest {
            request_id: Some(reqid.into()),
            reply_id: Some(reply.into()),
            req: Some(req),
            received: Some(received.into()),
        };
        self.req_svc.save(Request::new(saveq)).await
    }
}

impl<Q, S> Buffered<Q, S>
where
    S: Send + Sync + 'static + ResBufferService,
{
    async fn get(&self, reply: Uuid, retry: Retry) -> Result<Response<S::GetStream>, Status> {
        let reqid: Uuid = Uuid::new_v4();
        let req = GetRequest {
            request_id: Some(reqid.into()),
            reply_id: Some(reply.into()),
            retry: Some(retry),
        };
        self.res_svc.get(Request::new(req)).await
    }
}

#[tonic::async_trait]
impl<Q, S> ConvertService for Buffered<Q, S>
where
    Q: Send + Sync + 'static + ReqBufferService,
    S: Send + Sync + 'static + ResBufferService,
    <S as ResBufferService>::GetStream: Unpin,
{
    async fn convert(
        &self,
        req: Request<ConvertRequest>,
    ) -> Result<Response<ConvertResponse>, Status> {
        let received: SystemTime = SystemTime::now();
        let cr: ConvertRequest = req.into_inner();
        let reply: Uuid = Uuid::new_v4();

        self.save(received, cr, reply).await?;
        let got: Response<_> = self.get(reply, self.retry.clone()).await?;
        let mut gs: S::GetStream = got.into_inner();
        let ro: Option<_> = gs.next().await;
        let res: Result<_, _> = ro.ok_or_else(|| Status::internal("No reply from upstream"))?;
        let gr: GetResponse = res?;
        let reply: ConvertResponse = gr.res.ok_or_else(|| Status::internal("empty response"))?;
        Ok(Response::new(reply))
    }
}
