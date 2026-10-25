// The sizing gate: every screen [cover, each question, thank-you] at many window sizes, loaded fresh and resized live,
// in Chromium and WebKit. Owns the layout rules the page must keep at any size; exits 1 on any fault.
// Invariant: a fault names the engine, size, screen and rule, so a loop round can fix it without guessing.
// Run: PLAYWRIGHT_CORE=<path to playwright-core's index.mjs> node tests/e2e/sizing.mjs <site base url> <screenshot dir>
const { chromium, webkit } = await import(process.env.PLAYWRIGHT_CORE || "playwright-core");
const [base, out] = process.argv.slice(2);
// The last five rows are phones as their browsers show the page with the toolbars out [Playwright's device viewports:
// iPhone Plus/Pro Max, 16 Pro, 15/16, 16e, mini]: the first view a guest really gets, not the full screen.
const SIZES = [[2560, 1440], [1920, 1080], [1536, 864], [1440, 900], [1366, 768], [1280, 800], [1280, 600], [1180, 820],
  [1100, 800], [1024, 768], [960, 640], [900, 1000], [880, 700], [860, 900], [859, 900], [820, 1180], [768, 1024],
  [700, 900], [600, 800], [480, 900], [430, 932], [414, 896], [390, 844], [393, 852], [375, 667], [360, 640], [320, 568],
  [430, 739], [402, 681], [393, 659], [390, 651], [375, 629]];
// SIZING_ONLY="390x844,1280x800" limits a quick run to those sizes [the live resizes are skipped then].
const ONLY = (process.env.SIZING_ONLY || "").split(",").filter(Boolean);
const RESIZES = [[1000, 800], [860, 700], [859, 700], [600, 900], [390, 844], [1280, 800], [1600, 700], [900, 1000], [1237, 767]];
const LV = new Set(["2560x1440", "1920x1080", "1280x800", "860x900", "768x1024", "390x844", "375x667", "320x568"]);
const SHOT = new Set(["1920x1080", "1280x800", "1024x768", "860x900", "768x1024", "390x844", "375x667", "320x568", "390x651"]);

