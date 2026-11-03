"""The druid levelling page: three routes from the hand-written talent orders,
built from the sim's results. Run from the repository root after `page/run.sh`:

    python3 page/page.py      # writes docs/index.html

Talent icons and tooltips come from foreverchanges.pro (the two JSON snapshots
beside this file; icons are downloaded once into page/icons/).
"""
import base64, json, subprocess, time, urllib.request
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent
BIN = ROOT / "target/release/forever"
OUT = ROOT / "out"
DOCS = ROOT / "docs"
load = lambda p: json.loads(p.read_text())
SITE, BOOK = load(HERE / "site_talents.json"), load(HERE / "site_spellbook.json")
BY_NAME = {a["name"]: a for a in SITE.values()}
TALENT = {n for n, s in BOOK.items() if s["talent"]}
FIRST = {n: min((r[2] for r in s["ranks"] if r[2] is not None), default=0) for n, s in BOOK.items()}
TIE = 1.0  # percent: closer than this is the runs' noise
FIRST_TALENT = 10


def icon(slug):
    d = HERE / "icons"
    d.mkdir(exist_ok=True)
    p = d / f"{slug}.jpg"
    if not p.exists():
        req = urllib.request.Request(f"https://foreverchanges.pro/icon/{slug}.jpg", headers={"User-Agent": "wow-forever-sim"})
        p.write_bytes(urllib.request.urlopen(req, timeout=30).read())
        time.sleep(0.3)
    return "data:image/jpeg;base64," + base64.b64encode(p.read_bytes()).decode()


def trees():
    t = json.loads(subprocess.run([BIN, "druid-level", "tree"], capture_output=True, text=True, cwd=ROOT, check=True).stdout)
    tabs = []
    for i, name in enumerate(("Balance", "Feral", "Restoration")):
        mine = [x for x in t["talents"] if x["tab"] == i]
        left = min(x["col"] for x in mine)
        tabs.append({"name": name, "talents": [{
            "name": x["name"], "col": x["col"] - left, "row": x["row"], "max": x["max"], "img": icon(BY_NAME[x["name"]]["icon"]),
            "text": BY_NAME[x["name"]]["rank_texts"].get(str(x["max"]), ""),
        } for x in sorted(mine, key=lambda x: (x["row"], x["col"]))]})
    return tabs, [list(a) for a in t["arrows"]]


def pct(x):
    return f"{round(x * 100)}%"


