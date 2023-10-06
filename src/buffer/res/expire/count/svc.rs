use core::ops::Bound::{Excluded, Unbounded};

use std::collections::BTreeMap;

use tokio::sync::mpsc::Sender;
use tokio_stream::wrappers::ReceiverStream;

use tonic::{Code, Status};

use crate::uuid::Uuid;

use crate::buffer::res::expire::svc::ExpireService;

#[derive(Default)]
pub struct Container {
    m: BTreeMap<Uuid, u64>,
}

impl Container {
    fn first_expired_key(&self, max_cnt: u64) -> Option<Uuid> {
        let i = self.m.iter();
        let filtered = i.filter(|t| Self::is_expired(*t.1, max_cnt));
        let mut mapd = filtered.map(|t| t.0);
        mapd.next().copied()
    }

    fn is_expired(cnt: u64, max_cnt: u64) -> bool {
        max_cnt < cnt
    }

    fn next_expired_key(&self, prev: Uuid, max_cnt: u64) -> Option<Uuid> {
        let i = self.m.range((Excluded(prev), Unbounded));
        let filtered = i.filter(|t| Self::is_expired(*t.1, max_cnt));
        let mut mapd = filtered.map(|t| t.0);
        mapd.next().copied()
    }

    pub fn expired_key(&self, prev: Option<Uuid>, max_cnt: u64) -> Result<Uuid, Status> {
        let o: Option<Uuid> = match prev {
            None => self.first_expired_key(max_cnt),
            Some(p) => self.next_expired_key(p, max_cnt),
        };
        o.ok_or_else(|| Status::not_found("no expired keys found"))
    }

    pub fn register(&mut self, key: Uuid) -> Result<(), Status> {
        match self.m.insert(key, 0) {
            None => Ok(()),
            Some(overwritten) => {
                self.m.insert(key, overwritten);
                Err(Status::already_exists(format!("dup found. key: {key}")))
            }
        }
    }
}

pub enum Req {
    ExpiredKey(Option<Uuid>, Sender<Result<Uuid, Status>>),
    Register(Uuid, Sender<Result<(), Status>>),
}

impl Req {
    pub async fn handle_expired_key(
        c: &Container,
        prev: Option<Uuid>,
        max_cnt: u64,
        reply: Sender<Result<Uuid, Status>>,
    ) {
        let r: Result<_, _> = c.expired_key(prev, max_cnt);
        match reply.send(r).await {
            Ok(_) => {}
            Err(e) => log::warn!("Unable to send an expired key: {e}"),
        }
    }

    pub async fn handle_register_key(
        c: &mut Container,
        key: Uuid,
        reply: Sender<Result<(), Status>>,
    ) {
        let r: Result<_, _> = c.register(key);
        match reply.send(r).await {
            Ok(_) => {}
            Err(e) => log::warn!("Unable to send a register evt: {e}"),
        }
    }
}

pub struct Svc {
    sender: Sender<Req>,
}

impl Svc {
    pub async fn get_expired_key(s: &Sender<Req>, prev: Option<Uuid>) -> Result<Uuid, Status> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let req: Req = Req::ExpiredKey(prev, tx);
        s.send(req)
            .await
            .map_err(|e| Status::internal(format!("UNABLE TO SEND A REQUEST: {e}")))?;
        rx.recv()
            .await
            .ok_or_else(|| Status::internal("NO RESPONSE GOT"))?
    }

    pub async fn register_key(s: &Sender<Req>, key: Uuid) -> Result<(), Status> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let req: Req = Req::Register(key, tx);
        s.send(req)
            .await
            .map_err(|e| Status::internal(format!("UNABLE TO SEND A REQUEST: {e}")))?;
        rx.recv()
            .await
            .ok_or_else(|| Status::internal("NO RESPONSE GOT"))?
    }
}

#[tonic::async_trait]
impl ExpireService for Svc {
    type ExpiredKeysStream = ReceiverStream<Result<Uuid, Status>>;

    async fn expired_keys(&self) -> Result<Self::ExpiredKeysStream, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let s: Sender<Req> = self.sender.clone();
        tokio::spawn(async move {
            let mut prev: Option<Uuid> = None;
            loop {
                match Self::get_expired_key(&s, prev).await {
                    Err(e) => match e.code() {
                        Code::NotFound => return,
                        _ => match tx.send(Err(e)).await {
                            Ok(_) => return,
                            Err(e) => {
                                log::warn!("UNABLE TO SEND AN ERROR: {e}");
                                return;
                            }
                        },
                    },
                    Ok(key) => match tx.send(Ok(key)).await {
                        Err(e) => {
                            log::warn!("Unable to send an expired key: {e}");
                            return;
                        }
                        Ok(_) => {
                            prev = Some(key);
                        }
                    },
                }
            }
        });
        Ok(ReceiverStream::new(rx))
    }

    async fn register_key(&self, key: Uuid) -> Result<(), Status> {
        Self::register_key(&self.sender, key).await
    }
}

async fn svc_new(max_cnt: u64, mut c: Container) -> Svc {
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                None => return,
                Some(Req::ExpiredKey(prev, reply)) => {
                    Req::handle_expired_key(&c, prev, max_cnt, reply).await
                }
                Some(Req::Register(key, reply)) => {
                    Req::handle_register_key(&mut c, key, reply).await
                }
            }
        }
    });
    Svc { sender: tx }
}

pub async fn expire_service_new(max_cnt: u64) -> impl ExpireService {
    svc_new(max_cnt, Container::default()).await
}
