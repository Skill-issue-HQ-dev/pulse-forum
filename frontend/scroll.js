// Where the guest page scrolls: on iPhone inside main [styles.css], so the L-line always meets the screen's bottom
// edge; elsewhere the page itself. Owns the jump back to the top and the page's return after the iPhone keyboard.
// Invariant: on iPhone the page itself sits at the top whenever no text field has the focus.

const main = () => document.querySelector("main");
/** True on iPhone, where main is the scroller [styles.css]; elsewhere main never scrolls. */
const inMain = () => getComputedStyle(main()).overflowY === "auto";
const typing = () => !!document.activeElement && document.activeElement.matches("textarea, input:not([type=checkbox]):not([type=radio])");

/** Both scrollers back to the top: the page's own and main's. */
export function toTop() {
  window.scrollTo(0, 0);
  main().scrollTop = 0;
}

/** A keyboard can push the whole page up to show its field even though only main scrolls, and Safari leaves it there
 *  when the keyboard closes: put it back once no text field has the focus. */
export function settleAfterKeyboard() {
  if (!inMain()) return;
  const settle = () => { if (!typing() && window.scrollY) window.scrollTo(0, 0); };
  document.addEventListener("focusout", () => setTimeout(settle, 80));
  if (window.visualViewport) window.visualViewport.addEventListener("resize", settle);
}