def rotation(p, b, level):
    """A play in words: its form and mobs, the priority list, the rules beside it."""
    def has(n):
        return b.get(n, 0) > 0 if n in TALENT else FIRST.get(n, 99) <= level
    heal = "Healing Touch" if not has("Rejuvenation") else ("Rejuvenation, Healing Touch" if not has("Regrowth") else "Rejuvenation, Regrowth")
    rules = [f"between pulls, heal below {pct(p['heal_below'])} health up to {pct(p['heal_to'])} ({heal}); eat and drink when mana runs short"]
    steps = []
    if p["form"] == "cat":
        form, mobs = "cat", 1
        if p.get("opener") == 1 and has("Pounce"):
            steps.append("Prowl for the last 15 yards, Pounce from behind (50 energy)")
        if p.get("opener") == 2 and has("Ravage"):
            steps.append("Prowl for the last 15 yards, Ravage from behind (60 energy)")
        if p["faerie_fire"] and has("Faerie Fire"):
            steps.append("Faerie Fire on the mob")
        if p["rip_cp"] and has("Rip"):
            steps.append(f"Rip at {p['rip_cp']}+ combo points if not bleeding")
        if p["bite_cp"] and has("Ferocious Bite"):
            keep = "; leftover energy is kept (Predatory Discipline)" if b.get("Predatory Discipline") else "; spends all energy"
            pool = f", pooling to {p['bite_pool']:.0f} energy first" if p.get("bite_pool") else ""
            steps.append(f"Ferocious Bite at {p['bite_cp']}+ combo points" + pool + (" while Rip is up" if p["rip_cp"] and has("Rip") else "") + keep)
        if has("Rake"):
            steps.append("Rake if not bleeding")
        steps.append("Claw")
        if p.get("shifting") and b.get("Shifting Power"):
            steps.append("Shifting Power when nothing is affordable and 40 energy fits")
        if has("Omen of Clarity"):
            rules.append("Clearcasting: a free Rake or Claw")
        if p["drag"]:
            rules.append("leave a mob its bleeds will kill (it follows and hits) and pull the next")
        if b.get("Feral Charge"):
            rules.append("Feral Charge (Cat) onto the mob from 8–25 yards")
    elif p["form"] == "bear":
        form, mobs = "bear", p["mobs"]
        if p["enrage"] and has("Enrage"):
            rules.append("Enrage at the start of a pull when ready")
        if p["roar"] and has("Demoralizing Roar") and mobs > 1:
            steps.append("Demoralizing Roar while 2+ mobs stand")
        if has("Swipe") and p["swipe_at"] <= mobs:
            steps.append(f"Swipe while {p['swipe_at']}+ mobs stand")
        if b.get("Primal Bite"):
            steps.append("Primal Bite on cooldown")
        if p.get("lacerate") and b.get("Lacerate"):
            steps.append("Lacerate to 5 stacks, then with spare rage" if p["lacerate"] == 6 else f"Lacerate to {p['lacerate']} stacks, refreshed before it ends")
        if p["faerie_fire"] and has("Faerie Fire") and mobs > 1:
            steps.append("Faerie Fire on the other mobs")
        steps.append(f"Maul when {p['maul_spare']:.0f} rage is left after the next Swipe or Primal Bite")
        if p.get("frenzied") and has("Frenzied Regeneration"):
            rules.append(f"Frenzied Regeneration below {pct(p['frenzied'])} health, staying in form")
        if p.get("emergency"):
            rules.append(f"leave form and heal below {pct(p['emergency'])} health in a fight")
        if b.get("Feral Charge"):
            rules.append("Feral Charge onto the first mob from 8–25 yards")
        if mobs > 1:
            rules.append(f"pull the first mob; the other {mobs - 1} run in from 10 yards")
    else:
        moon = p.get("moonkin") and b.get("Moonkin Form")
        form, mobs = "moonkin" if moon else "caster", 1
        if p.get("hurricane") and has("Hurricane"):
            mobs = p["hurricane"]
            steps.append(f"Moonfire the first mob; the other {mobs - 1} come with it from 10 yards")
            if p.get("barkskin") and has("Barkskin"):
                steps.append("Barkskin before channelling when ready (less damage, no lost ticks)")
            steps.append("Hurricane while mana lasts (no cooldown)")
            steps.append("Wrath on what is left; melee when out of mana")
            rules.append("Innervate below 50% mana")
            rules.append("Clearcasting: a free Hurricane" + (" (Moonkin Form doubles Omen's chance)" if moon else ""))
            rules.insert(1, "when sitting down to eat and drink, Read Leyline first if ready")
            return {"form": form, "mobs": mobs, "steps": steps, "rules": rules}
        if p.get("barkskin") and has("Barkskin"):
            rules.append("Barkskin on the way to the pull when ready")
        if p.get("roots") and has("Entangling Roots"):
            steps.append("Entangling Roots on the mob running in")
        if has("Innervate"):
            rules.append("Innervate below 50% mana")
        sf = p["starfire_opener"] and has("Starfire")
        steps.append(("Starfire" if sf else "Wrath") + " to open from 30 yards while the mob runs in")
        if has("Starfire") and has("Omen of Clarity"):
            steps.append("Clearcasting: a free Starfire" + (" (Moonkin Form doubles Omen's chance)" if moon else ""))
        if p["moonfire"] and has("Moonfire"):
            steps.append("Moonfire if not up")
        if p.get("swarm") and b.get("Insect Swarm"):
            steps.append("Insect Swarm if not up")
        steps.append("Wrath")
        if p["melee_below"] > 0:
            steps.insert(0, f"melee with the weapon below {pct(p['melee_below'])} mana")
        rules.append("melee when out of mana")
        if moon:
            rules.append("fight in Moonkin Form; leave it to heal")
        if p["drag"]:
            rules.append("leave a mob its DoTs will kill (it follows and hits) and pull the next")
    rules.insert(1, "when sitting down to eat and drink, Read Leyline first if ready")
    return {"form": form, "mobs": mobs, "steps": steps, "rules": rules}


def route(label, key, r):
    steps, early, rows = r["path"], r["early"], {c["level"]: c for c in r["rows"]}
    respec = r["respec_at"]
    order, gains, prev = [], {}, {}
    for s in steps:
        c = rows[s["level"]]
        new = {k: v for k, v in s["build"].items() if prev.get(k, 0) != v}
        g = c["xp"] / c["before_build"] - 1
        is_respec = s["level"] == respec
        gone = [k for k in prev if k not in s["build"]]
        order.append({"level": s["level"], "new": [f"{k} {v}" for k, v in new.items()], "gone": gone, "gain": g, "respec": is_respec})
        if not is_respec:
            for k in new:
                gains[k] = gains.get(k, 1.0) * (1 + g)
        prev = s["build"]
    # the last level actually played: 60 brings ranks no route spends time with
    end = rows[59]
    phase = lambda c: {"level": c["level"], "build": c["build"], "without": {t: c["xp"] / x - 1 for t, _, x in c["without"]}}
    phases = [phase(rows[respec - 1]), phase(end)] if respec else [phase(end)]
    bought = []
    if respec:
        before, after = rows[respec - 1]["build"], rows[respec]["build"]
        bought = [k for k, v in after.items() if before.get(k, 0) != v]
    rot = [{"level": s["level"], "new": [], "respec": False, "xp": s["xp_per_hour"], "hours": s["hours"], "rot": rotation(s["play"], {}, s["level"])} for s in early]
    prev = {}
    for s in steps:
        new = [f"{k} {v}" for k, v in s["build"].items() if prev.get(k) != v]
        rot.append({"level": s["level"], "new": new, "respec": s["level"] == respec, "xp": rows[s["level"]]["xp"], "hours": s["hours"], "rot": rotation(rows[s["level"]]["play"], s["build"], s["level"])})
        prev = s["build"]
    return {"key": key, "label": label, "hours": r["hours"], "early": sum(s["hours"] for s in early), "respec": respec, "phases": phases,
            "gains": {k: v - 1 for k, v in gains.items()}, "bought": bought, "order": order, "rot": rot}


