use core::time::Duration;
use std::time::SystemTime;

use rs_perf_test_helper::tonic;

use tonic::Status;

use rs_perf_test_helper::uuid::Uuid;

use rs_perf_test_helper::direct::cmd::convert::ConvertReq;

use rs_perf_test_helper::convert::svc::ConvService;

use rs_perf_test_helper::rpc::perf::helper;

use helper::proto::direct::v1::conv_svc::{ConvertRequest, ConvertResponse};
use helper::proto::direct::v1::convert_service_server::ConvertService;

pub struct ConvSvcTime {}

#[derive(serde::Serialize)]
pub struct Detail {
    pub current: f32,
    pub voltage: f32,
    pub time_ms_local: u16,
}

#[derive(serde::Serialize)]
pub struct Meta {
    pub serial: i64,
    pub time_abs: SystemTime,
    pub battery: Option<f32>,
    pub model: String,
}

#[derive(serde::Serialize)]
pub struct OutSample {
    pub meta: Meta,
    pub detail: Vec<Detail>,
}

#[tonic::async_trait]
impl ConvService for ConvSvcTime {
    type Input = SystemTime;
    type Output = OutSample;

    fn req2i(&self, req: ConvertRequest) -> Result<Self::Input, Status> {
        let checked: ConvertReq = req.try_into()?;
        let reqid: Uuid = checked.as_request();
        let seed: Vec<u8> = checked.into_seed();
        let a: [u8; 8] = seed.try_into().map_err(|_| {
            Status::invalid_argument(format!("invalid timestamp(unit: us). request id: {reqid} "))
        })?;
        let us: u64 = u64::from_be_bytes(a);
        let unixtime: Duration = Duration::from_micros(us);
        let st: SystemTime = SystemTime::UNIX_EPOCH
            .checked_add(unixtime)
            .ok_or_else(|| {
                Status::invalid_argument(format!(
                    "invalid timestamp(out of range). request id: {reqid}"
                ))
            })?;
        Ok(st)
    }

    fn o2res(&self, o: Self::Output) -> Result<ConvertResponse, Status> {
        let v: Vec<u8> = serde_json::to_vec(&o)
            .map_err(|e| Status::internal(format!("Unable to serialize to json: {e}")))?;
        let converted: SystemTime = SystemTime::now();
        Ok(ConvertResponse {
            converted: Some(converted.into()),
            generated: v,
        })
    }

    async fn conv(&self, i: Self::Input) -> Result<Self::Output, Status> {
        let d: Duration = i
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| Status::invalid_argument(format!("Invalid clock: {e}")))?;
        let us: u64 = d.as_micros() as u64;
        let f: f32 = us as f32;
        let z2o: f32 = 0.5 * (f.sin() + 1.0);
        let meta: Meta = Meta {
            serial: us as i64,
            time_abs: i,
            battery: Some(z2o),
            model: "Pro Max Hyper Ultra Super".into(),
        };
        let detail: Vec<Detail> = (0..=255)
            .map(|i: u8| {
                let lus: u64 = us + (u64::from(i) << 25);
                let lf: f32 = lus as f32;
                Detail {
                    current: lf,
                    voltage: lf.cos(),
                    time_ms_local: i.into(),
                }
            })
            .collect();
        Ok(Self::Output { meta, detail })
    }
}

pub fn convert_service_new() -> impl ConvertService {
    ConvSvcTime {}
}
