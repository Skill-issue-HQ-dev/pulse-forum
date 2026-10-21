// The sizing gate: every screen [cover, each question, thank-you] at many window sizes, loaded fresh and resized live,
// in Chromium and WebKit. Owns the layout rules the page must keep at any size; exits 1 on any fault.
// Invariant: a fault names the engine, size, screen and rule, so a loop round can fix it without guessing.
// Run: PLAYWRIGHT_CORE=<path to playwright-core's index.mjs> node tests/e2e/sizing.mjs <site base url> <screenshot dir>
const { chromium, webkit } = await import(process.env.PLAYWRIGHT_CORE || "playwright-core");
const [base, out] = process.argv.slice(2);
const SIZES = [[2560, 1440], [1920, 1080], [1536, 864], [1440, 900], [1366, 768], [1280, 800], [1280, 600], [1180, 820],
  [1100, 800], [1024, 768], [960, 640], [900, 1000], [880, 700], [860, 900], [859, 900], [820, 1180], [768, 1024],
  [700, 900], [600, 800], [480, 900], [430, 932], [414, 896], [390, 844], [393, 852], [375, 667], [360, 640], [320, 568]];
const RESIZES = [[1000, 800], [860, 700], [859, 700], [600, 900], [390, 844], [1280, 800], [1600, 700], [900, 1000], [1237, 767]];
const SHOT = new Set(["1920x1080", "1280x800", "1024x768", "860x900", "768x1024", "390x844", "375x667", "320x568"]);

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
  // headline words never break inside a word
  for (const el of screen.querySelectorAll(".display")) {
    if (!shown(el)) continue;
    const node = el.firstChild;
    if (!node || node.nodeType !== 3) continue;
    const text = node.textContent;
    let at = 0;
    for (const word of text.split(" ")) {
      const range = document.createRange();
      range.setStart(node, at); range.setEnd(node, at + word.length);
      const lines = new Set([...range.getClientRects()].map((r) => Math.round(r.top)));
      if (word && lines.size > 1) f.push(`word broken across lines: "${word}"`);
      at += word.length + 1;
    }
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
    const body = screen.querySelector(".cover-body").getBoundingClientRect();
    if (phone && body.top < letters.bottom - 1) f.push("text overlaps the logo letters");
    if (!phone && body.width < 300) f.push(`text column only ${Math.round(body.width)} px`);
    const pick = screen.querySelector(".lang-pick");
    if (pick && shown(pick)) {
      const [a, b] = [...pick.querySelectorAll("button")].map((x) => x.getBoundingClientRect());
      if (Math.abs(a.width - b.width) > 1) f.push(`language buttons unequal ${Math.round(a.width)} vs ${Math.round(b.width)}`);
      if (first && pick.getBoundingClientRect().bottom > vh) f.push("language buttons below the fold");
    }
  } else {
    const rail = document.querySelector(".rail").getBoundingClientRect();
    const lineLeft = rail.left + rail.width * 0.5324;
    const step = [...document.querySelectorAll(".step")].find((x) => !x.hidden);
    let right = 0;
    for (const el of step.querySelectorAll("*")) { const r = el.getBoundingClientRect(); if (r.width && r.height) right = Math.max(right, r.right); }
    if (lineLeft - right < 4) f.push(`content touches the line: ${Math.round(lineLeft - right)} px`);
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
async function walk(p, label, shots) {
  // Screenshots are for the eye: settle every finite animation [intro, draw, confetti] first; the pulse keeps beating.
  const settle = () => p.evaluate(() => { for (const a of document.getAnimations()) if (a.effect && a.effect.getComputedTiming().iterations !== Infinity) a.finish(); });
  const snap = async (name) => { if (shots) { await settle(); await p.waitForTimeout(80); await p.screenshot({ path: `${out}/${label.replace(/ /g, "_")}-${name}.png`, fullPage: true }); } };
  note(`${label} cover`, await p.evaluate(faults, true)); await snap("cover");
  await p.click("button[data-lang=en]"); await p.waitForTimeout(450);
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
    const ctx = await browser.newContext({ viewport: { width: w, height: h } });
    const p = await ctx.newPage();
    await mock(p);
    await p.goto(`${base}/index.html`);
    await p.waitForTimeout(1700);
    try { await walk(p, `${name} ${w}x${h}`, name === "chromium" && SHOT.has(`${w}x${h}`)); } catch (e) { note(`${name} ${w}x${h} walk`, [String(e).slice(0, 160)]); }
    await ctx.close();
  }
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
