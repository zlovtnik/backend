# FP Playground (Bun + Svelte + TypeScript)

Interactive frontend for functional programming experiments with user-provided data structures and live endpoint calls.

## Stack

- Bun runtime
- Svelte
- TypeScript
- Vite

## Features

- **Data Transformation Studio**
  - Input any JSON
  - Queue transformation actions
  - Reorder/remove actions
  - Play transformations step-by-step with timeline and visual board
- **Backend API Runner**
  - Calls existing endpoints:
    - `POST /api/functional/demo/filter`
    - `POST /api/functional/demo/map`
    - `POST /api/functional/demo/chain`
    - `POST /api/functional/demo/state-transitions`

## Run

```bash
cd ./frontend/fp-playground
bun install
bun run dev
```

Open `http://localhost:5173`.

Set **API Base URL** in the UI if backend is on a different origin (example `http://localhost:8000`).

## Build

```bash
bun run build
bun run preview
```
