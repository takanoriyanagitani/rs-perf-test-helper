use std::sync::Arc;

use futures::StreamExt;

use tokio_stream::wrappers::ReceiverStream;

use tonic::{Request, Response, Status};

use crate::uuid::Uuid;

use crate::indirect::conv::req::get::req::GetReq;

use crate::rpc::perf::helper;

use helper::proto::common::v1::Retry;

use helper::proto::direct::v1::conv_svc::ConvertRequest;

use helper::proto::buffer::v1::req_buf::{LoadRequest, LoadResponse};

use helper::proto::indirect::v1::conv_req::{GetRequest, GetResponse};
use helper::proto::indirect::v1::get_conv_req_service_server::GetConvReqService;

use helper::proto::buffer::v1::req_buffer_service_server::ReqBufferService;

pub struct Buffered<Q> {
    req_svc: Arc<Q>,

    retry: Retry,
}

impl<Q> Buffered<Q>
where
    Q: ReqBufferService,
{
    pub async fn load(req_svc: &Q, req: LoadRequest) -> Result<Q::LoadStream, Status> {
        let res: Response<_> = req_svc.load(Request::new(req)).await?;
        Ok(res.into_inner())
    }

    pub async fn load_retry(
        req_svc: &Q,
        reqid: Uuid,
        retry: Retry,
    ) -> Result<Q::LoadStream, Status> {
        let req = LoadRequest {
            request_id: Some(reqid.into()),
            retry: Some(retry),
        };
        Self::load(req_svc, req).await
    }
}

#[tonic::async_trait]
impl<Q> GetConvReqService for Buffered<Q>
where
    Q: Send + Sync + 'static + ReqBufferService,
{
    type GetStream = ReceiverStream<Result<GetResponse, Status>>;

    async fn get(&self, req: Request<GetRequest>) -> Result<Response<Self::GetStream>, Status> {
        let gr: GetRequest = req.into_inner();
        let checked: GetReq = gr.try_into()?;
        let reqid: Uuid = checked.as_request_id();
        let retry: Retry = self.retry.clone();
        let (tx, rx) = tokio::sync::mpsc::channel(1);

        let req_svc: Arc<Q> = self.req_svc.clone();
        tokio::spawn(async move {
            match Self::load_retry(&req_svc, reqid, retry)
                .await
                .map(|ls: Q::LoadStream| {
                    ls.map(|r: Result<LoadResponse, _>| {
                        r.map(|lr: LoadResponse| {
                            let req: Option<ConvertRequest> = lr.req;
                            let reply_id: Option<_> = lr.reply_id;
                            GetResponse { req, reply_id }
                        })
                    })
                    .all(|r: Result<GetResponse, Status>| async {
                        match tx.send(r).await {
                            Ok(_) => true,
                            Err(e) => {
                                log::warn!("Unable to send a request(GetResponse): {e}");
                                false
                            }
                        }
                    })
                }) {
                Ok(all) => {
                    all.await;
                }
                Err(e) => match tx.send(Err(e)).await {
                    Ok(_) => {}
                    Err(se) => log::warn!("Unable to send an error: {se}"),
                },
            }
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
