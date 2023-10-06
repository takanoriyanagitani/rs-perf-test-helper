use core::time::Duration;
use std::collections::BTreeMap;
use std::time::{Instant, SystemTime};

use tokio::sync::mpsc::Sender;
use tokio::time::Interval;
use tokio_stream::wrappers::ReceiverStream;

use tonic::{Code, Request, Response, Status};

use crate::uuid::Uuid;

use crate::retry::Retry;

use crate::buffer::res::cmd::del::DelReq;
use crate::buffer::res::cmd::get::GetReq;
use crate::buffer::res::cmd::set::SetReq;

use crate::rpc::perf::helper;

use helper::proto::buffer::v1::res_buf::{DelRequest, DelResponse};
use helper::proto::buffer::v1::res_buf::{GetRequest, GetResponse};
use helper::proto::buffer::v1::res_buf::{SetRequest, SetResponse};
use helper::proto::buffer::v1::res_buffer_service_server::ResBufferService;

pub enum Req {
    Set(SetReq, Sender<Result<SystemTime, Status>>),
    Get(Uuid, Sender<Result<GetResponse, Status>>),
    Del(Uuid, Sender<Result<(), Status>>),
}

impl Req {
    async fn handle_set(
        d: &mut BTreeMap<Uuid, GetResponse>,
        req: SetReq,
        reply: Sender<Result<SystemTime, Status>>,
        max_size: usize,
    ) {
        let sz: usize = d.len();
        let too_many: bool = max_size < sz;
        let available: bool = !too_many;
        let r = available
            .then_some(())
            .ok_or_else(|| Status::unavailable(format!("too many requests. size: {sz}")))
            .and_then(|_| {
                let reply_id: Uuid = req.as_reply_id();
                let dup_found: bool = d.contains_key(&reply_id);
                let available: bool = !dup_found;
                available.then_some(()).ok_or_else(|| {
                    Status::already_exists(format!(
                        "response for reply id({reply_id}) already exists"
                    ))
                })?;
                let mut gr: GetResponse = req.into();
                let set: SystemTime = SystemTime::now();
                gr.set = Some(set.into());
                d.insert(reply_id, gr);
                Ok(set)
            });
        match reply.send(r).await {
            Ok(_) => {}
            Err(e) => log::warn!("Unable to send a set evt: {e}"),
        }
    }

    async fn handle_get(
        d: &mut BTreeMap<Uuid, GetResponse>,
        reply_id: Uuid,
        reply: Sender<Result<GetResponse, Status>>,
    ) {
        let r = d.remove(&reply_id).ok_or_else(|| {
            Status::not_found(format!("No reply found(for now). reply id: {reply_id}"))
        });
        match reply.send(r).await {
            Ok(_) => {}
            Err(e) => log::warn!("Unable to send a get evt: {e}"),
        }
    }

    async fn handle_del(
        d: &mut BTreeMap<Uuid, GetResponse>,
        reply_id: Uuid,
        reply: Sender<Result<(), Status>>,
    ) {
        let r = d.remove(&reply_id).map(|_| ()).ok_or_else(|| {
            Status::not_found(format!(
                "No reply found(may be consumed). reply id: {reply_id}"
            ))
        });
        match reply.send(r).await {
            Ok(_) => {}
            Err(e) => log::warn!("Unable to send a del evt: {e}"),
        }
    }
}

pub struct BufSvcSt {
    sender: Sender<Req>,
}

