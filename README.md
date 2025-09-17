# Rust CRUD API

A minimal Rust HTTP server that implements CRUD operations for a `User` resource backed by PostgreSQL. It uses the `postgres` crate and a simple `TcpListener` to handle requests.

## Quickstart

### Run with Docker Compose (recommended)
```bash
docker compose up --build
```
- App: http://localhost:8080
- DB: localhost:5432 (user: postgres, password: postgres, db: postgres)

### Run locally (without Docker)
Ensure you have PostgreSQL running and accessible. Then set `DATABASE_URL` and start the app.

- Windows Command Prompt (cmd.exe):
```bat
set DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres && cargo run --release
```
- PowerShell:
```powershell
$env:DATABASE_URL="postgres://postgres:postgres@localhost:5432/postgres"; cargo run --release
```
- Git Bash:
```bash
DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres cargo run --release
```

## Environment
- `DATABASE_URL` (required): e.g. `postgres://postgres:postgres@db:5432/postgres` in Docker, or `...@localhost:5432/postgres` locally.
- With Docker Compose this is already set for the `rustapp` service.

## API
Base URL: `http://localhost:8080`

- Create user
  - `POST /users`
  - Body:
```json
{"name":"Ada","email":"ada@example.com"}
```
  - Example:
```bash
curl -s -X POST http://localhost:8080/users \
  -H "Content-Type: application/json" \
  -d '{"name":"Ada","email":"ada@example.com"}'
```

- Get user by id
  - `GET /users/{id}`
```bash
curl -s http://localhost:8080/users/1
```

- Get all users
  - `GET /users`
```bash
curl -s http://localhost:8080/users
```

- Update user by id
  - `PUT /users/{id}`
  - Body:
```json
{"name":"Ada Lovelace","email":"ada.l@example.com"}
```
  - Example:
```bash
curl -s -X PUT http://localhost:8080/users/1 \
  -H "Content-Type: application/json" \
  -d '{"name":"Ada Lovelace","email":"ada.l@example.com"}'
```

- Delete user by id
  - `DELETE /users/{id}`
```bash
curl -s -X DELETE http://localhost:8080/users/1
```

## Implementation Notes
- Server listens on `0.0.0.0:8080`.
- On startup, it creates the `users` table if it does not exist.
- `User` has fields: `id (SERIAL, PK)`, `name (TEXT)`, `email (TEXT UNIQUE)`.

## Troubleshooting
- Panic: `DATABASE_URL must be set`
  - Ensure `DATABASE_URL` is exported in your shell or configured in Docker Compose.
- Cannot connect to database
  - Wait for the `db` service to be healthy, or verify Postgres is reachable (`psql` or `pg_isready`).
- Port already in use
  - Change the app port mapping in `docker-compose.yml` if `8080` is busy.

## License
MIT
