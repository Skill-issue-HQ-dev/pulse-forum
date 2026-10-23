// The flow gate: the guest flow at phone and desktop sizes in Chromium and WebKit with the collector mocked: language,
// steps, validation, the break option, sending, thanks, confetti, the pulse, reduced motion, the phone's Back button.
// Invariant: exits 1 on any failed check. Run: PLAYWRIGHT_CORE=<playwright-core index.mjs> node tests/e2e/flow.mjs <base url> <shots dir>
const { chromium, webkit } = await import(process.env.PLAYWRIGHT_CORE || "playwright-core");
const [base, shots] = process.argv.slice(2);
const API = "https://dbautomatizacijas.com/pulse/api";
const SIZES = [[360, 640], [375, 667], [390, 844], [430, 932], [1280, 800]];
const results = [];
const check = (name, ok, detail = "") => { results.push({ name, ok, detail }); };

async function mock(page, { open = true, count = 37, entry = { status: 200, body: { ok: true, status: "new" } } } = {}) {
  const cors = { "access-control-allow-origin": "*", "content-type": "application/json" };
  await page.route(`${API}/status`, (r) => r.fulfill({ status: 200, headers: cors, body: JSON.stringify({ ok: true, open, count }) }));
  await page.route(`${API}/entry`, async (r) => {
    await new Promise((x) => setTimeout(x, 350));
    r.fulfill({ status: entry.status, headers: cors, body: JSON.stringify(entry.body) });
  });
}

async function noOverflow(page, label) {
  const w = await page.evaluate(() => [document.documentElement.scrollWidth, window.innerWidth]);
  check(`${label}: no sideways scroll`, w[0] <= w[1], `${w[0]} vs ${w[1]}`);
}

async function railClear(page, label) {
  const gap = await page.evaluate(() => {
    const rail = document.querySelector(".rail").getBoundingClientRect();
    const step = [...document.querySelectorAll(".step")].find((s) => !s.hidden);
    let right = 0;
    for (const el of step.querySelectorAll("*")) {
      const r = el.getBoundingClientRect();
      if (r.width && r.height) right = Math.max(right, r.right);
    }
    return Math.round(rail.left + rail.width * 0.5324 - right);
  });
  check(`${label}: content clear of the rail line`, gap >= 4, `${gap}px`);
}

