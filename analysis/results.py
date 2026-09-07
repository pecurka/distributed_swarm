#!/usr/bin/env python3
"""Turns a results file into tables and charts.

    python3 analysis/results.py                       # tables to the terminal
    python3 analysis/results.py data/analysis.html    # and a page of charts

Reads data/results.csv by default. Uses nothing outside Python's standard
library, so there is nothing to install.

Every number is a median across repeats. Timing on a laptop varies by a few
percent between identical runs, so a single run is not a measurement.
"""

import collections
import csv
import statistics
import sys
from pathlib import Path

RESULTS = Path("data/results.csv")

# The categorical palette, in its fixed validated order. Slots are assigned to
# swarm sizes and to the parts of a step, and never reordered or cycled: a
# reader who learns "16000 agents is blue" must not have it repainted.
LIGHT = ["#2a78d6", "#eb6834", "#1baf7a", "#eda100", "#e87ba4"]
DARK = ["#3987e5", "#d95926", "#199e70", "#c98500", "#d55181"]

PHASES = [
    ("computing_seconds", "computing"),
    ("waiting_seconds", "waiting for others"),
    ("communicating_seconds", "communicating"),
    ("finishing_seconds", "finishing together"),
]


def load(path=RESULTS):
    """Groups runs by configuration, keeping only the longest step count."""
    rows = list(csv.DictReader(open(path)))
    steps = max(int(row["steps"]) for row in rows)
    grouped = collections.defaultdict(list)
    for row in rows:
        if int(row["steps"]) != steps:
            continue
        key = (row["runner"], int(row["processes"]), int(row["agents"]),
               row["measured_phases"] == "true")
        grouped[key].append(row)
    return grouped, steps


def middle(runs, column):
    """The median, which is what a handful of noisy runs can honestly give."""
    return statistics.median(float(run[column]) for run in runs)


def sizes_and_processes(grouped):
    sizes = sorted({key[2] for key in grouped if key[0] == "sequential"})
    processes = sorted({key[1] for key in grouped if key[0] == "distributed"})
    return sizes, processes


def speedup_table(grouped, sizes, processes):
    """How many times faster than one process, for each swarm size."""
    table = {}
    for agents in sizes:
        baseline = grouped.get(("sequential", 1, agents, False))
        if not baseline:
            continue
        base = middle(baseline, "simulating_seconds")
        for count in processes:
            runs = grouped.get(("distributed", count, agents, False))
            if runs:
                table[(agents, count)] = base / middle(runs, "simulating_seconds")
    return table


def phase_table(grouped, sizes, count):
    """What share of a step went where, as percentages."""
    table = {}
    for agents in sizes:
        runs = grouped.get(("distributed", count, agents, True))
        if not runs:
            continue
        parts = [middle(runs, column) for column, _ in PHASES]
        total = sum(parts)
        table[agents] = [100 * part / total for part in parts] if total else [0] * 4
    return table


def imbalance_table(grouped, sizes, processes):
    """How uneven the work was, averaged across the whole run."""
    table = {}
    for agents in sizes:
        for count in processes:
            runs = grouped.get(("distributed", count, agents, False))
            if runs:
                table[(agents, count)] = middle(runs, "average_imbalance")
    return table


# ---------------------------------------------------------------- tables ----

