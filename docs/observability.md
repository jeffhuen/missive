# Observability

Missive provides built-in support for monitoring email delivery through tracing (telemetry) and metrics.

## Table of Contents

- [Telemetry (Tracing)](#telemetry-tracing)
  - [Setup](#setup)
  - [Emitted Spans](#emitted-spans)
  - [Log Levels](#log-levels)
  - [Filtering](#filtering)
  - [JSON Logging](#json-logging)
  - [OpenTelemetry Integration](#opentelemetry-integration)
  - [Custom Spans](#custom-spans)
- [Metrics](#metrics)
  - [Setup](#metrics-setup)
  - [Available Metrics](#available-metrics)
  - [Prometheus](#prometheus)
  - [StatsD](#statsd)
  - [Grafana Dashboards](#grafana-dashboards)
  - [Alerting](#alerting)
- [Production Setup](#production-setup)
- [Distributed Tracing](#distributed-tracing)

---

## Telemetry (Tracing)

Missive uses the [`tracing`](https://docs.rs/tracing) crate for structured logging and distributed tracing.

Every email delivery creates a span with relevant context:

```
missive.deliver { provider="resend", to=["user@example.com"], subject="Welcome!" }
  ├── DEBUG: Delivering email
  └── INFO: Email delivered { message_id="abc123" }
```

### Setup

Add a tracing subscriber to your application:

```rust
use tracing_subscriber;

fn main() {
    // Simple console logging
    tracing_subscriber::fmt::init();

    // Now all missive operations are logged
}
```

### Emitted Spans

| Span | Level | Fields | Description |
|------|-------|--------|-------------|
| `missive.deliver` | INFO | provider, to, subject | Single email delivery |
| `missive.deliver_many` | INFO | provider, count | Batch delivery |

**Span Fields:**

- `provider` - The mailer being used (resend, sendgrid, smtp, etc.)
- `to` - List of recipient email addresses
- `subject` - Email subject line
- `count` - Number of emails (batch only)
- `message_id` - Provider's message ID (on success)

### Log Levels

| Level | What's Logged |
|-------|---------------|
| ERROR | Delivery failures with error details |
| INFO | Successful deliveries with message ID |
| DEBUG | Pre-delivery details |

### Filtering

Control verbosity with `RUST_LOG`:

```bash
# Only errors
RUST_LOG=missive=error cargo run

# Info and above (recommended)
RUST_LOG=missive=info cargo run

# Full debug output
RUST_LOG=missive=debug cargo run

# Combined with other crates
RUST_LOG=info,missive=debug cargo run
```

### JSON Logging

For production log aggregation (ELK, Loki, etc.):

```toml
[dependencies]
tracing-subscriber = { version = "0.3", features = ["json"] }
```

```rust
fn main() {
    tracing_subscriber::fmt()
        .json()
        .init();
}
```

Output:
```json
{"timestamp":"2024-01-15T10:30:00Z","level":"INFO","target":"missive","span":{"name":"missive.deliver","provider":"resend","to":["user@example.com"],"subject":"Welcome!"},"message":"Email delivered","message_id":"abc123"}
```

### OpenTelemetry Integration

Export traces to Jaeger, Zipkin, or other backends:

```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = "0.3"
tracing-opentelemetry = "0.22"
opentelemetry = "0.21"
opentelemetry-jaeger = "0.20"
```

```rust
use opentelemetry::global;
use tracing_subscriber::prelude::*;

fn main() {
    // Initialize Jaeger exporter
    let tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_service_name("my-app")
        .install_simple()
        .expect("Failed to install Jaeger exporter");

    // Create OpenTelemetry layer
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

    // Combine with fmt layer
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(telemetry)
        .init();

    // Email deliveries now appear in Jaeger
}
```

### Custom Spans

Wrap email operations in your own spans for additional context:

```rust
use tracing::{instrument, info_span};

#[instrument(skip(user), fields(user_id = %user.id))]
async fn send_welcome_email(user: &User) -> Result<(), missive::MailError> {
    let email = Email::new()
        .to(&user.email)
        .subject("Welcome!");

    missive::deliver(&email).await
}
```

Result:
```
send_welcome_email { user_id=42 }
  └── missive.deliver { provider="resend", to=["user@example.com"], subject="Welcome!" }
```

---

## Metrics

Missive can emit Prometheus-style metrics for dashboards and alerting.

### Metrics Setup

Enable the `metrics` feature:

```toml
[dependencies]
missive = { version = "0.4", features = ["resend", "metrics"] }
```

### Available Metrics

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `missive_emails_total` | Counter | `provider`, `status` | Total emails sent |
| `missive_delivery_duration_seconds` | Histogram | `provider` | Time to deliver email |
| `missive_batch_total` | Counter | `provider`, `status` | Batch operations count |
| `missive_batch_size` | Histogram | `provider` | Emails per batch |

**Labels:**

- `provider`: The email provider used (`resend`, `sendgrid`, `smtp`, etc.)
- `status`: Either `success` or `error`

### Prometheus

Install a Prometheus recorder:

```toml
[dependencies]
metrics-exporter-prometheus = "0.16"
```

```rust
use metrics_exporter_prometheus::PrometheusBuilder;

fn main() {
    PrometheusBuilder::new()
        .install()
        .expect("failed to install Prometheus recorder");
}
```

With Axum:

```rust
use axum::{Router, routing::get};
use metrics_exporter_prometheus::PrometheusBuilder;

#[tokio::main]
async fn main() {
    let recorder = PrometheusBuilder::new()
        .install_recorder()
        .expect("failed to install Prometheus recorder");

    let app = Router::new()
        .route("/metrics", get(move || {
            std::future::ready(recorder.render())
        }));

    // Start server...
}
```

Example output:

```
# HELP missive_emails_total Total emails sent
# TYPE missive_emails_total counter
missive_emails_total{provider="resend",status="success"} 142
missive_emails_total{provider="resend",status="error"} 3

# HELP missive_delivery_duration_seconds Time to deliver email
# TYPE missive_delivery_duration_seconds histogram
missive_delivery_duration_seconds_bucket{provider="resend",le="0.1"} 95
missive_delivery_duration_seconds_bucket{provider="resend",le="0.5"} 140
missive_delivery_duration_seconds_bucket{provider="resend",le="+Inf"} 142
```

### StatsD

```toml
[dependencies]
metrics-exporter-statsd = "0.7"
```

```rust
use metrics_exporter_statsd::StatsdBuilder;

fn main() {
    StatsdBuilder::from("127.0.0.1", 8125)
        .install()
        .expect("failed to install StatsD recorder");
}
```

### Grafana Dashboards

Example PromQL queries:

```promql
# Emails sent per minute
rate(missive_emails_total[1m])

# Success rate
sum(rate(missive_emails_total{status="success"}[5m])) /
sum(rate(missive_emails_total[5m]))

# Average delivery time
rate(missive_delivery_duration_seconds_sum[5m]) /
rate(missive_delivery_duration_seconds_count[5m])

# 95th percentile delivery time
histogram_quantile(0.95, rate(missive_delivery_duration_seconds_bucket[5m]))

# Errors by provider
sum by (provider) (rate(missive_emails_total{status="error"}[5m]))
```

### Alerting

Example Prometheus alert rules:

```yaml
# Alert on high error rate
- alert: EmailDeliveryErrors
  expr: |
    sum(rate(missive_emails_total{status="error"}[5m])) /
    sum(rate(missive_emails_total[5m])) > 0.05
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "Email delivery error rate above 5%"

# Alert on slow delivery
- alert: SlowEmailDelivery
  expr: |
    histogram_quantile(0.95, rate(missive_delivery_duration_seconds_bucket[5m])) > 5
  for: 10m
  labels:
    severity: warning
  annotations:
    summary: "95th percentile email delivery time above 5 seconds"
```

### Zero-Cost When Disabled

- If you don't enable the `metrics` feature, metric calls are not compiled into your binary
- If you enable `metrics` but don't install a recorder, metric calls are no-ops with negligible overhead

---

## Production Setup

Combine tracing and metrics for full observability:

```toml
[dependencies]
missive = { version = "0.4", features = ["resend", "metrics"] }
tracing-subscriber = { version = "0.3", features = ["json", "env-filter"] }
metrics-exporter-prometheus = "0.16"
```

```rust
use metrics_exporter_prometheus::PrometheusBuilder;
use tracing_subscriber::EnvFilter;

fn setup_observability() {
    // JSON logging for ELK/Loki
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::new("info,missive=info"))
        .init();

    // Prometheus metrics for Grafana
    PrometheusBuilder::new()
        .install()
        .expect("failed to install metrics");
}
```

**Recommendations:**

1. **Use INFO level** - DEBUG is verbose, ERROR misses successful deliveries
2. **Enable JSON logging** - Easier to parse in log aggregators
3. **Add OpenTelemetry** - For distributed tracing across services
4. **Use both tracing and metrics** - Tracing for debugging, metrics for alerting

---

## Distributed Tracing

For microservices, propagate trace context so email deliveries appear in the same trace as the originating request:

```rust
use tracing_opentelemetry::OpenTelemetrySpanExt;
use opentelemetry::propagation::Extractor;

async fn handle_request(headers: HeaderMap) -> Response {
    // Extract trace context from incoming request
    let parent_cx = global::get_text_map_propagator(|propagator| {
        propagator.extract(&HeaderExtractor(&headers))
    });

    // Create span with parent context
    let span = info_span!("handle_request");
    span.set_parent(parent_cx);

    // Email delivery inherits the trace context
    async move {
        missive::deliver(&email).await
    }
    .instrument(span)
    .await
}
```

Now your email deliveries appear in the same trace as the HTTP request that triggered them, providing end-to-end visibility across your system.
