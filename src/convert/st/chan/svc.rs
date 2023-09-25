use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};

use tonic::{Request, Response, Status};

use crate::rpc::perf::helper;
use helper::proto::direct::v1::conv_svc::{ConvertRequest, ConvertResponse};
use helper::proto::direct::v1::convert_service_server::ConvertService;

#[tonic::async_trait]
pub trait ConvertServiceMut {
    async fn convert_mut(&mut self, req: ConvertRequest) -> Result<ConvertResponse, Status>;
}

struct Req {
    request: ConvertRequest,
    reply: Sender<Result<ConvertResponse, Status>>,
}

struct ConvLoop<G> {
    conv_svc_mut: G,
    requests: Receiver<Req>,
}

impl<G> ConvLoop<G>
where
    G: ConvertServiceMut,
{
    async fn convert(&mut self, req: ConvertRequest) -> Result<ConvertResponse, Status> {
        self.conv_svc_mut.convert_mut(req).await
    }

    async fn start(&mut self) -> Result<(), Status> {
        loop {
            match self.requests.recv().await {
                None => {
                    return Ok(());
                }
                Some(req) => {
                    let reply: Sender<_> = req.reply;
                    let q: ConvertRequest = req.request;
                    let rs: Result<ConvertResponse, Status> = self.convert(q).await;
                    reply
                        .send(rs)
                        .await
                        .map_err(|e| Status::internal(format!("Unable to send a response: {e}")))?;
                }
            }
        }
    }
}

pub struct ConvSvc {
    sender: Sender<Req>,
}

#[tonic::async_trait]
impl ConvertService for ConvSvc {
    async fn convert(
        &self,
        req: Request<ConvertRequest>,
    ) -> Result<Response<ConvertResponse>, Status> {
        let cr: ConvertRequest = req.into_inner();
        let (tx, mut rx) = mpsc::channel(1);
        let req = Req {
            request: cr,
            reply: tx,
        };
        self.sender
            .send(req)
            .await
            .map_err(|e| Status::internal(format!("Unable to send a request: {e}")))?;
        let o: Option<Result<_, _>> = rx.recv().await;
        let res: Result<_, _> = o.ok_or_else(|| Status::internal("No response got"))?;
        let reply: ConvertResponse = res?;
        Ok(Response::new(reply))
    }
}

pub fn conv_svc_new<G>(conv_svc_mut: G) -> impl ConvertService
where
    G: ConvertServiceMut + Send + 'static,
{
    let (tx, rx) = mpsc::channel(1);
    let mut cloop = ConvLoop {
        conv_svc_mut,
        requests: rx,
    };

    tokio::spawn(async move {
        match cloop.start().await {
            Ok(_) => {}
            Err(e) => log::warn!("Unexpected error: {e}"),
        }
    });

    ConvSvc { sender: tx }
}
