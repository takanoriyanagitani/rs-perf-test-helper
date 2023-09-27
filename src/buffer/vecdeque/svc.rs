use core::time::Duration;
use std::collections::VecDeque;
use std::time::{Instant, SystemTime};

use tokio::sync::mpsc::Sender;
use tokio::time::Interval;
use tokio_stream::wrappers::ReceiverStream;

use tonic::{Code, Request, Response, Status};

use crate::uuid::Uuid;

use crate::retry::Retry;

use crate::buffer::cmd::load::req::LoadReq;
use crate::buffer::cmd::save::req::{SaveInfo, SaveReq};

use crate::rpc::perf::helper;

use helper::proto::buffer::v1::req_buf::{LoadRequest, LoadResponse};
use helper::proto::buffer::v1::req_buf::{SaveRequest, SaveResponse};
use helper::proto::buffer::v1::req_buffer_service_server::ReqBufferService;

pub enum Req {
    PushBack(SaveInfo, Sender<Result<SystemTime, Status>>),
    PopFront(Sender<Result<SaveInfo, Status>>),
}

impl Req {
    async fn handle_save(
        mv: &mut VecDeque<SaveInfo>,
        si: SaveInfo,
        reply: Sender<Result<SystemTime, Status>>,
        max_size: usize,
    ) {
        let sz: usize = mv.len();
        let too_many: bool = max_size < sz;
        let r = match too_many {
            true => Err(Status::unavailable(format!(
                "too many requests. size: {sz}"
            ))),
            false => {
                mv.push_back(si);
                Ok(SystemTime::now())
            }
        };
        match reply.send(r).await {
            Ok(_) => {}
            Err(e) => log::warn!("Unable to send a save evt: {e}"),
        }
    }

    async fn handle_get(mv: &mut VecDeque<SaveInfo>, reply: Sender<Result<SaveInfo, Status>>) {
        let r = match mv.pop_front() {
            None => Err(Status::not_found("no request for now. try again")),
            Some(si) => Ok(si),
        };
        match reply.send(r).await {
            Ok(_) => {}
            Err(e) => log::warn!("Unable to send a get evt: {e}"),
        }
    }
}

pub struct BufSvcSt {
    sender: Sender<Req>,
}

impl BufSvcSt {
    pub async fn save(sender: &Sender<Req>, i: SaveInfo) -> Result<SystemTime, Status> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let req = Req::PushBack(i, tx);
        sender
            .send(req)
            .await
            .map_err(|e| Status::internal(format!("Unable to send a save request: {e}")))?;
        match rx.recv().await {
            None => Err(Status::internal("Unable to save")),
            Some(r) => r,
        }
    }

    pub async fn try_get(
        sender: &Sender<Req>,
        interval: &mut Interval,
        timeout: Duration,
        i: u64,
        started: Instant,
    ) -> Result<SaveInfo, Status> {
        interval.tick().await;
        let elapsed: Duration = Instant::now()
            .checked_duration_since(started)
            .ok_or_else(|| Status::internal("CLOCK BROKEN!!!"))?;
        let too_slow: bool = timeout < elapsed;
        (!too_slow).then_some(()).ok_or_else(|| {
            Status::deadline_exceeded(format!("timeout. elapsed={elapsed:#?}, tried: {i}"))
        })?;
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let req = Req::PopFront(tx);
        sender
            .send(req)
            .await
            .map_err(|e| Status::internal(format!("Unable to send a get request: {e}")))?;
        match rx.recv().await {
            None => Err(Status::internal("No reply got")),
            Some(r) => r,
        }
    }
}

#[tonic::async_trait]
impl ReqBufferService for BufSvcSt {
    type LoadStream = ReceiverStream<Result<LoadResponse, Status>>;

    async fn save(&self, req: Request<SaveRequest>) -> Result<Response<SaveResponse>, Status> {
        let sr: SaveRequest = req.into_inner();
        let checked: SaveReq = sr.try_into()?;
        let saved: SystemTime = SystemTime::now();
        let si: SaveInfo = SaveInfo::new(checked, saved);
        Self::save(&self.sender, si).await?;
        let reply = SaveResponse {
            saved: Some(saved.into()),
        };
        Ok(Response::new(reply))
    }

    async fn load(&self, req: Request<LoadRequest>) -> Result<Response<Self::LoadStream>, Status> {
        let lr: LoadRequest = req.into_inner();
        let checked: LoadReq = lr.try_into()?;

        let retry: &Retry = checked.as_retry();

        let retry_max: u64 = retry.as_retry_max();
        let interval: Duration = retry.as_interval();
        let timeout: Duration = retry.as_timeout();

        let sender: Sender<Req> = self.sender.clone();

        let (tx, rx) = tokio::sync::mpsc::channel(1);

        tokio::spawn(async move {
            let mut invl: Interval = tokio::time::interval(interval);
            let started: Instant = Instant::now();
            for i in 0..retry_max {
                match Self::try_get(&sender, &mut invl, timeout, i, started).await {
                    Ok(si) => {
                        let saved: SystemTime = si.as_saved();
                        let req: SaveReq = si.into_req();
                        let reply_id: Uuid = req.as_reply_id();
                        let received = req.as_received().clone();
                        let reply = LoadResponse {
                            req: Some(req.into_request()),
                            reply_id: Some(reply_id.into()),
                            received: Some(received),
                            saved: Some(saved.into()),
                        };
                        match tx.send(Ok(reply)).await {
                            Ok(_) => {}
                            Err(e) => {
                                log::warn!("Unable to send a reply: {e}");
                            }
                        }
                        return;
                    }
                    Err(e) => match e.code() {
                        Code::NotFound => continue,
                        _ => {
                            match tx.send(Err(e)).await {
                                Ok(_) => {}
                                Err(e) => {
                                    log::warn!("Unable to send a reply: {e}");
                                }
                            }
                            return;
                        }
                    },
                }
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

async fn buf_svc_st_new(max_size: usize) -> BufSvcSt {
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    tokio::spawn(async move {
        let mut vd: VecDeque<SaveInfo> = VecDeque::new();
        loop {
            match rx.recv().await {
                None => return,
                Some(req) => match req {
                    Req::PushBack(si, reply) => {
                        Req::handle_save(&mut vd, si, reply, max_size).await
                    }
                    Req::PopFront(reply) => Req::handle_get(&mut vd, reply).await,
                },
            }
        }
    });
    BufSvcSt { sender: tx }
}

pub async fn request_buffer_service_new(max_buf_size: usize) -> impl ReqBufferService {
    buf_svc_st_new(max_buf_size).await
}
