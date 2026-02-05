# Log Ingestion System - Implementation Notes

## What We Built

A log ingestion pipeline using **Vector** (log collector) and **Quickwit** (log storage/search):

### Components

1. **Quickwit** (`src/images/quickwit.rs`)
   - Log storage and search engine
   - Internal only (not exposed via Traefik)
   - Auto-creates `logs` index on post_startup with:
     - Dynamic field mapping for Vercel log format
     - 7-day retention policy (daily cleanup)
     - Unix timestamp support (milliseconds)

2. **Vector** (`src/images/vector.rs`)
   - HTTP log ingestion endpoint
   - Exposed via Traefik at `vector.{host}`
   - Auth via `Authorization: Bearer <token>` header
   - Token is auto-generated or inherited from boltwall.stakwork_secret if linked
   - Routes:
     - `POST /vercel` - Vercel log drain (NDJSON)
     - `POST /logs` - Generic JSON logs

### Files Created/Modified

- `src/images/quickwit.rs` - New image
- `src/images/vector.rs` - New image
- `src/images/mod.rs` - Added Quickwit/Vector to Image enum
- `src/config.rs` - Added to remove_tokens match
- `src/secondbrain.rs` - Added `only_logs()` function
- `src/defaults.rs` - Added `ONLY_LOGS` env check

## How to Run

```bash
export ONLY_LOGS=true
export RUST_ENV=local  # Important! Otherwise tries to use CloudWatch logging
cargo run --bin stack
```

Look for this line in logs to get the auth token:
```
=> vector auth token: <your_token_here>
```

YOU CAN READ DOCKER LOGS USING `docker logs` for vector.sphinx or quickwit.sphinx

## Testing

### Send test logs

```bash
# Vercel format
curl -X POST http://localhost:9000/vercel \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_token>" \
  --data-binary @test_logs/vercel.ndjson

# Generic format
curl -X POST http://localhost:9000/logs \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your_token>" \
  --data-binary @test_logs/generic.ndjson
```

### Query logs

```bash
curl "http://localhost:7280/api/v1/logs/search?query=*"
```

## Current Issue

Logs are being sent to Vector (auth works, i think) but they're not appearing in Quickwit. The search returns 0 hits.

### Things to investigate:

1. **Vector -> Quickwit connection**: Check if Vector can reach Quickwit at `quickwit.sphinx:7280`
   - Check Vector logs for any HTTP errors to Quickwit
   - Try: `docker exec -it vector.sphinx curl http://quickwit.sphinx:7280/api/v1/indexes`

2. **Quickwit index creation**: Verify the `logs` index was created
   - `curl http://localhost:7280/api/v1/indexes`
   - `curl http://localhost:7280/api/v1/indexes/logs`

3. **Quickwit ingest endpoint**: Verify the ingest URL is correct
   - Currently using: `http://quickwit.sphinx:7280/api/v1/logs/ingest`
   - Might need to be: `http://quickwit.sphinx:7280/api/v1/logs/ingest?commit=force` to see results immediately

4. **VRL transform issues**: The path metadata access might not be working
   - `%http_server.path` might not be the correct way to access request path
   - Check Vector docs for http_server source metadata fields

5. **Timestamp format**: Quickwit expects unix timestamp but test data has old timestamps (2019)
   - Try sending a log with current timestamp

### Quick debug commands

```bash
# Check if Quickwit index exists
curl http://localhost:7280/api/v1/indexes

# Check index mapping
curl http://localhost:7280/api/v1/indexes/logs

# Check Quickwit health
curl http://localhost:7280/health/livez

# View Vector container logs
docker logs vector.sphinx

# View Quickwit container logs  
docker logs quickwit.sphinx

# Test Quickwit ingest directly (bypassing Vector)
curl -X POST "http://localhost:7280/api/v1/logs/ingest" \
  -H "Content-Type: application/json" \
  -d '{"timestamp": 1706745600000, "message": "test", "level": "info"}'
```

## Vector Config (for reference)

The generated `/etc/vector/vector.toml` uses:
- Single `http_server` source with `strict_path = false`
- `remap` transform for auth check and path-based routing
- `http` sink to Quickwit's ingest endpoint

Key VRL logic:
```
auth_header = .Authorization
if is_null(auth_header) || auth_header != expected {
  abort
}
request_path = string!(%http_server.path)
if starts_with(request_path, "/vercel") {
  .log_source = "vercel"
} else if starts_with(request_path, "/logs") {
  .log_source = "generic"
  # ... add defaults
}
```
