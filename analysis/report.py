#!/usr/bin/env python3
"""Builds a page of charts from a results file.

    python3 analysis/report.py data/analysis.html
    python3 analysis/report.py --serbian data/analysis-sr.html

Reads its numbers from `results.py`, which is where the tables live. Kept
separate so the numbers can be used without the page, and the page can be
changed without touching them.

Uses nothing outside Python's standard library.
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from results import (  # noqa: E402  (path has to be set first)
    imbalance_table,
    load,
    phase_table,
    sizes_and_processes,
    speedup_table,
)

# reader who learns "16000 agents is blue" must not have it repainted.
LIGHT = ["#2a78d6", "#eb6834", "#1baf7a", "#eda100", "#e87ba4"]
DARK = ["#3987e5", "#d95926", "#199e70", "#c98500", "#d55181"]

STRINGS = {
    "en": {
        "lang": "en",
        "phases": ["computing", "waiting for others", "communicating",
                   "finishing together"],
        "processes": "processes",
        "agents_at": "agents (at {count} processes)",
        "imbalance_axis": "average imbalance (1.0 is perfectly even)",
        "agents_label": "{agents} agents",
        "aria_speedup": "Speedup against process count, one line per swarm size",
        "aria_phases": "Share of a step spent computing, waiting and communicating",
        "aria_scatter": "Speedup falls as the work is shared less evenly",
        "tip_speedup": "{agents} agents, {count} processes: {value:.2f}x",
        "tip_phase": "{agents} agents, {name}: {value:.1f}%",
        "tip_scatter": "{agents} agents, {count} processes: {speed:.2f}x at {value:.2f}x imbalance",
        "title": "Splitting the Flock",
        "eyebrow_report": "Bachelor thesis &middot; measurement report",
        "h1": "Splitting the flock across processes",
        "standfirst": "A boids simulation divided between processes runs the same "
                      "simulation, exactly. Whether it runs it any <em>faster</em> "
                      "depends almost entirely on one thing &mdash; and it is not "
                      "the network.",
        "meta_sizes": "{n} swarm sizes",
        "meta_processes": "{first}&ndash;{last} processes",
        "meta_steps": "{steps} steps per run",
        "meta_medians": "medians across repeats",
        "q1": "Question 1 &middot; fidelity",
        "q1_h": "Splitting the work changes nothing about the result",
        "q1_p": "Both runners end by reducing every agent&rsquo;s position and "
                "velocity, bit for bit, to a single fingerprint. Across every swarm "
                "size and every process count in this report, that fingerprint is "
                "identical &mdash; not close, not statistically similar, the same "
                "simulation.",
        "q1_callout_h": "Why every timing below can be trusted",
        "q1_callout_p": "A fast run and a slow run are only comparable if they "
                        "computed the same thing. Each row of the results file "
                        "carries its fingerprint, so that is checked rather than "
                        "assumed. Any run that had silently diverged would show up "
                        "immediately as a different number.",
        "q2": "Question 2 &middot; scaling",
        "q2_h": "Bigger swarms are worth splitting; small ones are not",
        "q2_p": "At {small} agents, adding processes past four makes the "
                "simulation <em>slower</em> &mdash; the best it manages is "
                "{best_small:.2f}&times;. At {large} agents it reaches "
                "{best_large:.2f}&times; and is still improving at {biggest} "
                "processes. The question is not whether splitting helps, but "
                "whether the problem is big enough to be worth splitting.",
        "q3": "Question 3 &middot; crossover",
        "q3_h": "Communication never becomes the bottleneck",
        "q3_p": "Moving agents between processes costs at most {worst_comm:.1f}% "
                "of a step, and it <em>shrinks</em> as the swarm grows: the work "
                "grows faster than the borders do. Waiting for other processes to "
                "finish costs up to {worst_wait:.0f}%.",
        "q3_callout_h": "Separating waiting from communicating",
        "q3_callout_p": "These two are easy to confuse and the confusion inverts "
                        "the conclusion. A process that finishes early sits inside "
                        "its receive call waiting for a slower neighbour, and a "
                        "na&iuml;ve measurement counts that wait as communication. "
                        "Runs marked <code>--measure-phases</code> add a barrier "
                        "after computing and before communicating, which absorbs "
                        "the waiting so what remains is really data movement. That "
                        "barrier costs a little speed, so it is used for this "
                        "breakdown only and never for the speedup figures above.",
        "q4": "Question 4 &middot; balance",
        "q4_h": "Speedup tracks how evenly the agents are spread",
        "q4_p": "Each point is one swarm size at one process count. The "
                "relationship is the whole result: the more unevenly agents are "
                "distributed across the strips, the less splitting them up helps. "
                "Flocks bunch together, and a fixed division of the world cannot "
                "follow them.",
        "q4_callout_h": "Measured across the run, not at the end",
        "q4_callout_p": "Imbalance is sampled throughout rather than read once at "
                        "the finish. The difference matters: a run that stays even "
                        "until the last moment and one that goes bad immediately "
                        "look identical at the finish line, and they cost "
                        "completely different amounts. Two swarm sizes here end "
                        "with almost the same imbalance and scale quite "
                        "differently, which only the running measurement explains.",
        "lim": "Limitations",
        "lim_h": "What these numbers do not show",
        "lim_p1": "Every run is {biggest} processes or fewer on a single ten-core "
                  "machine. Past that, processes share cores and the timings "
                  "measure the operating system rather than the simulation, so the "
                  "sweep refuses to run oversubscribed.",
        "lim_p2": "These are therefore processes sharing one memory bus, not "
                  "separate machines with a network between them. Communication is "
                  "cheaper here than it would be on a cluster, so the point at "
                  "which it starts to matter would arrive earlier on real "
                  "distributed hardware than these figures suggest.",
        "lim_p3": "The world stays the same size as the swarm grows, so a larger "
                  "swarm is also a denser one: each agent has more neighbours, and "
                  "the work grows roughly with the square of the swarm size rather "
                  "than linearly.",
        "table_speedup": "Speedup, every configuration",
        "table_phases": "Share of a step at {biggest} processes",
        "table_imbalance": "Average imbalance, every configuration",
        "col_agents": "agents",
        "col_proc": "{count} proc",
        "col_proc_one": "1 proc",
        "thousands": ",",
    },
    "sr": {
        "lang": "sr",
        "phases": ["računanje", "čekanje na ostale", "komunikacija",
                   "završna barijera"],
        "processes": "procesi",
        "agents_at": "agenti (pri {count} procesa)",
        "imbalance_axis": "prosečna neravnoteža (1.0 je savršeno ravnomerno)",
        "agents_label": "{agents} agenata",
        "aria_speedup": "Ubrzanje u odnosu na broj procesa, po jedna linija za svaku veličinu roja",
        "aria_phases": "Udeo koraka utrošen na računanje, čekanje i komunikaciju",
        "aria_scatter": "Ubrzanje opada kako je posao neravnomernije raspoređen",
        "tip_speedup": "{agents} agenata, {count} procesa: {value:.2f}x",
        "tip_phase": "{agents} agenata, {name}: {value:.1f}%",
        "tip_scatter": "{agents} agenata, {count} procesa: {speed:.2f}x pri neravnoteži {value:.2f}x",
        "title": "Podela jata",
        "eyebrow_report": "Diplomski rad &middot; izveštaj o merenjima",
        "h1": "Podela jata na procese",
        "standfirst": "Simulacija jata podeljena na procese izvršava potpuno istu "
                      "simulaciju. Da li je izvršava <em>brže</em> zavisi gotovo "
                      "isključivo od jedne stvari &mdash; i to nije mreža.",
        "meta_sizes": "{n} veličina roja",
        "meta_processes": "{first}&ndash;{last} procesa",
        "meta_steps": "{steps} koraka po izvršavanju",
        "meta_medians": "medijane više ponavljanja",
        "q1": "Pitanje 1 &middot; vernost",
        "q1_h": "Podela posla ne menja rezultat",
        "q1_h_note": "",
        "q1_p": "Oba programa na kraju svode poziciju i brzinu svakog agenta, bit "
                "po bit, na jedan kontrolni otisak. Za svaku veličinu roja i svaki "
                "broj procesa u ovom izveštaju taj otisak je identičan &mdash; ne "
                "približan, ne statistički sličan, nego ista simulacija.",
        "q1_callout_h": "Zašto se svakom merenju ispod može verovati",
        "q1_callout_p": "Brzo i sporo izvršavanje uporediva su samo ako su "
                        "izračunala istu stvar. Svaki red u datoteci sa rezultatima "
                        "nosi svoj otisak, pa se to proverava umesto da se "
                        "pretpostavlja. Izvršavanje koje bi neprimetno odstupilo "
                        "odmah bi se videlo kao drugačiji broj.",
        "q2": "Pitanje 2 &middot; skaliranje",
        "q2_h": "Veće rojeve se isplati deliti, male ne",
        "q2_p": "Pri {small} agenata, dodavanje procesa preko četiri čini "
                "simulaciju <em>sporijom</em> &mdash; najbolje što postiže je "
                "{best_small:.2f}&times;. Pri {large} agenata dostiže "
                "{best_large:.2f}&times; i još uvek raste na {biggest} procesa. "
                "Pitanje nije da li podela pomaže, nego da li je problem dovoljno "
                "veliki da bi se isplatilo deliti ga.",
        "q3": "Pitanje 3 &middot; prelomna tačka",
        "q3_h": "Komunikacija nikada ne postaje usko grlo",
        "q3_p": "Prenos agenata između procesa košta najviše {worst_comm:.1f}% "
                "koraka, i <em>opada</em> kako roj raste: posao raste brže nego "
                "granice. Čekanje da ostali procesi završe košta do "
                "{worst_wait:.0f}%.",
        "q3_callout_h": "Razdvajanje čekanja od komunikacije",
        "q3_callout_p": "Ovo dvoje je lako pomešati, a zamena obrće zaključak. "
                        "Proces koji ranije završi stoji unutar poziva za prijem "
                        "čekajući sporijeg suseda, a naivno merenje to čekanje "
                        "uračunava u komunikaciju. Izvršavanja sa opcijom "
                        "<code>--measure-phases</code> uvode barijeru posle "
                        "računanja a pre komunikacije, koja upija čekanje, pa ono "
                        "što ostane zaista jeste prenos podataka. Ta barijera košta "
                        "nešto brzine, pa se koristi samo za ovu raspodelu, nikada "
                        "za prikaz ubrzanja iznad.",
        "q4": "Pitanje 4 &middot; ravnoteža",
        "q4_h": "Ubrzanje prati ravnomernost rasporeda agenata",
        "q4_p": "Svaka tačka je jedna veličina roja pri jednom broju procesa. Taj "
                "odnos je čitav rezultat: što su agenti neravnomernije raspoređeni "
                "po trakama, to podela manje pomaže. Jata se skupljaju, a "
                "nepromenljiva podela prostora ne može da ih prati.",
        "q4_callout_h": "Mereno tokom celog izvršavanja, ne na kraju",
        "q4_callout_p": "Neravnoteža se uzorkuje kroz celo izvršavanje umesto da se "
                        "očita jednom na kraju. Razlika je bitna: izvršavanje koje "
                        "ostaje ravnomerno do poslednjeg trenutka i ono koje se "
                        "pokvari odmah izgledaju isto na cilju, a koštaju sasvim "
                        "različito. Dve veličine roja ovde završavaju sa gotovo "
                        "istom neravnotežom a skaliraju se znatno različito, što "
                        "objašnjava jedino merenje tokom izvršavanja.",
        "lim": "Ograničenja",
        "lim_h": "Šta ovi brojevi ne pokazuju",
        "lim_p1": "Svako izvršavanje koristi najviše {biggest} procesa na jednoj "
                  "mašini sa deset jezgara. Preko toga procesi dele jezgra, pa "
                  "merenja opisuju operativni sistem umesto simulacije, zbog čega "
                  "skripta odbija da radi sa više procesa nego jezgara.",
        "lim_p2": "Reč je dakle o procesima koji dele istu memorijsku magistralu, a "
                  "ne o odvojenim mašinama povezanim mrežom. Komunikacija je ovde "
                  "jeftinija nego što bi bila na klasteru, pa bi trenutak u kome "
                  "ona počinje da bude bitna na stvarnom distribuiranom hardveru "
                  "nastupio ranije nego što ove brojke pokazuju.",
        "lim_p3": "Svet ostaje iste veličine dok roj raste, pa je veći roj ujedno i "
                  "gušći: svaki agent ima više suseda, a posao raste približno sa "
                  "kvadratom veličine roja umesto linearno.",
        "table_speedup": "Ubrzanje, sve konfiguracije",
        "table_phases": "Udeo koraka pri {biggest} procesa",
        "table_imbalance": "Prosečna neravnoteža, sve konfiguracije",
        "col_agents": "agenti",
        # Serbian agrees the noun with the number: one proces, two procesa.
        "col_proc": "{count} procesa",
        "col_proc_one": "1 proces",
        "thousands": ".",
    },
}

# The language the page is written in. Set once by main(); every string on the
# page comes from here rather than being written inline.
TEXT = STRINGS["en"]


def process_column(count):
    """Column heading for a process count, with the noun agreed in number."""
    if count == 1:
        return TEXT["col_proc_one"]
    return TEXT["col_proc"].format(count=count)


def number(value):
    """A whole number grouped the way the page's language groups them.

    English writes 16,000 and Serbian writes 16.000. Getting this wrong is
    small but obvious in a thesis figure.
    """
    return f"{value:,}".replace(",", TEXT["thousands"])


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
           f'aria-label="{TEXT["aria_speedup"]}">']

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
               f'text-anchor="middle">{TEXT["processes"]}</text>')

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
                       f'data-label="{TEXT["tip_speedup"].format(agents=agents, count=count, value=speedups[(agents, count)])}"/>')
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
           f'aria-label="{TEXT["aria_phases"]}">']
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
        for slot, (value, name) in enumerate(zip(phases[agents], TEXT["phases"])):
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
                       f'data-label="{TEXT["tip_phase"].format(agents=agents, name=name, value=value)}"/>')
            cursor = y
        out.append(f'<text class="tick" x="{centre:.1f}" y="{bottom + 20}" '
                   f'text-anchor="middle">{number(agents)}</text>')
    out.append(f'<text class="axis" x="{(left + right) / 2:.0f}" y="{height - 8}" '
               f'text-anchor="middle">{TEXT["agents_at"].format(count=count)}</text>')
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
           f'aria-label="{TEXT["aria_scatter"]}">']
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
               f'text-anchor="middle">{TEXT["imbalance_axis"]}</text>')

    for value, speed, (agents, count) in sorted(pairs):
        out.append(f'<circle class="dot s0" cx="{x_of(value):.1f}" cy="{y_of(speed):.1f}" '
                   f'r="5" data-label="{TEXT["tip_scatter"].format(agents=agents, count=count, speed=speed, value=value)}"/>')
    out.append("</svg>")
    return "\n".join(out)


# ------------------------------------------------------------------ page ----

def swatches(names, slots):
    return "".join(
        f'<span class="key"><i class="s{slot}"></i>{name}</span>'
        for slot, name in zip(slots, names)
    )


def html_table_body(headings, rows):
    """The table that carries every value the charts only show as a shape."""
    head = "".join(f"<th>{h}</th>" for h in headings)
    body = "".join("<tr>" + "".join(f"<td>{cell}</td>" for cell in row) + "</tr>"
                   for row in rows)
    return f"<table><thead><tr>{head}</tr></thead><tbody>{body}</tbody></table>"


def build_page(grouped, steps, sizes, processes):
    speedups = speedup_table(grouped, sizes, processes)
    imbalance = imbalance_table(grouped, sizes, processes)
    biggest = max(processes)
    phases = phase_table(grouped, sizes, biggest)

    light = "".join(f"  --s{i}: {c};\n" for i, c in enumerate(LIGHT))
    dark = "".join(f"    --s{i}: {c};\n" for i, c in enumerate(DARK))
    # `fill` is an SVG property and does nothing to an HTML element, so the
    # legend swatch needs `background` — with `fill` alone it renders as an
    # invisible box and the legend loses its colours.
    series_css = "".join(
        f".s{i} {{ stroke: var(--s{i}); }} "
        f"rect.s{i}, circle.s{i} {{ fill: var(--s{i}); }} "
        f".key i.s{i} {{ background: var(--s{i}); }} "
        f"text.s{i} {{ fill: var(--ink-2); stroke: none; }}"
        for i in range(5)
    )

    speed_rows = [[number(a)] + [f"{speedups.get((a, c), 0):.2f}x" for c in processes]
                  for a in sizes]
    imbalance_rows = [[number(a)] + [f"{imbalance.get((a, c), 0):.2f}x" for c in processes]
                      for a in sizes]
    phase_rows = [[number(a)] + [f"{v:.1f}%" for v in phases[a]]
                  for a in sizes if a in phases]

    small, large = sizes[0], sizes[-1]
    best_small = max(speedups[(small, c)] for c in processes)
    best_large = max(speedups[(large, c)] for c in processes)
    worst_comm = max(phases[a][2] for a in phases)
    worst_wait = max(phases[a][1] for a in phases)

    return f"""<!doctype html>