def compare(f, b):
    get = lambda r: {s["level"]: s for s in r["early"] + r["path"] if s["level"] < 60}
    F, B = get(f), get(b)
    levels = [{"level": l, "f": F[l]["hours"], "b": B[l]["hours"], "fx": F[l]["xp_per_hour"], "bx": B[l]["xp_per_hour"]} for l in sorted(F)]
    def who(r):
        d = r["b"] / r["f"] - 1
        return "tie" if abs(d) * 100 < TIE else ("feral" if d > 0 else "balance")
    def merge(x, y):
        z = {"lo": x["lo"], "hi": y["hi"], "f": x["f"] + y["f"], "b": x["b"] + y["b"]}
        z["who"] = who(z)
        return z
    # before the first talent (10) every druid levels the same way: one row, outside the merging
    early = [{"who": "", "lo": r["level"], "hi": r["level"], "f": r["f"], "b": r["b"]} for r in levels if r["level"] < FIRST_TALENT]
    start = early[0]
    for r in early[1:]:
        start = merge(start, r)
    ranges = [{"who": who(r), "lo": r["level"], "hi": r["level"], "f": r["f"], "b": r["b"]} for r in levels if r["level"] >= FIRST_TALENT]
    # a run of one level is that level's noise: it joins its shorter neighbour, until none is left
    while True:
        out = []
        for r in ranges:
            if out and out[-1]["who"] == r["who"]:
                out[-1] = merge(out[-1], r)
            else:
                out.append(r)
        ranges = out
        short = [i for i, r in enumerate(ranges) if r["hi"] == r["lo"]]
        if not short or len(ranges) == 1:
            break
        i = short[0]
        j = i + 1 if i == 0 else i - 1 if i == len(ranges) - 1 or ranges[i - 1]["hi"] - ranges[i - 1]["lo"] <= ranges[i + 1]["hi"] - ranges[i + 1]["lo"] else i + 1
        lo, hi = min(i, j), max(i, j)
        ranges[lo:hi + 1] = [merge(ranges[lo], ranges[hi])]
    ranges.insert(0, {**start, "early": True})
    for r in ranges:
        r["pct"] = abs(max(r["f"], r["b"]) / min(r["f"], r["b"]) - 1) * 100
    return {"tie": TIE, "ranges": ranges, "levels": levels, "total": {"f": sum(r["f"] for r in levels), "b": sum(r["b"] for r in levels)}}


ROUTES = (("balance", "Balance: Moonkin, Hurricane"), ("feral", "Feral: cat, then bear"), ("cat_feral", "Cat build"))


def main():
    r = {k: load(OUT / f"{k}.json") for k, _ in ROUTES}
    routes = [route(f"{label}, respec at {r[k]['respec_at']}" if r[k]["respec_at"] else label, k, r[k]) for k, label in ROUTES]
    tabs, edges = trees()
    table = lambda a, b, title, names: {"title": title, "names": names, **compare(r[a], r[b])}
    data = {"tabs": tabs, "edges": edges, "routes": routes,
            "compare": [table("feral", "balance", "Feral against Balance", ["Feral", "Balance"]),
                        table("cat_feral", "balance", "Cat build against Balance", ["Cat", "Balance"])]}
    html = (HERE / "template.html").read_text().replace("__DATA__", json.dumps(data, ensure_ascii=False, separators=(",", ":")))
    DOCS.mkdir(exist_ok=True)
    (DOCS / "index.html").write_text(html)
    print(len(html) // 1024, "KB")
    for c in data["compare"]:
        print(c["title"], {k: round(v, 2) for k, v in c["total"].items()})
    for x in routes:
        print(x["label"], round(x["hours"] + x["early"], 2))


if __name__ == "__main__":
    main()
