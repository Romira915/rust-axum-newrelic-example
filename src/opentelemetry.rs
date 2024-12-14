use opentelemetry::trace::TraceError;
use opentelemetry::trace::TracerProvider;
use opentelemetry::KeyValue;
use opentelemetry_otlp::{
    LogExporter, MetricExporter, Protocol, SpanExporter, WithExportConfig, WithHttpConfig,
};
use opentelemetry_sdk::logs::LoggerProvider;
use opentelemetry_sdk::metrics::MetricError;
use opentelemetry_sdk::trace::TracerProvider as SdkTracerProvider;
use opentelemetry_sdk::{runtime, Resource};
use std::collections::HashMap;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

fn resource(new_relic_service_name: &str, host_name: &str) -> Resource {
    Resource::new(vec![
        KeyValue::new(
            opentelemetry_semantic_conventions::resource::SERVICE_NAME,
            new_relic_service_name.to_string(),
        ),
        KeyValue::new(
            opentelemetry_semantic_conventions::resource::HOST_NAME,
            host_name.to_string(),
        ),
    ])
}

fn init_logger_provider(
    new_relic_otlp_endpoint: &str,
    new_relic_license_key: &str,
    new_relic_service_name: &str,
    host_name: &str,
) -> Result<LoggerProvider, opentelemetry_sdk::logs::LogError> {
    let exporter = LogExporter::builder()
        .with_http()
        .with_endpoint(format!("{}/v1/logs", new_relic_otlp_endpoint))
        .with_headers(HashMap::from([(
            "api-key".into(),
            new_relic_license_key.into(),
        )]))
        .with_protocol(Protocol::HttpJson)
        .build()?;

    Ok(LoggerProvider::builder()
        .with_resource(resource(new_relic_service_name, host_name))
        .with_batch_exporter(exporter, runtime::Tokio)
        .build())
}

fn init_tracer_provider(
    new_relic_otlp_endpoint: &str,
    new_relic_license_key: &str,
    new_relic_service_name: &str,
    host_name: &str,
) -> Result<SdkTracerProvider, TraceError> {
    let exporter = SpanExporter::builder()
        .with_http()
        .with_endpoint(format!("{}/v1/traces", new_relic_otlp_endpoint))
        .with_headers(HashMap::from([(
            "api-key".into(),
            new_relic_license_key.into(),
        )]))
        .with_protocol(Protocol::HttpJson)
        .build()?;

    Ok(SdkTracerProvider::builder()
        .with_resource(resource(new_relic_service_name, host_name))
        .with_batch_exporter(exporter, runtime::Tokio)
        .build())
}

fn init_metrics(
    new_relic_otlp_endpoint: &str,
    new_relic_license_key: &str,
    new_relic_service_name: &str,
    host_name: &str,
) -> Result<opentelemetry_sdk::metrics::SdkMeterProvider, MetricError> {
    let exporter = MetricExporter::builder()
        .with_http()
        .with_endpoint(format!("{}/v1/metrics", new_relic_otlp_endpoint))
        .with_headers(HashMap::from([(
            "api-key".into(),
            new_relic_license_key.into(),
        )]))
        .with_protocol(Protocol::HttpJson)
        .build()?;

    let reader =
        opentelemetry_sdk::metrics::PeriodicReader::builder(exporter, runtime::Tokio).build();

    Ok(opentelemetry_sdk::metrics::SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(resource(new_relic_service_name, host_name))
        .build())
}

pub fn init_opentelemetry(
    new_relic_otlp_endpoint: &str,
    new_relic_license_key: &str,
    new_relic_service_name: &str,
    host_name: &str,
) -> anyhow::Result<()> {
    let tracer_provider = init_tracer_provider(
        &new_relic_otlp_endpoint,
        &new_relic_license_key,
        &new_relic_service_name,
        &host_name,
    )?;
    opentelemetry::global::set_tracer_provider(tracer_provider.clone());
    let tracer = tracer_provider.tracer(new_relic_service_name.to_string());

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(true)
        .with_file(true)
        .with_line_number(true)
        .with_target(true);
    let env_filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    let otel_trace_layer = tracing_opentelemetry::OpenTelemetryLayer::new(tracer)
        .with_error_records_to_exceptions(true)
        .with_error_fields_to_exceptions(true)
        .with_error_events_to_status(true)
        .with_error_events_to_exceptions(true)
        .with_location(true);
    let otel_metrics_layer = tracing_opentelemetry::MetricsLayer::new(init_metrics(
        new_relic_otlp_endpoint,
        new_relic_license_key,
        new_relic_service_name,
        host_name,
    )?);
    let logger_provider = init_logger_provider(
        new_relic_otlp_endpoint,
        new_relic_license_key,
        new_relic_service_name,
        host_name,
    )?;
    let otel_logs_layer =
        opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge::new(&logger_provider);

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(env_filter)
        .with(otel_trace_layer)
        .with(otel_metrics_layer)
        .with(otel_logs_layer)
        .init();

    Ok(())
}