<html lang="{TEXT['lang']}"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{TEXT['title']}</title>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link rel="stylesheet" href="https://fonts.googleapis.com/css2?\
family=Newsreader:opsz,wght@6..72,400;6..72,500;6..72,600&\
family=IBM+Plex+Mono:wght@400;500&display=swap">
<style>
:root {{
  color-scheme: light;
  --paper: #f7f8f7;
  --raised: #ffffff;
  --ink: #12161a;
  --ink-2: #5a6570;
  --ink-3: #8b949c;
  --rule: #dfe3e2;
  --accent: #2a78d6;
  --surface: var(--raised);
  --grid: #e6eae9;
{light}}}
@media (prefers-color-scheme: dark) {{
  :root:not([data-theme="light"]) {{
    color-scheme: dark;
    --paper: #14171a;
    --raised: #1a1e21;
    --ink: #f2f4f3;
    --ink-2: #a7b0b6;
    --ink-3: #71797f;
    --rule: #2b3033;
    --accent: #5c9ee8;
    --surface: var(--raised);
    --grid: #2b3033;
{dark}  }}
}}
:root[data-theme="dark"] {{
  color-scheme: dark;
  --paper: #14171a;
  --raised: #1a1e21;
  --ink: #f2f4f3;
  --ink-2: #a7b0b6;
  --ink-3: #71797f;
  --rule: #2b3033;
  --accent: #5c9ee8;
  --surface: var(--raised);
  --grid: #2b3033;
{dark}}}

