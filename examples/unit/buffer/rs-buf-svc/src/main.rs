use std::net::SocketAddr;

use tonic::transport::{server::Router, Server};

use rs_perf_test_helper::rpc::perf::helper;

use helper::proto::buffer::v1::req_buffer_service_server::ReqBufferServiceServer;
use helper::proto::buffer::v1::res_buffer_service_server::ResBufferServiceServer;

const LISTEN_ADDR_DEFAULT: &str = "127.0.0.1:50051";

#[tokio::main]
async fn main() -> Result<(), String> {
    let qbsvc = rs_perf_test_helper::buffer::vecdeque::svc::request_buffer_service_new(16).await;
    let qbsvr: ReqBufferServiceServer<_> = ReqBufferServiceServer::new(qbsvc);

    let sbsvc = rs_perf_test_helper::buffer::res::btree::svc::res_buffer_service_new(16).await;
    let sbsvr: ResBufferServiceServer<_> = ResBufferServiceServer::new(sbsvc);

    let listen_addr: String = std::env::var("ENV_LISTEN_ADDR")
        .ok()
        .unwrap_or_else(|| LISTEN_ADDR_DEFAULT.into());
    let la: SocketAddr =
        str::parse(listen_addr.as_str()).map_err(|e| format!("invalid addr: {e}"))?;

    let mut s: Server = Server::builder();
    let r: Router<_> = s.add_service(sbsvr).add_service(qbsvr);

    r.serve(la)
        .await
        .map_err(|e| format!("Unable to listen: {e}"))?;
    Ok(())
}
