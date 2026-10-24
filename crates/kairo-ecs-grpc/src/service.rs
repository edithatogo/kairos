use std::marker::PhantomData;
use tonic::codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder};
use tonic::Status;

#[derive(Debug, Clone, Default)]
pub struct ProstCodec<T, U> {
    _pd: PhantomData<(T, U)>,
}

impl<T, U> Codec for ProstCodec<T, U>
where
    T: prost::Message + 'static,
    U: prost::Message + Default + 'static,
{
    type Encode = T;
    type Decode = U;
    type Encoder = ProstEncoder<T>;
    type Decoder = ProstDecoder<U>;

    fn encoder(&mut self) -> Self::Encoder {
        ProstEncoder(PhantomData)
    }

    fn decoder(&mut self) -> Self::Decoder {
        ProstDecoder(PhantomData)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ProstEncoder<T>(PhantomData<T>);

impl<T: prost::Message> Encoder for ProstEncoder<T> {
    type Item = T;
    type Error = Status;

    fn encode(&mut self, item: Self::Item, dst: &mut EncodeBuf<'_>) -> Result<(), Self::Error> {
        item.encode(dst)
            .map_err(|e| Status::internal(format!("failed to encode message: {e}")))
    }
}

#[derive(Debug, Clone, Default)]
pub struct ProstDecoder<U>(PhantomData<U>);

impl<U: prost::Message + Default> Decoder for ProstDecoder<U> {
    type Item = U;
    type Error = Status;

    fn decode(&mut self, buf: &mut DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error> {
        let item = U::decode(buf)
            .map_err(|e| Status::invalid_argument(format!("failed to decode message: {e}")))?;
        Ok(Some(item))
    }
}

/// Generated client implementations.
pub mod simulation_transport_client {
    #![allow(
        unused_variables,
        dead_code,
        missing_docs,
        clippy::wildcard_imports,
        clippy::let_unit_value
    )]
    use tonic::codegen::http::Uri;
    use tonic::codegen::*;
    #[derive(Debug, Clone)]
    pub struct SimulationTransportClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl SimulationTransportClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> SimulationTransportClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::Body>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + std::marker::Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + std::marker::Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_origin(inner: T, origin: Uri) -> Self {
            let inner = tonic::client::Grpc::with_origin(inner, origin);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> SimulationTransportClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::Body>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::Body>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<http::Request<tonic::body::Body>>>::Error:
                Into<StdError> + std::marker::Send + std::marker::Sync,
        {
            SimulationTransportClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with the given encoding.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.send_compressed(encoding);
            self
        }
        /// Enable decompressing responses.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.accept_compressed(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_decoding_message_size(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_encoding_message_size(limit);
            self
        }
        pub async fn exchange_events(
            &mut self,
            request: impl tonic::IntoRequest<crate::envelope::WireRemoteEvent>,
        ) -> std::result::Result<tonic::Response<crate::envelope::WireRemoteEvent>, tonic::Status>
        {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::unknown(format!("Service was not ready: {}", e.into()))
            })?;
            let codec = crate::service::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/kairo.ecs.simulation.v1.SimulationTransport/ExchangeEvents",
            );
            let mut req = request.into_request();
            req.extensions_mut().insert(GrpcMethod::new(
                "kairo.ecs.simulation.v1.SimulationTransport",
                "ExchangeEvents",
            ));
            self.inner.unary(req, path, codec).await
        }
        pub async fn migrate_entity(
            &mut self,
            request: impl tonic::IntoRequest<crate::envelope::WireMigrationRequest>,
        ) -> std::result::Result<tonic::Response<crate::envelope::WireMigrationAck>, tonic::Status>
        {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::unknown(format!("Service was not ready: {}", e.into()))
            })?;
            let codec = crate::service::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/kairo.ecs.simulation.v1.SimulationTransport/MigrateEntity",
            );
            let mut req = request.into_request();
            req.extensions_mut().insert(GrpcMethod::new(
                "kairo.ecs.simulation.v1.SimulationTransport",
                "MigrateEntity",
            ));
            self.inner.unary(req, path, codec).await
        }
        pub async fn gvt_sync(
            &mut self,
            request: impl tonic::IntoRequest<crate::envelope::WireGvtProposal>,
        ) -> std::result::Result<tonic::Response<crate::envelope::WireGvtDecision>, tonic::Status>
        {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::unknown(format!("Service was not ready: {}", e.into()))
            })?;
            let codec = crate::service::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/kairo.ecs.simulation.v1.SimulationTransport/GvtSync",
            );
            let mut req = request.into_request();
            req.extensions_mut().insert(GrpcMethod::new(
                "kairo.ecs.simulation.v1.SimulationTransport",
                "GvtSync",
            ));
            self.inner.unary(req, path, codec).await
        }
        pub async fn heartbeat(
            &mut self,
            request: impl tonic::IntoRequest<crate::envelope::WireHeartbeat>,
        ) -> std::result::Result<tonic::Response<crate::envelope::WireHeartbeatAck>, tonic::Status>
        {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::unknown(format!("Service was not ready: {}", e.into()))
            })?;
            let codec = crate::service::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/kairo.ecs.simulation.v1.SimulationTransport/Heartbeat",
            );
            let mut req = request.into_request();
            req.extensions_mut().insert(GrpcMethod::new(
                "kairo.ecs.simulation.v1.SimulationTransport",
                "Heartbeat",
            ));
            self.inner.unary(req, path, codec).await
        }
        pub async fn send_optimistic_envelope(
            &mut self,
            request: impl tonic::IntoRequest<crate::envelope::WireOptimisticEnvelope>,
        ) -> std::result::Result<tonic::Response<crate::envelope::WireOptimisticAck>, tonic::Status>
        {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::unknown(format!("Service was not ready: {}", e.into()))
            })?;
            let codec = crate::service::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/kairo.ecs.simulation.v1.SimulationTransport/SendOptimisticEnvelope",
            );
            let mut req = request.into_request();
            req.extensions_mut().insert(GrpcMethod::new(
                "kairo.ecs.simulation.v1.SimulationTransport",
                "SendOptimisticEnvelope",
            ));
            self.inner.unary(req, path, codec).await
        }
    }
}
/// Generated server implementations.
pub mod simulation_transport_server {
    #![allow(
        unused_variables,
        dead_code,
        missing_docs,
        clippy::wildcard_imports,
        clippy::let_unit_value
    )]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with SimulationTransportServer.
    #[async_trait]
    pub trait SimulationTransport: std::marker::Send + std::marker::Sync + 'static {
        async fn exchange_events(
            &self,
            request: tonic::Request<crate::envelope::WireRemoteEvent>,
        ) -> std::result::Result<tonic::Response<crate::envelope::WireRemoteEvent>, tonic::Status>;
        async fn migrate_entity(
            &self,
            request: tonic::Request<crate::envelope::WireMigrationRequest>,
        ) -> std::result::Result<tonic::Response<crate::envelope::WireMigrationAck>, tonic::Status>;
        async fn gvt_sync(
            &self,
            request: tonic::Request<crate::envelope::WireGvtProposal>,
        ) -> std::result::Result<tonic::Response<crate::envelope::WireGvtDecision>, tonic::Status>;
        async fn heartbeat(
            &self,
            request: tonic::Request<crate::envelope::WireHeartbeat>,
        ) -> std::result::Result<tonic::Response<crate::envelope::WireHeartbeatAck>, tonic::Status>;
        async fn send_optimistic_envelope(
            &self,
            request: tonic::Request<crate::envelope::WireOptimisticEnvelope>,
        ) -> std::result::Result<tonic::Response<crate::envelope::WireOptimisticAck>, tonic::Status>;
    }
    #[derive(Debug)]
    pub struct SimulationTransportServer<T> {
        inner: Arc<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    impl<T> SimulationTransportServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
                max_decoding_message_size: None,
                max_encoding_message_size: None,
            }
        }
        pub fn with_interceptor<F>(inner: T, interceptor: F) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
        /// Enable decompressing requests with the given encoding.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.accept_compression_encodings.enable(encoding);
            self
        }
        /// Compress responses with the given encoding, if the client supports it.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.send_compression_encodings.enable(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.max_decoding_message_size = Some(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.max_encoding_message_size = Some(limit);
            self
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for SimulationTransportServer<T>
    where
        T: SimulationTransport,
        B: Body + std::marker::Send + 'static,
        B::Error: Into<StdError> + std::marker::Send + 'static,
    {
        type Response = http::Response<tonic::body::Body>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<std::result::Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            match req.uri().path() {
                "/kairo.ecs.simulation.v1.SimulationTransport/ExchangeEvents" => {
                    #[allow(non_camel_case_types)]
                    struct ExchangeEventsSvc<T: SimulationTransport>(pub Arc<T>);
                    impl<T: SimulationTransport>
                        tonic::server::UnaryService<crate::envelope::WireRemoteEvent>
                        for ExchangeEventsSvc<T>
                    {
                        type Response = crate::envelope::WireRemoteEvent;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<crate::envelope::WireRemoteEvent>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SimulationTransport>::exchange_events(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = ExchangeEventsSvc(inner);
                        let codec = crate::service::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/kairo.ecs.simulation.v1.SimulationTransport/MigrateEntity" => {
                    #[allow(non_camel_case_types)]
                    struct MigrateEntitySvc<T: SimulationTransport>(pub Arc<T>);
                    impl<T: SimulationTransport>
                        tonic::server::UnaryService<crate::envelope::WireMigrationRequest>
                        for MigrateEntitySvc<T>
                    {
                        type Response = crate::envelope::WireMigrationAck;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<crate::envelope::WireMigrationRequest>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SimulationTransport>::migrate_entity(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = MigrateEntitySvc(inner);
                        let codec = crate::service::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/kairo.ecs.simulation.v1.SimulationTransport/GvtSync" => {
                    #[allow(non_camel_case_types)]
                    struct GvtSyncSvc<T: SimulationTransport>(pub Arc<T>);
                    impl<T: SimulationTransport>
                        tonic::server::UnaryService<crate::envelope::WireGvtProposal>
                        for GvtSyncSvc<T>
                    {
                        type Response = crate::envelope::WireGvtDecision;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<crate::envelope::WireGvtProposal>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SimulationTransport>::gvt_sync(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = GvtSyncSvc(inner);
                        let codec = crate::service::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/kairo.ecs.simulation.v1.SimulationTransport/Heartbeat" => {
                    #[allow(non_camel_case_types)]
                    struct HeartbeatSvc<T: SimulationTransport>(pub Arc<T>);
                    impl<T: SimulationTransport>
                        tonic::server::UnaryService<crate::envelope::WireHeartbeat>
                        for HeartbeatSvc<T>
                    {
                        type Response = crate::envelope::WireHeartbeatAck;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<crate::envelope::WireHeartbeat>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SimulationTransport>::heartbeat(&inner, request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = HeartbeatSvc(inner);
                        let codec = crate::service::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/kairo.ecs.simulation.v1.SimulationTransport/SendOptimisticEnvelope" => {
                    #[allow(non_camel_case_types)]
                    struct SendOptimisticEnvelopeSvc<T: SimulationTransport>(pub Arc<T>);
                    impl<T: SimulationTransport>
                        tonic::server::UnaryService<crate::envelope::WireOptimisticEnvelope>
                        for SendOptimisticEnvelopeSvc<T>
                    {
                        type Response = crate::envelope::WireOptimisticAck;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<crate::envelope::WireOptimisticEnvelope>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                <T as SimulationTransport>::send_optimistic_envelope(
                                    &inner, request,
                                )
                                .await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let method = SendOptimisticEnvelopeSvc(inner);
                        let codec = crate::service::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => Box::pin(async move {
                    let mut response = http::Response::new(tonic::body::Body::default());
                    let headers = response.headers_mut();
                    headers.insert(
                        tonic::Status::GRPC_STATUS,
                        (tonic::Code::Unimplemented as i32).into(),
                    );
                    headers.insert(
                        http::header::CONTENT_TYPE,
                        tonic::metadata::GRPC_CONTENT_TYPE,
                    );
                    Ok(response)
                }),
            }
        }
    }
    impl<T> Clone for SimulationTransportServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
                max_decoding_message_size: self.max_decoding_message_size,
                max_encoding_message_size: self.max_encoding_message_size,
            }
        }
    }
    /// Generated gRPC service name
    pub const SERVICE_NAME: &str = "kairo.ecs.simulation.v1.SimulationTransport";
    impl<T> tonic::server::NamedService for SimulationTransportServer<T> {
        const NAME: &'static str = SERVICE_NAME;
    }
}
