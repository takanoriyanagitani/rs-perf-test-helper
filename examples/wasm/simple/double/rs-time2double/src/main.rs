use std::net::SocketAddr;

use wasmtime::{Config, Engine};

use rs_perf_test_helper::tonic;

use tonic::transport::{server::Router, Server};

use rs_perf_test_helper::rpc::perf::helper;

use helper::proto::direct::v1::convert_service_server::ConvertServiceServer;

use rs_time2double::wasm::wasmtime::conv::svc::ConvEngine;
use rs_time2double::wasm::wasmtime::conv::svc::ConvModule;

pub const LISTEN_ADDR: &str = "127.0.0.1:50051";

pub const WASM_FILENAME: &str = "rs_time2double_wasm.wasm";
pub const WASM_FUNCNAME: &str = "double64u";

#[tokio::main]
async fn main() -> Result<(), String> {
    let wasm_bytes: Vec<u8> =
        std::fs::read(WASM_FILENAME).map_err(|e| format!("Unable to read wasm: {e}"))?;
    let mut cfg: Config = Config::default();
    cfg.async_support(true);
    let e: Engine = Engine::new(&cfg).map_err(|e| format!("Invalid cfg: {e}"))?;
    let ce: ConvEngine = ConvEngine::new(e, wasm_bytes, WASM_FUNCNAME.into());
    let cm: ConvModule = ce.build().map_err(|e| format!("{e}"))?;

    let conv_svc = cm.new_conv_service().await.map_err(|e| format!("{e}"))?;
    let conv_svr: ConvertServiceServer<_> = ConvertServiceServer::new(conv_svc);

    let listen_addr: String = std::env::var("ENV_LISTEN_ADDR")
        .ok()
        .unwrap_or_else(|| LISTEN_ADDR.into());
    let la: SocketAddr =
        str::parse(listen_addr.as_str()).map_err(|e| format!("invalid addr: {e}"))?;

    let mut sv: Server = Server::builder();
    let router: Router<_> = sv.add_service(conv_svr);

    router
        .serve(la)
        .await
        .map_err(|e| format!("Unable to listen: {e}"))?;
    Ok(())
}