def print_tables(grouped, steps, sizes, processes):
    speedups = speedup_table(grouped, sizes, processes)
    imbalance = imbalance_table(grouped, sizes, processes)
    biggest = max(processes)

    print(f"\nAll numbers are medians across repeats, {steps} steps.\n")

    print("SPEEDUP  (times faster than one process)")
    print("  agents  " + "".join(f"{count:>9}" for count in processes))
    for agents in sizes:
        row = "".join(f"{speedups.get((agents, c), float('nan')):>8.2f}x"
                      for c in processes)
        print(f"{agents:>8}  {row}")

    print("\nEFFICIENCY  (speedup divided by process count)")
    print("  agents  " + "".join(f"{count:>9}" for count in processes))
    for agents in sizes:
        row = "".join(f"{100 * speedups.get((agents, c), 0) / c:>8.0f}%"
                      for c in processes)
        print(f"{agents:>8}  {row}")

    print("\nAVERAGE IMBALANCE  (busiest process over the average, 1.00 is even)")
    print("  agents  " + "".join(f"{count:>9}" for count in processes))
    for agents in sizes:
        row = "".join(f"{imbalance.get((agents, c), float('nan')):>8.2f}x"
                      for c in processes)
        print(f"{agents:>8}  {row}")

    print(f"\nWHERE THE TIME WENT at {biggest} processes  (share of a step)")
    phases = phase_table(grouped, sizes, biggest)
    print("  agents  " + "".join(f"{name:>20}" for _, name in PHASES))
    for agents in sizes:
        if agents in phases:
            row = "".join(f"{value:>19.1f}%" for value in phases[agents])
            print(f"{agents:>8}  {row}")
    print()


# ---------------------------------------------------------------- charts ----

def axes(width, height, pad):
    """Plot area as (left, top, right, bottom)."""
    return pad["left"], pad["top"], width - pad["right"], height - pad["bottom"]


def line_chart(speedups, sizes, processes):
    """Speedup against process count, one line per swarm size."""
    width, height = 620, 380
    pad = {"left": 54, "top": 16, "right": 96, "bottom": 44}
    left, top, right, bottom = axes(width, height, pad)
    top_value = max(2.0, max(speedups.values()) * 1.1)

    def x_of(count):
        return left + (count - processes[0]) / (processes[-1] - processes[0]) * (right - left)

    def y_of(value):
        return bottom - value / top_value * (bottom - top)

    out = [f'<svg viewBox="0 0 {width} {height}" role="img" '
           f'aria-label="Speedup against process count, one line per swarm size">']

    # Gridlines: hairline, solid, one step off the surface.
    for tick in range(0, int(top_value) + 1):
        y = y_of(tick)
        out.append(f'<line class="grid" x1="{left}" y1="{y:.1f}" x2="{right}" y2="{y:.1f}"/>')
        out.append(f'<text class="tick" x="{left - 8}" y="{y + 4:.1f}" '
                   f'text-anchor="end">{tick}x</text>')
    for count in processes:
        out.append(f'<text class="tick" x="{x_of(count):.1f}" y="{bottom + 20}" '
                   f'text-anchor="middle">{count}</text>')
    out.append(f'<text class="axis" x="{(left + right) / 2:.0f}" y="{height - 8}" '
               f'text-anchor="middle">processes</text>')

    for slot, agents in enumerate(sizes):
        points = [(x_of(c), y_of(speedups[(agents, c)]))
                  for c in processes if (agents, c) in speedups]
        if not points:
            continue
        path = " ".join(f"{'M' if i == 0 else 'L'}{x:.1f} {y:.1f}"
                        for i, (x, y) in enumerate(points))
        out.append(f'<path class="series s{slot}" d="{path}"/>')
        for (x, y), count in zip(points, processes):
            # 2px surface ring so dots stay legible where lines cross.
            out.append(f'<circle class="dot s{slot}" cx="{x:.1f}" cy="{y:.1f}" r="4" '
                       f'data-label="{agents} agents, {count} processes: '
                       f'{speedups[(agents, count)]:.2f}x"/>')
        # Direct labels only on the extremes; the legend carries the rest.
        if agents in (sizes[0], sizes[-1]):
            x, y = points[-1]
            out.append(f'<text class="direct s{slot}" x="{x + 10:.0f}" y="{y + 4:.0f}">'
                       f'{agents:,}</text>')
    out.append("</svg>")
    return "\n".join(out)