* {{ box-sizing: border-box; }}
body {{
  margin: 0;
  background: var(--paper);
  color: var(--ink);
  font-family: Newsreader, Georgia, "Times New Roman", serif;
  font-size: 18px;
  line-height: 1.62;
  -webkit-font-smoothing: antialiased;
}}
main {{ max-width: 46rem; margin: 0 auto; padding: 4.5rem 1.5rem 6rem;
  display: flex; flex-direction: column; gap: 3.5rem; }}
header {{ display: flex; flex-direction: column; gap: 0.9rem;
  border-bottom: 1px solid var(--rule); padding-bottom: 2.25rem; }}
.eyebrow {{ font-family: "IBM Plex Mono", ui-monospace, Menlo, monospace;
  font-size: 0.72rem; letter-spacing: 0.14em; text-transform: uppercase;
  color: var(--ink-3); }}
h1 {{ font-size: clamp(2.1rem, 5vw, 2.9rem); font-weight: 500; line-height: 1.12;
  letter-spacing: -0.02em; margin: 0; text-wrap: balance; }}
.standfirst {{ color: var(--ink-2); font-size: 1.12rem; margin: 0; max-width: 34rem; }}
.meta {{ display: flex; flex-wrap: wrap; gap: 1.4rem; margin: 0.4rem 0 0;
  font-family: "IBM Plex Mono", ui-monospace, Menlo, monospace;
  font-size: 0.78rem; color: var(--ink-3); }}
