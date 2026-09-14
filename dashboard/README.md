# @pipefish-recon/dashboard

Operator surface for the PipeFish RECON Agentic OS. Next.js 15 App Router
talks to the kernel operator API (`GET /health`, `/api/status`, `/api/events`,
`/api/policy`). Private keys never leave the Rust kernel; this UI only reads
public JSON.

## Run

In one terminal:

```bash
cargo run --manifest-path kernel/Cargo.toml
```

In another:

```bash
cd dashboard
npm install
npm run dev
```

Open `http://localhost:3000`. Override the kernel URL with
`NEXT_PUBLIC_KERNEL_URL` (default `http://127.0.0.1:8080`).

Pages: `/` status, `/events` bus stream, `/policy` ACL matrix, `/agents` SOUL
roles plus online flags.

LibOQS WASM signature verification is **not** in this package (see
[`ROADMAP.md`](../ROADMAP.md) v0.6).