def stacked_bars(phases, sizes, count):
    """Where a step's time went, one column per swarm size."""
    width, height = 620, 360
    # The legend is rendered below the figure in HTML, so the plot area does
    # not need to reserve room for one.
    pad = {"left": 54, "top": 16, "right": 24, "bottom": 44}
    left, top, right, bottom = axes(width, height, pad)
    band = (right - left) / len(sizes)
    bar_width = min(24, band * 0.5)          # never fill the slot
    gap = 2                                   # surface gap between segments

    out = [f'<svg viewBox="0 0 {width} {height}" role="img" '
           f'aria-label="Share of a step spent computing, waiting and communicating">']
    for tick in range(0, 101, 25):
        y = bottom - tick / 100 * (bottom - top)
        out.append(f'<line class="grid" x1="{left}" y1="{y:.1f}" x2="{right}" y2="{y:.1f}"/>')
        out.append(f'<text class="tick" x="{left - 8}" y="{y + 4:.1f}" '
                   f'text-anchor="end">{tick}%</text>')

    for column, agents in enumerate(sizes):
        if agents not in phases:
            continue
        centre = left + band * (column + 0.5)
        x = centre - bar_width / 2
        cursor = bottom
        for slot, (value, (_, name)) in enumerate(zip(phases[agents], PHASES)):
            span = value / 100 * (bottom - top)
            # A share this small cannot be drawn honestly: after the surface gap
            # there is less than a pixel left, and a sliver that thin renders
            # unpredictably. The value is still in the table below the chart, so
            # nothing is hidden — it just is not drawn.
            if span - gap < 1.5:
                continue
            y = cursor - span
            radius = 4 if slot == 0 else 0    # rounded data-end only at the top
            out.append(f'<rect class="bar s{slot}" x="{x:.1f}" y="{y + gap / 2:.1f}" '
                       f'width="{bar_width:.1f}" height="{span - gap:.1f}" rx="{radius}" '
                       f'data-label="{agents} agents, {name}: {value:.1f}%"/>')
            cursor = y
        out.append(f'<text class="tick" x="{centre:.1f}" y="{bottom + 20}" '
                   f'text-anchor="middle">{agents:,}</text>')
    out.append(f'<text class="axis" x="{(left + right) / 2:.0f}" y="{height - 8}" '
               f'text-anchor="middle">agents (at {count} processes)</text>')
    out.append("</svg>")
    return "\n".join(out)


def scatter(speedups, imbalance, sizes, processes):
    """Speedup against how unevenly the work was shared.

    One series, so every point is the same colour: the story is the trend, not
    which point is which. The tooltip and the table say which is which.
    """
    width, height = 620, 360
    pad = {"left": 54, "top": 16, "right": 24, "bottom": 48}
    left, top, right, bottom = axes(width, height, pad)
    pairs = [(imbalance[k], speedups[k], k) for k in speedups if k in imbalance]
    worst = max(value for value, _, _ in pairs) * 1.05
    best = max(value for _, value, _ in pairs) * 1.1

    def x_of(value):
        return left + (value - 1.0) / (worst - 1.0) * (right - left)

    def y_of(value):
        return bottom - value / best * (bottom - top)

    out = [f'<svg viewBox="0 0 {width} {height}" role="img" '
           f'aria-label="Speedup falls as the work is shared less evenly">']
    for tick in range(0, int(best) + 1):
        y = y_of(tick)
        out.append(f'<line class="grid" x1="{left}" y1="{y:.1f}" x2="{right}" y2="{y:.1f}"/>')
        out.append(f'<text class="tick" x="{left - 8}" y="{y + 4:.1f}" '
                   f'text-anchor="end">{tick}x</text>')
    step = 0.2
    tick = 1.0
    while tick <= worst:
        out.append(f'<text class="tick" x="{x_of(tick):.1f}" y="{bottom + 20}" '
                   f'text-anchor="middle">{tick:.1f}x</text>')
        tick += step
    out.append(f'<text class="axis" x="{(left + right) / 2:.0f}" y="{height - 10}" '
               f'text-anchor="middle">average imbalance (1.0 is perfectly even)</text>')

    for value, speed, (agents, count) in sorted(pairs):
        out.append(f'<circle class="dot s0" cx="{x_of(value):.1f}" cy="{y_of(speed):.1f}" '
                   f'r="5" data-label="{agents} agents, {count} processes: '
                   f'{speed:.2f}x at {value:.2f}x imbalance"/>')
    out.append("</svg>")
    return "\n".join(out)


