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
- v0.2.006: the same-phone check reads an index [entries(phone_key)]. Before, every entry read the whole entries
  table, so a big room got slower as it filled. 20,000 entries a minute on a laptop: 3.6 s CPU instead of 28.4 s,
  every answer 200. The collector's /version now carries the release number [0.2.6, was 0.1.0].
- v0.3.000 [the organisers' corrections, 23.10]: their Latvian words and the same in English [question "Kur", lead,
  prize plus today's SIXT suitcase on its own line, story lead and example, contact title, consent, rules, thanks].
  Eight answers in their two-per-row order: "Jauna draudzība" replaces "Jauns kontakts" [code new_contact retired,
  now refused]. A story is 1-500 characters, or the guest ticks "Dodos pauzē meklēt savu negaidītākā kontakta
  stāstu!": no story, no share box, still in the draw [the collector takes an empty story; the organisers' page says
  "Bez stāsta"]. Share image re-rendered from the cover. tests/frontend_rules.rs holds the page's categories and
  limits to the collector's. Open with the operator: rule 1 [organisers + questions address] was not in their list
  and stays; their data rule says "only for the draw and contact" while the optional share box stays.
- v0.3.001: the share image's Latvian line says "Kur" [rendered from the cover version that made it, 0 px off
  before the edit; after it only that line's box changed].
- v0.3.002 [the organisers' corrections, 25.10]: phones show the SIXT logo without scrolling. On a phone the date
  line is gone, the prize partner sits right under the question, then the language buttons, and the words start
  higher over the line: logo, question, SIXT logo and both buttons fit the first view of every phone browser from
  the iPhone mini up [375 x 629 with the toolbars out; the iPhone SE in Safari keeps the SIXT logo in view, the
  buttons one short scroll down]. Computers unchanged pixel for pixel. Every SIXT logo links to
  https://www.sixt.lv/partners/pulse/ in a new tab. Copy: the English lead says "the boldest or most unexpected
  destination"; the prize line names the SIXT Lexus RZ show car in both languages ["show car" in italics in the
  Latvian line, an English term]; the Latvian prizes say "ceļojumu koferis", never "čemodāns". Gate rules added:
  on a phone the prize partner sits between the question and the buttons and its logo is in the first view; five
  phone sizes as the browser shows them.
- v0.3.003 [25.10]: on iPhone Safari [iOS 26] the L-line reaches the screen's bottom edge. Safari paints the page
  under its floating bar down to the edge, but 100lvh stops 58px above it [the same on iPhone SE, 16e, 17 and 17 Pro
  Max in the simulator], so every screen ended there and the strip under the bar was plain colour. On iOS every screen
  now reaches 64px past 100lvh [desktop and Android unchanged], and the thank-you confetti box is no longer fixed [a
  fixed box at the bottom edge makes Safari paint that strip solid]. Scrolled to its very end, any page shows only its
  colour under the bar. New check tests/ios/edge.sh [simulator: the pixel under the line against one beside it]:
  exit 1 on v0.3.002, 0 now; cover, step 2 and thanks checked on the 17 Pro Max, cover and thanks on the SE.
- v0.3.004 [25.10]: on iPhone the quiz's side line reaches the screen's bottom edge also scrolled to the end of a
  step. At the end of a page Safari shows only the page colour under its bar, so on iOS the guest page now scrolls
  inside main, which reaches past that edge; each screen's last buttons keep clear of the bar [--bar-zone]. A keyboard
  can still push the whole page up and Safari leaves it there [seen at 104px]: scroll.js puts it back once no text
  field has the focus. Checked in the simulator with real taps and swipes [idb]: cover, steps 1 and 3 at their end,
  thanks, the push-and-close case; computers and Android unchanged pixel for pixel.
