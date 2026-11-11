# Audit

What this repo does: A public portfolio built with Leptos and Axum, serving photos, music, videos, show dates, and contact email. I assumed modest public traffic, several simultaneous visitors, and occasional automated abuse. I treated the documented per-process mail limit and content caching as intentional decisions.

## **Must fix**

All four items below are resolved. The original findings are retained for reference.

1. **ZIP imports can publish files outside the public directory** — [update_json.sh:23](/home/vkav/Work/RUST/leptos-website-maj/scripts/update_json.sh:23).
   - **What this is:** The maintenance script extracts a ZIP and copies everything into publicly served content.
   - **Problem:** A ZIP containing a symbolic link—a file pointing elsewhere—exposed a dummy private file over HTTP in my test. Malformed JSON also replaced valid content while the script reported success.
   - **Fix:** Accept only expected, regular JSON files; reject links and validate their contents before replacing existing files.
   - **Resolution:** Imports now allow only top-level `shows.json` and `yt_videos.json`, reject links and unexpected or duplicate entries, validate record fields and show dates, and stage the entire archive before replacing files. Python regression checks cover safe and unsafe imports.
   - **If we skip it:** Importing an unsafe or incorrect archive can expose readable files or break content pages. This requires an operator to import the archive; there is no public upload endpoint.

