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
- v0.1.002 LIVE: the intro shows the PULSE logo first [the line drawn and the letters up by 0.9 s], then the words,
  then the pulse.
- v0.1.003 LIVE: every screen fits its window. Phone: the logo is centred and as wide as the text column; screens are
  at least 100lvh, so the line reaches the bottom edge whatever the toolbar does. The line spans each screen from the
  top edge to the bottom and refits on any size change [ResizeObserver]. PC: the logo narrows with the window and the
  headline scales with its height. Proof: every screen at 17 window sizes plus live resizing, Chromium and WebKit,
  0 faults [the same check found 26 faulty screens on v0.1.002].
- Sizing loop round 1 [v0.2.001]: gates clean on live; the eye caught a lone short word ending a headline ["DRAW!"
  on phones]. Headlines now balance their lines, paragraphs wrap without orphans; the sizing gate flags a lone short
  last word, walks the Latvian flow too and settles animations before its screenshots.
- v0.2.002 [loop round 2 + a phone report]: the white line no longer breaks [the SIXT logo sits left under its words
  instead of an invisible clear-space box over the line]; the logo is drawn in layers [line, pulse, each letter], so
  letters rise on the GPU and a beat repaints only the pulse: WebKit intro frames over 20 ms 6-8 -> 1; the pulse tail
  has 20 finer steps. Gate rule added: nothing may hide the line behind a box in the screen's own colour.
- v0.2.003 [loop round 3]: wide windows. The cover and thank-you screens centre logo and words as one group [at
  1920 px: 115 | 432 px of room before]; from 1200 px each question step splits into question | answers, so no dark
  band is left between the step and the line [up to 1085 px before]. Gate rules added: logo and words centred on a
  computer; no empty band before the line on a wide window. Phones and tablets unchanged pixel for pixel.
- v0.2.004 [loop round 4]: big windows keep the laptop's proportions. Every size is in rem and the guest page's
  root size follows the window from 1440 x 900 up [1.2x at 1920 x 1080, 1.6x at 2560 x 1440]; the organisers' page
  keeps its size; every size up to 1440 x 900 is unchanged pixel for pixel. Gate rules added: the cover headline at
  least 27% of the logo letters' height; on a computer the step headline at least 7.5% and its text 1.6% of the window
  height; the Latvian walk also at 1920 x 1080 and 2560 x 1440.
- v0.2.005 [loop round 5]: wraps. The answer button keeps its label on one line on a 320 px phone ["Enter the draw",
  "Piedalīties izlozē" broke in two]; the prize note, checkbox labels and rules wrap like the lead, never one short
  word alone on the last line ["only.", "money.", "again.", "[Rīgas robežās]." kept together]. Gate rules added:
  a button label on one line; no lone short last word in a text block. A walk error now prints 400 characters.
