# N+1 Comprehensible-Input Learning Platform

A full-stack SaaS scaffold for language/knowledge acquisition using the **N+1
("comprehensible input")** methodology: serve content where ~90% is already
understood and ~10% is new, tracking memory freshness with **FSRS** and
selecting material by comprehension fit and interest.

- **Backend:** Rust (axum) + FSRS freshness + N+1 selection engine + Postgres.
- **Frontend:** React + Vite + TypeScript + Tailwind (flip flashcards,
  GitHub-style activity graph, switchable profiles, live stats).
- **External services:** Anki (AnkiConnect), AI grading/test generation,
  Playwright scraping, YouTube captions, AWS (identity + object store),
  telemetry (Grafana / Sentry / PostHog) — all with real implementations and
  swappable mocks for offline E2E and unit testing.

## Architecture

```
backend/
  core/            # Pure domain logic (no I/O): FSRS + N+1 selection + models
    src/fsrs.rs        FSRS 4.5 freshness scheduler
    src/nplus1.rs      Comprehensible-input selection engine
  api/             # HTTP API, persistence, services
    src/engine.rs      Orchestrates FSRS + N+1 + repo + telemetry
    src/handlers.rs    REST endpoints
    src/repo/          Repository trait + Postgres + in-memory impls
    src/services/      Anki / AI / ingest / AWS / telemetry (real + mock)
    src/seed.rs        3 profiles + Physics 201 & Japanese 103 courses
    tests/e2e.rs       HTTP-level end-to-end tests
    benches/           Retrieval/storage/selection benchmarks
  migrations/      Postgres schema
frontend/          React + Vite + Tailwind app + component/integration tests
telemetry/         Sample Grafana / Sentry / PostHog data
docker-compose.yml Postgres + backend + frontend
```

## The N+1 loop

1. **Baseline** — import Anki decks (`/api/anki/*`) or "test in" (`/api/test-in`)
   to seed a comprehension baseline; interests are captured as tags.
2. **Freshness** — every interaction updates FSRS memory state; freshness =
   retrievability (`backend/core/src/fsrs.rs`).
3. **Selection** — `/api/.../recommend` ranks content by closeness to the 90/10
   ratio and interest overlap (`backend/core/src/nplus1.rs`).
4. **Analytics** — `/api/.../stats` powers the frontend graphs.
5. **Flywheel** — each review emits a weighted success signal to telemetry so
   effective material is served more often.

## Prerequisites

- Rust (stable) — https://rustup.rs
- Node 20+
- Docker Desktop (for the containerized run)

> On Windows PowerShell you may need: `Set-ExecutionPolicy -Scope CurrentUser RemoteSigned`
> to allow `npm` scripts.

## Run — Docker (recommended)

Brings up Postgres + backend + frontend; the frontend is the single entrypoint
and proxies `/api` to the backend.

```bash
docker compose up --build
```

- Frontend: http://localhost:8081
- Backend API: http://localhost:8080/api/health

The stack keeps running until you stop it (`docker compose down`).

## Run — local (no Docker)

Backend (in-memory store + mocks, zero external deps):

```bash
cargo run --bin scaffold-api
# API on http://localhost:8080
```

Frontend:

```bash
cd frontend
npm install
npm run dev        # http://localhost:5173 (proxies /api -> :8080)
```

## Testing

```bash
# Rust unit + integration/E2E tests (uses mocks + in-memory store)
cargo test

# Rust benchmarks (retrieval/storage/selection baselines)
cargo bench

# Frontend component + integration tests
cd frontend && npm test
```

## Configuration

See `.env.example`. Key switches:

- `DATABASE_URL` — omit to use the in-memory store; set to use Postgres.
- `USE_MOCKS=1` — mock all external services (default); set `0` and provide
  credentials to use live Anki / AI / scraping / YouTube.

## Seeded data

- **Profiles:** Aiko (beginner), Ben (intermediate), Chen (advanced) — each with
  tier-appropriate freshness so N+1 selection differs per profile.
- **Courses:** Physics 201 (limits → derivatives → L'Hôpital → kinematics →
  Newton's 2nd law → energy) and Japanese 103 (vocab → て-form → conditionals →
  keigo), each with flashcards, documents, quizzes, and videos of graded
  complexity.

## Roadmap (scaffolded, not yet live)

- Live AI grading/tagging and continuous complexity calibration.
- eBook tracking, real scraped corpora, YouTube caption ingestion pipeline.
- AWS Cognito/S3 for user management and the master data lake.
- Full telemetry pipelines feeding the ranking flywheel.
```
