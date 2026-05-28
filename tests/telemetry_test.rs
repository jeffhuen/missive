#![cfg(feature = "local")]

use missive::providers::LocalMailer;
use missive::{configure, deliver, deliver_with, reset, Email};
use std::sync::{Arc, Mutex};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Level, Metadata, Subscriber};

#[derive(Clone, Debug)]
struct CapturedTelemetry {
    name: String,
    level: Option<Level>,
    fields: Vec<(String, String)>,
}

#[derive(Clone, Default)]
struct CapturingSubscriber {
    spans: Arc<Mutex<Vec<CapturedTelemetry>>>,
    events: Arc<Mutex<Vec<CapturedTelemetry>>>,
}

impl Subscriber for CapturingSubscriber {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, attrs: &Attributes<'_>) -> Id {
        let mut visitor = FieldVisitor::default();
        attrs.record(&mut visitor);

        let mut spans = self.spans.lock().unwrap();
        spans.push(CapturedTelemetry {
            name: attrs.metadata().name().to_string(),
            level: Some(*attrs.metadata().level()),
            fields: visitor.fields,
        });

        Id::from_u64(spans.len() as u64)
    }

    fn record(&self, span: &Id, values: &Record<'_>) {
        let mut visitor = FieldVisitor::default();
        values.record(&mut visitor);

        let span_index = span.clone().into_u64() as usize - 1;
        if let Some(captured) = self.spans.lock().unwrap().get_mut(span_index) {
            captured.fields.extend(visitor.fields);
        }
    }

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, event: &Event<'_>) {
        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);

        self.events.lock().unwrap().push(CapturedTelemetry {
            name: event.metadata().name().to_string(),
            level: Some(*event.metadata().level()),
            fields: visitor.fields,
        });
    }

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}
}

#[derive(Default)]
struct FieldVisitor {
    fields: Vec<(String, String)>,
}

impl Visit for FieldVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.fields
            .push((field.name().to_string(), format!("{value:?}")));
    }
}

struct ResetGlobalMailer;

impl Drop for ResetGlobalMailer {
    fn drop(&mut self) {
        reset();
    }
}

#[tokio::test(flavor = "current_thread")]
async fn delivery_spans_do_not_record_recipient_or_subject() {
    let subscriber = CapturingSubscriber::default();
    let spans = Arc::clone(&subscriber.spans);
    let events = Arc::clone(&subscriber.events);
    let _guard = tracing::subscriber::set_default(subscriber);

    let mailer = LocalMailer::new();
    let email = Email::new()
        .from("noreply@example.com")
        .to("steve.rogers@example.com")
        .cc("natasha.romanova@example.com")
        .subject("Sensitive password reset")
        .text_body("Hello");

    deliver_with(&email, &mailer).await.unwrap();
    configure(LocalMailer::new());
    let _reset = ResetGlobalMailer;
    deliver(&email).await.unwrap();

    let spans = spans.lock().unwrap();
    let delivery_spans: Vec<_> = spans
        .iter()
        .filter(|span| span.name == "missive.deliver")
        .collect();
    assert_eq!(delivery_spans.len(), 2);

    for span in delivery_spans {
        let field_names: Vec<&str> = span
            .fields
            .iter()
            .map(|(name, _value)| name.as_str())
            .collect();
        assert!(field_names.contains(&"provider"));
        assert!(field_names.contains(&"recipient_count"));
        assert!(field_names.contains(&"attachment_count"));
        assert!(field_names.contains(&"status"));
        assert!(field_names.contains(&"duration_ms"));
        assert!(!field_names.contains(&"to"));
        assert!(!field_names.contains(&"subject"));
        assert!(!field_names.contains(&"message_id"));

        let rendered_fields = span
            .fields
            .iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join(" ");
        assert!(!rendered_fields.contains("steve.rogers@example.com"));
        assert!(!rendered_fields.contains("natasha.romanova@example.com"));
        assert!(!rendered_fields.contains("Sensitive password reset"));
    }

    let events = events.lock().unwrap();
    let default_level_events: Vec<_> = events
        .iter()
        .filter(|event| matches!(event.level, Some(Level::INFO | Level::ERROR)))
        .collect();

    for event in &default_level_events {
        let field_names: Vec<&str> = event
            .fields
            .iter()
            .map(|(name, _value)| name.as_str())
            .collect();
        assert!(!field_names.contains(&"to"));
        assert!(!field_names.contains(&"subject"));
        assert!(!field_names.contains(&"message_id"));

        let rendered_fields = event
            .fields
            .iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join(" ");
        assert!(!rendered_fields.contains("steve.rogers@example.com"));
        assert!(!rendered_fields.contains("natasha.romanova@example.com"));
        assert!(!rendered_fields.contains("Sensitive password reset"));
    }

    let delivery_events: Vec<_> = default_level_events
        .iter()
        .filter(|event| {
            let field_names: Vec<&str> = event
                .fields
                .iter()
                .map(|(name, _value)| name.as_str())
                .collect();
            field_names.contains(&"provider")
                && field_names.contains(&"status")
                && field_names.contains(&"recipient_count")
                && field_names.contains(&"attachment_count")
                && field_names.contains(&"duration_ms")
        })
        .collect();
    assert_eq!(delivery_events.len(), 2);
}