section {{ display: flex; flex-direction: column; gap: 0.9rem; }}
h2 {{ font-size: 1.6rem; font-weight: 500; line-height: 1.2; margin: 0;
  letter-spacing: -0.015em; text-wrap: balance; }}
p {{ margin: 0; }}
.lede {{ color: var(--ink-2); }}
figure {{ margin: 0.6rem 0 0; background: var(--raised); border: 1px solid var(--rule);
  border-radius: 10px; padding: 1.25rem 1.25rem 0.9rem; }}
svg {{ width: 100%; height: auto; overflow: visible; display: block; }}
.grid {{ stroke: var(--grid); stroke-width: 1; }}
.tick, .axis, .direct {{ font-family: "IBM Plex Mono", ui-monospace, Menlo, monospace;
  font-size: 11px; fill: var(--ink-3); }}
.axis {{ font-size: 12px; fill: var(--ink-2); }}
.series {{ fill: none; stroke-width: 2; stroke-linejoin: round; stroke-linecap: round; }}
.dot {{ stroke: var(--surface); stroke-width: 2; }}
{series_css}
.legend {{ display: flex; flex-wrap: wrap; gap: 0.35rem 1.1rem; margin: 0.85rem 0 0;
  font-family: "IBM Plex Mono", ui-monospace, Menlo, monospace;
  font-size: 0.75rem; color: var(--ink-2); }}
