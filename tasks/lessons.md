# Lessons

- Logo: measure against the client's live site art as well as the brand book. pulseforum.co's hero and the brand
  book cover differ at the L's step [line 2 width and start]; overlay at equal letter height before shipping.
- A draw-on animation is one continuous stroke [pathLength + dashoffset] clipped to the shape, never its parts
  animated as separate blocks.
- A light signal along a line is a narrow tapered comet with a glow, never a solid block the line's width.
- WebKit clips a clip-path to the element's fill box: a stroked zig-zag lost half its width. Wrap the stroked path
  in a group that also holds a fill-less rect over the whole viewBox.
- On a phone the logo's line must reach the bottom of the screen on every screen; a line that ends mid-screen or a
  pulse that stops short reads as broken. Measure the line's end against the screen per step, not only the first.
- Test layouts as a matrix of window sizes AND live resizing [860-1100 px wide broke the PC cover], and measure
  geometry after fonts load: one fit at page load goes stale. Refit on ResizeObserver, not window resize.
- Check the fold at the size the browser shows, not the screen's: Safari with its toolbars out shows 390 x 664 of a
  390 x 844 iPhone [Playwright's device viewports]. The gate passed at 390 x 844 while guests had the SIXT logo
  144 px below the fold.
- iPhone Safari [iOS 26] shows the page under its floating bar only where the document reaches: 100lvh ends 58px
  above the screen's edge, so a one-screen page must reach past it. Fixed boxes are cut at the visible bottom, and a
  fixed box touching the bottom edge makes Safari paint the strip under the bar solid. Playwright sees none of this:
  check in the iOS simulator [tests/ios/edge.sh].
