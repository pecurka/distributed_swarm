#!/usr/bin/env python3
"""Turns a recorded run into an HTML page you can watch in a browser.

    python3 analysis/render.py run.csv run.html        # interactive page
    python3 analysis/render.py run.csv --svg strip.svg  # still frames for a README
    python3 analysis/render.py run.csv --animate loop.svg  # looping animation

Uses nothing outside Python's standard library, so there is nothing to install.
The page has all the data inside it, so it works offline and can be sent to
someone as a single file.
"""

import json
import math
import sys
from pathlib import Path

# Agents are coloured by the direction they are heading, so a flock all moving
# together shows up as a patch of one colour. That makes flocks, and the moment
# two flocks merge, obvious at a glance.

TEMPLATE = """<!doctype html>
<html><head><meta charset="utf-8"><title>__TITLE__</title>
<style>
  body { margin: 0; background: #11131a; color: #c9d1d9;
         font: 13px ui-monospace, SFMono-Regular, Menlo, monospace; }
  header { padding: 10px 14px; display: flex; gap: 14px; align-items: center;
           border-bottom: 1px solid #262b36; flex-wrap: wrap; }
  button { background: #1f6feb; color: #fff; border: 0; border-radius: 5px;
           padding: 6px 14px; font: inherit; cursor: pointer; }
  button:hover { background: #2f81f7; }
  input[type=range] { width: 320px; }
  canvas { display: block; margin: 14px auto; background: #0b0d12;
           border: 1px solid #262b36; }
  .label { color: #7d8590; }
</style></head><body>
<header>
  <button id="play">Pause</button>
  <input type="range" id="scrub" min="0" max="0" value="0">
  <span><span class="label">step</span> <b id="stepLabel">0</b></span>
  <span><span class="label">agents</span> <b>__AGENTS__</b></span>
  <span><span class="label">world</span> <b>__WORLD__</b></span>
</header>
<canvas id="view" width="760" height="760"></canvas>
<script>
const FRAMES = __DATA__;
const WORLD = __WORLD_JSON__;
const canvas = document.getElementById('view');
const context = canvas.getContext('2d');
const scrub = document.getElementById('scrub');
const stepLabel = document.getElementById('stepLabel');
const playButton = document.getElementById('play');

scrub.max = FRAMES.length - 1;
let index = 0;
let playing = true;

function draw(frameIndex) {
  const frame = FRAMES[frameIndex];
  const scaleX = canvas.width / WORLD[0];
  const scaleY = canvas.height / WORLD[1];
  context.fillStyle = '#0b0d12';
  context.fillRect(0, 0, canvas.width, canvas.height);

  for (const [x, y, vx, vy] of frame.agents) {
    // Hue from heading: agents going the same way share a colour.
    const hue = (Math.atan2(vy, vx) * 180 / Math.PI + 360) % 360;
    const screenX = x * scaleX;
    const screenY = canvas.height - y * scaleY;
    const speed = Math.hypot(vx, vy) || 1;

    context.strokeStyle = `hsl(${hue} 70% 62%)`;
    context.lineWidth = 1.6;
    context.beginPath();
    context.moveTo(screenX, screenY);
    context.lineTo(screenX - (vx / speed) * 6, screenY + (vy / speed) * 6);
    context.stroke();

    context.fillStyle = `hsl(${hue} 75% 68%)`;
    context.fillRect(screenX - 1.3, screenY - 1.3, 2.6, 2.6);
  }
  stepLabel.textContent = frame.step;
  scrub.value = frameIndex;
}

playButton.onclick = () => {
  playing = !playing;
  playButton.textContent = playing ? 'Pause' : 'Play';
};
scrub.oninput = () => { playing = false; playButton.textContent = 'Play';
                        index = +scrub.value; draw(index); };

function tick() {
  if (playing) { index = (index + 1) % FRAMES.length; draw(index); }
  setTimeout(tick, 60);
}
draw(0);
tick();
</script></body></html>
"""


def read_run(path):
    """Reads the CSV into a list of frames, one per recorded step."""
    world = [1000.0, 1000.0]
    frames = {}
    with open(path) as handle:
        for line in handle:
            line = line.strip()
            if not line:
                continue
            if line.startswith("#"):
                parts = line.split()
                if len(parts) == 4 and parts[1] == "world":
                    world = [float(parts[2]), float(parts[3])]
                continue
            if line.startswith("step,"):
                continue
            step, _id, x, y, vx, vy = line.split(",")
            frames.setdefault(int(step), []).append(
                [round(float(x), 1), round(float(y), 1),
                 round(float(vx), 2), round(float(vy), 2)]
            )
    ordered = [{"step": step, "agents": frames[step]} for step in sorted(frames)]
    return world, ordered


