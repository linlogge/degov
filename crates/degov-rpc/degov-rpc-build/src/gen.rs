use proc_macro2::TokenStream;
use prost_build::{Method, Service, ServiceGenerator};
use quote::{format_ident, quote};
use syn::parse_str;

#[derive(Default)]
pub struct AxumConnectServiceGenerator {}

impl AxumConnectServiceGenerator {
    pub fn new() -> Self {
        Default::default()
    }

    fn generate_service(&mut self, service: Service, buf: &mut String) {
        let service_name = format_ident!("{}", service.name);
        let client_name = format_ident!("{}Client", service.name);
        let path_root = format!("{}.{}", service.package, service.proto_name);
        
        // Split methods into owned vectors for server and client generation
        let server_methods: Vec<_> = service.methods.iter()
            .filter(|m| !m.client_streaming)
            .cloned()
            .collect();
        
        let client_methods: Vec<_> = service.methods.iter()
            .filter(|m| !m.client_streaming)
            .cloned()
            .collect();

        // Generate server methods (collect to avoid borrow issues)
        let server_method_impls: Vec<_> = server_methods.into_iter()
            .map(|m| self.generate_service_method(m, &path_root))
            .collect();

        // Generate client methods (collect to avoid borrow issues)
        let client_method_impls: Vec<_> = client_methods.into_iter()
            .map(|m| self.generate_client_method(m, &path_root))
            .collect();

        buf.push_str(
            quote! {
                // Server struct
                pub struct #service_name;

                #[allow(dead_code)]
                impl #service_name {
                    #(#server_method_impls)*
                }

                // Client struct
                #[derive(Clone)]
                pub struct #client_name {
                    client: degov_rpc::client::RpcClient,
                }

                #[allow(dead_code)]
                impl #client_name {
                    pub fn new(client: degov_rpc::client::RpcClient) -> Self {
                        Self { client }
                    }

                    pub fn from_config(config: degov_rpc::client::RpcClientConfig) -> Self {
                        Self {
                            client: degov_rpc::client::RpcClient::new(config),
                        }
                    }

                    #(#client_method_impls)*
                }
            }
            .to_string()
            .as_str(),
        );
    }

    fn generate_service_method(&mut self, method: Method, path_root: &str) -> TokenStream {
        let method_name = format_ident!("{}", method.name);
        let method_name_unary_get = format_ident!("{}_unary_get", method.name);
        let input_type: syn::Type = parse_str(&method.input_type).unwrap();
        let output_type: syn::Type = parse_str(&method.output_type).unwrap();
        let path = format!("/{}/{}", path_root, method.proto_name);

        if method.server_streaming {
            quote! {
                pub fn #method_name<T, H, S>(
                    handler: H
                ) -> impl FnOnce(axum::Router<S>) -> degov_rpc::server::router::RpcRouter<S>
                where
                    H: degov_rpc::server::handler::RpcHandlerStream<#input_type, #output_type, T, S>,
                    T: 'static,
                    S: Clone + Send + Sync + 'static,
                {
                    move |router: axum::Router<S>| {
                        router.route(
                            #path,
                            axum::routing::post(|
                                axum::extract::State(state): axum::extract::State<S>,
                                request: axum::http::Request<axum::body::Body>
                            | async move {
                                handler.call(request, state).await
                            }),
                        )
                    }
                }
            }
        } else {
            quote! {
                pub fn #method_name<T, H, S>(
                    handler: H
                ) -> impl FnOnce(axum::Router<S>) -> degov_rpc::server::router::RpcRouter<S>
                where
                    H: degov_rpc::server::handler::RpcHandlerUnary<#input_type, #output_type, T, S>,
                    T: 'static,
                    S: Clone + Send + Sync + 'static,
                {
                    move |router: axum::Router<S>| {
                        router.route(
                            #path,
                            axum::routing::post(|
                                axum::extract::State(state): axum::extract::State<S>,
                                request: axum::http::Request<axum::body::Body>
                            | async move {
                                handler.call(request, state).await
                            }),
                        )
                    }
                }

                pub fn #method_name_unary_get<T, H, S>(
                    handler: H
                ) -> impl FnOnce(axum::Router<S>) -> degov_rpc::server::router::RpcRouter<S>
                where
                    H: degov_rpc::server::handler::RpcHandlerUnary<#input_type, #output_type, T, S>,
                    T: 'static,
                    S: Clone + Send + Sync + 'static,
                {
                    move |router: axum::Router<S>| {
                        router.route(
                            #path,
                            axum::routing::get(|
                                axum::extract::State(state): axum::extract::State<S>,
                                request: axum::http::Request<axum::body::Body>
                            | async move {
                                handler.call(request, state).await
                            }),
                        )
                    }
                }
            }
        }
    }

    fn generate_client_method(&mut self, method: Method, path_root: &str) -> TokenStream {
        let method_name = format_ident!("{}", method.name);
        let method_name_get = format_ident!("{}_get", method.name);
        let input_type: syn::Type = parse_str(&method.input_type).unwrap();
        let output_type: syn::Type = parse_str(&method.output_type).unwrap();
        let path = format!("/{}/{}", path_root, method.proto_name);

        if method.server_streaming {
            quote! {
                pub async fn #method_name(
                    &self,
                    request: #input_type,
                ) -> Result<degov_rpc::client::RpcStream<#output_type>, degov_rpc::server::error::RpcError> {
                    self.client.server_stream(#path, request).await
                }
            }
        } else {
            quote! {
                pub async fn #method_name(
                    &self,
                    request: #input_type,
                ) -> Result<#output_type, degov_rpc::server::error::RpcError> {
                    self.client.unary(#path, request).await
                }

                pub async fn #method_name_get(
                    &self,
                    request: #input_type,
                ) -> Result<#output_type, degov_rpc::server::error::RpcError> {
                    self.client.unary_get(#path, request).await
                }
            }
        }
    }
}

impl ServiceGenerator for AxumConnectServiceGenerator {
    fn generate(&mut self, service: Service, buf: &mut String) {
        self.generate_service(service, buf);
    }
}
