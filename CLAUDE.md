# pulse-forum

The PULSE Forum 2026 story draw [28-29.10.2026, Riga]: guests scan a QR, tell where a new contact took them, and
one story wins a SIXT ride to the forum. Skill Issue HQ client work. Public repo
`Skill-issue-HQ-dev/pulse-forum`: no personal names, keys, IPs or home paths in it; every push is the Skill Issue
HQ Publisher app (credential helper `hq-publisher credential`), author "Skill Issue HQ".

## Map

Layout: backend = `src/` (Rust, Cargo rule: cli `src/main.rs` -> `src/web/` -> `src/core/` | `src/io/`) . frontend = `frontend/` (the whole site, GitHub Pages) . deploy = `.github/workflows/pages.yml` (the server side lives in the private gateway repo, key `pulse`) . data = `data/` (gitignored, local runs) . docs = `docs/` (`api.md` = the contract, design) . tasks = `tasks/`.

- Page: https://skill-issue-hq-dev.github.io/pulse-forum/ (QR target) . organisers: `admin.html#k=<key>`.
- Collector: `https://dbautomatizacijas.com/pulse/api`, its own service behind the operator's gateway; the route, limits and access rules live in the private gateway repo [key `pulse`], not here.
- `frontend/index.html` screens . `strings.js` every word LV + EN, categories, rules . `form.js` the field rules (same numbers as `src/config.rs`) + the token . `app.js` the flow, the draft and the outbox (resends with the same token until the collector answers) . `config.js` THE one address . `admin.html`/`admin.js`/`admin.css` the organisers' page (key from the link fragment into sessionStorage, guest text only via textContent) . `styles.css` tokens + layout (PULSE brand: coal, lime, white, orange; the logo's L-line rail) . `motion.css` every animation (the L-line draws in one stroke) . `pulse.js` the orange comet along the L-line, a few beats per screen . `fonts/` Barlow Condensed + Saira Extra Condensed (OFL; Saira stands in for the brand's Komu New C) . the PULSE mark is inline in index.html three times (letters cut from the brand book, the L-line's step as on pulseforum.co's own art) . `img/` icons, share image, SIXT logos as supplied.
- `src/config.rs` every constant . `core/entry.rs` input + rules in the contract's order . `core/labels.rs` categories + languages . `core/draw.rs` uniform pick (rejection sampling) . `core/auth.rs` bearer check, constant time . `core/record.rs` stored shapes . `core/riga.rs` Riga time . `core/workbook.rs` + `core/xlsx.rs` the Excel (hand-written SpreadsheetML) . `io/store.rs` SQLite (entries, draws, settings) . `io/random.rs` /dev/urandom . `io/clock.rs` . `web/mod.rs` router + layers . `web/api.rs` status + entry . `web/admin.rs` the seven admin routes . `web/cors.rs` . `web/reply.rs` envelopes.
- Data flow: phone -> `POST /pulse/api/entry` (text/plain JSON) -> gateway [rate limit sized for about 400 guests behind one venue address] -> `web::api` -> `core::entry` -> `io::store` (SQLite in the service's own state directory) -> `web::admin` -> admin page / Excel / draw.
- Invariants: kept apart from every other project: own service, own database, own admin key (`PULSE_ADMIN_KEY`, never in git); the site calls only `config.js`'s API (`tests/frontend_contract.rs`); first entry per e-mail wins; the same phone is one chance [a later entry starts excluded, `dup_of`]; a draw runs only once entries are closed, and its pick and its row are one transaction; logs never carry personal data.
- Run: `cargo test` . local `PULSE_ADMIN_KEY=<32+ chars> PULSE_DATA=data cargo run -- serve` . page preview `cd frontend && python3 -m http.server`.
- Deploy: page = merge to `main` (push = Pages) . collector = the private gateway's deploy target `pulse` (server paths, unit, key file and Access app are in the gateway's Map and the home index, never here).
- QR: generated once for the page URL and decoded back (`QR AUDIT GREEN`); it lives with the event files, not in git.
- Delete the entries by 30.11.2026 (the privacy text promises it): admin page, close, "Dzēst visus datus".
- Symptom -> file: guests see the no-connection line -> the gateway's `/pulse/` route, then the collector's journal; admin says the key is wrong -> the server's key file vs the link; a 429 burst at the venue -> the gateway's `rate_limits`; wrong words -> `frontend/strings.js`; a field refused on the server but not the page -> `src/config.rs` vs `frontend/form.js`.
