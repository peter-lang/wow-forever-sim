# wow-forever-sim

Levelling simulations for **World of Warcraft: Forever**, built from the game
client's own data tables: the druid from 1 to 60 (three routes, talent order
and rotation at every level).

> **This repository is purely vibe-coded.** Every line was written by an AI
> assistant under direction; nothing has been reviewed the way hand-written
> code would be, and none of the results has been checked in game. Treat the
> numbers as a model's output, not as facts about the game.

**The druid page:** [`docs/index.html`](docs/index.html), served by GitHub
Pages (Settings → Pages → Deploy from a branch → `main`, `/docs`).

## Results: the druid, 1 to 60

Hours to level 60 on client 1.60.1.70753 (the launch talent tree) with the
numbers of the 2026-11-03 patch notes, a High Order skyborne, three-mob pulls:

| Route | Hours | How |
| --- | --- | --- |
| Balance | 80.3 | one target to 39, respec at 40 into Moonkin Form, Hurricane on three mobs |
| Feral | 86.5 | caster to 12, bear 13–19, cat 20–39, bear from 40; one build |
| Cat build | 86.8 | bear 13–19 (before Cat Form), cat from 20; one build |
| Cat, then Balance | ~78.1 | the cat build to 39, respec to Balance at 40 (stitched from the routes) |

- Levels 40–60 hold 72% of the XP from 1 to 60, and over half the hours.
  Balance's lead is all there: Hurricane on three mobs beats both melee forms
  by 16–23% a level.
- The cat and the bear are level from 40 on their own builds; the cat is
  faster at 20–39.
- Not worth it while levelling, in this model: openers from stealth (no chain
  pulls while prowling), Starfire as the filler, Shifting Power (its mana is
  drunk back between pulls), Predatory Discipline (a chain-pulling cat leaves
  its mobs to the bleeds and rarely casts Ferocious Bite). A cat build with
  Predatory Discipline, Shifting Power and Rend and Tear 5/5 instead of
  Improved Moonfire and Natural Shapeshifter took 88.9 hours.
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
page/run.sh                   # the three druid routes into out/
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
