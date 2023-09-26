use std::time::SystemTime;

use rs_perf_test_helper::tonic;
use tonic::Status;

use wasmtime::{Engine, Instance, Module, Store, TypedFunc};

use rs_perf_test_helper::uuid::Uuid;

use rs_perf_test_helper::direct::cmd::convert::ConvertReq;

use rs_perf_test_helper::convert::st::chan::svc::ConvertServiceMut;

use rs_perf_test_helper::rpc::perf::helper;

use helper::proto::direct::v1::conv_svc::{ConvertRequest, ConvertResponse};
use helper::proto::direct::v1::convert_service_server::ConvertService;

pub struct ConvSvcSt {
    f: TypedFunc<u64, u64>,
    store: Store<()>,
}

impl ConvSvcSt {
    pub async fn convert(&mut self, unixtime_us: u64) -> Result<u64, Status> {
        self.f
            .call_async(&mut self.store, unixtime_us)
            .await
            .map_err(|e| Status::internal(format!("Unable to convert: {e}")))
    }
}

#[tonic::async_trait]
impl ConvertServiceMut for ConvSvcSt {
    async fn convert_mut(&mut self, req: ConvertRequest) -> Result<ConvertResponse, Status> {
        let checked: ConvertReq = req.try_into()?;
        let reqid: Uuid = checked.as_request();
        let seed: Vec<u8> = checked.into_seed();
        let a: [u8; 8] = seed.try_into().map_err(|_| {
            Status::invalid_argument(format!("invalid unixtime us. request id: {reqid}"))
        })?;
        let unixtime_us: u64 = u64::from_be_bytes(a);
        let double: u64 = self.convert(unixtime_us).await?;
        let converted: SystemTime = SystemTime::now();
        let reply = ConvertResponse {
            converted: Some(converted.into()),
            generated: double.to_be_bytes().into(),
        };
        Ok(reply)
    }
}

pub struct ConvInstance {
    i: Instance,
    s: Store<()>,
    name: String,
}

impl ConvInstance {
    fn get_typed(&mut self) -> Result<TypedFunc<u64, u64>, Status> {
        self.i
            .get_typed_func(&mut self.s, self.name.as_str())
            .map_err(|e| Status::internal(format!("Unable to get func. name={}: {e}", self.name)))
    }

    pub fn build(mut self) -> Result<ConvSvcSt, Status> {
        let f = self.get_typed()?;
        Ok(ConvSvcSt { f, store: self.s })
    }
}

pub struct ConvModule {
    m: Module,
    e: Engine,
    name: String,
}

impl ConvModule {
    pub async fn new_instance(&self) -> Result<ConvInstance, Status> {
        let mut s: Store<()> = Store::new(&self.e, ());
        let i: Instance = Instance::new_async(&mut s, &self.m, &[])
            .await
            .map_err(|e| Status::internal(format!("Unable to create an instance: {e}")))?;
        Ok(ConvInstance {
            i,
            s,
            name: self.name.clone(),
        })
    }

    pub async fn new_conv_service_mut(&self) -> Result<impl ConvertServiceMut, Status> {
        let i: ConvInstance = self.new_instance().await?;
        i.build()
    }

    pub async fn new_conv_service(&self) -> Result<impl ConvertService, Status> {
        self.new_conv_service_mut()
            .await
            .map(rs_perf_test_helper::convert::st::chan::svc::conv_svc_new)
    }
}

pub struct ConvEngine {
    e: Engine,
    wasm: Vec<u8>,
    name: String,
}

impl ConvEngine {
    fn new_module(&self) -> Result<Module, Status> {
        Module::new(&self.e, &self.wasm).map_err(|e| Status::internal(format!("invalid wasm: {e}")))
    }

    pub fn build(&self) -> Result<ConvModule, Status> {
        let m: Module = self.new_module()?;
        Ok(ConvModule {
            m,
            e: self.e.clone(),
            name: self.name.clone(),
        })
    }

    pub fn new(e: Engine, wasm_bytes: Vec<u8>, conv_func_name: String) -> Self {
        Self {
            e,
            wasm: wasm_bytes,
            name: conv_func_name,
        }
    }
}
