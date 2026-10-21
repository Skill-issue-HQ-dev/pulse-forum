# Ledger

## 21.10.2026: v0.1.000 built

- Brief: the organisers' activation text [LV/EN choice, category, story, contacts, QR, draw, winner driven to the
  forum on 29.10 within Riga]; brand book + SIXT logos supplied; page copies pulseforum.co's look.
- Hosting decided: page on GitHub Pages under Skill Issue HQ; collector on the existing server at
  dbautomatizacijas.com/pulse, separate from every other project; no Cloudflare Worker, no Access login.
- To confirm with the organisers [the page ships with these defaults]: who runs the draw and owns the data
  [rules text says "PULSE Forum 2026 organizatori kopā ar SIXT", info@pulseforum.co]; close and draw time
  [default: close after the break, draw 28.10 evening]; the prize wording; Komu New C webfont files if licensed;
  the LV and EN copy.
- Open: delete all entries by 30.11.2026.

## 21.10.2026: v0.1.000 LIVE

- Page https://skill-issue-hq-dev.github.io/pulse-forum/ [Pages workflow on main]; collector behind
  dbautomatizacijas.com/pulse/api, deployed through the private gateway [server in sync]; tag v0.1.000.
- Live proof: health, version, CORS only for the Pages origin, 401 without the key; one test entry sent from the
  real page, listed, drawn after closing, then cleared and reopened: 0 entries, 0 draws. Other projects' stores
  counted before and after: unchanged.
- Independent review [Opus] fixed in v0.1.000: same phone = one chance [later entry starts excluded, dup_of]; a draw
  needs entries closed; delete one entry on request [never a winner]; a resend after closing hears retry; draws keep
  their pool ids; page fields locked while sending; phone Back steps through the form; honeypot not rendered; CSP
  connect-src = the API path only; the pulse stops after a few beats.
- Logo: the L-line's step follows pulseforum.co's own hero art [line 2 as thick as line 1, starting above the foot];
  the letters and line 1 are the brand book's.
- Sent to nobody from here: the organisers' mail with both links is kept with the event files, forwarded by hand.
- Still to confirm with the organisers: data controller wording, close and draw time, prize wording, copy, custom
  link [extra cost] and then the QR for the chosen link [the QR for the GitHub link is ready].
- After the event: delete every entry by 30.11.2026 [admin page: close, "Dzēst visus datus"]; then retire the admin
  key and the collector's route [steps in the private gateway's notes].

## 21.10.2026: v0.1.001 LIVE

- Phone cover: the logo's lower line runs on through the text to the bottom of the screen; the language buttons part
  exactly at the line; the SIXT logo keeps clear space where the line passes [picked from four layouts: through the
  text, cut by the text, down the side, two-tone].
- pulse.js stretches every mark's lower line and its pulse to the bottom of its screen [cover, thank-you, the
  question rail on every step], refit on step changes and resize; the rail's CSS extension is gone.
