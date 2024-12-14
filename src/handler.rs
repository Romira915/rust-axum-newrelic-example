use axum::response::Html;
use tracing::instrument;

#[instrument]
pub async fn handler() -> Html<&'static str> {
    tracing::info!("request received");
    sub1("handler");
    sub2("handler");

    Html("<h1>Hello, World!</h1>")
}

#[instrument]
pub fn sub1(call_fn: &str) {
    tracing::info!(call_fn = call_fn, "sub1");
    sub2("sub1");
}

#[instrument]
pub fn sub2(call_fn: &str) {
    tracing::error!("sub2");
}