def write_snapshot_strip(world, frames, destination, panels=3, size=300, gap=14):
    """Writes a few frames side by side as a single SVG.

    SVG is plain text, so it lives happily in git and renders straight into a
    README, unlike the interactive page.
    """
    chosen = [frames[round(index * (len(frames) - 1) / (panels - 1))]
              for index in range(panels)]
    width = panels * size + (panels - 1) * gap
    height = size + 26

    out = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" '
           f'height="{height}" viewBox="0 0 {width} {height}" '
           f'font-family="ui-monospace,Menlo,monospace" font-size="11">']

    for panel, frame in enumerate(chosen):
        left = panel * (size + gap)
        out.append(f'<rect x="{left}" y="0" width="{size}" height="{size}" fill="#0b0d12"/>')
        for x, y, vx, vy in frame["agents"]:
            hue = (math.degrees(math.atan2(vy, vx)) + 360) % 360
            screen_x = left + x / world[0] * size
            screen_y = size - y / world[1] * size
            out.append(f'<circle cx="{screen_x:.1f}" cy="{screen_y:.1f}" r="1.5" '
                       f'fill="hsl({hue:.0f} 72% 65%)"/>')
        out.append(f'<text x="{left + size / 2:.0f}" y="{size + 17}" fill="#7d8590" '
                   f'text-anchor="middle">step {frame["step"]}</text>')

    out.append("</svg>")
    destination.write_text("\n".join(out))
    print(f"{destination}  —  {panels} panels, {destination.stat().st_size // 1000} KB")


def write_animated_svg(world, frames, destination, size=440, seconds=12):
    """Writes a looping animated SVG.

    Every agent becomes one dot that steps through its recorded positions. The
    steps are discrete rather than smoothly interpolated on purpose: the world
    wraps around, so an agent leaving the right edge reappears on the left, and
    smooth movement would draw it streaking back across the whole picture.

    Plain text and self-contained, so it animates directly in a README.
    """
    per_agent = {}
    for frame in frames:
        for index, (x, y, vx, vy) in enumerate(frame["agents"]):
            per_agent.setdefault(index, []).append((x, y, vx, vy))

    frame_count = len(frames)
    out = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" '
           f'viewBox="0 0 {size} {size}">',
           f'<rect width="{size}" height="{size}" fill="#0b0d12"/>']

    for track in per_agent.values():
        xs, ys, colours = [], [], []
        for x, y, vx, vy in track:
            xs.append(f"{x / world[0] * size:.0f}")
            ys.append(f"{size - y / world[1] * size:.0f}")
            hue = (math.degrees(math.atan2(vy, vx)) + 360) % 360
            colours.append(f"hsl({hue:.0f} 72% 65%)")
        timing = f'dur="{seconds}s" calcMode="discrete" repeatCount="indefinite"'
        out.append(
            f'<circle r="1.6" fill="{colours[0]}">'
            f'<animate attributeName="cx" values="{";".join(xs)}" {timing}/>'
            f'<animate attributeName="cy" values="{";".join(ys)}" {timing}/>'
            f'<animate attributeName="fill" values="{";".join(colours)}" {timing}/>'
            f"</circle>"
        )

    out.append("</svg>")
    destination.write_text("".join(out))
    print(f"{destination}  —  {len(per_agent)} agents, {frame_count} frames, "
          f"{destination.stat().st_size // 1000} KB")


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        raise SystemExit(1)

    for flag, writer in (("--animate", write_animated_svg), ("--svg", write_snapshot_strip)):
        if flag not in sys.argv:
            continue
        position = sys.argv.index(flag)
        if position + 1 >= len(sys.argv):
            raise SystemExit(f"{flag} needs a filename after it")
        world, frames = read_run(Path(sys.argv[1]))
        if not frames:
            raise SystemExit(f"no data found in {sys.argv[1]}")
        writer(world, frames, Path(sys.argv[position + 1]))
        return
    source = Path(sys.argv[1])
    destination = Path(sys.argv[2]) if len(sys.argv) > 2 else source.with_suffix(".html")

    world, frames = read_run(source)
    if not frames:
        raise SystemExit(f"no data found in {source}")

    page = (TEMPLATE
            .replace("__TITLE__", source.stem)
            .replace("__DATA__", json.dumps(frames, separators=(",", ":")))
            .replace("__WORLD_JSON__", json.dumps(world))
            .replace("__WORLD__", f"{world[0]:g} x {world[1]:g}")
            .replace("__AGENTS__", str(len(frames[0]["agents"]))))
    destination.write_text(page)

    size_mb = destination.stat().st_size / 1_000_000
    print(f"{destination}  —  {len(frames)} frames, "
          f"{len(frames[0]['agents'])} agents, {size_mb:.1f} MB")


if __name__ == "__main__":
    main()
