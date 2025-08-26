# QED API Services

QED API Services provides REST and WebSocket APIs for the QED blockchain frontend, collecting and serving telemetry data from coordinator and realm nodes.

## Architecture

This service acts as a data aggregation and serving layer for the QED blockchain system:

- **Data Sources**: Coordinator and Realm nodes send telemetry events via HTTP
- **Storage**: PostgreSQL with TimescaleDB for time-series data
- **APIs**: REST endpoints for queries and WebSocket for real-time subscriptions
- **Features**: User registration, event tracking, aggregated statistics

## API Endpoints

### HTTP REST API

#### User Management

**POST `/register`**

Register a new user with Twitter OAuth verification.

```json
{
  "public_key": "0x1234...",
  "twitter_handle": "@username",
  "label": "User Label",
  "signature": "0xabcd..."
}
```

Response:
```json
{
  "success": true,
  "user_id": "uuid"
}
```

**GET `/user_info?public_key={key}`**

Get user information by public key.

Response:
```json
{
  "id": "uuid",
  "public_key": "0x1234...",
  "twitter_handle": "@username",
  "label": "User Label",
  "created_at": "2023-01-01T00:00:00Z",
  "updated_at": "2023-01-01T00:00:00Z"
}
```

#### Event Queries

**GET `/worker_events`**

Query worker events with optional filters:
- `realm_id` - Filter by realm ID
- `status` - Filter by status (PENDING, PROCESSING, COMPLETED, FAILED)
- `public_key` - Filter by worker public key
- `start_time` - Start time (ISO 8601)
- `end_time` - End time (ISO 8601)

**GET `/user_events`**

Query user events with optional filters:
- `user_id` - Filter by user ID
- `tx_type` - Filter by transaction type (REGISTER_USER, DEPLOY_CONTRACT, GUTA)
- `start_time` - Start time (ISO 8601)
- `end_time` - End time (ISO 8601)

#### Aggregated Statistics

**GET `/worker_events_aggregations`**

Get aggregated worker event statistics:
- `start_time` - Start time (optional)
- `end_time` - End time (optional)
- `bucket` - Time bucket (1h, 1d, 1w)

Returns counts by status, average/min/max durations, grouped by time buckets.

**GET `/user_events_aggregations`**

Get aggregated user event statistics:
- `start_time` - Start time (optional)
- `end_time` - End time (optional)
- `bucket` - Time bucket (1h, 1d, 1w)

Returns event counts by transaction type, grouped by time buckets.

**GET `/stats/realms`**

Get overall realm statistics:

**GET `/stats/realms/{realm_id}`**

Get realm statistics for a specific realm.

**GET `/stats/workers/{worker_public_key}`**

Get worker statistics for a specific worker.

### Telemetry API

**POST `/telemetry/events`**

Endpoint for coordinator and realm nodes to send events:

```json
{
  "worker_events": [
    {
      "realm_id": 0,
      "public_key": "0x1234...",
      "status": "COMPLETED",
      "source": "REALM",
      "job_id": {...},
      "checkpoint_id": 123,
      "duration": 1500,
      "metadata": {},
      "timestamp": "2023-01-01T00:00:00Z"
    }
  ],
  "user_events": [
    {
      "user_id": "uuid",
      "public_key": "0x1234...",
      "tx_type": "REGISTER_USER",
      "metadata": {},
      "timestamp": "2023-01-01T00:00:00Z"
    }
  ]
}
```

### WebSocket API

**WS /ws/subscribe**

Real-time event subscription with configurable filters.

Send filter configuration:
```json
{
  "filters": {
    "user_ids": ["uuid1", "uuid2"],
    "realm_ids": ["0", "16384"],
    "event_types": ["worker_event", "user_event"]
  }
}
```

Receive events:
```json
{
  "event_type": "worker_event",
  "data": {...},
  "timestamp": "2023-01-01T00:00:00Z"
}
```

## Configuration

The service uses environment variables for configuration:

```bash
# Database
DATABASE_URL=postgres://postgres:password@localhost:5432/postgres
DATABASE_MAX_CONNECTIONS=10

# Server
SERVER_HOST="0.0.0.0"
SERVER_PORT=3000

# Logging
RUST_LOG=info  # Log level: error, warn, info, debug, trace
```

## Running the Service

### Prerequisites

1. **PostgreSQL with TimescaleDB**:
```bash
docker run -d --name timescaledb \
  -p 5432:5432 \
  -e POSTGRES_PASSWORD=password \
  timescale/timescaledb:latest-pg17
```

2. **Setup Database**:
```bash
export DATABASE_URL="postgres://postgres:password@localhost/postgres"
cargo sqlx database create
cargo sqlx migrate run
```

### Start the Service

```bash
cargo run
```

The service will be available at:
- REST API: `http://localhost:3000`
- WebSocket: `ws://localhost:3000/ws/subscribe`

## Examples

### Register a User

```bash
curl -X POST http://localhost:3000/register \
  -H "Content-Type: application/json" \
  -d '{
    "public_key": "0x1234567890abcdef",
    "twitter_handle": "@alice",
    "label": "Alice",
    "signature": "0xsignature..."
  }'
```

### Query Worker Events

```bash
# Get all events
curl "http://localhost:3000/worker_events"

# Filter by realm and status
curl "http://localhost:3000/worker_events?realm_id=0&status=COMPLETED"

# Filter by time range
curl "http://localhost:3000/worker_events?start_time=2023-01-01T00:00:00Z&end_time=2023-01-02T00:00:00Z"
```

### Get Aggregated Statistics

```bash
# Hourly worker event statistics
curl "http://localhost:3000/worker_events_aggregations?end_time=2023-01-02T00:00:00Z&bucket=1h"

# Daily user event statistics  
curl "http://localhost:3000/user_events_aggregations?end_time=2023-01-08T00:00:00Z&bucket=1d"

# With time range
curl "http://localhost:3000/worker_events_aggregations?start_time=2023-01-01T00:00:00Z&end_time=2023-01-02T00:00:00Z&bucket=1h"
```

### WebSocket Subscription

```javascript
const ws = new WebSocket('ws://localhost:3000/ws/subscribe');

// Configure filters
ws.send(JSON.stringify({
  filters: {
    realm_ids: ["0"],
    event_types: ["worker_event"]
  }
}));

// Receive real-time events
ws.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Received event:', data);
};
```

### Send Telemetry Events

Coordinator and realm nodes can send events:

```bash
curl -X POST http://localhost:3000/telemetry/events \
  -H "Content-Type: application/json" \
  -d '{
    "worker_events": [{
      "realm_id": 0,
      "public_key": "0x1234...",
      "status": "COMPLETED",
      "source": "REALM",
      "job_id": {"task_id": 123, "circuit_type": "AddL1Deposit"},
      "checkpoint_id": 456,
      "duration": 1500,
      "timestamp": "2023-01-01T12:00:00Z"
    }]
  }'
```

## Development

### Database schema validation

```bash
docker run -d --name timescaledb -p 5432:5432 -e POSTGRES_PASSWORD=password timescale/timescaledb:latest-pg17
export DATABASE_URL="postgres://postgres:password@localhost/postgres"
cargo sqlx database drop -y && cargo sqlx database setup
```

### Sqlx static checking

```bash
cargo sqlx prepare --database-url postgres://postgres:password@localhost/postgres
```

Run the command and submit updated files in `.sqlx/` to git.

### Run migrations

```bash
cargo sqlx migrate run --database-url postgres://postgres:password@localhost/postgres
```