.key {{ display: inline-flex; align-items: center; gap: 0.4rem; }}
.key i {{ width: 10px; height: 10px; border-radius: 2px; display: inline-block; }}
details {{ margin: 0.55rem 0 0; }}
summary {{ cursor: pointer; font-family: "IBM Plex Mono", ui-monospace, Menlo, monospace;
  font-size: 0.75rem; color: var(--ink-3); }}
summary:focus-visible {{ outline: 2px solid var(--accent); outline-offset: 3px; }}
.scroller {{ overflow-x: auto; }}
table {{ border-collapse: collapse; margin: 0.7rem 0 0.2rem; width: 100%;
  font-family: "IBM Plex Mono", ui-monospace, Menlo, monospace;
  font-size: 0.76rem; font-variant-numeric: tabular-nums; }}
th, td {{ text-align: right; padding: 0.32rem 0.7rem; white-space: nowrap;
  border-bottom: 1px solid var(--rule); }}
th {{ color: var(--ink-3); font-weight: 500; }}
th:first-child, td:first-child {{ text-align: left; }}
.callout {{ background: var(--raised); border: 1px solid var(--rule);
  border-radius: 10px; padding: 1.15rem 1.35rem; display: flex;
  flex-direction: column; gap: 0.6rem; }}
.callout h3 {{ margin: 0; font-size: 1rem; font-weight: 600; }}
.callout p {{ color: var(--ink-2); font-size: 0.95rem; }}
code {{ font-family: "IBM Plex Mono", ui-monospace, Menlo, monospace;
  font-size: 0.86em; color: var(--ink); }}
