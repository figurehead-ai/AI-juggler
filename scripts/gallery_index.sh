#!/usr/bin/env bash
# Generate the browsable gallery page (AJ-E802 Req.3): screen → state →
# viewport → theme, each cell labelled with fixture + owning task.
# AJ-E2814 (D-57): each family's design critique (its critique.md) beside
# its captures, rendered escaped by scripts/critique_check.py --html.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
G=docs/qa/evidence/_gallery
OUT="$G/index.html"
{
echo '<!doctype html><html><head><meta charset="utf-8"><title>AI Juggler — visual states gallery</title>'
echo '<style>body{font:13px/1.5 system-ui;background:#0a0d13;color:#e8ecf3;margin:24px}h2{margin-top:32px}h3{color:#9d9dfb}'
echo 'img{max-width:420px;border:1px solid #232a38;border-radius:8px;margin:4px;background:#fff}'
echo '.cell{display:inline-block;margin:8px;vertical-align:top}.lbl{font-size:11px;color:#8b95a7}'
echo '.crit{max-width:960px;border:1px solid #232a38;border-radius:8px;padding:4px 16px 12px;background:#111622}.crit table{border-collapse:collapse}'
echo '.crit th,.crit td{border-bottom:1px solid #232a38;padding:4px 8px;text-align:left;vertical-align:top}.crit h3{margin:8px 0 4px}</style></head><body>'
echo '<h1>Visual states gallery (generated — run scripts/gallery_index.sh after pnpm visual:capture)</h1>'
echo '<p>Matrix: viewports 1440×900 / 1280×800 / 1024×680 (Tauri min) / 390×844 · themes dark+light (D-16). Seed set: AJ-E802.</p>'
if [ -f "$G/not-captured.md" ]; then
  echo '<h2>Explicitly not captured</h2><pre>'; sed 's/|/ · /g' "$G/not-captured.md" | tail -n +4; echo '</pre>'
fi
for screen_dir in "$G"/*/; do
  screen="$(basename "$screen_dir")"; [ "$screen" = index.html ] && continue
  echo "<h2>$screen</h2>"
  if [ -f "$screen_dir/critique.md" ]; then
    echo "<div class='crit'>"; python3 scripts/critique_check.py --html "$screen_dir/critique.md"; echo "</div>"
  fi
  for img in "$screen_dir"*.png; do
    [ -e "$img" ] || continue
    name="$(basename "$img" .png)"
    echo "<div class='cell'><div class='lbl'>$name</div><a href='$(basename "$screen_dir")/$(basename "$img")'><img src='$(basename "$screen_dir")/$(basename "$img")' loading='lazy' alt='$screen $name'></a></div>"
  done
done
echo '</body></html>'
} > "$OUT"
echo "generated $OUT"
