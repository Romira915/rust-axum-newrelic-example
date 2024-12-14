use axum::extract::MatchedPath;
use axum::http::Request;
use axum::{routing::get, Router};
use rust_axum_newrelic_example::handler::handler;
use rust_axum_newrelic_example::opentelemetry::init_opentelemetry;
use tower::layer::util::{Identity, Stack};
use tower::ServiceBuilder;
use tower_http::classify::{ServerErrorsAsFailures, SharedClassifier};
use tower_http::trace::{MakeSpan, TraceLayer};
use tracing::Span;

#[tokio::main]
async fn main() {
    let newrelic_license_key =
        std::env::var("NEWRELIC_LICENSE_KEY").expect("NEWRELIC_LICENSE_KEY env var is required");

    init_opentelemetry(
        "https://otlp.nr-data.net",
        &newrelic_license_key,
        "axum-newrelic-example",
        "localhost",
    )
    .expect("Failed to initialize OpenTelemetry");

    // let _ = easy_init_newrelic_opentelemetry::NewRelicSubscriberInitializer::default()
    //     .newrelic_license_key(&newrelic_license_key)
    //     .newrelic_service_name("axum-newrelic-example")
    //     .host_name("localhost")
    //     .init();

    // build our application with a route
    let app = Router::new()
        .route("/", get(handler))
        .layer(MakeSpanForHttp.into_tracing_service());

    // run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

#[derive(Clone)]
pub(crate) struct MakeSpanForHttp;

impl<B> MakeSpan<B> for MakeSpanForHttp {
    fn make_span(&mut self, request: &Request<B>) -> Span {
        let matched_path = request
            .extensions()
            .get::<MatchedPath>()
            .map_or(request.uri().to_string(), |m| m.as_str().to_string());
        tracing::info_span!(
            "request",
            http.method = %request.method(),
            http.uri = %request.uri(),
            http.version = ?request.version(),
            otel.name = format!("{} {}", request.method(), matched_path),
            otel.kind = "server",
        )
    }
}

impl MakeSpanForHttp {
    pub(crate) fn into_tracing_service(
        self,
    ) -> ServiceBuilder<Stack<TraceLayer<SharedClassifier<ServerErrorsAsFailures>, Self>, Identity>>
    {
        ServiceBuilder::new().layer(TraceLayer::new_for_http().make_span_with(self))
    }
}
