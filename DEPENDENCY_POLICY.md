# Dependency security decisions

This record contains scoped advisory assessments, accepted maintenance risks,
and temporary source pins. Review dates describe individual decisions, not the
security status of a deployed release. Keep these decisions aligned with
[Cargo.toml](Cargo.toml), [Cargo.lock](Cargo.lock), [deny.toml](deny.toml), and
[.cargo/audit.toml](.cargo/audit.toml). Build/check commands are in
[BUILD.md](BUILD.md#dependency-security-checks); routine update procedures are in
[MAINTENANCE.md](MAINTENANCE.md#dependency-and-toolchain-updates).

## Transitive Chrono maintenance-risk acceptance

Reviewed **2026-11-17**, with review due **2026-12-17** and on every dependency or
advisory change. Application dates use Jiff, but the latest stable `oauth2 5.0.0`
and `openidconnect 4.1.0` still require `chrono 0.4.45` for protocol timestamp
serialization, expiry and claim verification. No compatible stable Chrono-free
upgrade is available. Replacing tested authentication libraries solely to remove
this maintenance dependency would introduce a larger authentication change.

Only the unmaintained advisory `RUSTSEC-2026-0543` is temporarily accepted in
cargo-deny. It reports no known vulnerability; future vulnerability advisories
remain enforced. This is not a claim that Chrono is maintained or that its inputs
are all trusted: it still parses provider timestamp claims. Cargo-audit keeps the
warning visible. Remove the exception when compatible maintained upstream versions
replace Chrono, and rerun authentication/expiry tests and fresh security checks.
Do not extend this decision to other advisories or add new direct Chrono usage.

See [BUILD.md](BUILD.md#dependency-security-checks) for commands and prerequisites.

## Patched SQLite dependency

SQLx's stable 0.9.0 release caps `libsqlite3-sys` below the versions patched for
[RUSTSEC-2026-0459](https://rustsec.org/advisories/RUSTSEC-2026-0459.html).
`Cargo.toml` therefore patches SQLx to the first upstream fix,
[`10462fa4414c6c7f1ed1d39bb31678e3b726878c`](https://github.com/launchbadge/sqlx/commit/10462fa4414c6c7f1ed1d39bb31678e3b726878c)
(SQLx #4502). This permits the locked `libsqlite3-sys` 0.39.1, which bundles SQLite
3.54.1. All SQLx workspace crates come from the same revision, including the
session store's SQLx dependency; there is no custom fork or system-library setup.
The database initialization test queries the actual linked SQLite version and
requires at least 3.54.0, so checking the lockfile alone is insufficient.

`deny.toml` allows only this named Git repository, requires revision-pinned Git
sources, and continues to deny unknown repositories/registries. Review the pin by
**2026-12-17**, on dependency/advisory changes, and before release. Switch back to
a compatible stable SQLx release that permits patched SQLite when available,
remove the Cargo patch and Git allowance, update the lockfile, and rerun security,
database, migration, session, and backup/recovery checks. Do not select a moving branch.
Use the application operator commands for database maintenance; the separately
installed SQLx CLI used to create migration files has its own dependency graph.

## Paste maintenance-risk acceptance

Reviewed **2026-11-17** for `paste` **1.0.15** under
[RUSTSEC-2024-0436](https://rustsec.org/advisories/RUSTSEC-2024-0436.html).
The current stable `leptos` 0.8.23, `either_of` 0.1.9, `reactive_graph` 0.2.16,
`reactive_stores` 0.4.4, and `tachys` 0.2.19 depend on it. There is no patched
`paste` release or compatible stable framework update that removes these uses.

The accepted risk is lack of future maintenance: this advisory reports no known
vulnerability. `paste` is a compile-time procedural macro over trusted Rust source;
public requests/uploads are not supplied to it. Its generated code is exercised
by server/browser checks. This does not protect against a compromised dependency
or make an abandoned crate maintained. Cargo-deny permits only this advisory ID
with a documented reason; **cargo-audit keeps its warning visible**. Other advisory
IDs and source checks remain enforced.

Reassess by **2026-12-17**, whenever dependencies/advisories change, and before
adding new macro usage. Remove the exception when the stable framework replaces
`paste`; do not extend it to a newly reported vulnerability without a fresh
applicability review. This acceptance does not change the upstream maintenance status.

## RSA advisory applicability

Reviewed **2026-11-16** for `openidconnect` **4.1.0** and its `rsa` **0.9.10**
dependency. Both [.cargo/audit.toml](.cargo/audit.toml) and [deny.toml](deny.toml)
except [RUSTSEC-2023-0071](https://rustsec.org/advisories/RUSTSEC-2023-0071.html);
cargo-deny's separate Paste exception is described above.
This private-key timing advisory does not apply to our Google sign-in verification:
the library constructs an `RsaPublicKey` from Google's published modulus/exponent
and verifies an RS256 signature. The application supplies no RSA private key and
does not call RSA signing or decryption. Encrypted ID tokens and non-RS256
signatures are rejected; the OAuth client secret is used only for HTTPS token
exchange. Local fixture tokens are signed separately with AWS-LC.

The reviewed path is `verify_rsa_signature` → `RsaPublicKey::verify` → public
exponentiation and signature-padding verification. Private-key APIs exposed by
the dependency are outside this path. See the
[OpenID Connect implementation](https://github.com/ramosbugs/openidconnect-rs/blob/main/src/core/crypto.rs)
and [RSA verification implementation](https://github.com/RustCrypto/RSA/blob/v0.9.10/src/pkcs1v15.rs).

Review this exception by **2026-12-16**, whenever dependencies or the advisory
change, and before adding any new RSA use. Remove it once upstream adopts patched
`rsa >=0.10`. The configurations suppress this specific advisory ID, so each
dependency change must check for new RSA paths; they do not enforce call-path
reachability. This RSA exception does not suppress the SQLite or maintenance
advisories; their fixes and individual policies are documented above.
