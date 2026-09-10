# @pipefish-recon/dashboard

Intended operator surface for the PipeFish RECON Agentic OS: a Next.js 15
app with React Server Components, verifying SLH-DSA (FIPS 205) signatures
in the browser via LibOQS compiled to WebAssembly so private keys never
leave the Rust kernel.

## Current status

This package is a **placeholder**. `package.json` currently contains only
`name` and `version` (`0.1.0`, `private: true`). There is no `next`
dependency, no `scripts` block, and no app source tree.

Do not expect `npm install` / `npm run dev` to start a UI until that
milestone lands.

Dependabot is configured for `npm` in `/dashboard` (weekly). Those updates
will stay idle until a lockfile and real dependencies exist.

## Related docs

- Kernel boot and constraints: [`../README.md`](../README.md)
- Runtime constitution (design): [`../POLICY.md`](../POLICY.md)