async function flow(engine, browser, [w, h]) {
  const label = `${engine} ${w}x${h}`;
  const page = await browser.newPage({ viewport: { width: w, height: h }, deviceScaleFactor: 2, isMobile: engine === "chromium" && w < 800, hasTouch: w < 800 });
  const errors = [];
  let shooting = false; // Playwright's own WebKit screenshot style is refused by the page's CSP: not a page error
  const shot = async (opts) => { shooting = true; await page.screenshot({ caret: "initial", ...opts }); await page.waitForTimeout(50); shooting = false; };
  page.on("console", (m) => { if (m.type() === "error" && !shooting) errors.push(m.text().slice(0, 160)); });
  page.on("pageerror", (e) => errors.push(String(e).slice(0, 160)));
  await mock(page);
  await page.goto(`${base}/index.html`);
  await page.waitForTimeout(2600);
  await shot({ path: `${shots}/${engine}-${w}-start.png` });
  await noOverflow(page, `${label} start`);
  check(`${label}: live count shows 37`, (await page.textContent("#live-num")).trim() === "37");
  check(`${label}: English is the first language`, (await page.evaluate(() => document.documentElement.lang)) === "en" && (await page.getAttribute(".lang-pick button:first-child", "data-lang")) === "en");
  check(`${label}: pulse on the cover`, (await page.$$("#start .mk-comet")).length === 4);
  check(`${label}: bar colour lime on start`, (await page.getAttribute("#theme-color", "content")) === "#CBDC3E");
  await page.click("button[data-lang=lv]");
  await page.click("#step1 [data-next]");
  check(`${label}: missing category shakes`, await page.evaluate(() => document.querySelector("#choices").classList.contains("shake")));
  await page.waitForTimeout(450);
  await shot({ path: `${shots}/${engine}-${w}-step1.png`, fullPage: true });
  await noOverflow(page, `${label} step1`);
  await railClear(page, `${label} step1`);
  check(`${label}: pulse on the rail`, (await page.$$(".rail .mk-comet")).length === 4);
  check(`${label}: bar colour coal in the flow`, (await page.getAttribute("#theme-color", "content")) === "#2B2D2C");
  if (w >= 860) {
    // on a computer the eight answers sit two per row, in the organisers' order
    const grid = await page.evaluate(() => [...document.querySelectorAll("#choices .choice")].map((c) => [Math.round(c.getBoundingClientRect().top), c.textContent]));
    const rows = [0, 2, 4, 6].map((i) => grid[i][0] === grid[i + 1][0] && grid[i][0] < (grid[i + 2] || [Infinity])[0]);
    check(`${label}: answers two per row, organisers' order`, rows.every(Boolean) && grid[7][1] === "Cits, vēl labāks stāsts!", grid.map((g) => g[1]).join(" | "));
  }
  await page.click("#cat-idea");
  await page.click("#step1 [data-next]");
  await page.fill("#story", "Kafijas pauzē iepazinos ar producenti no Tallinas, un pēc mēneša mēs jau kopā vadījām festivāla skatuvi.");
  check(`${label}: counter turns ok`, (await page.getAttribute("#story-count", "data-state")) === "ok");
  await page.waitForTimeout(350);
  await shot({ path: `${shots}/${engine}-${w}-step2.png`, fullPage: true });
  await railClear(page, `${label} step2`);
  await page.click("#step2 [data-next]");
  await page.fill("#first", "Anna");
  await page.press("#first", "Enter");
  check(`${label}: Enter moves to the next field`, await page.evaluate(() => document.activeElement.id === "last"));
  await page.fill("#last", "Bērziņa");
  await page.fill("#email", "anna@example.lv");
  await page.fill("#phone", "+371 2000 0000");
  await page.click("label.check:has(#consent)");
  await page.waitForTimeout(350);
  await shot({ path: `${shots}/${engine}-${w}-step3.png`, fullPage: true });
  await noOverflow(page, `${label} step3`);
  await railClear(page, `${label} step3`);
  await page.click("#submit");
  check(`${label}: sending bar shows`, await page.evaluate(() => document.querySelector("#submit").classList.contains("is-sending")));
  check(`${label}: fields locked while sending`, await page.evaluate(() => document.querySelector("#first").disabled && document.querySelector("#story").disabled && document.querySelector("#step3 [data-back]").disabled));
  await page.waitForSelector("#done:not([hidden])", { timeout: 5000 });
  await page.waitForTimeout(700);
  check(`${label}: confetti falls`, (await page.$$(".cf")).length > 20);
  await shot({ path: `${shots}/${engine}-${w}-done.png` });
  await noOverflow(page, `${label} done`);
  check(`${label}: no console errors`, errors.length === 0, errors.join(" | "));
  await page.close();
}

async function edgeCases(browser) {
  const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
  await mock(page, { open: false });
  await page.goto(`${base}/index.html`);
  await page.waitForSelector("#done:not([hidden])", { timeout: 5000 });
  check("closed status shows the closed screen", /closed|slēgta/i.test(await page.textContent("#done-title")));
  await page.close();
  const p2 = await browser.newPage({ viewport: { width: 390, height: 844 } });
  await mock(p2, { count: 3 });
  await p2.goto(`${base}/index.html`);
  await p2.waitForTimeout(800);
  check("live count hidden under 10", await p2.evaluate(() => document.querySelector("#live").hidden));
  await p2.close();
  const p3 = await browser.newPage({ viewport: { width: 390, height: 844 } });
  await p3.emulateMedia({ reducedMotion: "reduce" });
  await mock(p3);
  await p3.goto(`${base}/index.html`);
  const anim = await p3.evaluate(() => getComputedStyle(document.querySelector(".mk-zz")).animationName + "|" + document.querySelectorAll(".mk-comet").length + "|" + getComputedStyle(document.querySelector(".mk-zz")).strokeDashoffset);
  check("reduced motion: line drawn at once, no pulse", /^none\|0\|0(px)?$/.test(anim), anim);
  await p3.close();
  const p4 = await browser.newPage({ viewport: { width: 390, height: 844 } });
  await mock(p4);
  await p4.goto(`${base}/index.html`);
  await p4.click("button[data-lang=lv]");
  await p4.click("#cat-idea");
  await p4.click("#step1 [data-next]");
  await p4.evaluate(() => history.back());
  await p4.waitForTimeout(400);
  check("phone Back on step 2 shows step 1", await p4.evaluate(() => !document.querySelector("#step1").hidden && document.body.dataset.screen === "flow"));
  await p4.evaluate(() => history.back());
  await p4.waitForTimeout(400);
  check("phone Back on step 1 shows the start, page still open", (await p4.evaluate(() => document.body.dataset.screen)) === "start" && p4.url().includes("index.html"));
  check("the anti-bot field is not rendered", await p4.evaluate(() => getComputedStyle(document.querySelector(".hp")).display === "none"));
  await p4.close();
}

