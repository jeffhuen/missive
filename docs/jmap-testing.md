# JMAP Testing with Stalwart

This guide documents how to set up a local JMAP server for testing the missive JMAP provider.

## Important Disclaimers

> **Development Environment Only**
>
> This guide describes a minimal Docker-based Stalwart configuration intended solely for local development and smoke testing of the JMAP adapter. This setup:
>
> - **Is not suitable for integration testing, QA, or production use**
> - **Uses `lvh.me`**, a public domain that resolves to `127.0.0.1` — this is a convenience workaround, not a security best practice
> - **Disables sender verification** (`must-match-sender = false`), which would be inappropriate in any secured environment
> - **Lacks TLS, DKIM, SPF, and DMARC configuration**
>
> For proper testing environments, deploy Stalwart with appropriate DNS records, TLS certificates, and authentication policies per your organization's security requirements. Refer to the [official Stalwart documentation](https://stalw.art/docs/) for production deployment guidance.

## Quick Start

```bash
# 1. Start Stalwart
docker run -d --name stalwart-test -p 8080:8080 -p 25:25 -p 587:587 \
  -v /tmp/stalwart-data:/opt/stalwart \
  stalwartlabs/stalwart:latest

# 2. Get admin password
docker logs stalwart-test 2>&1 | grep password
# Output: 🔑 Your administrator account is 'admin' with password 'XXXXXX'

# 3. Configure hostname and sender settings
docker exec stalwart-test sh -c 'cat >> /opt/stalwart/etc/config.toml << EOF

[server]
hostname = "lvh.me"

[session.auth]
must-match-sender = false
EOF'

# 4. Restart to apply config
docker restart stalwart-test

# 5. Create domain and user via API (replace ADMIN_PASS with actual password)
curl -u admin:ADMIN_PASS -X POST -H "Content-Type: application/json" \
  http://localhost:8080/api/principal \
  -d '{"type": "domain", "name": "lvh.me", "description": "Test domain"}'

curl -u admin:ADMIN_PASS -X POST -H "Content-Type: application/json" \
  http://localhost:8080/api/principal \
  -d '{"type": "individual", "name": "demo", "secrets": ["demopass"], "emails": ["demo@lvh.me"], "roles": ["user"]}'

# 6. Run the integration test
cargo run --example jmap_integration --features jmap
```

## Why lvh.me?

Stalwart validates email addresses against resolvable domains. Using non-existent TLDs like `test.local` will fail validation during email submission.

We use `lvh.me` as a pragmatic workaround because:
- It is a publicly registered domain that resolves to `127.0.0.1`
- It requires no local DNS configuration or `/etc/hosts` modifications
- It allows the JMAP submission flow to complete without DNS-related errors

**Trade-offs:** This approach relies on a third-party domain outside your control. For environments requiring deterministic behavior or network isolation, consider configuring a local DNS resolver or using a domain you control with appropriate A records.

## Configuration Details

### Required Settings

```toml
[server]
hostname = "lvh.me"

[session.auth]
must-match-sender = false
```

**`server.hostname`**: Stalwart returns this in the JMAP session response as the `apiUrl`. Without it, the Docker container's internal hostname is returned, which isn't accessible from the host.

**`session.auth.must-match-sender`**: When `true` (default), Stalwart requires the sender address to match the authenticated user's email exactly. Setting to `false` allows more flexibility during testing.

### Identity Auto-Creation

JMAP requires an `identityId` for email submission (per RFC 8621). Stalwart auto-creates identities when:

1. The config is properly set (especially `must-match-sender`)
2. `Identity/get` is called via JMAP
3. The user has an email address configured

Identities are NOT created until after a restart with proper config. If `Identity/get` returns an empty list, check your configuration and restart Stalwart.

## JMAP Submission Flow

Per RFC 8621, sending an email via JMAP requires:

1. **Session discovery**: `GET /.well-known/jmap` or `/jmap/session`
2. **Fetch mailboxes**: `Mailbox/get` to find the drafts mailbox
3. **Fetch identity**: `Identity/get` to get the sender identity
4. **Create email**: `Email/set` with `mailboxIds` pointing to drafts
5. **Submit**: `EmailSubmission/set` with `emailId` and `identityId`

The email MUST belong to at least one mailbox (RFC 8621 Section 4). We use the drafts mailbox and set `onSuccessDestroyEmail` to clean up after sending.

## Troubleshooting

### "Invalid e-mail address"
- Ensure the domain exists and resolves (use `lvh.me` not `test.local`)
- Check that the user's email matches the domain

### Empty Identity/get response
- Restart Stalwart after config changes
- Verify `must-match-sender = false` is set
- Check user has `email-send` permission

### "noRecipients" error
- The recipient domain must be valid/resolvable
- Use `lvh.me` for local testing

### API URL points to container hostname
- Set `server.hostname = "lvh.me"` in config
- Restart Stalwart

## Manual JMAP Testing

Test the JMAP API directly with curl:

```bash
# Get session
curl -s -u demo:demopass http://localhost:8080/jmap/session | jq '{apiUrl, accounts}'

# Get identities
curl -s -u demo:demopass -H "Content-Type: application/json" \
  http://localhost:8080/jmap/ -d '{
  "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:submission"],
  "methodCalls": [["Identity/get", {"accountId": "YOUR_ACCOUNT_ID"}, "i0"]]
}'

# Get mailboxes
curl -s -u demo:demopass -H "Content-Type: application/json" \
  http://localhost:8080/jmap/ -d '{
  "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail"],
  "methodCalls": [["Mailbox/get", {"accountId": "YOUR_ACCOUNT_ID"}, "m0"]]
}'

# Send email (replace IDs with actual values)
curl -s -u demo:demopass -H "Content-Type: application/json" \
  http://localhost:8080/jmap/ -d '{
  "using": ["urn:ietf:params:jmap:core", "urn:ietf:params:jmap:mail", "urn:ietf:params:jmap:submission"],
  "methodCalls": [
    ["Email/set", {
      "accountId": "YOUR_ACCOUNT_ID",
      "create": {
        "draft": {
          "mailboxIds": {"DRAFTS_MAILBOX_ID": true},
          "from": [{"email": "demo@lvh.me"}],
          "to": [{"email": "demo@lvh.me"}],
          "subject": "Test",
          "bodyValues": {"text": {"value": "Hello!"}},
          "textBody": [{"partId": "text", "type": "text/plain"}]
        }
      }
    }, "e0"],
    ["EmailSubmission/set", {
      "accountId": "YOUR_ACCOUNT_ID",
      "create": {
        "sub": {
          "emailId": "#draft",
          "identityId": "IDENTITY_ID"
        }
      },
      "onSuccessDestroyEmail": ["#sub"]
    }, "s0"]
  ]
}'
```

## Cleanup

```bash
docker stop stalwart-test && docker rm stalwart-test
rm -rf /tmp/stalwart-data
```

## References

- [JMAP Mail Specification (RFC 8621)](https://www.rfc-editor.org/rfc/rfc8621)
- [Stalwart Documentation](https://stalw.art/docs/)
- [lvh.me - localhost domain](http://lvh.me)
