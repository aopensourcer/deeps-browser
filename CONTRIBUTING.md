# Contributing to Deeps

Thanks for your interest. Bug reports, ideas and pull requests are welcome.

## Before you start

- Search the existing [issues](https://github.com/aopensourcer/deeps-browser/issues): the
  problem may already be reported.
- For large changes (new features, refactors, architecture changes) open an issue first and
  let's agree on the approach. Avoid huge PRs without prior discussion.
- For security issues **do not** open a public issue: read [SECURITY.md](SECURITY.md).

## Setup

Requirements: Windows 10/11, Rust stable (MSVC toolchain), Node 20.9+, WebView2 Runtime.

```powershell
npm --prefix chrome install
npm --prefix chrome run build   # writes chrome/out
cargo run                       # starts the app
```

UI development with hot reload, inside the real app:

```powershell
npm --prefix chrome run dev
$env:DEEPS_DEV_URL = "http://localhost:3000"
cargo run
```

Without Rust, `npm --prefix chrome run dev` opens the UI at `http://localhost:3000` with a
fake bridge (`chrome/lib/ipc.ts`).

## Before opening a PR

```powershell
npm --prefix chrome run typecheck
npm --prefix chrome run build
cargo test
cargo build --release
```

- Match the surrounding style: plain Rust without needless dependencies, TypeScript without
  external state libraries, CSS in the existing stylesheet.
- Do not add telemetry, analytics or undeclared network calls: privacy is part of the
  product.
- If you add user-visible strings, add the key in both Italian and English
  (`chrome/lib/i18n.ts`).
- Update the README when behaviour visible to users changes.
- One PR = one thing. Describe the problem, the solution and how you tested it.

## Licensing of contributions

Deeps is dual-licensed: **AGPL-3.0** for the public and a commercial license for those who
cannot comply with the AGPL (see [LICENSE-COMMERCIAL.md](LICENSE-COMMERCIAL.md)).

By submitting a pull request you confirm that:

1. the contribution is yours, or you have the right to submit it;
2. you release it under AGPL-3.0;
3. you grant the author of Deeps the right to distribute it under a commercial license as
   well, with no additional compensation.

If the contribution contains third-party code, state its origin and license in the PR.