/** The break option: an empty story is refused until the guest ticks it; then no story and no share go out. */
async function breakOption(engine, browser) {
  const check = (name, ok, detail) => results.push({ name: `${engine} ${name}`, ok, detail: detail || "" });
  const p = await browser.newPage({ viewport: { width: 390, height: 844 } });
  await mock(p);
  let sent = null;
  await p.route(`${API}/entry`, (r) => {
    sent = JSON.parse(r.request().postData());
    r.fulfill({ status: 200, headers: { "access-control-allow-origin": "*", "content-type": "application/json" }, body: '{"ok":true,"status":"new"}' });
  });
  await p.goto(`${base}/index.html`);
  await p.click("button[data-lang=lv]");
  await p.click("#cat-friendship");
  await p.click("#step1 [data-next]");
  await p.click("#step2 [data-next]");
  check("break option: an empty story stays on step 2 with a hint", !(await p.isHidden("#step2")) && (await p.textContent("#err-story")) === "Uzraksti stāstu vai izvēlies otru variantu.");
  await p.fill("#story", "Iesāku rakstīt");
  await p.click("label.choice--pause");
  check("break option: ticking it sets the story aside", await p.evaluate(() => document.querySelector("#step2").classList.contains("is-pause") && document.querySelector("#err-story").textContent === ""));
  await p.type("#story", " un");
  check("break option: writing takes it back", await p.evaluate(() => !document.querySelector("#pause").checked && !document.querySelector("#step2").classList.contains("is-pause")));
  await p.click("label.choice--pause");
  await p.click("#step2 [data-next]");
  check("break option: Tālāk goes on to the contacts", !(await p.isHidden("#step3")));
  check("break option: no share box without a story", await p.evaluate(() => document.querySelector("#share").closest(".check").hidden));
  await p.click(".rules summary");
  check("rules: seven, the prizes as a list of two", await p.evaluate(() => document.querySelectorAll("#rules-list > li").length === 7 && document.querySelectorAll("#rules-list li li").length === 2));
  await p.fill("#first", "Anna");
  await p.fill("#last", "Bērziņa");
  await p.fill("#email", "anna@example.lv");
  await p.fill("#phone", "+371 2000 0000");
  await p.click("label.check:has(#consent)");
  await p.click("#submit");
  await p.waitForSelector("#done:not([hidden])", { timeout: 5000 });
  check("break option: sent with no story and no share", !!sent && sent.story === "" && sent.share === false && sent.category === "friendship", JSON.stringify(sent));
  await p.close();
}

for (const [name, type] of [["chromium", chromium], ["webkit", webkit]]) {
  const browser = await type.launch();
  for (const size of SIZES) {
    try { await flow(name, browser, size); } catch (e) { check(`${name} ${size.join("x")}: flow ran`, false, String(e).slice(0, 200)); }
  }
  if (name === "chromium") await edgeCases(browser);
  try { await breakOption(name, browser); } catch (e) { check(`${name}: break option walk ran`, false, String(e).slice(0, 200)); }
  await browser.close();
}
const bad = results.filter((r) => !r.ok);
for (const r of bad) console.log(`FAIL ${r.name} ${r.detail}`);
console.log(`${results.length - bad.length}/${results.length} checks passed`);
process.exit(bad.length ? 1 : 0);
