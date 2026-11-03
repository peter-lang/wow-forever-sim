# wow-forever-sim

Levelling simulations for **World of Warcraft: Forever**, built from the game
client's own data tables: the druid from 1 to 60 (four routes, talent order
and rotation at every level).

> **This repository is purely vibe-coded.** Every line was written by an AI
> assistant under direction; nothing has been reviewed the way hand-written
> code would be, and none of the results has been checked in game. Treat the
> numbers as a model's output, not as facts about the game.

**The druid page:** [`docs/index.html`](docs/index.html), served by GitHub
Pages (Settings → Pages → Deploy from a branch → `main`, `/docs`).

## Results: the druid, 1 to 60

Hours to level 60 on client 1.60.1.70753 (the launch talent tree) with the
numbers of the 2026-11-03 patch notes, a High Order skyborne. The bear and
Hurricane pull three mobs, with 7.5% of their kills from single-mob pulls:

| Route | Hours | How |
| --- | --- | --- |
| Balance | 81.3 | one target to 39, respec at 40 into Moonkin Form, Hurricane on three mobs |
| Feral | 89.6 | caster to 12, bear 13–19, cat 20–33, bear from 34; one build |
| Balance single-target | 88.7 | Balance's first leg, respec at 40 into Moonkin Form, one mob at a time to 60 (no Hurricane) |
| Feral cat-only | 89.0 | bear 13–19 (before Cat Form), cat from 20; one build |
| Cat, then Balance | ~79.1 | the cat build to 39, respec to Balance at 40 (stitched from the routes) |

- Levels 40–60 hold 72% of the XP from 1 to 60, and over half the hours.
  Balance's lead is all there: Hurricane on three mobs beats both melee forms
  by 19–26% a level.
- The cat build and the cat-then-bear build are level (0.6% apart); the cat
  is faster at 20–33, the bear about even from 34.
- Not worth it while levelling, in this model: openers from stealth (no chain
  pulls while prowling), Starfire as the filler, Shifting Power (its mana is
  drunk back between pulls), Predatory Discipline (a chain-pulling cat leaves
  its mobs to the bleeds and rarely casts Ferocious Bite). A cat build with
  Predatory Discipline, Shifting Power and Rend and Tear 5/5 instead of
  Improved Moonfire and Natural Shapeshifter was about 2 hours slower than
  the order used here.
- **Gear:** armour is a green set at each level (two levels behind), its
  stat shares fitted on items up to level 30 (visible in the beta, matching
  Wowhead) and scaled by the item budget above that; the client's later items
  are not trusted. The feral set is calibrated stat by stat to typical greens
  over levels 20–32: per slot, the mean of the 3 best real greens, ranked by
  the sim's measured worth of each stat. The Balance set is within 5–25% of
  typical greens there, and is kept. Weapons are a guide's blue path, with
  the guide's spell power for caster weapons (the client stores none on main
  hands). With Hurricane, spell power is worth almost nothing (its
  coefficient is 0.03 a tick; the storm runs on mana).
- **What gear stats are worth** (the page's tables; each route's build and
  rotation held, one stat added at a time). For 10 strength at 40–59: the
  bear takes 9.8 agility, 20 attack power, 0.68% crit or 3.4% dodge, and
  barely values stamina or armour (it never dies); the cat 9.2 agility, 21
  attack power, 1.4% crit or 250 armour (10.5 agility and 2.5% crit at
  20–39, where stamina beats strength: the cat dies there). For 10 spell
  power: to 39, one target, spell power only (4.4% XP an hour); from 40
  Hurricane runs on mana, so 6.5 intellect or 9.8 spirit match it, and spell
  power adds 0.55%.
- **Not modelled:** the extra health the 2026-11-03 notes give higher-level
  mobs (no number was published), and whether three mobs really stay inside
  Hurricane. Either could change the ranking.

## Sources

- **The client's tables** from [wago.tools](https://wago.tools), pinned to
  1.60.1.70753, the last build before launch: the talent tree, spell ranks,
  items, mobs. Hotfix rows go on top: the build's own, and 1.60.1.70532's for
  records the build's tables lack (many dungeon and quest items exist only as
  hotfix rows). Where 70753's tables have a record, they win.
- **Tooltip-only values** (which tick spell each Hurricane rank casts,
  Ravage's flat damage) come from
  [foreverchanges.pro](https://foreverchanges.pro)'s reading of the launch
  realms, in `src/druid/launch.rs`.
- **The 2026-11-03 patch notes** (Blizzard's beta development notes): Shred,
  Claw and Ravage's weapon damage, Rip, Rake, Ferocious Bite and Pounce +25%,
  Starfire +40%, Rend and Tear 3–15%. Where the notes say "about", their
  figure is used. 70753 does not carry these yet; they are in
  `src/druid/launch.rs`.
- **What neither shows** (server rules, mob behaviour, walking time) is an
  assumption, collected in `src/assumptions.rs` and labelled where it is used.

## Running it

Needs Rust (stable) and Python 3 (standard library only).

```sh
python3 tools/fetch.py        # client tables into data/ (a few minutes)
cargo test --release          # the rules' tests
page/run.sh                   # the four druid routes into out/
python3 page/page.py          # docs/index.html from them
```

Other commands: `target/release/forever druid-level <command>` (see
`src/druid/beam.rs`: `hand <route>`, `plays <style> <level> <build>`,
`tree`, `scenario`, ...).

## Layout

| Path | What |
| --- | --- |
| `src/client.rs`, `items.rs`, `talents.rs` | the client's tables, items from their stat budgets, talent trees |
| `src/assumptions.rs` | numbers the client does not hold |
| `src/route.rs` | levelling routes: talent orders, beam search, climbs |
| `src/druid/` | the druid: scenario per level (`levels`), fights (`farm`), rotations (`play`, `peak`), routes (`style`, `beam`), the launch tree and patch numbers (`launch`) |
| `tools/fetch.py` | downloads the client tables |
| `page/` | the druid page: generator, template, foreverchanges.pro snapshots |
| `docs/` | the generated page |

World of Warcraft is a trademark of Blizzard Entertainment. This project is
not affiliated with Blizzard.