impl BufSvcSt {
    pub async fn get1(sender: &Sender<Req>, reply_id: Uuid) -> Result<GetResponse, Status> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let req = Req::Get(reply_id, tx);
        sender
            .send(req)
            .await
            .map_err(|e| Status::internal(format!("Unable to send a request: {e}")))?;
        let res: Result<_, _> = rx
            .recv()
            .await
            .ok_or_else(|| Status::internal("no response got"))?;
        res
    }

    pub async fn set(&self, req: SetReq) -> Result<SystemTime, Status> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let req = Req::Set(req, tx);
        self.sender
            .send(req)
            .await
            .map_err(|e| Status::internal(format!("Unable to send a set request: {e}")))?;
        let res: Result<_, _> = rx
            .recv()
            .await
            .ok_or_else(|| Status::internal("no response got"))?;
        res
    }

    pub async fn del(&self, id: Uuid) -> Result<SystemTime, Status> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let req = Req::Del(id, tx);
        self.sender
            .send(req)
            .await
            .map_err(|e| Status::internal(format!("Unable to send a delete request: {e}")))?;
        let res: Result<_, _> = rx
            .recv()
            .await
            .ok_or_else(|| Status::internal("no response got"))?;
        res.map(|_| SystemTime::now())
    }

    pub async fn get(
        &self,
        req: GetReq,
    ) -> Result<ReceiverStream<Result<GetResponse, Status>>, Status> {
        let sender = self.sender.clone();
        let reply_id: Uuid = req.as_reply_id();
        let retry: &Retry = req.as_retry();
        let retry_max: u64 = retry.as_retry_max();
        let invl: Duration = retry.as_interval();
        let timeout: Duration = retry.as_timeout();
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        tokio::spawn(async move {
            let mut interval: Interval = tokio::time::interval(invl);
            let started: Instant = Instant::now();
            for i in 0..retry_max {
                interval.tick().await;
                let r = Instant::now()
                    .checked_duration_since(started)
                    .ok_or_else(|| Status::internal("CLOCK BROKEN"))
                    .and_then(|elapsed: Duration| {
                        let too_slow: bool = timeout < elapsed;
                        let fast: bool = !too_slow;
                        fast.then_some(()).ok_or_else(|| {
                            Status::deadline_exceeded(format!(
                                "timeout. elapsed={elapsed:#?}, try count={i}"
                            ))
                        })
                    });
                let r = match r {
                    Ok(_) => Self::get1(&sender, reply_id).await,
                    Err(e) => Err(e),
                };
                match r {
                    Ok(r) => {
                        match tx.send(Ok(r)).await {
                            Ok(_) => {}
                            Err(e) => log::warn!("Unable to send a response: {e}"),
                        };
                        return;
                    }
                    Err(e) => match e.code() {
                        Code::NotFound => continue,
                        _ => {
                            match tx.send(Err(e)).await {
                                Ok(_) => {}
                                Err(e) => log::warn!("Unable to send a reply: {e}"),
                            }
                            return;
                        }
                    },
                }
            }
        });
        Ok(ReceiverStream::new(rx))
    }
}

#[tonic::async_trait]
impl ResBufferService for BufSvcSt {
    type GetStream = ReceiverStream<Result<GetResponse, Status>>;

    async fn get(&self, req: Request<GetRequest>) -> Result<Response<Self::GetStream>, Status> {
        let gr: GetRequest = req.into_inner();
        let checked: GetReq = gr.try_into()?;
        let reply = self.get(checked).await?;
        Ok(Response::new(reply))
    }

    async fn set(&self, req: Request<SetRequest>) -> Result<Response<SetResponse>, Status> {
        let sr: SetRequest = req.into_inner();
        let checked: SetReq = sr.try_into()?;
        let set: SystemTime = self.set(checked).await?;
        let reply = SetResponse {
            set: Some(set.into()),
        };
        Ok(Response::new(reply))
    }

    async fn del(&self, req: Request<DelRequest>) -> Result<Response<DelResponse>, Status> {
        let dr: DelRequest = req.into_inner();
        let checked: DelReq = dr.try_into()?;
        let reply_id: Uuid = checked.as_reply_id();
        let removed: SystemTime = self.del(reply_id).await?;
        let reply = DelResponse {
            removed: Some(removed.into()),
        };
        Ok(Response::new(reply))
    }
}

async fn buf_svc_st_new(max_size: usize) -> BufSvcSt {
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    tokio::spawn(async move {
        let mut bm: BTreeMap<Uuid, GetResponse> = BTreeMap::new();
        loop {
            match rx.recv().await {
                None => return,
                Some(req) => match req {
                    Req::Set(q, reply) => Req::handle_set(&mut bm, q, reply, max_size).await,
                    Req::Get(reply_id, reply) => Req::handle_get(&mut bm, reply_id, reply).await,
                    Req::Del(reply_id, reply) => Req::handle_del(&mut bm, reply_id, reply).await,
                },
            }
        }
    });
    BufSvcSt { sender: tx }
}

pub async fn res_buffer_service_new(max_size: usize) -> impl ResBufferService {
    buf_svc_st_new(max_size).await
}
