use std::env;
use std::net::SocketAddr;

use rs_perf_test_helper::tonic;
use tonic::transport::{server::Router, Server};

use rs_perf_test_helper::rpc::perf::helper;

use helper::proto::direct::v1::convert_service_server::ConvertServiceServer;

const LISTEN_ADDR_DEFAULT: &str = "127.0.0.1:50051";

#[tokio::main]
async fn main() -> Result<(), String> {
    let listen_addr: String = env::var("ENV_LISTEN_ADDR")
        .ok()
        .unwrap_or_else(|| LISTEN_ADDR_DEFAULT.into());
    let la: SocketAddr =
        str::parse(listen_addr.as_str()).map_err(|e| format!("Invalid listen addr: {e}"))?;

    let conv_svc = rs_time2json::convert::svc::convert_service_new();
    let conv_svr: ConvertServiceServer<_> = ConvertServiceServer::new(conv_svc);

    let mut sv: Server = Server::builder();
    let router: Router<_> = sv.add_service(conv_svr);

    router
        .serve(la)
        .await
        .map_err(|e| format!("Unable to listen: {e}"))?;
    Ok(())
}
