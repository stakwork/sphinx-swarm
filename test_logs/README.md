# Testing Vector Log Ingestion

## Setup

1. Start the logs stack:
```bash
export ONLY_LOGS=true
export RUST_ENV=local
export QUICKWIT_MAX_STORAGE_MB=1
cargo run --bin stack
```

2. Get the auth token from the logs or config. It will be printed during startup or found in the saved config.

## Test with curl

### Generic JSON logs

```bash
# Single log entry
curl -X POST http://localhost:9000/logs \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN_HERE" \
  -d '{"message": "Hello from test", "level": "info", "service": "my-app"}'

# Multiple logs (NDJSON)
curl -X POST http://localhost:9000/logs \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN_HERE" \
  --data-binary @test_logs/generic.ndjson
```

### Vercel logs

```bash
curl -X POST http://localhost:9000/vercel \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer Sne9AExs6lOxngJgzrNJDEwSz8cxjKXc" \
  --data-binary @test_logs/vercel.ndjson
```

### Test auth rejection (should fail)

```bash
# No auth header - should be rejected
curl -X POST http://localhost:9000/logs \
  -H "Content-Type: application/json" \
  -d '{"message": "This should fail"}'

# Wrong token - should be rejected
curl -X POST http://localhost:9000/logs \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer wrong_token" \
  -d '{"message": "This should also fail"}'
```

## Query logs in Quickwit

Quickwit UI is available at http://localhost:7280 (internal only, not exposed via Traefik).

### Search via API

```bash
# Search all logs
curl "http://localhost:7280/api/v1/logs/search?query=*"

# Search by level
curl "http://localhost:7280/api/v1/logs/search?query=level:error"

# Search by source (vercel vs generic)
curl "http://localhost:7280/api/v1/logs/search?query=log_source:vercel"

# Search by message content
curl "http://localhost:7280/api/v1/logs/search?query=message:error"
```

## Production (with Traefik/HTTPS)

Replace `localhost:9000` with `https://vector.yourdomain.com`

For Vercel log drain setup:
1. Go to Vercel Project → Settings → Log Drains
2. Add custom endpoint: `https://vector.yourdomain.com/vercel`
3. Format: NDJSON
4. Add header: `Authorization: Bearer YOUR_TOKEN_HERE`
