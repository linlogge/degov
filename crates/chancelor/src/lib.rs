pub mod proto {
    tonic::include_proto!("degov.chancelor");
}

use proto::frontdoor_server::{Frontdoor, FrontdoorServer};
use proto::{GetServicesRequest, GetServicesResponse};
use tonic::transport::Server;
use tonic::{Request, Response, Status};

#[derive(Debug, Default)]
pub struct FrontdoorImpl {}

#[tonic::async_trait]
impl Frontdoor for FrontdoorImpl {
    async fn get_services(
        &self,
        _request: Request<GetServicesRequest>,
    ) -> Result<Response<GetServicesResponse>, Status> {
        Ok(Response::new(GetServicesResponse { services: vec![] }))
    }
}

pub struct Chancelor {}

impl Chancelor {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let addr = "[::1]:50051".parse()?;
        let frontdoor = FrontdoorImpl::default();

        Server::builder()
            .add_service(FrontdoorServer::new(frontdoor))
            .serve(addr)
            .await?;

        Ok(())
    }
}
