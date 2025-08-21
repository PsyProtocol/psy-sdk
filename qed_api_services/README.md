
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
