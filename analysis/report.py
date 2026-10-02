#!/usr/bin/env python3
"""Builds a page of charts from a results file.

    python3 analysis/report.py data/analysis.html
    python3 analysis/report.py --serbian data/analysis-sr.html

Reads its numbers from `results.py`, which is where the tables live. Kept
separate so the numbers can be used without the page, and the page can be
changed without touching them.

Uses nothing outside Python's standard library.
"""

import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

from results import (  # noqa: E402  (path has to be set first)
    CONSTANT_DENSITY,
    FIXED_WORLD,
    WEAK_PER_PROCESS,
    crowding_table,
    imbalance_table,
    load,
    middle,
    phase_table,
    sizes_and_processes,
    speedup_table,
    throughput_table,
    weak_table,
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
        "lim_p3": "Questions 2 to 4 use the fixed world, where a larger swarm is "
                  "also a denser one. Question 5 shows that this changes how long "
                  "a step takes far more than how well it splits.",
        "meta_world": "questions 2&ndash;4 in a fixed 1000&times;1000 world",
        "setup_fixed": "fixed 1000&times;1000 world",
        "setup_grown": "world grown with the swarm",
        "tip_setup": "{setup}, {agents} agents: {value}",
        "agents_one_process": "agents (one process, time per step)",
        "aria_crowding_time": "Time per step against swarm size, in the fixed world and with constant density",
        "aria_crowding_speedup": "Speedup against swarm size, in the fixed world and with constant density",
        "q5": "Question 5 &middot; crowding",
        "q5_h": "Crowding makes every step slower, not harder to split",
        "q5_p": "In the fixed world a bigger swarm is also a more crowded one. "
                "Growing the world with the swarm keeps every agent at the same "
                "number of neighbours as {smallest} agents in 1000&times;1000, so "
                "the two can be told apart. With one process, each doubling of "
                "the swarm makes a step {growth_fixed:.1f}&times; longer in the "
                "fixed world and {growth_grown:.1f}&times; longer with room to "
                "spread out. At {agents} agents that is {crowded_ms:.1f} ms against "
                "{roomy_ms:.1f} ms a step: crowding alone makes it "
                "{times_slower:.1f}&times; slower.",
        "q5_p2": "Splitting the work helps about as much either way. At {count} "
                 "processes and {agents} agents the speedup is "
                 "{crowded_speedup:.2f}&times; in the fixed world and "
                 "{roomy_speedup:.2f}&times; with constant density, and the two "
                 "stay close at most sizes. How well the work splits follows how "
                 "many agents there are, not how crowded they are.",
        "q5_callout_h": "Why crowding changes the cost and not the speedup",
        "q5_callout_p": "Crowding gives every agent more neighbours to look at, "
                        "wherever it is, so every process&rsquo;s share of the "
                        "work grows by about the same factor and the ratio between "
                        "one process and ten stays put. What does change is how "
                        "much computing there is to set the communication against: "
                        "at {agents} agents communicating takes {crowded_comm:.1f}% "
                        "of a step in the fixed world and {roomy_comm:.1f}% with "
                        "constant density.",
        "table_crowding": "Fixed world / constant density, at {count} processes",
        "col_ms": "ms per step, 1 process",
        "col_speedup": "speedup",
        "col_imbalance": "imbalance",
        "col_waiting": "waiting %",
        "col_communicating": "communicating %",
        "q6": "Question 6 &middot; weak scaling",
        "q6_h": "{biggest} processes do {roomy_gain:.1f}&times; the work of one, not {biggest}&times;",
        "q6_p": "Here each process always holds {per_process} agents, so if "
                "splitting the work were free a step would take as long at every "
                "process count: 100% on this chart. With constant density, "
                "{biggest} processes keep {roomy_efficiency:.0f}% of that. In the "
                "fixed world they keep {crowded_efficiency:.0f}%, because the "
                "extra agents also crowd every process&rsquo;s own agents, so "
                "each process&rsquo;s work grows even though its agent count "
                "does not.",
        "q6_p2": "Throughput is the number of agents moved forward one step each "
                 "second. With constant density it rises from "
                 "{roomy_one:.2f} million at one process to {roomy_many:.2f} "
                 "million at {biggest}, {roomy_gain:.1f}&times; as much. In the "
                 "fixed world it goes from {crowded_one:.2f} to "
                 "{crowded_many:.2f} million: the extra processes are used up "
                 "by the extra crowding.",
        "q6_callout_h": "Where the rest goes",
        "q6_callout_p": "The gap between {roomy_gain:.1f}&times; and "
                        "{biggest}&times; is the cost of splitting the work: "
                        "processes waiting at the end of each step for the "
                        "busiest one, and the border copies sent between "
                        "neighbours. Question 3 shows that waiting is by far "
                        "the larger of the two.",
        "ideal": "if splitting were free",
        "aria_weak": "Weak-scaling efficiency against process count, in the fixed world and with constant density",
        "aria_throughput": "Million agent-steps per second against process count, in the fixed world and with constant density",
        "tip_weak": "{setup}, {count} processes: {value}",
        "table_weak": "Weak scaling, every configuration",
        "table_throughput": "Throughput, million agent-steps per second",
        "col_per_process": "agents per process",
        "col_setup": "setup",
        "col_ms_step": "ms per step",
        "col_efficiency": "efficiency",
        "col_throughput": "million agent-steps/s",
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
        "lim_p3": "Pitanja 2 do 4 koriste nepromenljiv svet, u kome je veći roj "
                  "ujedno i gušći. Pitanje 5 pokazuje da to mnogo više menja "
                  "trajanje koraka nego to koliko se dobro posao deli.",
        "meta_world": "pitanja 2&ndash;4 u nepromenljivom svetu 1000&times;1000",
        "setup_fixed": "nepromenljiv svet 1000&times;1000",
        "setup_grown": "svet koji raste sa rojem",
        "tip_setup": "{setup}, {agents} agenata: {value}",
        "agents_one_process": "agenti (jedan proces, vreme po koraku)",
        "aria_crowding_time": "Vreme po koraku u odnosu na veličinu roja, u nepromenljivom svetu i pri stalnoj gustini",
        "aria_crowding_speedup": "Ubrzanje u odnosu na veličinu roja, u nepromenljivom svetu i pri stalnoj gustini",
        "q5": "Pitanje 5 &middot; gustina",
        "q5_h": "Gustina usporava svaki korak, ali ne otežava podelu",
        "q5_p": "U nepromenljivom svetu veći roj je ujedno i gušći. Kada svet "
                "raste zajedno sa rojem, svaki agent ima isti broj suseda kao "
                "{smallest} agenata u svetu 1000&times;1000, pa se ta dva efekta "
                "mogu razdvojiti. Sa jednim procesom, svako udvostručavanje roja "
                "produžava korak {growth_fixed:.1f}&times; u nepromenljivom svetu i "
                "{growth_grown:.1f}&times; kada roj ima mesta da se raširi. Pri "
                "{agents} agenata to je {crowded_ms:.1f} ms naspram "
                "{roomy_ms:.1f} ms po koraku: sama gustina usporava korak "
                "{times_slower:.1f}&times;.",
        "q5_p2": "Podela posla pomaže približno isto u oba slučaja. Pri {count} "
                 "procesa i {agents} agenata ubrzanje je "
                 "{crowded_speedup:.2f}&times; u nepromenljivom svetu i "
                 "{roomy_speedup:.2f}&times; pri stalnoj gustini, a te dve "
                 "vrednosti ostaju bliske kod većine veličina. Koliko se dobro "
                 "posao deli zavisi od broja agenata, a ne od toga koliko su "
                 "zbijeni.",
        "q5_callout_h": "Zašto gustina menja cenu, a ne ubrzanje",
        "q5_callout_p": "Gustina daje svakom agentu više suseda, gde god da se "
                        "nalazi, pa udeo posla svakog procesa raste približno "
                        "istim faktorom i odnos između jednog i deset procesa "
                        "ostaje isti. Ono što se menja je koliko računanja ima "
                        "naspram komunikacije: pri {agents} agenata komunikacija "
                        "zauzima {crowded_comm:.1f}% koraka u nepromenljivom "
                        "svetu i {roomy_comm:.1f}% pri stalnoj gustini.",
        "table_crowding": "Nepromenljiv svet / stalna gustina, pri {count} procesa",
        "col_ms": "ms po koraku, 1 proces",
        "col_speedup": "ubrzanje",
        "col_imbalance": "neravnoteža",
        "col_waiting": "čekanje %",
        "col_communicating": "komunikacija %",
        "q6": "Pitanje 6 &middot; slabo skaliranje",
        "q6_h": "{biggest} procesa uradi {roomy_gain:.1f}&times; više posla od jednog, a ne {biggest}&times;",
        "q6_p": "Ovde svaki proces uvek ima {per_process} agenata, pa bi, kada bi "
                "podela posla bila besplatna, korak trajao isto pri svakom broju "
                "procesa: 100% na ovom grafikonu. Pri stalnoj gustini {biggest} "
                "procesa zadržava {roomy_efficiency:.0f}% od toga. U "
                "nepromenljivom svetu zadržava {crowded_efficiency:.0f}%, jer "
                "dodatni agenti zbijaju i agente svakog procesa, pa posao "
                "svakog procesa raste iako broj njegovih agenata ostaje isti.",
        "q6_p2": "Propusnost je broj agenata pomerenih za jedan korak u sekundi. "
                 "Pri stalnoj gustini raste sa {roomy_one:.2f} miliona pri "
                 "jednom procesu na {roomy_many:.2f} miliona pri {biggest}, "
                 "{roomy_gain:.1f}&times; više. U nepromenljivom svetu ide sa "
                 "{crowded_one:.2f} na {crowded_many:.2f} miliona: dodatne "
                 "procese potroši dodatna gustina.",
        "q6_callout_h": "Gde odlazi ostatak",
        "q6_callout_p": "Razlika između {roomy_gain:.1f}&times; i "
                        "{biggest}&times; je cena podele posla: procesi na kraju "
                        "svakog koraka čekaju najopterećenijeg, a susedi "
                        "razmenjuju kopije graničnih agenata. Pitanje 3 "
                        "pokazuje da je čekanje daleko veće od to dvoje.",
        "ideal": "kada bi podela bila besplatna",
        "aria_weak": "Efikasnost slabog skaliranja u odnosu na broj procesa, u nepromenljivom svetu i pri stalnoj gustini",
        "aria_throughput": "Miliona agent-koraka u sekundi u odnosu na broj procesa, u nepromenljivom svetu i pri stalnoj gustini",
        "tip_weak": "{setup}, {count} procesa: {value}",
        "table_weak": "Slabo skaliranje, sve konfiguracije",
        "table_throughput": "Propusnost, miliona agent-koraka u sekundi",
        "col_per_process": "agenata po procesu",
        "col_setup": "postavka",
        "col_ms_step": "ms po koraku",
        "col_efficiency": "efikasnost",
        "col_throughput": "miliona agent-koraka/s",
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


# The narrowest space, in chart units, that two axis labels like "10,000" need
# between their centres so they do not overlap.
MINIMUM_LABEL_GAP = 48


def setup_chart(series, label_of_value, axis_label, aria, tip, logarithmic):
    """One line per setup, against swarm size.

    `series` is a list of (setup name, {agents: value}), fixed world first.
    Swarm sizes run from 1000 to a million, so they are spaced by their
    logarithm: an even step along the axis is the same multiple of agents.
    Time per step spans as wide a range, so `logarithmic` spaces the values
    the same way; speedup does not need it.

    The two setups are told apart by line style, solid against dashed, rather
    than colour. The colours on this page already stand for swarm sizes.
    """
    width, height = 620, 360
    pad = {"left": 64, "top": 16, "right": 24, "bottom": 48}
    left, top, right, bottom = axes(width, height, pad)
    sizes = sorted({agents for _, values in series for agents in values})
    values = [value for _, by_size in series for value in by_size.values()]

    def x_of(agents):
        if len(sizes) == 1:
            return (left + right) / 2
        span = math.log10(sizes[-1]) - math.log10(sizes[0])
        return left + (math.log10(agents) - math.log10(sizes[0])) / span * (right - left)

    if logarithmic:
        lowest = 10 ** math.floor(math.log10(min(values)))
        highest = 10 ** math.ceil(math.log10(max(values)))
        ticks = [lowest * 10 ** power
                 for power in range(round(math.log10(highest / lowest)) + 1)]

        def y_of(value):
            share = math.log10(value / lowest) / math.log10(highest / lowest)
            return bottom - share * (bottom - top)
    else:
        highest = max(2.0, max(values) * 1.1)
        ticks = list(range(0, int(highest) + 1))

        def y_of(value):
            return bottom - value / highest * (bottom - top)

    out = [f'<svg viewBox="0 0 {width} {height}" role="img" aria-label="{aria}">']
    for tick in ticks:
        y = y_of(tick)
        out.append(f'<line class="grid" x1="{left}" y1="{y:.1f}" x2="{right}" y2="{y:.1f}"/>')
        out.append(f'<text class="tick" x="{left - 8}" y="{y + 4:.1f}" '
                   f'text-anchor="end">{label_of_value(tick)}</text>')
    # Sizes close together, like 8000 and 10000, would print on top of each
    # other, so a label too near the last one is left off. Its point is still
    # drawn and its tooltip still names it.
    last_labelled = None
    for agents in sizes:
        x = x_of(agents)
        if last_labelled is not None and x - last_labelled < MINIMUM_LABEL_GAP:
            continue
        out.append(f'<text class="tick" x="{x:.1f}" y="{bottom + 20}" '
                   f'text-anchor="middle">{number(agents)}</text>')
        last_labelled = x
    out.append(f'<text class="axis" x="{(left + right) / 2:.0f}" y="{height - 8}" '
               f'text-anchor="middle">{axis_label}</text>')

    for style, (name, by_size) in zip(["fixed", "grown"], series):
        points = [(x_of(agents), y_of(by_size[agents]), agents)
                  for agents in sorted(by_size)]
        path = " ".join(f"{'M' if i == 0 else 'L'}{x:.1f} {y:.1f}"
                        for i, (x, y, _) in enumerate(points))
        out.append(f'<path class="setup {style}" d="{path}"/>')
        for x, y, agents in points:
            label = tip.format(setup=name, agents=number(agents),
                               value=label_of_value(by_size[agents]))
            out.append(f'<circle class="setup-dot {style}" cx="{x:.1f}" cy="{y:.1f}" '
                       f'r="4" data-label="{label}"/>')
    out.append("</svg>")
    return "\n".join(out)


def process_chart(series, label_of_value, aria, tip, reference=None, value_heading=None):
    """One line per setup, against process count.

    `series` is a list of (setup name, {process count: value}), fixed world
    first, drawn solid and dashed like `setup_chart`. `reference`, if given,
    is a value to draw as a thin dotted line across the chart, such as the
    100% a perfect result would reach. `value_heading`, if given, is written
    above the chart to say what the vertical axis counts, for values whose
    tick labels cannot carry their own unit.
    """
    width, height = 620, 360
    pad = {"left": 64, "top": 36 if value_heading else 16, "right": 24, "bottom": 48}
    left, top, right, bottom = axes(width, height, pad)
    processes = sorted({count for _, values in series for count in values})
    values = [value for _, by_count in series for value in by_count.values()]
    if reference is not None:
        values.append(reference)
    highest = max(values) * 1.1
    # Four or five gridlines whatever the range, on round numbers.
    spacing = 10 ** math.floor(math.log10(highest / 4))
    for multiple in (1, 2, 5, 10):
        if highest / (spacing * multiple) <= 5:
            spacing *= multiple
            break

    def x_of(count):
        return left + (count - processes[0]) / (processes[-1] - processes[0]) * (right - left)

    def y_of(value):
        return bottom - value / highest * (bottom - top)

    out = [f'<svg viewBox="0 0 {width} {height}" role="img" aria-label="{aria}">']
    if value_heading:
        out.append(f'<text class="axis" x="{left - 8}" y="12">{value_heading}</text>')
    tick = 0.0
    while tick <= highest:
        y = y_of(tick)
        out.append(f'<line class="grid" x1="{left}" y1="{y:.1f}" x2="{right}" y2="{y:.1f}"/>')
        out.append(f'<text class="tick" x="{left - 8}" y="{y + 4:.1f}" '
                   f'text-anchor="end">{label_of_value(tick)}</text>')
        tick += spacing
    for count in processes:
        out.append(f'<text class="tick" x="{x_of(count):.1f}" y="{bottom + 20}" '
                   f'text-anchor="middle">{count}</text>')
    out.append(f'<text class="axis" x="{(left + right) / 2:.0f}" y="{height - 8}" '
               f'text-anchor="middle">{TEXT["processes"]}</text>')
    if reference is not None:
        y = y_of(reference)
        out.append(f'<line class="reference" x1="{left}" y1="{y:.1f}" x2="{right}" y2="{y:.1f}"/>')

    for style, (name, by_count) in zip(["fixed", "grown"], series):
        points = [(x_of(count), y_of(by_count[count]), count) for count in sorted(by_count)]
        path = " ".join(f"{'M' if i == 0 else 'L'}{x:.1f} {y:.1f}"
                        for i, (x, y, _) in enumerate(points))
        out.append(f'<path class="setup {style}" d="{path}"/>')
        for x, y, count in points:
            label = tip.format(setup=name, count=count, value=label_of_value(by_count[count]))
            out.append(f'<circle class="setup-dot {style}" cx="{x:.1f}" cy="{y:.1f}" '
                       f'r="4" data-label="{label}"/>')
    out.append("</svg>")
    return "\n".join(out)


def setup_swatches():
    return (f'<span class="key"><i class="line fixed"></i>{TEXT["setup_fixed"]}</span>'
            f'<span class="key"><i class="line grown"></i>{TEXT["setup_grown"]}</span>')


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


def milliseconds_per_step(grouped, steps):
    """One process, for each swarm size: how long a step takes with no splitting."""
    return {key[2]: 1000 * middle(runs, "simulating_seconds") / steps
            for key, runs in grouped.items()
            if key[0] == "sequential" and not key[3]}


def crowding_section(fixed_world, constant_density):
    """The section comparing the fixed world with constant density.

    Empty when there are no constant-density runs to compare against.
    """
    table, count = crowding_table(fixed_world, constant_density)
    if not table:
        return ""
    names = [TEXT["setup_fixed"], TEXT["setup_grown"]]
    timing = [(name, milliseconds_per_step(grouped, steps))
              for name, (grouped, steps) in zip(names, [fixed_world, constant_density])]
    scaling = []
    for name, (grouped, _) in zip(names, [fixed_world, constant_density]):
        sizes, _ = sizes_and_processes(grouped)
        at_count = speedup_table(grouped, sizes, [count])
        scaling.append((name, {agents: value for (agents, _), value in at_count.items()}))

    # How many times longer a step takes each time the swarm doubles, over the
    # sizes measured both ways. About 2 means the work grows in step with the
    # swarm; about 4 would mean it grows with the square.
    shared = sorted(set(timing[0][1]) & set(timing[1][1]))
    doublings = math.log2(shared[-1] / shared[0])
    growth_fixed, growth_grown = (
        (by_size[shared[-1]] / by_size[shared[0]]) ** (1 / doublings)
        for _, by_size in timing)

    # The largest size measured both ways carries the headline numbers.
    largest = max(table)
    crowded, roomy = table[largest]
    rows = []
    for agents, (fixed, grown) in table.items():
        rows.append([number(agents)] + [
            f"{fixed[key]:{shape}} / {grown[key]:{shape}}"
            for key, shape in [("milliseconds_per_step", ".1f"), ("speedup", ".2f"),
                               ("imbalance", ".2f"), ("waiting", ".1f"),
                               ("communicating", ".1f")]])
    headings = [TEXT["col_agents"], TEXT["col_ms"], TEXT["col_speedup"],
                TEXT["col_imbalance"], TEXT["col_waiting"], TEXT["col_communicating"]]
    figures = dict(agents=number(largest), count=count,
                   crowded_ms=crowded["milliseconds_per_step"],
                   roomy_ms=roomy["milliseconds_per_step"],
                   times_slower=crowded["milliseconds_per_step"] / roomy["milliseconds_per_step"],
                   crowded_speedup=crowded["speedup"], roomy_speedup=roomy["speedup"],
                   crowded_imbalance=crowded["imbalance"], roomy_imbalance=roomy["imbalance"],
                   crowded_comm=crowded["communicating"], roomy_comm=roomy["communicating"],
                   growth_fixed=growth_fixed, growth_grown=growth_grown,
                   smallest=number(shared[0]))
    return f"""
<section>
  <span class="eyebrow">{TEXT['q5']}</span>
  <h2>{TEXT['q5_h']}</h2>
  <p class="lede">{TEXT['q5_p'].format(**figures)}</p>
  <figure>
    {setup_chart(timing, lambda value: f"{value:g} ms", TEXT["agents_one_process"],
                 TEXT["aria_crowding_time"], TEXT["tip_setup"], logarithmic=True)}
    <div class="legend">{setup_swatches()}</div>
  </figure>
  <p class="lede">{TEXT['q5_p2'].format(**figures)}</p>
  <figure>
    {setup_chart(scaling, lambda value: f"{value:.2f}x" if value % 1 else f"{value:.0f}x",
                 TEXT["agents_at"].format(count=count), TEXT["aria_crowding_speedup"],
                 TEXT["tip_setup"], logarithmic=False)}
    <div class="legend">{setup_swatches()}</div>
  </figure>
  <div class="callout">
    <h3>{TEXT['q5_callout_h']}</h3>
    <p>{TEXT['q5_callout_p'].format(**figures)}</p>
  </div>
  <details><summary>{TEXT['table_crowding'].format(count=count)}</summary>
  <div class="scroller">{html_table_body(headings, rows)}</div>
  </details>
</section>
"""


def weak_section(fixed_world, constant_density, processes):
    """Weak scaling and throughput, in both setups.

    Charts the largest per-process count measured in both setups, since
    small swarms are dominated by fixed costs. The table has every one.
    Empty when there are no weak-scaling runs.
    """
    setups = [(TEXT["setup_fixed"], fixed_world), (TEXT["setup_grown"], constant_density)]
    tables = {}
    for per_process in WEAK_PER_PROCESS:
        for name, (grouped, steps) in setups:
            table = weak_table(grouped, steps, per_process, processes)
            if table:
                tables[(per_process, name)] = table
    charted = [per_process for per_process in WEAK_PER_PROCESS
               if all((per_process, name) in tables for name, _ in setups)]
    if not charted:
        return ""
    per_process = charted[-1]
    crowded, roomy = (tables[(per_process, name)] for name, _ in setups)
    biggest = max(set(crowded) & set(roomy))

    efficiency = [(name, {count: row["efficiency"] for count, row in tables[(per_process, name)].items()})
                  for name, _ in setups]
    rates = [(name, {count: row["throughput"] / 1e6 for count, row in tables[(per_process, name)].items()})
             for name, _ in setups]

    rows = []
    for (each, name), table in tables.items():
        for count, row in table.items():
            rows.append([number(each), name, count, f"{row['milliseconds_per_step']:.2f}",
                         f"{row['efficiency']:.0f}%", f"{row['throughput'] / 1e6:.2f}"])
    headings = [TEXT["col_per_process"], TEXT["col_setup"], TEXT["processes"],
                TEXT["col_ms_step"], TEXT["col_efficiency"], TEXT["col_throughput"]]
    figures = dict(per_process=number(per_process), biggest=biggest,
                   agents=number(per_process * biggest),
                   crowded_efficiency=crowded[biggest]["efficiency"],
                   roomy_efficiency=roomy[biggest]["efficiency"],
                   crowded_one=crowded[1]["throughput"] / 1e6,
                   roomy_one=roomy[1]["throughput"] / 1e6,
                   crowded_many=crowded[biggest]["throughput"] / 1e6,
                   roomy_many=roomy[biggest]["throughput"] / 1e6,
                   crowded_gain=crowded[biggest]["throughput"] / crowded[1]["throughput"],
                   roomy_gain=roomy[biggest]["throughput"] / roomy[1]["throughput"])
    return f"""
<section>
  <span class="eyebrow">{TEXT['q6']}</span>
  <h2>{TEXT['q6_h'].format(**figures)}</h2>
  <p class="lede">{TEXT['q6_p'].format(**figures)}</p>
  <figure>
    {process_chart(efficiency, lambda value: f"{value:.0f}%", TEXT["aria_weak"],
                   TEXT["tip_weak"], reference=100)}
    <div class="legend">{setup_swatches()}<span class="key"><i class="line reference"></i>{TEXT["ideal"]}</span></div>
  </figure>
  <p class="lede">{TEXT['q6_p2'].format(**figures)}</p>
  <figure>
    {process_chart(rates, lambda value: f"{value:g}", TEXT["aria_throughput"], TEXT["tip_weak"],
                   value_heading=TEXT["col_throughput"])}
    <div class="legend">{setup_swatches()}</div>
  </figure>
  <div class="callout">
    <h3>{TEXT['q6_callout_h']}</h3>
    <p>{TEXT['q6_callout_p'].format(**figures)}</p>
  </div>
  <details><summary>{TEXT['table_weak']}</summary>
  <div class="scroller">{html_table_body(headings, rows)}</div>
  </details>
</section>
"""


def build_page(grouped, steps, sizes, processes, constant_density):
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
    rates = throughput_table(grouped, steps, sizes, processes)
    throughput_rows = [[number(a)] + [f"{rates.get((a, c), 0) / 1e6:.2f}" for c in processes]
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
.setup {{ fill: none; stroke-width: 2; stroke-linejoin: round; stroke-linecap: round; }}
.setup.fixed {{ stroke: var(--ink); }}
.setup.grown {{ stroke: var(--ink-2); stroke-dasharray: 6 4; }}
.setup-dot {{ stroke-width: 2; }}
.setup-dot.fixed {{ fill: var(--ink); stroke: var(--surface); }}
.setup-dot.grown {{ fill: var(--raised); stroke: var(--ink-2); }}
.key i.line {{ width: 18px; height: 2px; border-radius: 0; }}
.key i.line.fixed {{ background: var(--ink); }}
.reference {{ stroke: var(--ink-3); stroke-width: 1; stroke-dasharray: 1 4; }}
.key i.line.reference {{ background: repeating-linear-gradient(90deg,
  var(--ink-3) 0 1px, transparent 1px 5px); }}
.key i.line.grown {{ background: repeating-linear-gradient(90deg,
  var(--ink-2) 0 6px, transparent 6px 10px); }}
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
    <span>{TEXT['meta_world']}</span>
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
  <details><summary>{TEXT['table_throughput']}</summary>
  <div class="scroller">{html_table_body([TEXT["col_agents"]] + [process_column(c) for c in processes], throughput_rows)}</div>
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

{crowding_section((grouped, steps), constant_density)}
{weak_section((grouped, steps), constant_density, processes)}
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

    grouped, steps = load(setup=FIXED_WORLD)
    constant_density = load(setup=CONSTANT_DENSITY)
    sizes, processes = sizes_and_processes(grouped)
    destination = Path(arguments[0]) if arguments else Path("data/analysis.html")
    destination.write_text(build_page(grouped, steps, sizes, processes, constant_density))
    print(f"charts written to {destination}")


if __name__ == "__main__":
    main()