2. **External tabs can redirect the original tab** — [home.rs:112](/home/vkav/Work/RUST/leptos-website-maj/src/routes/home.rs:112).
   - **What this is:** The external-link dialog opens another website with `window.open()`.
   - **Problem:** The new page retains `window.opener`. A controlled external-page fixture successfully navigated the original portfolio tab.
   - **Fix:** Open with `_blank` and the `noopener` feature. [MDN documentation](https://developer.mozilla.org/en-US/docs/Web/API/Window/open#noopener)
   - **Resolution:** The shared link dialog uses `_blank` with `noopener`. A browser regression checks that the opened page has no `window.opener`.
   - **If we skip it:** A hostile external page can replace the original tab with another destination.

3. **Video loading errors leave Media blank** — [media.rs:19](/home/vkav/Work/RUST/leptos-website-maj/src/routes/media.rs:19).
   - **What this is:** Media renders the result of its video loader inside `Suspense`.
   - **Problem:** It has no `ErrorBoundary`. With malformed video JSON, the page displayed only “Media,” without videos or an error message.
   - **Fix:** Wrap the result in the existing `ErrorBoundary`/`ErrorTemplate` pattern used by Shows and About.
   - **Resolution:** Media uses the shared error boundary and template. Rust and browser regressions cover missing content and invalid video responses.
   - **If we skip it:** Missing or invalid video content silently removes the page’s main content.

4. **The show list becomes unreadable at high zoom** — [\_shows.scss:9](/home/vkav/Work/RUST/leptos-website-maj/style/_shows.scss:9).
   - **What this is:** Shows divides a fixed-height page between its heading, navigation, and scrolling event list.
   - **Problem:** At a 320×180 CSS viewport, equivalent to 400% zoom on 1280×720, the event list gets only about six pixels of visible height. Scrolling cannot reveal a readable row.
   - **Fix:** Allow the Shows page to scroll when its heading and controls no longer fit, rather than clipping the list into the remaining strip.
   - **Resolution:** Shows allows page overflow to scroll and gives its content a minimum height in rem units. Browser checks cover first and last event cells at small viewports and doubled text size.
   - **If we skip it:** Visitors using high magnification cannot read the concert listings.

## **Should fix**

All six items below are resolved.

1. **Content error pages still return HTTP `200`** — [shows.rs:117](/home/vkav/Work/RUST/leptos-website-maj/src/routes/shows.rs:117), [error.rs:39](/home/vkav/Work/RUST/leptos-website-maj/src/routes/error.rs:39).
   - **What this is:** `ErrorTemplate` sets an error status after asynchronous content loading.
   - **Problem:** The response has already started streaming. With malformed show data, `/shows/past` displayed “500 Internal Server Error” but returned `200`; its JSON endpoint correctly returned `500`.
   - **Fix:** Use blocking resources for content whose failure determines the page status, and test the actual HTTP status. [Leptos rendering guidance](https://book.leptos.dev/ssr/23_ssr_modes.html#blocking-resources)
   - **Resolution:** Shows, Media, and the gallery use blocking resources. HTTP regressions check `500` for malformed shows/videos and missing gallery content, and `200` for valid content pages.
   - **If we skip it:** Monitoring and crawlers can treat broken pages as successful responses.

2. **The gallery treats every directory entry as a usable image URL** — [about.rs:329](/home/vkav/Work/RUST/leptos-website-maj/src/routes/about.rs:329).
   - **What this is:** The loader lists directory entries and places their names directly into URLs.
   - **Problem:** A text file and subdirectory appeared as gallery entries. A valid `photo#1.jpg` failed because the browser interpreted `#1.jpg` as a URL fragment; the encoded URL worked.
   - **Fix:** Filter supported regular image files and encode filenames as URL path segments.
   - **Resolution:** The gallery accepts supported regular image files, ignores links and non-images, and percent-encodes UTF-8 filename segments. Tests cover directories, text files, links, uppercase extensions, spaces, Unicode, `#`, `?`, and `%`.
   - **If we skip it:** Ordinary content updates can create broken images and unusable gallery buttons.

3. **Similar filenames can trap gallery navigation** — [about.rs:196](/home/vkav/Work/RUST/leptos-website-maj/src/routes/about.rs:196).
   - **What this is:** Next and Previous find the current image using substring matching.
   - **Problem:** For a valid list ordered as `portrait.jpg.webp`, then `portrait.jpg`, Next from the second image selects that same image again.
   - **Fix:** Use exact equality in both searches.
   - **Resolution:** Both directions compare complete URLs. A browser regression covers colliding filenames and wraparound in both directions.
   - **If we skip it:** Some filename combinations make navigation repeat or skip photos.

4. **Two icon controls have incorrect or missing names** — [head.rs:65](/home/vkav/Work/RUST/leptos-website-maj/src/head.rs:65), [about.rs:160](/home/vkav/Work/RUST/leptos-website-maj/src/routes/about.rs:160).
   - **What this is:** These are the Discography navigation icon and biography text-color button.
   - **Problem:** The accessibility tree exposes two links named “Shows” and none named “Discography.” The color-change control is an unnamed button.
   - **Fix:** Correct the navigation image’s alternative text and label the color-change button; mark its decorative arrow with `alt=""`.
   - **Resolution:** Discography has the correct alternative text. The biography color button has an accessible name and its arrow has empty alternative text. Browser checks verify the names and keyboard activation.
   - **If we skip it:** Screen-reader and voice-control users cannot reliably identify these actions.

5. **Missing SMTP configuration becomes an empty password** — [home.rs:448](/home/vkav/Work/RUST/leptos-website-maj/src/routes/home.rs:448).
   - **What this is:** Password loading uses `unwrap_or_default()` after reserving a mail attempt.
   - **Problem:** An absent password becomes an empty credential and reaches SMTP. Each submission consumes an allowance slot, while logs provide only a generic delivery failure.
   - **Fix:** Reject absent or empty credentials before reserving an attempt, and log a static configuration error without the credential value.
   - **Resolution:** Missing or empty credentials produce HTTP `500` and a static configuration log before reserving an attempt or contacting SMTP. The handler regression checks both cases without sending email.
   - **If we skip it:** A missing deployment secret causes repeated delivery failures and misleading rate-limit errors.

6. **Deployment shutdown interrupts active requests immediately** — [serve.rs:88](/home/vkav/Work/RUST/leptos-website-maj/src/serve.rs:88), [fly.toml:8](/home/vkav/Work/RUST/leptos-website-maj/fly.toml:8).
   - **What this is:** Fly sends `SIGINT`, but the server has no graceful shutdown handler.
   - **Problem:** A temporary server exited in roughly one millisecond during an active image transfer, truncating it. An in-progress SMTP request can also be interrupted.
   - **Fix:** Handle the configured signal with Axum’s graceful shutdown and allow bounded in-flight work to finish within the deployment timeout. [Axum documentation](https://docs.rs/axum/latest/axum/serve/struct.Serve.html#method.with_graceful_shutdown)
   - **Resolution:** SIGINT and SIGTERM stop new connections and drain active requests for up to 25 seconds. SMTP delivery is bounded to 20 seconds, and Fly allows 30 seconds for shutdown. Native process tests cover complete downloads under both signals and a stalled connection reaching the drain deadline.
   - **If we skip it:** Deployments can interrupt downloads and contact submissions.

## **Nice to have**

All three items below are resolved.

1. **Background framing does not follow browser history** — [head.rs:16](/home/vkav/Work/RUST/leptos-website-maj/src/head.rs:16).
   - **What this is:** Navigation click handlers manually change the background position.
   - **Problem:** About → Media → browser Back returns to About while retaining Media’s background position. Reloading produces another position.
   - **Fix:** Derive the background position from the current route and remove the individual click handlers.
   - **Resolution:** The shared page layout derives its background from the router location. The five navigation click handlers and background node reference are removed. Browser checks cover Back/Forward, reloads, and nested URLs.
   - **If we skip it:** The same page looks different depending on how visitors reach it.

2. **Unused event mutators include an incorrect getter** — [shows.rs:68](/home/vkav/Work/RUST/leptos-website-maj/src/routes/shows.rs:68).
   - **What this is:** Two public methods expose mutable event lists.
   - **Problem:** Neither has callers in the repository, and `get_desc_mut()` incorrectly returns the ascending list.
   - **Fix:** Delete both unused mutators.
   - **Resolution:** Both mutators are removed; no callers needed changes.
   - **If we skip it:** Maintainers retain an unnecessary API containing misleading behavior.

3. **Retired music adds 47.7 MB to deployments** — [public/music](/home/vkav/Work/RUST/leptos-website-maj/public/music).
   - **What this is:** Every file under `public/` is copied into the built site and Docker image.
   - **Problem:** Four tracks have no application, content, or test references. Together they occupy 47,656,089 bytes; only the Minor Flaw promo and `audio-promo_small.mp3` test fixture are referenced.
   - **Fix:** Remove those four tracks from the shipped directory, or archive them outside `public/`.
   - **Resolution:** The four unused tracks are removed from `public/music`, reducing shipped assets by 47,656,089 bytes. The active Minor Flaw promo and audio regression fixture remain. The removed tracks are recoverable from Git history.
   - **If we skip it:** Builds and deployment images continue carrying unnecessary audio.

Original audit validation passed: **21 Rust tests, 50 Chromium/Firefox checks, formatting, strict SSR/hydration Clippy, and the server/WASM build**. The lockfile advisory check returned only an informational notice for transitive `paste` being unmaintained. [RustSec notice](https://rustsec.org/advisories/RUSTSEC-2024-0436.html)

The original audit also checked Modern Web Guidance’s icon and accessibility guides. It changed no repository files, and all temporary audit servers were stopped.

Must fix resolution validation passed: **22 Rust tests, 3 Python import tests,
56 Chromium/Firefox checks, formatting, strict SSR/hydration Clippy, and the
server/WASM build**. Import checks also accepted the current source JSON files in
an isolated temporary directory. No email was sent.

Remaining recommendation validation passed: **23 Rust tests, 3 Python import
tests, 2 native shutdown tests, and 62 Chromium/Firefox checks**, plus the
server/WASM build, formatting, and strict SSR/hydration Clippy. The browser run
passed 60 checks, then the two history checks passed after their expectations were
corrected to accept the browser's CSS serialization of zero. Native shutdown tests
verified full 16 MB downloads under SIGINT/SIGTERM and the 25-second drain limit.
Generated site assets also contain only the active promo and audio test fixture.
No email was sent; Fly deployment and live SMTP were not exercised.

**Verdict:** All recommendations in this audit are resolved.
**Lean:** Removed the six unused mutator lines, five background click handlers, and 47.7 MB of shipped audio.
**Not checked:** Live SMTP, Fly/Docker deployment, WebKit, physical screen-reader testing, or Git history.