# ------------------------------------------------------------------ page ----

def swatches(names, slots):
    return "".join(
        f'<span class="key"><i class="s{slot}"></i>{name}</span>'
        for slot, name in zip(slots, names)
    )


def html_table(headings, rows, caption):
    head = "".join(f"<th>{h}</th>" for h in headings)
    body = "".join("<tr>" + "".join(f"<td>{cell}</td>" for cell in row) + "</tr>"
                   for row in rows)
    return (f'<details><summary>{caption} — table</summary>'
            f'<table><thead><tr>{head}</tr></thead><tbody>{body}</tbody></table>'
            f"</details>")


def build_page(grouped, steps, sizes, processes):
    speedups = speedup_table(grouped, sizes, processes)
    imbalance = imbalance_table(grouped, sizes, processes)
    biggest = max(processes)
    phases = phase_table(grouped, sizes, biggest)

    light = "".join(f"  --s{i}: {c};\n" for i, c in enumerate(LIGHT))
    dark = "".join(f"    --s{i}: {c};\n" for i, c in enumerate(DARK))

    speed_rows = [[f"{a:,}"] + [f"{speedups.get((a, c), 0):.2f}x" for c in processes]
                  for a in sizes]
    imbalance_rows = [[f"{a:,}"] + [f"{imbalance.get((a, c), 0):.2f}x" for c in processes]
                      for a in sizes]
    phase_rows = [[f"{a:,}"] + [f"{v:.1f}%" for v in phases[a]]
                  for a in sizes if a in phases]

    return f"""<!doctype html>
<html><head><meta charset="utf-8"><title>Scaling results</title>
<style>
:root {{
  color-scheme: light;
  --surface: #fcfcfb; --ink: #0b0b0b; --ink-2: #52514e; --grid: #e6e6e3;
{light}}}
@media (prefers-color-scheme: dark) {{
  :root:not([data-theme="light"]) {{
    color-scheme: dark;
    --surface: #1a1a19; --ink: #ffffff; --ink-2: #c3c2b7; --grid: #2f2f2c;
{dark}  }}
}}
:root[data-theme="dark"] {{
  color-scheme: dark;
  --surface: #1a1a19; --ink: #ffffff; --ink-2: #c3c2b7; --grid: #2f2f2c;
{dark}}}
body {{ margin: 0; background: var(--surface); color: var(--ink);
  font: 14px/1.55 ui-sans-serif, system-ui, -apple-system, Segoe UI, sans-serif; }}
main {{ max-width: 720px; margin: 0 auto; padding: 32px 20px 64px; }}
h1 {{ font-size: 22px; margin: 0 0 4px; }}
h2 {{ font-size: 16px; margin: 40px 0 2px; }}
p.sub {{ color: var(--ink-2); margin: 0 0 14px; }}
figure {{ margin: 0 0 8px; }}
svg {{ width: 100%; height: auto; overflow: visible; }}
.grid {{ stroke: var(--grid); stroke-width: 1; }}
.tick, .axis {{ fill: var(--ink-2); font-size: 11px;
  font-family: ui-monospace, Menlo, monospace; }}
.axis {{ fill: var(--ink-2); font-size: 12px; }}
.series {{ fill: none; stroke-width: 2; stroke-linejoin: round;
  stroke-linecap: round; }}
.dot {{ stroke: var(--surface); stroke-width: 2; }}
.direct {{ font-size: 11px; font-family: ui-monospace, Menlo, monospace;
  fill: var(--ink-2); }}
.legend {{ display: flex; flex-wrap: wrap; gap: 14px; margin: 8px 0 0;
  color: var(--ink-2); font-size: 12px; }}
.key {{ display: inline-flex; align-items: center; gap: 6px; }}
.key i {{ width: 11px; height: 11px; border-radius: 3px; display: inline-block; }}
{"".join(f".s{i} {{ stroke: var(--s{i}); }} rect.s{i}, circle.s{i}, .key i.s{i} {{ fill: var(--s{i}); }} text.s{i} {{ fill: var(--ink-2); stroke: none; }}" for i in range(5))}
details {{ margin: 10px 0 0; color: var(--ink-2); }}
summary {{ cursor: pointer; font-size: 12px; }}
table {{ border-collapse: collapse; margin: 10px 0; font-size: 12px;
  font-family: ui-monospace, Menlo, monospace; }}
th, td {{ text-align: right; padding: 4px 10px;
  border-bottom: 1px solid var(--grid); }}
th:first-child, td:first-child {{ text-align: left; }}
#tip {{ position: fixed; pointer-events: none; opacity: 0; background: var(--ink);
  color: var(--surface); padding: 5px 9px; border-radius: 5px; font-size: 12px;
  font-family: ui-monospace, Menlo, monospace; transition: opacity .08s; }}
</style></head><body>
<main>
<h1>Splitting the flock across processes</h1>
<p class="sub">Medians across repeats, {steps} steps per run, up to {biggest}
processes on one 10-core machine.</p>

<h2>Bigger swarms are worth splitting; small ones are not</h2>
<p class="sub">At 1,000 agents, adding processes past four makes it slower. At
16,000 it is still improving at {biggest}.</p>
<figure>{line_chart(speedups, sizes, processes)}</figure>
<div class="legend">{swatches([f"{a:,} agents" for a in sizes], range(len(sizes)))}</div>
{html_table(["agents"] + [f"{c} proc" for c in processes], speed_rows, "Speedup")}

<h2>Communication is not the problem — waiting is</h2>
<p class="sub">Moving data between processes never costs more than a few percent
of a step. Processes waiting for each other costs up to half of it.</p>
<figure>{stacked_bars(phases, sizes, biggest)}</figure>
<div class="legend">{swatches([name for _, name in PHASES], range(4))}</div>
{html_table(["agents"] + [name for _, name in PHASES], phase_rows, "Where the time went")}

<h2>Speedup tracks how evenly the work is shared</h2>
<p class="sub">Each point is one swarm size at one process count. The more
unevenly the agents are spread across processes, the less splitting them
helps — which is the whole result in one picture.</p>
<figure>{scatter(speedups, imbalance, sizes, processes)}</figure>
{html_table(["agents"] + [f"{c} proc" for c in processes], imbalance_rows,
            "Average imbalance")}
</main>
<div id="tip"></div>
<script>
const tip = document.getElementById('tip');
for (const mark of document.querySelectorAll('[data-label]')) {{
  mark.addEventListener('pointerenter', event => {{
    tip.textContent = mark.dataset.label;
    tip.style.opacity = 1;
  }});
  mark.addEventListener('pointermove', event => {{
    tip.style.left = (event.clientX + 14) + 'px';
    tip.style.top = (event.clientY - 10) + 'px';
  }});
  mark.addEventListener('pointerleave', () => {{ tip.style.opacity = 0; }});
}}
</script>
</body></html>
"""


def main():
    grouped, steps = load()
    sizes, processes = sizes_and_processes(grouped)
    print_tables(grouped, steps, sizes, processes)
    if len(sys.argv) > 1:
        destination = Path(sys.argv[1])
        destination.write_text(build_page(grouped, steps, sizes, processes))
        print(f"charts written to {destination}")


if __name__ == "__main__":
    main()
