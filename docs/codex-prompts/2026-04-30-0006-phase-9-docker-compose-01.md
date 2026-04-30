# Phase 9 task 12 — Docker compose for multi-container deployment

**Date:** 2026-04-30
**Slug:** `phase-9-docker-compose`
**Round:** 01 (initial dispatch)
**Subagent:** `codex:codex-rescue`

## Motivation

Provide a Docker-based deployment alternative to the `install`
subcommand. Minimal Alpine images with the statically-linked
musl binaries. Docker compose orchestrates the three services
+ MySQL.

## References

- `ROADMAP.md` §Phase 9 — task 12.
- `HUMANS.md` §Integration — "Optional docker compose support.
  Minimal Alpine images (we don't run `install` commands there).
  Local override files supply HTTPS certs locations, hostnames."
- `scripts/musl-build.sh` — builds static musl binaries.

## Scope

### In scope

#### 1. `Dockerfile` at workspace root

Multi-stage build:
- **Builder stage**: `rust:1.88-alpine` (or just copy pre-built
  musl binaries from `target-main/x86_64-unknown-linux-musl/release/`).
  Since `musl-build.sh` produces the binaries, the simplest
  Dockerfile just copies them into a minimal Alpine image.
- **Runtime stage**: `alpine:3.21` (or latest). Copy the three
  binaries. No build tools in the final image.

Actually, since the binaries are statically linked, we can use
`scratch` or `alpine` (Alpine for shell debugging). Use
separate Dockerfiles or a single multi-target Dockerfile.

**Simplest approach**: one Dockerfile per binary, each just
copies the pre-built binary.

```dockerfile
# Dockerfile.mechanics-worker
FROM alpine:3.21
COPY target-main/x86_64-unknown-linux-musl/release/mechanics-worker /usr/local/bin/
ENTRYPOINT ["mechanics-worker"]
CMD ["serve"]
```

Or a single `Dockerfile` with build args:

```dockerfile
FROM alpine:3.21
ARG BINARY
COPY target-main/x86_64-unknown-linux-musl/release/${BINARY} /usr/local/bin/${BINARY}
ENV BINARY=${BINARY}
ENTRYPOINT ["/bin/sh", "-c", "exec /usr/local/bin/${BINARY} \"$@\"", "--"]
CMD ["serve"]
```

#### 2. `docker-compose.yml` at workspace root

```yaml
services:
  mysql:
    image: mysql:8
    environment:
      MYSQL_ROOT_PASSWORD: philharmonic
      MYSQL_DATABASE: philharmonic
    volumes:
      - mysql-data:/var/lib/mysql
    healthcheck:
      test: ["CMD", "mysqladmin", "ping", "-h", "localhost"]
      interval: 5s
      timeout: 3s
      retries: 10

  mechanics-worker:
    build:
      context: .
      dockerfile: Dockerfile
      args:
        BINARY: mechanics-worker
    environment:
      LISTEN_ADDR: "0.0.0.0:3001"
    ports:
      - "3001:3001"

  connector:
    build:
      context: .
      dockerfile: Dockerfile
      args:
        BINARY: philharmonic-connector
    ports:
      - "3002:3002"
    volumes:
      - ./deploy/connector.toml:/etc/philharmonic/connector.toml:ro

  api:
    build:
      context: .
      dockerfile: Dockerfile
      args:
        BINARY: philharmonic-api
    ports:
      - "3000:3000"
      - "443:443"
    depends_on:
      mysql:
        condition: service_healthy
    volumes:
      - ./deploy/api.toml:/etc/philharmonic/api.toml:ro

volumes:
  mysql-data:
```

#### 3. `docker-compose.override.yml.example`

Example override for HTTPS certs, hostnames, etc.

#### 4. `deploy/` directory with example configs

- `deploy/api.toml` — example API config pointing at the
  `mysql` service.
- `deploy/connector.toml` — example connector config.
- `deploy/mechanics.toml` — example mechanics config.

#### 5. `.dockerignore`

Prevent copying the entire workspace into the build context.

### Out of scope

- Building inside Docker (we pre-build with `musl-build.sh`).
- Helm charts or Kubernetes manifests.
- CI/CD pipeline integration.

## Outcome

Completed. Created 7 files: `Dockerfile` (Alpine 3.21 +
BINARY arg), `docker-compose.yml` (4 services: mysql,
mechanics-worker, connector, api), `.dockerignore`,
`docker-compose.override.yml.example` (HTTPS certs),
`deploy/{api,connector,mechanics}.toml` example configs.
`docker compose config` validates cleanly.

Committed as parent `(pending)`. Did NOT run
`docker compose up` (no musl release binaries built yet).

---

## Prompt (verbatim)

<task>
You are working inside the `philharmonic-workspace` directory.

## IMPORTANT: Do NOT commit or push

**Do NOT run `./scripts/commit-all.sh`.** Do NOT run any git
commit command. Do NOT run `./scripts/push-all.sh`. Leave the
working tree dirty.

## What to build

Create Docker deployment infrastructure at the workspace root.
The three bin targets are pre-built as statically-linked musl
binaries via `./scripts/musl-build.sh --release`. Docker just
copies them into minimal Alpine images.

**Read these files first:**
- `HUMANS.md` §Integration — Docker compose requirements.
- `scripts/musl-build.sh` — how binaries are built.
- `philharmonic/src/bin/mechanics_worker/config.rs`
- `philharmonic/src/bin/philharmonic_connector/config.rs`
- `philharmonic/src/bin/philharmonic_api/config.rs`
- `CONTRIBUTING.md`

### Files to create

1. **`Dockerfile`** at workspace root — single Dockerfile
   with a `BINARY` build arg:
   ```dockerfile
   FROM alpine:3.21
   RUN apk add --no-cache ca-certificates
   ARG BINARY
   COPY target-main/x86_64-unknown-linux-musl/release/${BINARY} /usr/local/bin/${BINARY}
   RUN chmod +x /usr/local/bin/${BINARY}
   # Use shell form so $BINARY is expanded
   ENTRYPOINT ["/bin/sh", "-c", "exec /usr/local/bin/$BINARY \"$@\"", "--"]
   CMD ["serve"]
   ```

2. **`docker-compose.yml`** — orchestrates four services:
   - `mysql`: MySQL 8, healthcheck, persistent volume.
   - `mechanics-worker`: the JS executor, port 3001.
   - `connector`: the connector service, port 3002.
   - `api`: the API server, ports 3000 + 443, depends on mysql
     healthy. Mounts config from `deploy/api.toml`.

   Each service builds from the same Dockerfile with
   different `BINARY` arg. Mount config files from `deploy/`.

   Use `bind = "0.0.0.0:PORT"` in configs so the containers
   listen on all interfaces (not 127.0.0.1).

3. **`docker-compose.override.yml.example`** — example with
   HTTPS volume mounts for certs.

4. **`deploy/api.toml`** — example config:
   ```toml
   bind = "0.0.0.0:3000"
   database_url = "mysql://root:philharmonic@mysql/philharmonic"
   issuer = "philharmonic"
   ```

5. **`deploy/connector.toml`** — example config:
   ```toml
   bind = "0.0.0.0:3002"
   realm_id = "default"
   ```

6. **`deploy/mechanics.toml`** — example config:
   ```toml
   bind = "0.0.0.0:3001"
   tokens = ["change-me-in-production"]
   ```

7. **`.dockerignore`** at workspace root — exclude everything
   except the binary output and deploy configs:
   ```
   *
   !target-main/x86_64-unknown-linux-musl/release/mechanics-worker
   !target-main/x86_64-unknown-linux-musl/release/philharmonic-connector
   !target-main/x86_64-unknown-linux-musl/release/philharmonic-api
   !deploy/
   !Dockerfile
   ```

### Notes

- The binaries must be built BEFORE `docker compose build`.
  The README/comments should mention running
  `./scripts/musl-build.sh --release` first.
- The `api` service depends on `mysql` being healthy.
- The `connector` and `mechanics-worker` don't need MySQL.
- Config files in `deploy/` are examples — users copy and
  customize. Do NOT put secrets in committed config files
  (the example tokens are clearly marked as change-me).

## Rules

- **Do NOT commit, push, or publish.**
- Create files only at the workspace root level (not inside
  any submodule).
- Do NOT modify any Rust source code.
</task>

<structured_output_contract>
When you are done, produce a summary listing:
1. Files created.
2. Confirmation that you did NOT commit or push.
</structured_output_contract>

<completeness_contract>
All files listed above must be created with real content.
`docker compose config` should parse without errors (if
docker compose is available). No TODOs.
</completeness_contract>

<verification_loop>
If docker compose is available:
1. `docker compose config` — verify YAML parses.
Otherwise just verify files exist.
</verification_loop>

<action_safety>
- Do NOT commit. Do NOT push.
- Do NOT modify files inside any submodule.
- Do NOT modify Rust source code.
</action_safety>