/** In the page: every layout fault of the visible screen, as text. */
function faults(first) {
  const vw = document.documentElement.clientWidth, vh = innerHeight, f = [];
  const phone = vw < 860;
  if (document.documentElement.scrollWidth > vw + 1) f.push(`sideways scroll ${document.documentElement.scrollWidth}>${vw}`);
  const screen = document.querySelector({ flow: "#entry", start: "#start", done: "#done" }[document.body.dataset.screen]);
  const shown = (el) => { const r = el.getBoundingClientRect(); return r.width > 0 && !el.closest("[hidden]") && !el.closest(".hp"); };
  for (const el of screen.querySelectorAll("h1, h2, p, button, label, .lang-pick, .partner, .actions, textarea, input:not([type=checkbox]):not([type=radio])")) {
    if (!shown(el)) continue;
    const r = el.getBoundingClientRect();
    if (r.right > vw + 1 || r.left < -1) f.push(`outside the window: ${el.tagName}#${el.id} ${Math.round(r.left)}..${Math.round(r.right)}`);
    else if (el.scrollWidth > el.clientWidth + 2 && getComputedStyle(el).overflowX !== "visible") f.push(`text spills: ${el.tagName}.${el.className}`);
  }
  // a button's label stays on one line
  for (const b of screen.querySelectorAll(".btn")) {
    if (!shown(b) || !b.textContent.trim()) continue;
    const range = document.createRange(); range.selectNodeContents(b);
    if (new Set([...range.getClientRects()].map((x) => Math.round(x.top))).size > 1) f.push(`button label wraps: "${b.textContent.trim()}"`);
  }
  /** Each word of an element's own text and its inline terms [an italic English word], never a list nested in it, with
      the top of its line [a word over two lines counts as broken]. */
  const words = (el) => {
    const nodes = [...el.childNodes].flatMap((n) => (n.nodeType === 3 ? [n] : n.matches("i") ? [...n.childNodes].filter((c) => c.nodeType === 3) : []));
    const out = [];
    for (const node of nodes) {
      let at = 0;
      for (const word of node.textContent.split(" ")) {
        const range = document.createRange();
        range.setStart(node, at); range.setEnd(node, at + word.length);
        const tops = new Set([...range.getClientRects()].map((r) => Math.round(r.top)));
        if (word) out.push({ word, top: Math.min(...tops), broken: tops.size > 1 });
        at += word.length + 1;
      }
    }
    return out;
  };
  /** The short word left alone on the last line of a text over several lines [a widow such as "DRAW!" or "only."]. */
  const widow = (ws) => {
    const tops = new Set(ws.map((w) => w.top));
    const last = ws.filter((w) => Math.abs(w.top - Math.max(...tops)) < 3);
    return tops.size > 1 && last.length === 1 && last[0].word.length <= 6 ? last[0].word : null;
  };
  // headline words never break inside a word, and a headline never ends with one short word alone
  for (const el of screen.querySelectorAll(".display")) {
    if (!shown(el)) continue;
    const ws = words(el);
    for (const w of ws) if (w.broken) f.push(`word broken across lines: "${w.word}"`);
    const lone = widow(ws);
    if (lone) f.push(`headline ends with a lone word: "${lone}"`);
  }
  // nor does a block of text [lead, each prize line, checkbox label, the break option]
  for (const el of screen.querySelectorAll(".lead, .prize-line, .check > span:last-child, .rules li, .choice--pause .choice-text")) {
    if (!shown(el)) continue;
    const lone = widow(words(el));
    if (lone) f.push(`text ends with a lone word: "${lone}"`);
  }
  // the line spans the screen, top edge to bottom edge
  const svg = screen.querySelector("svg[data-pulse]");
  const d = svg.querySelector(".mk-zz").getAttribute("d");
  const top = Number(d.match(/^M[\d.]+ (-?[\d.]+)/)[1]);
  const end = Number(d.split("V").pop());
  const vb = svg.viewBox.baseVal, box = svg.getBoundingClientRect(), s = screen.getBoundingClientRect();
  const px = (u) => box.top + (u - vb.y) * box.width / vb.width;
  if (px(top) > s.top + 1) f.push(`line starts ${Math.round(px(top))} below the screen's top ${Math.round(s.top)}`);
  if (px(end) < s.bottom - 1) f.push(`line ends ${Math.round(px(end))} above the screen's bottom ${Math.round(s.bottom)}`);
  if (s.bottom < vh - 1) f.push(`screen ${Math.round(s.bottom)} shorter than the window ${vh}`);
  if (screen.id !== "entry") {
    const letters = { top: px(338), bottom: px(591) };
    if (phone && Math.abs(box.left - (vw - box.right)) > 1.5) f.push(`logo off centre ${Math.round(box.left)} | ${Math.round(vw - box.right)}`);
    if (letters.top < -1 || letters.bottom > vh + 1 || box.left < -1 || box.right > vw + 1) f.push("logo letters cut off");
    // nothing may hide the line behind an invisible box [a block in the screen's own colour reads as a break]
    const bg = getComputedStyle(screen).backgroundColor;
    const lineX = [box.left + (360.64 - vb.x) * box.width / vb.width, box.left + (401.01 - vb.x) * box.width / vb.width];
    for (const el of screen.querySelectorAll(".cover-body *, .cover-body")) {
      const cs = getComputedStyle(el), r = el.getBoundingClientRect();
      const hides = cs.backgroundColor === bg || (cs.boxShadow !== "none" && cs.boxShadow.includes(bg));
      if (hides && r.width && r.right > lineX[0] && r.left < lineX[1]) f.push(`an invisible box hides the line: ${el.tagName}.${el.className}`);
    }
    const body = screen.querySelector(".cover-body").getBoundingClientRect();
    if (phone && body.top < letters.bottom - 1) f.push("text overlaps the logo letters");
    if (!phone && body.width < 300) f.push(`text column only ${Math.round(body.width)} px`);
    // on a computer the logo and the words sit centred as one group: the room left of the logo matches the room right
    // of the words [measured on the words themselves, not their column]
    if (!phone) {
      let right = 0;
      for (const el of screen.querySelectorAll(".cover-body > *")) {
        if (!shown(el)) continue;
        const range = document.createRange(); range.selectNodeContents(el);
        right = Math.max(right, range.getBoundingClientRect().right);
      }
      if (Math.abs(box.left - (vw - right)) > vw * 0.1) f.push(`logo and words off centre ${Math.round(box.left)} | ${Math.round(vw - right)}`);
      // the words keep up with the logo on a big window: the headline is at least 27% as tall as the logo's letters
      const head = parseFloat(getComputedStyle(screen.querySelector(".cover-body .display")).fontSize);
      if (head < 0.27 * (letters.bottom - letters.top)) f.push(`words small for the logo: ${Math.round(head)} px to ${Math.round(letters.bottom - letters.top)} px letters`);
    }
    const pick = screen.querySelector(".lang-pick");
    if (pick && shown(pick)) {
      const [a, b] = [...pick.querySelectorAll("button")].map((x) => x.getBoundingClientRect());
      if (Math.abs(a.width - b.width) > 1) f.push(`language buttons unequal ${Math.round(a.width)} vs ${Math.round(b.width)}`);
      if (first && pick.getBoundingClientRect().bottom > vh) f.push("language buttons below the fold");
      // on a phone the prize partner sits right under the question, before the buttons [the organisers' order], and
      // its SIXT logo is in the first view
      if (phone) {
        const sixt = screen.querySelector(".partner img").getBoundingClientRect();
        const echo = screen.querySelector(".display--echo").getBoundingClientRect();
        if (sixt.top < echo.bottom || sixt.bottom > pick.getBoundingClientRect().top) f.push("prize partner not between the question and the buttons");
        if (first && sixt.bottom > vh) f.push("SIXT logo below the fold");
      }
    }
  } else {
    const rail = document.querySelector(".rail").getBoundingClientRect();
    const lineLeft = rail.left + rail.width * 0.5324;
    const step = [...document.querySelectorAll(".step")].find((x) => !x.hidden);
    let right = 0;
    for (const el of step.querySelectorAll("*")) { const r = el.getBoundingClientRect(); if (r.width && r.height) right = Math.max(right, r.right); }
    if (lineLeft - right < 4) f.push(`content touches the line: ${Math.round(lineLeft - right)} px`);
    // a wide window has no empty band between the step and the line [the step splits into question | answers]
    if (vw >= 1200 && lineLeft - right > vw * 0.12) f.push(`empty band before the line: ${Math.round(lineLeft - right)} px`);
    // on a computer the step's type keeps its share of a tall window [headline 7.5%, text 1.6% of the height]
    if (!phone) {
      const h = parseFloat(getComputedStyle(step.querySelector(".display")).fontSize), t = parseFloat(getComputedStyle(step).fontSize);
      if (h < vh * 0.075 || t < vh * 0.016) f.push(`type small for the window: headline ${Math.round(h)} px, text ${Math.round(t)} px at ${vh} px tall`);
    }
  }
  return f;
}