#tip {{ position: fixed; pointer-events: none; opacity: 0; z-index: 10;
  background: var(--ink); color: var(--paper); padding: 0.35rem 0.6rem;
  border-radius: 5px; font-family: "IBM Plex Mono", ui-monospace, Menlo, monospace;
  font-size: 0.74rem; transition: opacity .1s; }}
@media (prefers-reduced-motion: reduce) {{ * {{ transition: none !important; }} }}
</style></head>
<body>
<main>

<header>
  <span class="eyebrow">{TEXT['eyebrow_report']}</span>
  <h1>{TEXT['h1']}</h1>
  <p class="standfirst">{TEXT['standfirst']}</p>
  <div class="meta">
    <span>{TEXT['meta_sizes'].format(n=len(sizes))}</span>
    <span>{TEXT['meta_processes'].format(first=processes[0], last=biggest)}</span>
    <span>{TEXT['meta_steps'].format(steps=steps)}</span>
    <span>{TEXT['meta_medians']}</span>
  </div>
</header>

<section>
  <span class="eyebrow">{TEXT['q1']}</span>
  <h2>{TEXT['q1_h']}</h2>
  <p class="lede">{TEXT['q1_p']}</p>
  <div class="callout">
    <h3>{TEXT['q1_callout_h']}</h3>
    <p>{TEXT['q1_callout_p']}</p>
  </div>
