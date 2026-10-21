# PULSE Forum 2026: where did a new contact take you?

A story draw for the networking break at PULSE Forum 2026 [28-29 October, Hanzas perons, Riga]. Guests scan a
QR code, pick a language, a category and tell their story, then leave their contact details. One story wins a
SIXT ride to the forum on the morning of 29 October [within Riga].

- Guest page: https://skill-issue-hq-dev.github.io/pulse-forum/
- Organisers' page: `admin.html`, opened with a private link that carries the key.

## How it works

- `frontend/` is the whole site: plain HTML, CSS and JavaScript, published by GitHub Pages on every push to
  `main` (`.github/workflows/pages.yml`). It talks to one address only, `frontend/config.js`.
- `src/` is the collector: one small Rust service that stores the entries in SQLite, answers the organisers'
  page and runs the draw with the operating system's randomness. Contract: `docs/api.md`.

## Run it locally

```sh
cargo test
PULSE_ADMIN_KEY=local-dev-key-only-0123456789abcdef PULSE_DATA=data cargo run -- serve
```

## Credits

PULSE brand: colours, logo and type rules from the PULSE brand book. Fonts: Barlow Condensed and Saira Extra
Condensed, SIL Open Font License [`frontend/fonts/`]. SIXT logo as supplied for this activation.

Made by Skill Issue HQ.
