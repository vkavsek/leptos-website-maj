## AUDIT

What this repo does: A public portfolio for Maj Kavšek, built with Rust, Leptos, and Axum. It serves music, photos, concert dates, and videos, and sends contact emails. I assumed modest public traffic, several simultaneous visitors, and occasional automated form abuse.

**Must fix**

1. **The contact endpoint has no sending limit** — [home.rs:563](/home/vkav/PROJECTS/rust/leptos-website-maj/src/routes/home.rs:563)

   - **What this is:** The public contact form sends email through your account.
   - **Problem:** Direct requests bypass browser validation. Every accepted request attempts delivery, without a sending limit or server-side checks on the fields.
   - **Fix:** Validate required fields, email format, and lengths before sending. Add a server-side sending limit.
   - **If we skip it:** Bots can flood both recipient inboxes and consume the account’s mail allowance.
   - **Status:** Resolved on 2026-11-09. The server trims and validates fields before reserving
     one of 10 send attempts per rolling hour; validation errors return HTTP `400` and
     exhausted allowance returns HTTP `429`. The form retains input on failure and
     brings error messages into view; see [the documented policy](ARCHITECTURE.md#contact-form-protections).
   - **Verified:** Seven Rust tests passed, including concurrent admissions, rolling expiry,
     and real-handler rejection checks. Formatting, Clippy, the server/WASM build, and
     mocked browser checks passed; no SMTP delivery was attempted.
   - **Known limit:** The allowance resets per process on restart, each replica has its own
     allowance, and one visitor can consume all 10 attempts. Invalid inputs do not use
     a slot; SMTP failures do.

2. **Private messages are copied into production logs** — [home.rs:473](/home/vkav/PROJECTS/rust/leptos-website-maj/src/routes/home.rs:473)

   - **What this is:** Contact submissions are logged before delivery.
   - **Problem:** The log includes the sender’s name, address, subject, and complete message—even when sending fails.
   - **Fix:** Log the request ID and delivery result; remove message contents and personal fields. This follows [OWASP logging guidance](https://cheatsheetseries.owasp.org/cheatsheets/Logging_Cheat_Sheet.html).
   - **If we skip it:** Private correspondence also lives in the hosting logs, with separate access and retention.
   - **Status:** Resolved on 2026-11-10. Contact delivery logs now record only
     `delivery=sent` or `delivery=failed` in the existing request ID span. Personal
     fields, message contents, and raw SMTP success/error replies are omitted;
     see [the documented logging policy](ARCHITECTURE.md#contact-form-protections).
   - **Verified:** All seven Rust tests, formatting, Clippy, and a source allowlist check
     passed; the handler's only log events contain static delivery outcomes, and the
     request ID span is retained. No SMTP delivery was attempted.
   - **Known limit:** Previously recorded private data remains subject to the hosting
     provider's log access and retention controls; this code change does not delete it.

3. **Audio ranges and cache checks are discarded** — [fallback.rs:20](/home/vkav/PROJECTS/rust/leptos-website-maj/src/fallback.rs:20), [serve.rs:158](/home/vkav/PROJECTS/rust/leptos-website-maj/src/serve.rs:158)

   - **What this is:** This serves assets, including requests for only part of an audio file.
   - **Problem:** A request for two audio bytes returned a full-file response of 6.9 MB. Matching cache checks also returned `200`, because headers are removed and the replacement date comparison excludes equality.
   - **Fix:** Pass the original headers to `ServeDir`, preserve its responses except genuine missing-file responses, and delete the custom date handling.
   - **If we skip it:** Seeking and repeat visits transfer unnecessary data.
   - **Status:** Resolved on 2026-11-10. Original request headers reach `ServeDir`,
     and only its `404` responses render the app's error page. The custom date
     middleware/parser and direct `lazy-regex` dependency were removed. Existing
     compression and Cache-Control policy remain; see [file-response behavior](ARCHITECTURE.md#static-file-responses).
   - **Verified:** Four regression tests cover two-byte audio ranges, matching and
     older/newer/invalid cache dates, HEAD responses, unsatisfiable ranges, unsupported
     methods, failed preconditions, directory redirects, and the HTML `404` fallback.
     All eleven Rust tests, formatting, and Clippy passed. Production/browser playback
     and a full Docker/release build were not checked.

4. **One invalid show date can crash both show lists** — [shows.rs:146](/home/vkav/PROJECTS/rust/leptos-website-maj/src/routes/shows.rs:146)

   - **What this is:** Events are filtered into past and future shows.
   - **Problem:** JSON accepts missing or invalid dates, but rendering calls `expect()`. A record containing `"date": "31.02.2026"` reaches a panic.
   - **Fix:** Validate dates when loading the file and return a normal error before caching invalid records.
   - **If we skip it:** A content typo can break both show pages.
   - **Status:** Resolved on 2026-11-10. The shared loader validates every show date
     before sorting and caching show records. A missing or invalid date returns the
     existing data error with the record number instead of reaching the list filters.
     See [show date requirements and reload instructions](README.md#content-maintenance).
   - **Verified:** Three new tests cover missing/null/empty/impossible/malformed dates,
     leap days, chronological ordering, empty lists, and the current show file. All
     fourteen Rust tests, formatting, and Clippy passed.
   - **Known limit:** The existing per-process cache also retains load errors. Correct
     the file and restart or redeploy the server to reload it.

5. **Release content loading ignores the configured site directory** — [shows.rs:234](/home/vkav/PROJECTS/rust/leptos-website-maj/src/routes/shows.rs:234), [media.rs:72](/home/vkav/PROJECTS/rust/leptos-website-maj/src/routes/media.rs:72), [about.rs:338](/home/vkav/PROJECTS/rust/leptos-website-maj/src/routes/about.rs:338)

   - **What this is:** Three loaders read show data, video data, and gallery filenames.
   - **Problem:** Release builds always read `/app/site`, even when `LEPTOS_SITE_ROOT` points elsewhere. The gallery additionally unwraps the resulting error.
   - **Fix:** Build these paths from `LeptosOptions.site_root`, and display gallery loading errors without panicking.
   - **If we skip it:** Release runs outside the exact Docker directory layout lose these sections.
   - **Status:** Resolved on 2026-11-11. All three loaders build paths from the
     configured `LeptosOptions.site_root` in both development and release builds.
     Gallery load failures reach the existing error display; the resource no longer
     unwraps them. See [content directory configuration](README.md#runtime-configuration).
   - **Verified:** Five new tests cover relative/absolute roots, missing files, public
     gallery URLs, configuration supplied through the JSON API router, and a rendered
     gallery load error. All nineteen Rust tests, formatting, Clippy, and native
     `hydrate` feature compilation passed.
   - **Known limit:** Content results and errors remain cached per process; restart
     after changing configuration or correcting a failed load.

6. **Gallery and audio controls cannot be operated with a keyboard** — [about.rs:316](/home/vkav/PROJECTS/rust/leptos-website-maj/src/routes/about.rs:316), [player.rs:170](/home/vkav/PROJECTS/rust/leptos-website-maj/src/audio/player.rs:170)

   - **What this is:** Photos, seeking, and volume use custom click targets.
   - **Problem:** Gallery anchors have no `href`; the sliders are plain `div` elements. Browser checks confirmed that gallery thumbnails cannot receive focus.
   - **Fix:** Use labeled buttons for thumbnails and native range inputs for seeking and volume.
   - **If we skip it:** Keyboard users cannot use these controls.
   - **Status:** Resolved on 2026-11-11. Gallery thumbnails are labeled native buttons;
     seeking and volume use labeled range inputs with keyboard controls and visible
     focus indicators. Pixel-based seeking/volume handlers and their unused styles
     were removed. Metadata already loaded before hydration is read when mounting.
   - **Verified:** Keyboard seeking, volume, photo opening/navigation, and pointer
     photo navigation passed in Chromium and Firefox. See [keyboard behavior](README.md#keyboard-controls-and-dialogs).

7. **Dialogs leave keyboard focus behind the overlay** — [home.rs:387](/home/vkav/PROJECTS/rust/leptos-website-maj/src/routes/home.rs:387), [about.rs:294](/home/vkav/PROJECTS/rust/leptos-website-maj/src/routes/about.rs:294)

   - **What this is:** Mail, external-link, and photo overlays act as modal dialogs.
   - **Problem:** Focus is neither moved inside nor contained there. Browser checks also confirmed that Escape leaves the mail and photo dialogs open.
   - **Fix:** Use labeled native dialogs opened with `showModal()`, with close handling and focus restoration. See the [W3C dialog keyboard requirements](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/).
   - **If we skip it:** Keyboard and screen-reader users can navigate hidden background controls.
   - **Status:** Resolved on 2026-11-11. All three overlays are labeled native dialogs
     opened with `showModal()`, with deliberate initial focus, native Escape handling,
     close buttons, and focus restoration. `::backdrop` replaces overlay divs; a
     guarded pointer fallback preserves outside dismissal without closing on padding
     or text-selection drags. Contact failures retain input; success resets and closes.
   - **Verified:** All sixteen Playwright checks passed in Chromium and Firefox,
     including Tab/Shift+Tab, background focus rejection, Escape, explicit close,
     reopening, fallback dismissal, portrait sizing, and mocked email responses.
     Server/WASM builds, all nineteen Rust tests, formatting, and Clippy passed;
     landscape and portrait screenshots were inspected.
   - **Not checked:** WebKit needs system libraries requiring interactive sudo;
     assistive-technology testing, live SMTP, and deployment were not performed.

8. **The Docker bootstrap selects the wrong executable on ARM** — [Dockerfile:5](/home/vkav/PROJECTS/rust/leptos-website-maj/Dockerfile:5)

   - **What this is:** The builder downloads `cargo-binstall`.
   - **Problem:** The download is always for `x86_64`, while this workspace runs on `aarch64`. A native ARM builder cannot execute that binary without architecture emulation.
   - **Fix:** Select the download for the builder’s architecture.
   - **If we skip it:** The documented local Docker build fails on ARM machines.
   - **Status:** Resolved on 2026-11-11. The bootstrap uses the builder's `uname -m`
     in the download URL and a fixed local archive name for extraction.
     See [Docker build notes](BUILD.md#docker-build).
   - **Verified:** POSIX-shell checks passed for x86_64 and aarch64 download URLs,
     extraction, installation, and stopping after a failed download. Both upstream
     release assets were verified through GitHub's release API.
   - **Not checked:** A full Docker build or native ARM execution; Docker socket
     access was denied even outside the agent sandbox.

9. **Missing albums return a success status** — [discography.rs:137](/home/vkav/PROJECTS/rust/leptos-website-maj/src/routes/discography.rs:137)

   - **What this is:** Album URLs select a known release.
   - **Problem:** `/discography/missing-album` displays a missing-album message but returns HTTP `200`; this was reproduced locally.
   - **Fix:** Use the existing not-found error response when the album lookup fails.
   - **If we skip it:** Crawlers and monitoring treat nonexistent album pages as successful pages.
   - **Status:** Resolved on 2026-11-11. Failed album lookups use the shared
     `MajServerError::NotFound` error display, setting HTTP `404` during server rendering.
     See [album URL behavior](ARCHITECTURE.md#album-urls).
   - **Verified:** The HTTP regression test reproduced `200` before the fix and now
     checks two unknown names return `404`, both releases return `200`, and the index
     returns `200`. All twenty Rust tests passed. The HTTP test uses a separate test
     binary to avoid Leptos route discovery suppressing parallel resource tests.
     Formatting, Clippy, and the server/WASM build also passed.

10. **The volume bar contradicts the mute state** — [player.rs:62](/home/vkav/PROJECTS/rust/leptos-website-maj/src/audio/player.rs:62)

    - **What this is:** The audio loop updates the volume indicator.
    - **Problem:** Muting sets the bar to zero, but the next loop overwrites it with `audio.volume()`. The browser showed a 100% bar while `muted` was true.
    - **Fix:** Display zero while muted, and synchronize the indicator on `volumechange`.
    - **If we skip it:** The controls give conflicting feedback about sound output.
    - **Status:** Resolved on 2026-11-11 while replacing the volume control for item 6.
      The native slider displays zero when muted and synchronizes on `volumechange`.
      Browser tests verify mute/unmute preserves and restores the actual volume.

**Should fix**

11. **The automated checks do not currently provide a working safety net** — [lib.rs:1](/home/vkav/PROJECTS/rust/leptos-website-maj/src/lib.rs:1), [example.spec.ts:4](/home/vkav/PROJECTS/rust/leptos-website-maj/end2end/tests/example.spec.ts:4)

    - **What this is:** Rust compilation checks and a browser test.
    - **Problem:** `cargo test --locked --offline --features ssr` fails with a recursion-depth error in the library. The only browser test targets port `3000` and starter-template text, while this site uses `8080`.
    - **Fix:** Add the recursion limit to the library crate and replace the starter test with a real site check. Add focused checks for invalid dates and file-response headers.
    - **If we skip it:** These regressions have no useful automated warning.
    - **Status:** Resolved across items 1, 3, 4, 6, and 7. The library recursion limit,
      date/file-response tests, real homepage check, and keyboard/dialog tests now run.
      Playwright defaults to port 8080, supports a test URL override, and its existing
      dependency lock was updated for current browsers. See [browser checks](BUILD.md#browser-checks).

12. **The gallery downloads full-size originals for thumbnails** — [about.rs:317](/home/vkav/PROJECTS/rust/leptos-website-maj/src/routes/about.rs:317)

    - **What this is:** The About page renders twelve photos.
    - **Problem:** Their originals total 17.8 MB. One thumbnail points to a 6000×4000 image weighing 14.5 MB, and none use lazy loading.
    - **Fix:** Serve smaller thumbnails and load originals when expanded; lazy-load offscreen images.
    - **If we skip it:** Mobile visitors pay a large download cost just to browse the biography.
    - **Status:** Resolved on 2026-11-11. The gallery uses committed 480×270 WebP
      thumbnails (149,330 bytes total versus 17,845,650 bytes of originals). The first
      four load eagerly and later thumbnails load lazily; originals load only after
      opening a photo. [Regeneration instructions](README.md#gallery-thumbnails) keep
      future content updates reproducible without modifying originals.
    - **Verified:** All twelve thumbnails have the expected dimensions and smaller
      payloads. A Rust asset check and Chromium/Firefox checks verify matching files,
      layout proportions, and no original-photo requests before opening the viewer.

13. **Docker builds include local build output** — [.dockerignore:7](/home/vkav/PROJECTS/rust/leptos-website-maj/.dockerignore:7), [Dockerfile:22](/home/vkav/PROJECTS/rust/leptos-website-maj/Dockerfile:22)

    - **What this is:** Docker copies the repository into its builder.
    - **Problem:** `.dockerignore` does not exclude `target`, so `COPY . .` includes compiled artifacts. That directory measured about 6 GB during this audit.
    - **Fix:** Add `/target` to `.dockerignore`.
    - **If we skip it:** Local deployments send and copy gigabytes of unnecessary files.
    - **Status:** Resolved on 2026-11-11. `.dockerignore` now excludes `/target`.
      The root exclusion was checked; a Docker context transfer/full image build
      was not run.

14. **Retired features and unused assets remain in the shipped tree** — [home.rs:109](/home/vkav/PROJECTS/rust/leptos-website-maj/src/routes/home.rs:109), [audio/mod.rs:32](/home/vkav/PROJECTS/rust/leptos-website-maj/src/audio/mod.rs:32), [about.rs:9](/home/vkav/PROJECTS/rust/leptos-website-maj/src/routes/about.rs:9)

    - **What this is:** An inactive album promotion, unused playlist machinery, an unused orientation helper, and obsolete backgrounds.
    - **Problem:** Repository-wide searches found no active consumers for these paths. Three unused background files alone total 61.7 MB.
    - **Fix:** Delete the unused components, bindings, matching styles, and backgrounds. Removing custom cache parsing also removes the direct `lazy-regex` dependency.
    - **If we skip it:** Releases carry unnecessary assets, and maintainers keep reading abandoned code.
    - **Status:** Resolved on 2026-11-11. Removed the retired album promotion and its
      styles/link variants, playlist component/state/metadata/wrapper, orientation
      helper, unused gallery reference, and three obsolete backgrounds (61,731,059
      bytes). The active background, player source, discography artwork, and original
      gallery photos remain. The custom cache parser and `lazy-regex` were removed
      with item 3.
    - **Verified:** Repository searches found no remaining references to the removed
      code or backgrounds. The built site contains only the active background.

**Nice to have**

15. **The audio player polls every 10 milliseconds while paused** — [player.rs:72](/home/vkav/PROJECTS/rust/leptos-website-maj/src/audio/player.rs:72)

    - **What this is:** A timer repeatedly updates playback and volume displays.
    - **Problem:** The browser confirmed a permanent 10 ms interval, including while playback is paused.
    - **Fix:** Use media events for time, duration, playback state, and volume; remove the custom interval helper.
    - **If we skip it:** Every open tab keeps doing unnecessary browser work.
    - **Status:** Resolved on 2026-11-11. Native metadata, duration, time, play/pause,
      ended, error, and volume events synchronize the audio UI. Removed the 10 ms
      loop and interval helper. Initial metadata still synchronizes on mount;
      unknown durations disable seeking and rejected play promises are handled.
    - **Verified:** Chromium and Firefox checks cover seeking via media events,
      programmatic pause, playback completion, non-finite duration recovery,
      rejected playback, and absence of the old polling interval.

Additional changes (2026-11-11): The mail limiter now uses server-only
`parking_lot::Mutex`. Its short critical section never spans async work; concurrent
admission, expiry, HTTP rejection, and panic-unwind tests pass with the existing cap.
Tokio's async `OnceCell` loaders and the test-only standard `Barrier` remain because
they have no appropriate parking_lot replacement. Deprecated `leptos-use` imports
were replaced with `leptos-use-event-listener` and its SSR feature; the retired hover
calls were deleted. Strict Clippy (`-D warnings`) passes without deprecation allowances.

Original audit verification (2026-11-08): Checked all application source, styles, content files, build/deploy configuration, and existing tests. Main routes and read-only APIs responded locally; all 78 show dates parsed. The WebAssembly compilation check passed. Browser checks used the existing build artifacts. No source files changed during the audit, and the audit server was stopped.

Verdict: All fifteen findings are fixed in source. Latest validation: 21 Rust tests,
22 Chromium/Firefox checks, formatting, strict Clippy, server/WASM builds, thumbnail
asset checks, and visual inspection passed.

Lean: Retired code and over 61 MB of unused backgrounds removed; gallery previews now total about 149 KB.

Not checked: Live email delivery, Fly production behavior, a full Docker/release build, dependency advisories, or load testing.