const results = [];
const note = (label, list) => results.push({ label, list });
async function mock(p) {
  const cors = { "access-control-allow-origin": "*", "content-type": "application/json" };
  await p.route("**/pulse/api/status", (r) => r.fulfill({ status: 200, headers: cors, body: '{"ok":true,"open":true,"count":37}' }));
  await p.route("**/pulse/api/entry", (r) => r.fulfill({ status: 200, headers: cors, body: '{"ok":true,"status":"new"}' }));
}
async function walk(p, label, shots, lang = "en") {
  // Screenshots are for the eye: settle every finite animation [intro, draw, confetti] first; the pulse keeps beating.
  const settle = () => p.evaluate(() => { for (const a of document.getAnimations()) if (a.effect && a.effect.getComputedTiming().iterations !== Infinity) a.finish(); });
  const snap = async (name) => { if (shots) { await settle(); await p.waitForTimeout(80); await p.screenshot({ path: `${out}/${label.replace(/ /g, "_")}-${name}.png`, fullPage: true }); } };
  note(`${label} cover`, await p.evaluate(faults, true)); await snap("cover");
  await p.click(`button[data-lang=${lang}]`); await p.waitForTimeout(450);
  note(`${label} step1`, await p.evaluate(faults, false)); await snap("step1");
  await p.click("#cat-idea"); await p.click("#step1 [data-next]"); await p.waitForTimeout(350);
  note(`${label} step2`, await p.evaluate(faults, false));
  await p.fill("#story", "At the coffee break I met a producer from Tallinn, and a month later we ran a festival stage together.");
  await p.click("#step2 [data-next]"); await p.waitForTimeout(350);
  note(`${label} step3`, await p.evaluate(faults, false)); await snap("step3");
  await p.fill("#first", "Anna"); await p.fill("#last", "Bērziņa"); await p.fill("#email", "anna@example.lv"); await p.fill("#phone", "+371 2000 0000");
  await p.click("label.check:has(#consent)"); await p.click("#submit");
  await p.waitForSelector("#done:not([hidden])"); await p.waitForTimeout(350);
  note(`${label} done`, await p.evaluate(faults, false)); await snap("done");
}
for (const [name, type] of [["chromium", chromium], ["webkit", webkit]]) {
  const browser = await type.launch();
  for (const [w, h] of SIZES) {
    if (ONLY.length && !ONLY.includes(`${w}x${h}`)) continue;
    const ctx = await browser.newContext({ viewport: { width: w, height: h } });
    const p = await ctx.newPage();
    await mock(p);
    await p.goto(`${base}/index.html`);
    await p.waitForTimeout(1700);
    try { await walk(p, `${name} ${w}x${h}`, name === "chromium" && SHOT.has(`${w}x${h}`)); } catch (e) { note(`${name} ${w}x${h} walk`, [String(e).replace(/\s+/g, " ").slice(0, 400)]); }
    await ctx.close();
    if (!LV.has(`${w}x${h}`)) continue;
    // the Latvian flow has longer words: walk it too on the sizes where wraps are tightest
    const lv = await browser.newContext({ viewport: { width: w, height: h } });
    const q = await lv.newPage();
    await mock(q);
    await q.goto(`${base}/index.html`);
    await q.waitForTimeout(1700);
    try { await walk(q, `${name} ${w}x${h} lv`, name === "chromium" && SHOT.has(`${w}x${h}`), "lv"); } catch (e) { note(`${name} ${w}x${h} lv walk`, [String(e).replace(/\s+/g, " ").slice(0, 400)]); }
    await lv.close();
  }
  if (ONLY.length) { await browser.close(); continue; }
  const ctx = await browser.newContext({ viewport: { width: 1280, height: 800 } });
  const p = await ctx.newPage();
  await mock(p);
  await p.goto(`${base}/index.html`);
  await p.waitForTimeout(1700);
  for (const [w, h] of RESIZES) {
    await p.setViewportSize({ width: w, height: h }); await p.waitForTimeout(450);
    note(`${name} resized cover ${w}x${h}`, await p.evaluate(faults, false));
  }
  await p.click("button[data-lang=en]");
  for (const [w, h] of RESIZES) {
    await p.setViewportSize({ width: w, height: h }); await p.waitForTimeout(450);
    note(`${name} resized step1 ${w}x${h}`, await p.evaluate(faults, false));
  }
  await ctx.close();
  await browser.close();
}
const bad = results.filter((r) => r.list.length);
for (const r of bad) console.log(`FAULT ${r.label}: ${r.list.join("; ")}`);
console.log(`sizing: ${results.length - bad.length}/${results.length} screens clean`);
process.exit(bad.length ? 1 : 0);
