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

The builder selects the x86_64 or aarch64 `cargo-binstall` archive using its own
architecture (`uname -m`). Both downloads use the local filename `cargo-binstall.tgz`.
For `--platform` builds, configure a native builder or emulation for the requested
platform.

The Docker context excludes `/target`, so local Rust output and staged site files
are rebuilt inside the builder instead of being copied into it. Source assets remain
in `public/`.

## Publishing to Fly.io

1. `fly deploy` to build on a Fly machinge or add `--local-only` to build locally

## Contact form protections

The server trims surrounding whitespace and validates each submission before reserving
a sending slot or contacting SMTP:

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

The limiter uses a server-only `parking_lot::Mutex` for its short counter update;
no lock is held during SMTP or other async work. This mutex does not poison on panic:
the lock is released and recorded attempts remain in effect. The loaders retain
Tokio's async `OnceCell` because initialization awaits file I/O. The concurrent test
retains `std::sync::Barrier`, which has no `parking_lot` equivalent.

Contact delivery logs contain only `delivery=sent` or `delivery=failed` within the
existing request span (`req`, with its `id` from `x-request-id`). Names, email addresses,
subjects, message bodies, and raw SMTP replies are not logged by the contact handler.
Use the request ID to correlate delivery results with HTTP request logs. This change
does not remove private data already present in hosting logs; manage those logs using
the hosting provider's access and retention controls.

On Linux, run the focused tests with the same component-erasure flag used by the
Dockerfile, without SMTP credentials:

```sh
env -u PORKMAIL_PWD RUSTFLAGS="--cfg erase_components" cargo test --locked --offline --lib --features ssr routes::home::tests
```

The tests cover validation, rolling expiry, simultaneous requests, and rejected HTTP
requests. They do not send email.

## Static file responses

Static requests pass their original headers to `ServeDir`, which handles byte ranges
and cache preconditions. Audio range requests return `206` with the requested bytes;
an `If-Modified-Since` value matching `Last-Modified` returns an empty `304` response.
Only missing files render the app's HTML `404` page. Other file-service responses,
including redirects, failed preconditions, and unsatisfiable ranges, pass through.

The existing cache policy remains: development sends
`no-store, no-cache, must-revalidate, proxy-revalidate`; release sends
`max-age=3600, must-revalidate`.

Run the file-response regression tests on Linux with:

```sh
env -u PORKMAIL_PWD RUSTFLAGS="--cfg erase_components" cargo test --locked --offline --lib --features ssr serve::tests
```

## Show dates

Every record in `public/data_json/shows.json` must include a real calendar date in
day.month.year format, for example `29.02.2024`. Missing, null, empty, malformed, or
impossible dates reject the entire file before sorting or caching any show records.
Both show lists use the existing error display when loading fails.

The loader caches its result, including errors, for the life of the server process.
After correcting the content file, restart or redeploy the server to reload it.

Run the date-validation regression tests on Linux with:

```sh
env -u PORKMAIL_PWD RUSTFLAGS="--cfg erase_components" cargo test --locked --offline --lib --features ssr routes::shows::tests
```

## Content directory

Shows, videos, and gallery filenames load from the same `LeptosOptions.site_root`
used for static files, in both development and release builds. The configured root
must contain `data_json/shows.json`, `data_json/yt_videos.json`, and `img/about_pics/`.
Gallery URLs remain `/img/about_pics/<filename>`.

The project defaults to `target/site`, populated by `cargo leptos watch` or
`cargo leptos build`. To run the server directly against source assets on Linux:

```sh
env LEPTOS_SITE_ROOT=public RUSTFLAGS="--cfg erase_components" cargo run --features ssr
```

For a built server, use `LEPTOS_SITE_ROOT` to select a different site directory:

```sh
env LEPTOS_SITE_ROOT=/path/to/site ./target/release/leptos-website-maj
```

Content results, including load errors, are cached per server process. Restart after
changing the root or correcting a failed load. Gallery loading failures use the
existing error display instead of panicking.

## Gallery thumbnails

The gallery displays 480×270 WebP thumbnails from `public/img/about_thumbnails/`.
The first four load eagerly; the remaining thumbnails use native lazy loading.
Original photos in `public/img/about_pics/` are requested only when the viewer opens.
The twelve thumbnails total about 149 KB, compared with 17.8 MB of originals.

After adding or replacing originals, generate thumbnails from the repository root
with ImageMagick (`convert` with WebP support):

```sh
sh scripts/generate-gallery-thumbnails.sh
```

The script applies EXIF orientation, crops thumbnails to the gallery's 16:9 frame,
and strips thumbnail metadata. Originals stay unchanged. Commit the generated
thumbnails alongside content changes; when deleting an original, also delete its
matching `<original filename>.webp` thumbnail. A Rust test checks that every original
has a smaller WebP thumbnail.

## Album URLs

Known album URLs (`/discography/minorflaw` and `/discography/galaterna`) return HTTP
`200`. Unknown album names return HTTP `404` with the shared not-found error display
and a link to the home page. The discography index remains available at `/discography`.

Run the HTTP regression check with:

```sh
env -u PORKMAIL_PWD RUSTFLAGS="--cfg erase_components" cargo test --locked --offline --features ssr --test album_routes
```

## Keyboard controls and dialogs

Gallery thumbnails are labeled buttons. Activate them with Enter or Space; the photo
viewer has named previous, next, and close buttons. Audio seeking and volume use native
range inputs with arrow-key, Home, and End support. Seeking stays disabled until the
audio duration is known. Volume changes and mute/unmute keep the slider synchronized.

Audio time, duration, playback state, and volume follow native media events rather
than a polling timer. Metadata already loaded before hydration is read on mount;
unknown/non-finite durations disable seeking, and rejected play requests leave the
control paused. The active player uses the Minor Flaw promo; retired playlist state
and promotion code have been removed. Dialog listeners now use the
`leptos-use-event-listener` micro crate with its SSR feature, replacing deprecated
umbrella-crate imports.

Contact, external-link confirmation, and photo viewers use native modal dialogs.
Opening moves focus inside, background controls become inert, and Escape or the close
button dismisses the dialog and restores focus. Clicking outside also dismisses it,
with a fallback for browsers without `closedby` support. Clicking dialog padding or
dragging a text selection outside does not dismiss it.

Contact drafts survive dismissal and failed submissions. A successful send resets the
form and closes it. Browser tests mock the email responses and never send mail.

## Browser checks

Browser tests require Node.js 20 or newer.

Playwright uses port 8080 by default and does not start a server. Reuse a development
server or build and start an isolated one on another port, for example on Linux:

```sh
env RUSTFLAGS="--cfg erase_components" cargo leptos build
env -u PORKMAIL_PWD LEPTOS_SITE_ADDR=127.0.0.1:8082 ./target/debug/leptos-website-maj
```

In a separate terminal, from `end2end/`:

```sh
npm ci
npx playwright install chromium firefox
env PLAYWRIGHT_BASE_URL=http://127.0.0.1:8082 npx playwright test --project=chromium --project=firefox
```

The WebKit project is also configured; install its browser and required system
libraries to include it. Tests cover the real homepage, audio controls, dialog focus,
Escape and button dismissal, photo navigation, portrait sizing, and mocked email
success/failure. The dismissal fallback is tested with native light dismiss disabled.
Additional checks verify thumbnail-only initial gallery requests, loading the
original after opening, media-event synchronization, rejected playback, and the
absence of the old 10 ms polling interval.
