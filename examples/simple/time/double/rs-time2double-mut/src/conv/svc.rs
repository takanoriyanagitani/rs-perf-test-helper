use std::time::SystemTime;

use rs_perf_test_helper::log;
use rs_perf_test_helper::tonic;

use tonic::Status;

use rs_perf_test_helper::convert::st::chan::svc::conv_svc_new;
use rs_perf_test_helper::convert::st::chan::svc::ConvertServiceMut;
use rs_perf_test_helper::direct::cmd::convert::ConvertReq;
use rs_perf_test_helper::uuid::Uuid;

use rs_perf_test_helper::rpc::perf::helper;

use helper::proto::direct::v1::conv_svc::{ConvertRequest, ConvertResponse};
use helper::proto::direct::v1::convert_service_server::ConvertService;

#[derive(Default)]
pub struct ConvSvcMut {
    cnt: u64,
}

#[tonic::async_trait]
impl ConvertServiceMut for ConvSvcMut {
    async fn convert_mut(&mut self, req: ConvertRequest) -> Result<ConvertResponse, Status> {
        let checked: ConvertReq = req.try_into()?;
        let reqid: Uuid = checked.as_request();
        let seed: Vec<u8> = checked.into_seed();
        let a: [u8; 8] = seed.try_into().map_err(|_| {
            Status::invalid_argument(format!("invalid seed(not unixtime). request id: {reqid}"))
        })?;
        let unixtime_us: u64 = u64::from_be_bytes(a);
        let double: u64 = 2 * unixtime_us;
        let converted: SystemTime = SystemTime::now();
        self.cnt += 1;
        log::info!("current count: {}", self.cnt);
        let reply = ConvertResponse {
            converted: Some(converted.into()),
            generated: double.to_be_bytes().into(),
        };
        Ok(reply)
    }
}

pub fn time2double_svc_new() -> impl ConvertService {
    conv_svc_new(ConvSvcMut::default())
}
