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
     brings error messages into view; see [the documented policy](BUILD.md#contact-form-protections).
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

3. **Audio ranges and cache checks are discarded** — [fallback.rs:20](/home/vkav/PROJECTS/rust/leptos-website-maj/src/fallback.rs:20), [serve.rs:158](/home/vkav/PROJECTS/rust/leptos-website-maj/src/serve.rs:158)

   - **What this is:** This serves assets, including requests for only part of an audio file.
   - **Problem:** A request for two audio bytes returned a full-file response of 6.9 MB. Matching cache checks also returned `200`, because headers are removed and the replacement date comparison excludes equality.
   - **Fix:** Pass the original headers to `ServeDir`, preserve its responses except genuine missing-file responses, and delete the custom date handling.
   - **If we skip it:** Seeking and repeat visits transfer unnecessary data.

4. **One invalid show date can crash both show lists** — [shows.rs:146](/home/vkav/PROJECTS/rust/leptos-website-maj/src/routes/shows.rs:146)

   - **What this is:** Events are filtered into past and future shows.
   - **Problem:** JSON accepts missing or invalid dates, but rendering calls `expect()`. A record containing `"date": "31.02.2026"` reaches a panic.
   - **Fix:** Validate dates when loading the file and return a normal error before caching invalid records.
   - **If we skip it:** A content typo can break both show pages.

5. **Release content loading ignores the configured site directory** — [shows.rs:234](/home/vkav/PROJECTS/rust/leptos-website-maj/src/routes/shows.rs:234), [media.rs:72](/home/vkav/PROJECTS/rust/leptos-website-maj/src/routes/media.rs:72), [about.rs:338](/home/vkav/PROJECTS/rust/leptos-website-maj/src/routes/about.rs:338)

   - **What this is:** Three loaders read show data, video data, and gallery filenames.
   - **Problem:** Release builds always read `/app/site`, even when `LEPTOS_SITE_ROOT` points elsewhere. The gallery additionally unwraps the resulting error.
   - **Fix:** Build these paths from `LeptosOptions.site_root`, and display gallery loading errors without panicking.
   - **If we skip it:** Release runs outside the exact Docker directory layout lose these sections.

6. **Gallery and audio controls cannot be operated with a keyboard** — [about.rs:316](/home/vkav/PROJECTS/rust/leptos-website-maj/src/routes/about.rs:316), [player.rs:170](/home/vkav/PROJECTS/rust/leptos-website-maj/src/audio/player.rs:170)

   - **What this is:** Photos, seeking, and volume use custom click targets.
   - **Problem:** Gallery anchors have no `href`; the sliders are plain `div` elements. Browser checks confirmed that gallery thumbnails cannot receive focus.
   - **Fix:** Use labeled buttons for thumbnails and native range inputs for seeking and volume.
   - **If we skip it:** Keyboard users cannot use these controls.

7. **Dialogs leave keyboard focus behind the overlay** — [home.rs:387](/home/vkav/PROJECTS/rust/leptos-website-maj/src/routes/home.rs:387), [about.rs:294](/home/vkav/PROJECTS/rust/leptos-website-maj/src/routes/about.rs:294)

   - **What this is:** Mail, external-link, and photo overlays act as modal dialogs.
   - **Problem:** Focus is neither moved inside nor contained there. Browser checks also confirmed that Escape leaves the mail and photo dialogs open.
   - **Fix:** Use labeled native dialogs opened with `showModal()`, with close handling and focus restoration. See the [W3C dialog keyboard requirements](https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/).
   - **If we skip it:** Keyboard and screen-reader users can navigate hidden background controls.

8. **The Docker bootstrap selects the wrong executable on ARM** — [Dockerfile:5](/home/vkav/PROJECTS/rust/leptos-website-maj/Dockerfile:5)

   - **What this is:** The builder downloads `cargo-binstall`.
   - **Problem:** The download is always for `x86_64`, while this workspace runs on `aarch64`. A native ARM builder cannot execute that binary without architecture emulation.
   - **Fix:** Select the download for the builder’s architecture.
   - **If we skip it:** The documented local Docker build fails on ARM machines.

9. **Missing albums return a success status** — [discography.rs:137](/home/vkav/PROJECTS/rust/leptos-website-maj/src/routes/discography.rs:137)

   - **What this is:** Album URLs select a known release.
   - **Problem:** `/discography/missing-album` displays a missing-album message but returns HTTP `200`; this was reproduced locally.
   - **Fix:** Use the existing not-found error response when the album lookup fails.
   - **If we skip it:** Crawlers and monitoring treat nonexistent album pages as successful pages.

10. **The volume bar contradicts the mute state** — [player.rs:62](/home/vkav/PROJECTS/rust/leptos-website-maj/src/audio/player.rs:62)

    - **What this is:** The audio loop updates the volume indicator.
    - **Problem:** Muting sets the bar to zero, but the next loop overwrites it with `audio.volume()`. The browser showed a 100% bar while `muted` was true.
    - **Fix:** Display zero while muted, and synchronize the indicator on `volumechange`.
    - **If we skip it:** The controls give conflicting feedback about sound output.

**Should fix**

11. **The automated checks do not currently provide a working safety net** — [lib.rs:1](/home/vkav/PROJECTS/rust/leptos-website-maj/src/lib.rs:1), [example.spec.ts:4](/home/vkav/PROJECTS/rust/leptos-website-maj/end2end/tests/example.spec.ts:4)

    - **What this is:** Rust compilation checks and a browser test.
    - **Problem:** `cargo test --locked --offline --features ssr` fails with a recursion-depth error in the library. The only browser test targets port `3000` and starter-template text, while this site uses `8080`.
    - **Fix:** Add the recursion limit to the library crate and replace the starter test with a real site check. Add focused checks for invalid dates and file-response headers.
    - **If we skip it:** These regressions have no useful automated warning.

12. **The gallery downloads full-size originals for thumbnails** — [about.rs:317](/home/vkav/PROJECTS/rust/leptos-website-maj/src/routes/about.rs:317)

    - **What this is:** The About page renders twelve photos.
    - **Problem:** Their originals total 17.8 MB. One thumbnail points to a 6000×4000 image weighing 14.5 MB, and none use lazy loading.
    - **Fix:** Serve smaller thumbnails and load originals when expanded; lazy-load offscreen images.
    - **If we skip it:** Mobile visitors pay a large download cost just to browse the biography.

13. **Docker builds include local build output** — [.dockerignore:7](/home/vkav/PROJECTS/rust/leptos-website-maj/.dockerignore:7), [Dockerfile:22](/home/vkav/PROJECTS/rust/leptos-website-maj/Dockerfile:22)

    - **What this is:** Docker copies the repository into its builder.
    - **Problem:** `.dockerignore` does not exclude `target`, so `COPY . .` includes compiled artifacts. That directory measured about 6 GB during this audit.
    - **Fix:** Add `/target` to `.dockerignore`.
    - **If we skip it:** Local deployments send and copy gigabytes of unnecessary files.

14. **Retired features and unused assets remain in the shipped tree** — [home.rs:109](/home/vkav/PROJECTS/rust/leptos-website-maj/src/routes/home.rs:109), [audio/mod.rs:32](/home/vkav/PROJECTS/rust/leptos-website-maj/src/audio/mod.rs:32), [about.rs:9](/home/vkav/PROJECTS/rust/leptos-website-maj/src/routes/about.rs:9)

    - **What this is:** An inactive album promotion, unused playlist machinery, an unused orientation helper, and obsolete backgrounds.
    - **Problem:** Repository-wide searches found no active consumers for these paths. Three unused background files alone total 61.7 MB.
    - **Fix:** Delete the unused components, bindings, matching styles, and backgrounds. Removing custom cache parsing also removes the direct `lazy-regex` dependency.
    - **If we skip it:** Releases carry unnecessary assets, and maintainers keep reading abandoned code.

**Nice to have**

15. **The audio player polls every 10 milliseconds while paused** — [player.rs:72](/home/vkav/PROJECTS/rust/leptos-website-maj/src/audio/player.rs:72)

    - **What this is:** A timer repeatedly updates playback and volume displays.
    - **Problem:** The browser confirmed a permanent 10 ms interval, including while playback is paused.
    - **Fix:** Use media events for time, duration, playback state, and volume; remove the custom interval helper.
    - **If we skip it:** Every open tab keeps doing unnecessary browser work.

Original audit verification (2026-11-08): Checked all application source, styles, content files, build/deploy configuration, and existing tests. Main routes and read-only APIs responded locally; all 78 show dates parsed. The WebAssembly compilation check passed. Browser checks used the existing build artifacts. No source files changed during the audit, and the audit server was stopped.

Verdict: Contact validation and sending limits are fixed. Fix private-message logging and file serving next; replace the obsolete browser test.  
Lean: -300 lines, -1 dependency possible, plus over 61 MB of unused backgrounds.  
Not checked: Live email delivery, Fly production behavior, a full Docker/release build, dependency advisories, or load testing.
