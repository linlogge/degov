use crate::v1::{
    data_model::DataModelField,
    service::ServiceBuild,
};
use std::borrow::Cow;

pub struct RemoteProcedureService<'a> {
    pub name: Cow<'a, str>,
    pub request: DataModelField<'a>,
    pub response: DataModelField<'a>,
    pub handler: ServiceHandler<'a>,
}

pub struct ServiceHandler<'a> {
    pub runtime: Cow<'a, str>,
    pub build: Option<ServiceBuild<'a>>,
}