</section>

<section>
  <span class="eyebrow">{TEXT['q2']}</span>
  <h2>{TEXT['q2_h']}</h2>
  <p class="lede">{TEXT['q2_p'].format(small=number(small), large=number(large), best_small=best_small, best_large=best_large, biggest=biggest)}</p>
  <figure>
    {line_chart(speedups, sizes, processes)}
    <div class="legend">{swatches([TEXT["agents_label"].format(agents=number(a)) for a in sizes], range(len(sizes)))}</div>
  </figure>
  <details><summary>{TEXT['table_speedup']}</summary>
  <div class="scroller">{html_table_body([TEXT["col_agents"]] + [process_column(c) for c in processes], speed_rows)}</div>
  </details>
</section>

<section>
  <span class="eyebrow">{TEXT['q3']}</span>
  <h2>{TEXT['q3_h']}</h2>
  <p class="lede">{TEXT['q3_p'].format(worst_comm=worst_comm, worst_wait=worst_wait)}</p>
  <figure>
    {stacked_bars(phases, sizes, biggest)}
    <div class="legend">{swatches(TEXT["phases"], range(4))}</div>
  </figure>
  <div class="callout">
    <h3>{TEXT['q3_callout_h']}</h3>
    <p>{TEXT['q3_callout_p']}</p>
  </div>
  <details><summary>{TEXT['table_phases'].format(biggest=biggest)}</summary>
  <div class="scroller">{html_table_body([TEXT["col_agents"]] + TEXT["phases"], phase_rows)}</div>
  </details>
</section>

<section>
  <span class="eyebrow">{TEXT['q4']}</span>
  <h2>{TEXT['q4_h']}</h2>
  <p class="lede">{TEXT['q4_p']}</p>
  <figure>{scatter(speedups, imbalance, sizes, processes)}</figure>
  <div class="callout">
    <h3>{TEXT['q4_callout_h']}</h3>
    <p>{TEXT['q4_callout_p']}</p>
  </div>
  <details><summary>{TEXT['table_imbalance']}</summary>
  <div class="scroller">{html_table_body([TEXT["col_agents"]] + [process_column(c) for c in processes], imbalance_rows)}</div>
  </details>
</section>

<section>
  <span class="eyebrow">{TEXT['lim']}</span>
  <h2>{TEXT['lim_h']}</h2>
  <p class="lede">{TEXT['lim_p1'].format(biggest=biggest)}</p>
  <p class="lede">{TEXT['lim_p2']}</p>
  <p class="lede">{TEXT['lim_p3']}</p>
</section>

</main>
<div id="tip" role="status"></div>
<script>
const tip = document.getElementById('tip');
for (const mark of document.querySelectorAll('[data-label]')) {{
  mark.addEventListener('pointerenter', () => {{
    tip.textContent = mark.dataset.label;
    tip.style.opacity = '1';
  }});
  mark.addEventListener('pointermove', event => {{
    tip.style.left = (event.clientX + 14) + 'px';
    tip.style.top = (event.clientY - 12) + 'px';
  }});
  mark.addEventListener('pointerleave', () => {{ tip.style.opacity = '0'; }});
}}
</script>
</body></html>
"""


def main():
    global TEXT
    arguments = [a for a in sys.argv[1:] if not a.startswith("--")]
    if "--serbian" in sys.argv:
        # The thesis is written in Serbian, so a chart screenshotted into it
        # has to be labelled in Serbian too.
        TEXT = STRINGS["sr"]

    grouped, steps = load()
    sizes, processes = sizes_and_processes(grouped)
    destination = Path(arguments[0]) if arguments else Path("data/analysis.html")
    destination.write_text(build_page(grouped, steps, sizes, processes))
    print(f"charts written to {destination}")


if __name__ == "__main__":
    main()
