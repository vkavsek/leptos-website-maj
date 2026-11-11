# Maj Kavšek's portfolio website

A personal website for trumpeter and composer Maj Kavšek, with a biography,
photo gallery, show listings, discography, videos, audio playback, and a contact form.
Built with Rust, Leptos, and Axum.

- [BUILD.md](BUILD.md): dependencies, development and release builds, Docker builds,
  and Rust/browser test instructions.
- [ARCHITECTURE.md](ARCHITECTURE.md): application structure, request handling,
  content loading, and contact form protections.
- [AUDIT.md](AUDIT.md): audit findings and their resolution history.

## Runtime configuration

| Variable | Purpose |
| --- | --- |
| `PORKMAIL_PWD` | SMTP password for contact email; supply through the environment or hosting secrets |
| `LEPTOS_SITE_ROOT` | Directory containing built assets and content; defaults to `target/site` for local builds |
| `LEPTOS_SITE_ADDR` | Server bind address; defaults to `0.0.0.0:8080` |
| `RUST_LOG` | Required logging filter for release binaries; use `info` for normal operation |

Never commit credentials or generated secret configuration. Builds and automated
tests do not need SMTP credentials.

After building locally, run the release server from the repository root:

```sh
env RUST_LOG=info ./target/release/leptos-website-maj
```

Set `PORKMAIL_PWD` in the process environment when real contact delivery is needed.
To serve a different content directory with an existing binary:

```sh
env RUST_LOG=info LEPTOS_SITE_ROOT=/path/to/site ./target/release/leptos-website-maj
```

For server-only development against source content, without rebuilding browser assets:

```sh
env LEPTOS_SITE_ROOT=public RUSTFLAGS="--cfg erase_components" cargo run --features ssr
```

This serves source content; use the complete build in [BUILD.md](BUILD.md#development-build)
when testing browser hydration and compiled styles.

To run the Docker image built by the build guide:

```sh
docker run --rm -p 8080:8080 maj-web
```

For real contact delivery, pass an already-set host variable with
`docker run --rm -p 8080:8080 --env PORKMAIL_PWD maj-web`.
The image sets `RUST_LOG=info`, serves `/app/site`, and listens on port 8080.
`GET /health-check` returns HTTP `200`.

## Deployment

Install the Fly.io CLI (`flyctl`) and authenticate to the configured application
before deploying. From the repository root:

```sh
fly deploy
```

Fly builds remotely by default; use `fly deploy --local-only` to build with local
Docker. The app and service settings are in `fly.toml`; the service forwards traffic
to port 8080 and redirects HTTP to HTTPS. Supply `PORKMAIL_PWD` through Fly secrets,
never through tracked configuration.

## Content maintenance

Edit source assets in `public/` and styles in `style/`. Rebuild to copy source
assets into the default `target/site/` directory. Deploy or restart the server
after content changes: content loaders cache both successes and failures for the
life of the process.

The configured site root must contain:

- `data_json/shows.json`: show records. Every record needs a real calendar date in
  day.month.year format, such as `29.02.2024`. Missing, null, empty, malformed, or
  impossible dates reject the whole file.
- `data_json/yt_videos.json`: video records.
- `img/about_pics/`: original gallery photos.
- `img/about_thumbnails/`: generated WebP thumbnails for the gallery.

Correct a failed content file and restart or redeploy to clear its cached error.
Changing `LEPTOS_SITE_ROOT` also requires a restart.

### Gallery thumbnails

After adding or replacing original photos, install ImageMagick with WebP support
and the `convert` command, then run from the repository root:

```sh
sh scripts/generate-gallery-thumbnails.sh
```

The script applies EXIF orientation, crops to 480×270, and strips thumbnail metadata.
Originals stay unchanged. Commit thumbnails alongside their source photos. When
deleting an original, delete its matching `<original filename>.webp` thumbnail.
A Rust test checks that every original has a smaller WebP thumbnail.

## Keyboard controls and dialogs

Gallery thumbnails are labeled buttons: activate them with Enter or Space. The
photo viewer has named previous, next, and close buttons. Audio seeking and volume
use range inputs with arrow-key, Home, and End support. Seeking stays disabled until
the duration is known; volume and mute controls stay synchronized. The player uses
the Minor Flaw promo.

Contact, external-link confirmation, and photo dialogs move focus inside, make the
background inert, and restore focus when closed. Escape, close buttons, and clicking
outside dismiss them. Clicking dialog padding or dragging a text selection outside
does not dismiss them.

Contact drafts survive dismissal and failed submissions. Errors are brought into
view. A successful send resets the form and closes the dialog. See
[contact form protections](ARCHITECTURE.md#contact-form-protections) for validation
and send limits.

## Logs and privacy

Contact delivery logs contain only `delivery=sent` or `delivery=failed` within the
request span (`req`, with its `id` from `x-request-id`). Use that ID to correlate
delivery results with HTTP request logs. The handler does not log names, email
addresses, subjects, message bodies, or raw SMTP replies.

Earlier private data already present in hosting logs is not removed by code changes.
Manage log access and retention through the hosting provider.
