#!/bin/zsh
# The bottom-edge check on a real iPhone Safari [iOS simulator]: Playwright cannot see Safari's floating bar.
# Opens the page in a booted simulator, then compares, near the screen's bottom edge, the pixel under the L-line with
# one 30pt beside it; the line must be lighter there by a clear margin. Exits 1 when the line stops short.
# Run: tests/ios/edge.sh <simulator udid> <page url> <line x in points> <screenshot dir> [wait seconds]
# [iPhone 17 Pro Max cover: x 247; iPhone SE cover: x 210; the form's rail: screen width - 46]
U=$1; URL=$2; XPT=$3; OUT=$4; WAIT=${5:-10}
[[ -n $OUT ]] || { echo "usage: edge.sh <udid> <url> <x points> <dir> [wait]"; exit 2; }
mkdir -p $OUT
SHOT=$OUT/edge-$(date +%s).png
xcrun simctl openurl $U "$URL" || exit 2
sleep $WAIT
xcrun simctl io $U screenshot $SHOT >/dev/null 2>&1 || exit 2
H=$(magick identify -format "%h" $SHOT); W=$(magick identify -format "%w" $SHOT)
SCALE=$(( W > 1000 ? 3 : 2 ))
px() { magick $SHOT -crop 3x3+$(( $1 - 1 ))+$2 +repage -scale 1x1! -format "%[fx:int(255*r)] %[fx:int(255*g)] %[fx:int(255*b)]" info:; }
bad=0
for back in 4 12 24; do
  Y=$(( H - back * SCALE ))
  L=($(px $(( XPT * SCALE )) $Y)); B=($(px $(( (XPT - 30) * SCALE )) $Y))
  D=$(( L[1] + L[2] + L[3] - B[1] - B[2] - B[3] ))
  (( D >= 30 )) && V=line || { V="NO LINE"; bad=1; }
  printf "%2dpt above the bottom: line rgb(%s) beside rgb(%s) +%d -> %s\n" $back "${(j:,:)L}" "${(j:,:)B}" $D "$V"
done
echo "edge: $([ $bad = 0 ] && echo 'the line reaches the bottom edge' || echo 'the line stops short') [$SHOT]"
exit $bad
