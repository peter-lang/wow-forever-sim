"""Download the client tables the druid sim reads into data/<build>/.

    python3 tools/fetch.py            # the pinned build, and its hotfix rows

A table is wago.tools' CSV of the build's DB2 with hotfix rows on top
(`src/client.rs` applies them, the latest push winning). The rows come from
two logs:

- the build's own (`BUILD`), applied as they are;
- the last build whose rows wago.tools logged before (`EARLIER`), only for
  records the build's DB2 does not carry. Many dungeon and quest items exist
  only as hotfix rows; where the DB2 has the record, the build's DB2 is newer.

Rerun with --force once wago.tools logs the build's own rows. Standard library
only.
"""
import csv
import html
import json
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

BUILD = "1.60.1.70753"  # `client::DRUID_BUILD`
EARLIER = "1.60.1.70532"
DATA_DIR = Path(__file__).resolve().parents[1] / "data"
USER_AGENT = "wow-forever-sim (personal research; one fetch per build)"

# `client::TABLES`
TABLES = (
    "SpellName", "SpellRadius", "SpellLevels", "SkillLineAbility", "SpellEffect", "SpellMisc", "SpellDuration",
    "SpellCooldowns", "SpellRange", "SpellCastTimes", "SpellPower", "SpellClassOptions", "TraitNodeEntry",
    "TraitDefinition", "CurvePoint", "TraitDefinitionEffectPoints", "TraitNode", "TraitNodeXTraitNodeEntry",
    "TraitEdge", "ItemSparse", "Item", "RandPropPoints", "ItemArmorTotal", "ItemArmorQuality", "ArmorLocation",
    "ItemDamageWand", "ExpectedStat", "ItemDamageOneHand", "ItemDamageTwoHand", "ItemDamageRanged",
    "SpellAuraOptions",
)
# `client::GAMETABLES`, by file id
GAMETABLES = {"basemp": 1391664, "hppersta": 1391642}
# the tables with hotfix rows
HOTFIX_TABLES = (
    "Item", "ItemSparse", "SpellEffect", "SpellName", "SpellMisc", "SpellLevels", "SpellPower", "SpellCooldowns",
    "SpellAuraOptions",
)
_PAGE = re.compile(r'data-page="([^"]+)"')


def get(url: str) -> bytes:
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(req, timeout=120) as resp:
        return resp.read()


def save(dest: Path, body: bytes) -> None:
    tmp = dest.with_suffix(dest.suffix + ".part")
    tmp.write_bytes(body)
    tmp.replace(dest)


def tables(out: Path, force: bool) -> list[str]:
    jobs = [(f"https://wago.tools/db2/{t}/csv?build={BUILD}", out / f"{t}.csv") for t in TABLES]
    jobs += [(f"https://wago.tools/api/casc/{fid}?version={BUILD}", out / f"gt_{n}.txt") for n, fid in GAMETABLES.items()]
    failed = []
    for url, dest in jobs:
        if dest.exists() and not force:
            continue
        try:
            save(dest, get(url))
            print(f"  {dest.name}", file=sys.stderr)
        except urllib.error.HTTPError as e:
            print(f"  {dest.name}: HTTP {e.code}", file=sys.stderr)
            failed.append(dest.name)
    return failed


def logged(build: str, name: str) -> list[dict]:
    """Every hotfix row wago.tools logged for one table at `build`. The log's
    search is "<locale> <push> <record> <build> <table> [<row>]", so
    "<build> <table> [" selects one table at one build."""
    rows, page, last = [], 1, 1
    while page <= last:
        q = urllib.parse.urlencode({"search": f"{build.split('.')[-1]} {name.lower()} [", "page": page})
        body = get(f"https://wago.tools/hotfixes?{q}").decode()
        h = json.loads(html.unescape(_PAGE.search(body).group(1)))["props"]["hotfixes"]
        rows += [{k: r[k] for k in ("push_id", "record_id", "status", "locale", "data")} for r in h["data"] if r["table_name"] == name]
        last = h["last_page"]
        page += 1
        time.sleep(0.5)
    return rows


def hotfixes(out: Path, force: bool) -> None:
    for name in HOTFIX_TABLES:
        dest = out / f"hotfix_{name}.json"
        if dest.exists() and not force:
            continue
        with open(out / f"{name}.csv", newline="") as f:
            in_db2 = {int(r["ID"]) for r in csv.DictReader(f)}
        own = logged(BUILD, name)
        mine = in_db2 | {r["record_id"] for r in own}
        earlier = [r for r in logged(EARLIER, name) if r["record_id"] not in mine]
        dest.write_text(json.dumps(earlier + own))
        print(f"  {dest.name}: {len(own)} rows of {BUILD}, {len(earlier)} of {EARLIER} for records it lacks", file=sys.stderr)


def main() -> int:
    force = "--force" in sys.argv[1:]
    out = DATA_DIR / BUILD
    out.mkdir(parents=True, exist_ok=True)
    failed = tables(out, force)
    hotfixes(out, force)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
