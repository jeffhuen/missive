# Templates

Missive integrates with [Askama](https://github.com/djc/askama) for type-safe email templates.

## Setup

Enable the `templates` feature:

```toml
[dependencies]
missive = { version = "0.4", features = ["resend", "templates"] }
askama = "0.13"
```

## Basic Usage

There are two ways to use templates with Missive:

1. **`EmailTemplate` trait** - Define subject and recipient in the template struct
2. **Manual rendering** - Render the template yourself and pass to `Email`

### Using EmailTemplate Trait

The `EmailTemplate` trait lets you encapsulate all email details in one struct:

```html
<!-- templates/welcome.html -->
<!DOCTYPE html>
<html>
<head>
    <title>Welcome to {{ app_name }}</title>
</head>
<body>
    <h1>Welcome, {{ username }}!</h1>
    <p>Thanks for signing up. Click below to verify your email:</p>
    <a href="{{ verify_url }}">Verify Email</a>
</body>
</html>
```

```rust
use askama::Template;
use missive::{Address, EmailTemplate};

#[derive(Template)]
#[template(path = "welcome.html")]
struct WelcomeEmail {
    app_name: String,
    username: String,
    verify_url: String,
    recipient: String,
}

impl EmailTemplate for WelcomeEmail {
    fn subject(&self) -> String {
        format!("Welcome to {}!", self.app_name)
    }

    fn to(&self) -> Address {
        self.recipient.as_str().into()
    }
}
```

Convert to `Email` and send:

```rust
let template = WelcomeEmail {
    app_name: "My App".to_string(),
    username: "Alice".to_string(),
    verify_url: "https://example.com/verify?token=abc123".to_string(),
    recipient: "alice@example.com".to_string(),
};

let email = template.into_email()?;
missive::deliver(&email).await?;
```

### Manual Rendering

For more control, render templates manually:

```rust
use askama::Template;
use missive::Email;

#[derive(Template)]
#[template(path = "welcome.html")]
struct WelcomeTemplate {
    app_name: String,
    username: String,
    verify_url: String,
}

let template = WelcomeTemplate {
    app_name: "My App".to_string(),
    username: "Alice".to_string(),
    verify_url: "https://example.com/verify?token=abc123".to_string(),
};

let html = template.render()?;

let email = Email::new()
    .to("alice@example.com")
    .subject("Welcome to My App!")
    .html_body(&html);

missive::deliver(&email).await?;
```

## EmailTemplate Options

The `EmailTemplate` trait has optional methods you can override:

```rust
impl EmailTemplate for WelcomeEmail {
    fn subject(&self) -> String {
        "Welcome!".to_string()
    }

    fn to(&self) -> Address {
        self.recipient.as_str().into()
    }

    // Optional: Set the sender
    fn from(&self) -> Option<Address> {
        Some(("My App", "hello@myapp.com").into())
    }

    // Optional: Set reply-to
    fn reply_to(&self) -> Option<Address> {
        Some("support@myapp.com".into())
    }

    // Optional: Add CC recipients
    fn cc(&self) -> Vec<Address> {
        vec!["team@myapp.com".into()]
    }

    // Optional: Add BCC recipients
    fn bcc(&self) -> Vec<Address> {
        vec![]
    }
}
```

## Both HTML and Text

For better deliverability, provide both HTML and plain text versions.

### With EmailTemplate

Use `into_email_with_text()` from the `EmailTemplateExt` trait:

```rust
use askama::Template;
use missive::{Address, EmailTemplate, EmailTemplateExt};

#[derive(Template)]
#[template(path = "welcome.html")]
struct WelcomeEmail {
    username: String,
    verify_url: String,
    recipient: String,
}

impl EmailTemplate for WelcomeEmail {
    fn subject(&self) -> String {
        "Welcome!".to_string()
    }

    fn to(&self) -> Address {
        self.recipient.as_str().into()
    }
}

// Create a separate text template
#[derive(Template)]
#[template(path = "welcome.txt")]
struct WelcomeText {
    username: String,
    verify_url: String,
}

let html_template = WelcomeEmail {
    username: "Alice".to_string(),
    verify_url: "https://example.com/verify".to_string(),
    recipient: "alice@example.com".to_string(),
};

let text_template = WelcomeText {
    username: "Alice".to_string(),
    verify_url: "https://example.com/verify".to_string(),
};

let text_body = text_template.render()?;
let email = html_template.into_email_with_text(&text_body)?;
```

### With Manual Rendering

```rust
#[derive(Template)]
#[template(path = "welcome.html")]
struct WelcomeHtml {
    username: String,
    verify_url: String,
}

#[derive(Template)]
#[template(path = "welcome.txt")]
struct WelcomeText {
    username: String,
    verify_url: String,
}

let html = WelcomeHtml { /* ... */ }.render()?;
let text = WelcomeText { /* ... */ }.render()?;

let email = Email::new()
    .to("alice@example.com")
    .subject("Welcome!")
    .html_body(&html)
    .text_body(&text);
```

Example templates:

```html
<!-- templates/welcome.html -->
<h1>Welcome, {{ username }}!</h1>
<a href="{{ verify_url }}">Verify</a>
```

```text
<!-- templates/welcome.txt -->
Welcome, {{ username }}!

Verify: {{ verify_url }}
```

## Template Location

By default, Askama looks for templates in a `templates/` directory at your crate root:

```
my-app/
├── Cargo.toml
├── src/
│   └── main.rs
└── templates/
    ├── welcome.html
    ├── welcome.txt
    └── password_reset.html
```

Configure in `askama.toml` if needed:

```toml
[general]
dirs = ["templates", "emails"]
```

## Template Inheritance

Use Askama's template inheritance for consistent layouts:

```html
<!-- templates/base.html -->
<!DOCTYPE html>
<html>
<head>
    <style>
        body { font-family: sans-serif; }
        .container { max-width: 600px; margin: 0 auto; }
        .footer { color: #666; font-size: 12px; }
    </style>
</head>
<body>
    <div class="container">
        {% block content %}{% endblock %}

        <div class="footer">
            <p>© 2024 {{ company_name }}</p>
            <p><a href="{{ unsubscribe_url }}">Unsubscribe</a></p>
        </div>
    </div>
</body>
</html>
```

```html
<!-- templates/welcome.html -->
{% extends "base.html" %}

{% block content %}
<h1>Welcome, {{ username }}!</h1>
<p>Thanks for joining us.</p>
{% endblock %}
```

```rust
#[derive(Template)]
#[template(path = "welcome.html")]
struct WelcomeEmail {
    username: String,
    company_name: String,
    unsubscribe_url: String,
    recipient: String,
}

impl EmailTemplate for WelcomeEmail {
    fn subject(&self) -> String {
        format!("Welcome, {}!", self.username)
    }

    fn to(&self) -> Address {
        self.recipient.as_str().into()
    }
}
```

## Inline Styles

Email clients have limited CSS support. Use inline styles or a CSS inliner:

```rust
// Option 1: Inline styles in template
// <h1 style="color: #333; font-size: 24px;">Welcome</h1>

// Option 2: Use css-inline crate
use css_inline::inline;

let html = template.render()?;
let inlined = inline(&html)?;

let email = Email::new()
    .to("user@example.com")
    .subject("Welcome!")
    .html_body(inlined);
```

## Dynamic Content

Askama supports loops, conditionals, and filters:

```html
<h1>Your Order #{{ order_id }}</h1>

<table>
    {% for item in items %}
    <tr>
        <td>{{ item.name }}</td>
        <td>{{ item.quantity }}</td>
        <td>${{ item.price|fmt("{:.2}") }}</td>
    </tr>
    {% endfor %}
</table>

<p><strong>Total: ${{ total|fmt("{:.2}") }}</strong></p>

{% if has_discount %}
<p>Discount applied: {{ discount_code }}</p>
{% endif %}
```

## Error Handling

Template rendering can fail. Handle errors appropriately:

```rust
use missive::MailError;

let template = WelcomeEmail { /* ... */ };

match template.into_email() {
    Ok(email) => {
        missive::deliver(&email).await?;
    }
    Err(MailError::TemplateError(e)) => {
        tracing::error!("Template rendering failed: {}", e);
        // Maybe send a fallback plain text email
    }
    Err(e) => return Err(e),
}
```

## Testing Templates

Test template rendering without sending:

```rust
#[test]
fn test_welcome_template() {
    let template = WelcomeEmail {
        username: "Alice".to_string(),
        verify_url: "https://example.com/verify".to_string(),
        recipient: "alice@example.com".to_string(),
    };

    // Test raw rendering
    let html = template.render().unwrap();
    assert!(html.contains("Welcome, Alice!"));
    assert!(html.contains("https://example.com/verify"));
}

#[test]
fn test_email_conversion() {
    let template = WelcomeEmail {
        username: "Alice".to_string(),
        verify_url: "https://example.com/verify".to_string(),
        recipient: "alice@example.com".to_string(),
    };

    let email = template.into_email().unwrap();
    assert_eq!(email.subject, "Welcome!");
    assert!(email.html_body.unwrap().contains("Alice"));
}
```

## Organize Email Templates

Suggested structure for larger apps:

```
src/
├── emails/
│   ├── mod.rs
│   ├── welcome.rs
│   ├── password_reset.rs
│   └── order_confirmation.rs
└── main.rs

templates/
├── emails/
│   ├── base.html
│   ├── welcome.html
│   ├── welcome.txt
│   ├── password_reset.html
│   └── order_confirmation.html
```

```rust
// src/emails/mod.rs
mod welcome;
mod password_reset;

pub use welcome::send_welcome_email;
pub use password_reset::send_password_reset;
```

```rust
// src/emails/welcome.rs
use askama::Template;
use missive::{Address, EmailTemplate, deliver};

#[derive(Template)]
#[template(path = "emails/welcome.html")]
struct WelcomeEmail {
    username: String,
    verify_url: String,
    recipient: String,
}

impl EmailTemplate for WelcomeEmail {
    fn subject(&self) -> String {
        "Welcome!".to_string()
    }

    fn to(&self) -> Address {
        self.recipient.as_str().into()
    }
}

pub async fn send_welcome_email(
    to: &str,
    username: &str,
    verify_url: &str,
) -> Result<(), missive::MailError> {
    let template = WelcomeEmail {
        username: username.to_string(),
        verify_url: verify_url.to_string(),
        recipient: to.to_string(),
    };

    let email = template.into_email()?;
    deliver(&email).await?;
    Ok(())
}
```
