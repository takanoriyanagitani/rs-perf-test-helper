use tonic::{Request, Response, Status};

use crate::rpc::perf::helper;
use helper::proto::direct::v1::conv_svc::{ConvertRequest, ConvertResponse};
use helper::proto::direct::v1::convert_service_server::ConvertService;

#[tonic::async_trait]
pub trait ConvService {
    type Input;
    type Output;

    fn req2i(&self, req: ConvertRequest) -> Result<Self::Input, Status>;
    fn o2res(&self, o: Self::Output) -> Result<ConvertResponse, Status>;

    async fn conv(&self, input: Self::Input) -> Result<Self::Output, Status>;
}

#[tonic::async_trait]
impl<T> ConvertService for T
where
    T: Sync + Send + 'static + ConvService,
    T::Input: Send,
{
    async fn convert(
        &self,
        req: Request<ConvertRequest>,
    ) -> Result<Response<ConvertResponse>, Status> {
        let cr: ConvertRequest = req.into_inner();
        let i = self.req2i(cr)?;
        let o = self.conv(i).await?;
        let reply: ConvertResponse = self.o2res(o)?;
        Ok(Response::new(reply))
    }
}
