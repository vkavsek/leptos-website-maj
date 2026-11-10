# Building Instructions

Notes on building for development and publishing to production.

1. You need rust installed, including the WASM target (`rustup target add wasm32-unknown-unknown`)
2. Install:

- mold
- cargo-leptos
- flyctl
- docker

3. Make sure docker is running or `sudo systemctl start docker`

## Local Dev

1. Build the docker image with: `docker build -t maj-web .`
2. Run the docker image with: `docker run -p 8080:8080 maj-web`

## Publishing to Fly.io

1. `fly deploy` to build on a Fly machinge or add `--local-only` to build locally

## Contact form protections

The server trims surrounding whitespace and validates each submission before logging it,
reserving a sending slot, or contacting SMTP:

| Field | Requirement |
| --- | --- |
| Name | Optional; at most 100 characters |
| Email | Required; valid address syntax and at most 254 UTF-8 bytes |
| Subject | Required; 1–200 characters on a single line |
| Message | Required; 1–10,000 characters; line breaks and tabs allowed |

Text lengths use UTF-16 code units to match browser `maxlength` behavior. Single-line
fields reject control characters. Messages reject control characters other than line
breaks and tabs. Email validation checks syntax without DNS lookups or verification mail.

Each server process permits **10 validated send attempts per rolling hour**, shared by
all visitors. Invalid submissions do not use a slot; SMTP failures do. Once full,
the form returns HTTP `429` until an attempt is at least one hour old. Validation
errors return HTTP `400`. The form brings errors into view and retains the entered text.

The allowance lives in memory and resets on restart. Each replica has its own allowance,
and one visitor can use the entire shared allowance. The policy is implemented beside
`send_mail` in `src/routes/home.rs`; no new environment settings or services are needed.
Continue supplying `PORKMAIL_PWD` through the environment.

On the Raspberry Pi, run the focused tests with one build job and the same
component-erasure flag used by the Dockerfile, without SMTP credentials:

```sh
env -u PORKMAIL_PWD RUSTFLAGS="--cfg erase_components" cargo test --locked --offline --lib --features ssr --jobs 1 routes::home::tests
```

The tests cover validation, rolling expiry, simultaneous requests, and rejected HTTP
requests. They do not send email.
