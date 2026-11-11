# Security

Deeps is a browser: it handles sessions, cookies, history and credentials. Security reports
matter.

## Reporting a vulnerability

**Do not open a public issue.** Use
[GitHub Security Advisories](https://github.com/aopensourcer/deeps-browser/security/advisories/new):
the report stays private between you and the author.

If you cannot use GitHub, contact the author (`@aopensourcer`) through a private address
listed on the GitHub profile.

Please include, where possible:

- Deeps version and Windows/WebView2 version;
- steps to reproduce the problem;
- impact: what an attacker can do, with which privileges;
- a proof of concept, if any, without real third-party data.

## What to expect

- Confirmation of receipt as soon as possible (best effort: the project is maintained by one
  person).
- Updates on progress and, if you want, credit in the release notes.
- Coordinated disclosure: please do not publish details before the fix.

## Scope

In scope: the Rust shell, the IPC bridge, the loopback server behind the internal pages, the
password vault, download handling and the embedding of web content.

Problems in WebView2 or Windows itself are not Deeps vulnerabilities: report them to
Microsoft. If a Deeps configuration makes them exploitable, report it anyway.
