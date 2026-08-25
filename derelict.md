###### derelict // the whole plan

> deck.boot **v1.1** ............ OK

###### > docs merged: plan + build ... one document, one universe, one function

###### > consistency pass 2026-08-01 .... tick language reconciled to DES · 6 laws filed · glossary current

###### > operator: **faye@nexus**

# derelict.plan

The ship restoration sim: what it is, the laws of its universe, every argument I've already had with myself, and the checklist that turns it real. *sim first. deck first. cubes forever.*

##### sys://concept — the game

## what am i actually making

Reverse Shipbreaker. They cut ships apart, I put them back together. Board a dead hull and bring it up in dependency order — power, then air, then thermal, then data, then gravity — and every stage changes what I can physically do. Helmet comes off when the compartment pressurises. Deck plating goes live and suddenly I'm walking instead of pulling myself along handholds. The ship becoming alive is the progression system. No XP bar required.

Jobs scale from "entire wreck in a decaying orbit" down to "bloke wants his oxygen tanks swapped". Same arc, different entry points, so I only build the loop once. And every small job is a lottery ticket — open the panel for the tank swap and find the previous owner's horrible bodge that's been holding on for a decade. Do I tell them? Do I quote for it? That's the game.

Ships get history the Dwarf Fortress way: **simulate it, don't write it**. Generate the ship working, then run years of ownership over it — refits, cheap owners, one bad day. The log never gets shown to the player. You dig it out of the physical evidence: mismatched serials, a cable taking the ugly short route through the galley, a schematic that's been wrong since 2149. Forensics, not lore dumps.

Depth-wise: modules with ports, four coupled networks (power / data / fluid / thermal), condition as a 0–1 scalar. Not simulating individual wires — did that maths, decided a pump with four different ways to be sad is plenty. But cables and ducts still get run *by hand* through finite conduit space, because routing a new circuit through forty runs of somebody else's terrible decisions is the GTNH bit, and the GTNH bit is non-negotiable. First person, eventually; the derelict worksite stays zero-g, so grab-and-brace is the default mode of being — ==that bet is unproven, and eight months of nice graph code doesn't get to pick the camera by default.==

> [!card] the actual bet
Is tracing a fault back to its root cause — in a terminal, on a graph, zero graphics — fun? Project Hail Mary in a REPL. If yes, everything else is just time. If no, I found out in weeks instead of finding out in year two, crying.

##### sys://axiom — the one law everything else derives from

## the universe is a function

```
U(seed, my_command_log) → exact state of everything, at any t
— the player is the one non-derivable process in the universe. everything else is a theorem.
```

Every big property of the game is a corollary of that signature, not a separate system:

- **State between events is analytic** — anchor + formula, events are *solved for*, ticks are just sampling. There is no coarse model, so drift between "tempos" is impossible by construction: one exact stream, read at any zoom.
- **The inner life is always real.** Crises unfold unwatched; walk-ins are genuine; global queries have true answers. Warp is a render ratio and costs proportional to what *happens*, not to time — the universe bills for intentions, not existence.
- **History is excavation.** Ships, people, and ideas leave residue because history is simulation output; the past of anything is derivable at any t. Any random NPC is a completed mystery the universe has already written — never shown, only dug out.
- **Saves are the clock, the seeds, and my fingerprints.** A universe is shareable as a text file; bug reports reproduce perfectly; my old runs are ghosts I can re-enter.
- **Time travel is branching** — two logs literally are two universes, so paradoxes are unrepresentable and the timeline diff is a queryable object. filed in later.md, where it belongs
- **Playing is a subscription.** The client holds a materialised view of the player's slice — a perception sphere (what renders: geometry by proximity, crowds from aggregates) and a consequence sphere (what the paracausal can touch, promoted to full grade). Commands in, events out, theatre interpolated between exact states; cascades resolve in the queue, never in the renderer, so rendering cannot break them. you don't render a world. you render the answer.
> [!card] what makes the function pure — non-negotiable, day one
One seeded RNG · ordered iteration (`BTreeMap`, never `HashMap`) · no wall-clock reads · **anchor + formula, never per-tick accumulation** (float behaviour is canonical: `pow` once beats ten multiplies) · sim transcendentals via the `libm` crate so the function holds bit-identical across machines · **the renderer never writes** · every fact has **one canonical owner-function** and everyone else queries it — rosters belong to hulls; careers are built by querying rosters, so all views agree, in any order, by construction. **And canon extends to everything deterministically derivable:** "in U" means derivable from canonical state, not computed in the sim crate — mesh slicing is pure geometry (store the plane, derive the capped mesh, bit-exact anywhere), seeded debris and scatter are canonical-but-inconsequential, and deterministic stepped episodes are inside U too, billing warp per-second rather than breaching purity. The true canon/theatre border is the **deterministic/nondeterministic-hardware boundary** — canon runs to the last CPU-derived vertex; only the GPU's pixels, and the player, live outside the function. **The GPU ruling, with reasons:** same-machine GPU determinism is achievable with kernel discipline, cross-machine only via fixed-point integers (the old lockstep-RTS trick) — so the exclusion is cost-benefit, not physics. The structural "no" is fit: DES is a branchy, causally-sequential queue, the most GPU-hostile workload there is. The clean split: **CPU is the historian, GPU is the calculator** — the queue owns what happens; wide derivations (map positions, conjunction broad-phase, scans, meshing) farm to shaders safely, because derivation without writes cannot breach canon.

##### sys://structure — repo & architecture

## the shape of the thing

```rust
`shipgame/                  # cargo workspace
├── crates/
│   ├── sim/               # THE GAME. no bevy. no graphics. plain rust.
│   ├── repl/              # stdin → commands, events → stdout
│   └── app/               # bevy + avian + big_space. does not exist yet. resist.
├── fixtures/              # hand-written .ron ships — test data AND design spec
└── later.md               # scope creep goes here to be honoured, not obeyed`
```

House rule that makes all the other rules work: **if I can't poke a feature from the REPL, it's in the wrong crate.** The REPL isn't a dev harness, it's the diagnostic deck — the in-fiction instrument, built first. When Bevy eventually shows up, the deck UI is just a skin over commands I've been living in for months.

### ecs-shaped, no ecs framework

Stealing the shape — IDs, per-aspect tables, systems as plain functions — and skipping the machinery. Frameworks pay for themselves by parallel-scheduling zillions of entities, and the price is unpredictable iteration order, which is the one thing I can't afford. A few hundred modules in strict order is the exact case where you pay and never collect. Bevy's ECS gets its turn later, in `app/`, herding thousands of visual objects that don't care about determinism.

```rust
`pub struct World {
pub tick: u64,
pub rng: ChaCha8Rng,                          // seeded. the ONLY randomness
pub modules: BTreeMap<ModuleId, ModuleMeta>,  // kind, serial, manufactured, maker
pub condition: BTreeMap<ModuleId, f32>,
pub power: BTreeMap<ModuleId, PowerState>,    // has a row = participates. that's it
pub connections: Vec<Connection>,             // live topology
pub as_built: Vec<Connection>,                // the schematic. allowed to be wrong. MEANT to be wrong
pub log: Vec<Event>,                          // the biography. players never see it raw
}

pub fn tick(world: &mut World, commands: &[Command]) -> Vec<Event>
// commands in. events out. nothing else crosses. the owner bot,
// the repl, the tests, the replay, and one day bevy: ALL just clients.`
```

> [!card] the sacred boundary
**Commands in. Events out. Nothing else crosses. Ever.** And the event stream *is* the ship's log, so the DF history costs me a `Vec` and a fold. Cheapest feature I will ever ship.

### settled arguments (stop relitigating, faye)

- **Things are IDs.** `ModuleId(u32)`, looked up in maps. Not structs holding references to each other — that's the version where the borrow checker eats a month of evenings and I rage-quit to Elixir.
- **Read T, write T+1 — plan, then apply.** Compute changes into a local `Vec`, apply after. Physically honest, dodges most borrow fights, and couplings between systems always cross a resolution boundary — tick boundaries in the phase 1–5 scaffold, event boundaries at the DES destination — so any loop in the dependency graph is broken by time, by construction.
- **The pure-style charter — purity via linearity.** Move semantics are linear types, so state-passing by value is purity for free (clean's uniqueness types, shipped as ownership): `tick(World, &[Command]) → (World, Vec<Event>)` — no `&mut` in any public sim signature, ever; `mut` only on local bindings; `&World` for queries; effects (io, clocks, seeding) live in the shell crates. Replay is literally `log.fold(World::new(seed), apply)` — the central equation, typechecked. The rng threads inside World: the state-monad pattern, pure by construction. The consumed old world is *unnameable* — stronger than haskell's convention, it's a compile error. Known door for the branching era: persistent structures (`im::OrdMap`, O(1) clone) when snapshots make Clone costs real — benchmark before adopting. ==purity where it buys guarantees, moves where it buys speed, and a type system that proves nobody can tell.==
- **Ship-ness is derived.** Connected components over cells. There is no "the ship" object, which is why cutting one in half is just a batch of deletions — both halves keep the full log and their stories diverge from one moment. If I ever write `cut_ship_in_half()`, something has gone architecturally wrong.
- **New module types are data** (RON: ports, draws, heat, fault modes — module #47 touches zero Rust). **New behaviours are enum variants**, and the compiler tells me every arm I forgot. Traits are for when strangers extend my catalogue. There are no strangers.
- **One substrate for all networks.** Nodes, ports, edges, a propagation pass. **The recipe for network N, forever O(1):** ports in RON (data) · one table + one `NetworkKind` variant (the compiler then lists every match needing an opinion) · the projection, where coupling to existing networks is one membership test each · *the rules — the only open-ended part, and the accretion surface* · a plan function · one chain call, whose position IS the coupling order · events passing the residue test, a damper, a schema bump. Existing networks are untouched; power never learns fuel exists. "Simulate anything" comes from boring uniform primitives, not from building a framework. I am not building a framework. ==i am not building a framework.==
- **Runs are routed by hand through finite conduit space**, drag-along-the-path, reach-in subfloor by default (crawl-through saved for dramatic trunk runs) — and derelicts are never greenfield: the routing puzzle is other people's decisions.
- **The CDDA clause.** Depth accretes forever, but every increment must add a *symptom*, a *residue*, or a *decision*. And deep sim, tiny catalogue: ten module types done properly beat two hundred done shallowly — fidelity multiplies content cost, and the catalogue is the unbounded expense, not the solver.
- **Functional core, imperative shell.** Pure plan functions over mutable storage, exhaustive matches, event log as truth. The GenServer instincts all transfer; no rebuilding the BEAM in Rust out of homesickness.

##### sys://power-up — the checklist — phases 0–5

## dependency order, same as the ships

Scaffold note: phases 1–5 use a fixed tick because it's the right Rust teacher — but the destination is discrete-event, so the **liftability law** applies from the first system: every system's state expressible as (anchor, formula, next-event-time). Write ticked code; keep DES-shaped state.

### PHASE 0 · rust on rails — 2–3 evenings · ownership intuition, not fluency · 0/4
- [ ] **Toolchain up.** — rustup, then RustRover or rust-analyzer — stay in JetBrains, it's home.
- [ ] **AoC day one, in Rust.** — Any old year. Structs, enums, match, iterators. Same trick that bootstrapped Elixir.
- [ ] **AoC day two, pick one that wants a map or a graph.** — This is the one that teaches why Rust wants IDs, before the sim makes it personal.
- [ ] **Read the ownership + borrowing chapters of the book. Once. Stop there.** — No cover-to-cover. No side quests. The sim is the curriculum.
### PHASE 1 · types + a dead ship — ~2 evenings · a ship I can interrogate that does nothing · 0/8
- [ ] **Cargo workspace:** `crates/sim` + `crates/repl`, `fixtures/`, `later.md`. — sim has zero deps beyond serde/ron/rand_chacha (+ libm when maths arrives). That's a promise, not a starting point.
- [ ] **Newtypes:** `ModuleId(u32)`, `PortName`, and a `NetworkKind` enum. — IDs everywhere. The first time I'm tempted to store a reference in a struct, re-read this line.
- [ ] **`ModuleMeta`:** kind, serial, manufactured date, manufacturer. — Serial + date go in NOW, not later — they are the history feature. A ship without serials can't testify.
- [ ] **`World` struct:** tick, seeded rng, BTreeMaps for meta/condition, `Vec<Connection>`, `log: Vec<Event>`. — BTreeMap, never HashMap. The log field exists from the first commit even while it's boring.
- [ ] **`Command` and `Event` enum skeletons** + the `tick(world, commands) -> Vec<Event>` signature. — The sacred boundary goes in before there's anything to bound. Cheapest it will ever be.
- [ ] **serde + ron; load `fixtures/testudo.ron`.** — One small ship, hand-written: a battery, a bus, a breaker, three loads, two rooms' worth of modules.
- [ ] **REPL v0:** hand-rolled parser, `list`, `inspect <id>`, `quit`. — inspect shows meta + condition + connections. This is the deck's first breath.
- [ ] **git init + CI stub:** clippy + `cargo test` on push. — Literally my day job. Free. No excuses.
### PHASE 2 · power — ~2 evenings · flipping a breaker changes downstream state · 0/5
- [ ] **`PowerState` table** — a module participates in power because it has a row. Sources, buses, breakers, loads.
- [ ] **`tick_power`:** propagate from sources through closed breakers to loads, over the connection graph. — Compute into a local Vec, apply after. Read T, write T+1, from the very first system.
- [ ] **Commands:** `power on|off <id>`, `breaker open|close <id>`.
- [ ] **Events:** `PowerChanged`, `BreakerTripped` — emitted, drained, printed by the REPL. — First entries in the log that will one day be a ship's biography.
- [ ] **`status <id>`** in the REPL: per-network state for one module.
### PHASE 3 · time + determinism — ~1 evening · locked in here, never reopened · 0/4
- [ ] **`advance N`:** fixed timestep, batteries drain, loads draw, breakers trip on overload. — Liftability law from the start: battery charge = anchor + rate × Δt, never `charge -= x` per tick in a loop.
- [ ] **Seed the RNG from the fixture.** One `ChaCha8Rng`. The only randomness in the crate.
- [ ] **The determinism test:** same fixture, same command script, run twice — assert the event streams are identical, byte for byte. — This test never gets deleted. It is the canary for every future sin.
- [ ] **Replay:** save a command log, refold it over a fresh world, arrive at the same state. — Savegames, bug reports, and the history engine are all this one trick wearing different hats.
### PHASE 4 · condition + cascade — gate one — the fault chase · the entire original bet · 0/5
- [ ] **Condition scalar** per module, 0.0–1.0. Below thresholds: reduced capacity, then intermittent shortfall under peak demand, then dead.
- [ ] **Capacity limits:** buses and sources have maximums; overdraw trips breakers. — A module at 0.4 works fine until demand peaks. That's the whole texture, one float.
- [ ] **`seed-fault`** dev command: pick a root cause (degrade one module or connection), hidden from normal output.
- [ ] **`scan`** — the deck's first real instrument: reports module *states*, never causes. — "scrubber: insufficient input" — the cause is three hops upstream. That gap is the game.
- [ ] **Playtest, honestly.** Seed a fault blind (or get someone else to), chase it through scan / status / inspect.
> [!warning] ⚠ gate one — do not skip the honesty part
Does chasing the fault have the Hail Mary pull, or am I just admiring my own graph? One session tells me. If it's flat: the answer is usually *symptoms too directly named* — widen the gap between what the deck says and where the cause lives, then test again. If it's still flat after that, stop and think hard before phase 5.

### PHASE 5 · history — gate two — owner verbs + the dumb owner bot + the excavation test · 0/9
- [ ] **`replace <id>`:** swap a module for a new one — fresh serial, fresh manufacture date, condition 1.0. — The single most legible residue there is: one 2158 aftermarket unit in a row of 2151 Vostoks.
- [ ] **`reroute`:** replace a connection with a different path/topology, and mark it as non-original. — The ugly shortcut through the galley. Doesn't need spatial cells yet — a "deviates from as-built" flag on the connection is enough to testify.
- [ ] **As-built schematic:** snapshot the ship's original topology at generation, keep it separate from live state. — The schematic being WRONG is the feature. It can only be wrong if it exists.
- [ ] **Owner bot v0:** a policy table + the seeded RNG. No goals, no planning. — cheap_owner: defer repairs below 0.4, buy secondhand (used serials!), reroute rather than refit. careful_owner: the opposite. Legible neglect beats optimal play.
- [ ] **`run-years N [policy]`:** the bot drives the ordinary tick loop; degradation accrues; the bot reacts per policy; the log fills itself. — History is not a second system. It's the same sim with a bad landlord.
- [ ] **`survey`** — the forensics view: serials + dates grouped by kind, non-original routing flagged against the as-built, condition patterns. — This is the player-facing instrument gate two gets judged through.
- [ ] **The excavation test itself.** `run-years 10`, write my reconstruction down from survey alone, THEN diff against the log.
- [ ] **Snapshots:** serialise `World` every simulated quarter during run-years, checksummed; `seek <year>` = nearest snapshot + replay forward. — The year-5 scrubber, the save format, and the fallback-on-corruption are all this one mechanism.
- [ ] **Version the log + snapshot format from day one.** Schema version field, explicit defaults for new fields on load, logs record their tempo. — Accretion changes the future's rules, never the past's records. The one place CDDA-style depth-forever can bite.
> [!warning] ⚠ gate two — the DF magic, or not
Generate ten years. Throw the log away. Hand myself only the ship. If the serials and the shortcut cable tell the story without the log — magic confirmed, and the whole game has a spine. If I need the log to make sense of the ship, the residue isn't legible yet: the fix is owner verbs with more *physical* consequence, not a smarter bot.
> the bot stays stupid. df's worldgen characters are profoundly thick and it has never once mattered.

### beyond the gates — earned, not scheduled

- **Phase 6 — networks two through four.** Thermal (needs power — lovely), fluid, then data last, at which point the deck stops being an omniscient dev tool and becomes a fallible in-fiction instrument that can itself be the thing that's broken. Every new network also widens the owner bot's vocabulary for free.
- **Phase 7 — trust infrastructure.** Property tests over fixtures, the bisection test (`bisect(world, plane)`: both halves tick, air only vents at real openings, solver restores each independently), the rate-consistency distribution test, the avalanche-size histogram standing watch on β, and the dumb auto-solver — solvability oracle and difficulty meter in one. **Plus the fleet benchmark:** `bench 100×run-years-10` — measure events/sec and the real per-event cost c under actual rule load, and re-derive the feasibility envelope from data. The envelope in sys://maths is *modelled*; this is the evening that makes it measured. If c disappoints, every fix on the ladder is a parameter change, never a rewrite — confluence guarantees performance tuning can't touch semantics.
- **Phase 8+ — everything else.** Hand-author three ships *before* any generator exists — they're the spec the generator has to hit. Then generate-working-then-break-it, trunk-first routing so conduits look human, the macro spine. Then, finally, Bevy + Avian + big_space (pin the version, upgrade at milestones only), and the zero-g grab-and-brace toy gets its day in court.
### the session ritual

End every session by writing **one line — what's next —** somewhere I'll trip over it next time. Cold-start cost is what actually kills two-evening-a-week projects; a session that starts with "read my own code to remember what I was doing" is a session that quietly doesn't happen. Keep a branch that always runs. This project survives on me *wanting* to open it, nothing else.

##### sys://phase2 — the build sheet — power. no llm required

## phase 2: the universe's first verb

Everything below is decided. If a question comes up that this sheet doesn't answer, make the smallest honest choice, write it in the decision log, and keep moving. The goal state: *flipping a breaker in the REPL makes something two connections away go dark, the log records why, and the same script replays byte-identical.*

### step 1 — types (one session, mostly compiler tour)

> [!card] new state, new vocabulary
**Draw belongs to the part:** add `draw_a: u32` to `ModuleDef` and `ModuleMeta` (0 for sources/buses/breakers/valves). Whole amps, integers — **phase 2 is float-free**; determinism costs nothing when there's nothing to round. Fixture edit required (numbers below). **Power state is a table:** `power: BTreeMap<ModuleId, PowerState>` where `PowerState { energised: bool }` — one row per participating module, populated at load. **Actuators are their own table:** `breakers: BTreeMap<ModuleId, BreakerState { closed: bool }>` (closed at load), `sources: BTreeMap<ModuleId, SourceState { online: bool }>` (offline at load — a dead ship until commanded, correct diegetically). Participation derives from `ModuleKind` via an exhaustive no-wildcard match. **EventId allocation:** `next_event: u64` counter on World; a private `World::emit(kind, cause)` helper appends to the log AND returns the event — tick returns the new events (the client's drain), the log keeps everything.

> [!card] the vocabulary — and the paracausal rule
**Commands:** `SetBreaker { id, closed }` · `SetSource { id, online }`. **Events:** `BreakerSet { id, closed }` · `SourceSet { id, online }` · `PowerChanged { id, energised }` · `BreakerTripped { id, load_a, rating_a }` · `CommandRejected { reason }` (the boundary is total — a bad command is history, not an error). **The cause law:** accepted commands emit their event with `cause: None` — **None means paracausal**; only player commands and `WorldLoaded` ever carry it. Every `PowerChanged` causes-refs the command or trip that made it; every `BreakerTripped` causes-refs the `PowerChanged` (or command event) that created the overload. The first cause chains in the universe, correct from birth.

### step 2 — tick_power (the flood-fill; the walk wins at four roles)

> [!card] the algorithm, exactly
Order inside `tick()`: **(1) apply commands** — validate id exists and is the right kind (else `CommandRejected`), flip actuator state, emit the Set event. **(2) propagate** — build adjacency from Power connections as a **directed** graph (the fixture wires supply→load: `from` is always the feeding side — one convention, and propagation and the overload sum become the same primitive; rebuild per tick, it's 14 modules — cache when the profiler says so, not before), BFS from every online source following the arrows; traversal enters a breaker only if closed; the reached set is energised. **(3) diff & emit** — compare new energised set against the power table, emit `PowerChanged` per difference, write new states (plan into a local Vec, apply after — read T, write T+1). **(4) overload check** — per closed breaker, sum `draw_a` of energised loads downstream of it (BFS from its load-side); if sum > `rating_a`, plan a trip: breaker opens **next tick** (schedule it — a real thermal breaker doesn't act instantly; the delay is honest physics AND kills same-tick feedback), emit `BreakerTripped` now with the cause ref. The re-propagation after a trip happens naturally on the following tick.

> [!card] the draw table — margins are the trap
VK-H90 heater **12** · VK-S200 scrubber **6** · VK-L8 lights **2** each · VK-N300 nav **2** · VK-T1 deck **1** · VK-X4 sensor **0** · VK-P60 pump **4**. BRK-01 as-built: 12+6+2+2+1+0 = **23 of 25** — two amps of margin, alive but honest. BRK-02: 4+2 = **6 of 25**. The Kirin KH-77 replacement heater draws **16**: BRK-01 becomes **27 of 25** → trips. The dossier's 2155 trap, in integers. Make a second fixture `fixtures/testudo-2155.ron` (Kirin swapped in, galley-routed) — phase 2's test ship and phase 5's opening state, one file early.

### step 3 — the REPL grows verbs

> [!card] grammar and wiring
`breaker open|close <LABEL>` · `source on|off <LABEL>` · `status <LABEL>` (kind, energised, draw, breaker/source state if actuator) · `tick [n]` (default 1) · `log [n]` (last n events, cause refs shown as `← #id`). REPL owns label→id resolution via a `World::find_label(&str) -> Option<ModuleId>` helper; commands queue up and submit on `tick` — **the REPL never mutates state, it only composes command lists for the boundary**. Printing events after each tick IS the drain. World needs `&mut` now: the deck holds the universe and tick borrows it per call.

### step 4 — the tests that define done

> [!card] write these before polishing anything
**(a) power_reaches_the_lights:** load testudo, source on, close breakers, tick → LT-01 energised, PowerChanged events present. **(b) breaker_isolates:** open BRK-01 → its loads dark, PMP-01 (BRK-02) untouched. **(c) the_galley_trap:** load testudo-2155, energise → BreakerTripped{27, 25} within two ticks, loads dark the tick after. **(d) trips_have_causes:** walk the trip event's cause refs and assert the chain terminates at a `cause: None` command — *the paracausal root, asserted by test*. **(e) canary v2:** same fixture, same command script, two worlds → `assert_eq!` on the full event logs. The canary grows from "same rng" to "same history" — this is the version that runs for five years.

### rails

> [!card] do not — phase 2 edition
No floats (draws are integer amps; condition doesn't move until phase 3). No crepe (four roles is a five-line walk; the datalog card says when). No thermal, no fluid (the valves sit closed-mouthed until phase 3's coupling). No caching adjacency, no premature Vec-tables. No REPL colours/polish beyond function. If tempted by any of these: later.md, one line, back to work. **Done =** five tests green, canary v2 in CI, boxes ticked, one decision-log line for anything this sheet didn't foresee.

##### sys://phase3 — the build sheet — time. the DES embryo, planted

## phase 3: the battery learns to die

Goal state: *`tick 7000` takes under a millisecond, the battery depletes at exactly tick 6260 every run, the whole ship goes dark with causes attached, and a saved command log refolds into a byte-identical universe.* This phase is small in code and enormous in law: the anchor+formula pattern installed here is the exact shape every future quantity copies, and the depletion field is the event queue's first cell.

### step 1 — charge, as anchor + formula (the liftability law, first blood)

> [!card] never decrement. derive.
**Capacity lives on the kind:** `BatteryBank { capacity: u64 }` in **amp-ticks** (1 tick = 1 second nominal; VK-B440 = 40Ah = **144_000**). Still integers — phase 3 stays float-free; exact arithmetic means exact depletion ticks means exact determinism. **SourceState grows the anchor:** `{ online: bool, charge_anchor: u64, anchor_tick: u64, depletion_tick: Option<u64> }`. Charge is *never stored as a current value and never decremented in a loop* — it is derived: `charge(t) = charge_anchor − rate × (t − anchor_tick)` (saturating_sub). **Rate** = sum of energised loads' `power_draw`, recomputed only when the energised set changes. **Re-anchoring:** whenever rate or online changes (i.e. inside `settle_power`): set `charge_anchor = charge(now)`, `anchor_tick = now`, and solve `depletion_tick = if online && rate > 0 { Some(now + charge/rate) } else { None }` (floor division: depleted when it can't serve a full tick). ==(anchor, formula, next-event-time) — that triple IS the DES destination, living inside the tick scaffold. lifting it later = deleting the loop, nothing else.==

### step 2 — the depletion event

> [!card] the first event the player didn't cause — sort of
New vocabulary: `EventKind::SourceDepleted { id }`. In `tick()`, per online source: `if world.tick >= depletion_tick` → set `online = false`, `charge_anchor = 0`, emit `SourceDepleted` — **cause: the event that last re-anchored the rate** (store the anchoring EventId alongside the anchor: the load change that set this death in motion is the cause; walk THAT chain back and it still ends at a paracausal None — the battery died because of the exact switch-flip that fixed its final burn rate). Then `settle_power` with the depletion event as cause: the whole ship goes DARK, correctly attributed. **`tick n` loops n times** but each iteration does almost nothing — a compare against `depletion_tick`, no decrements, no scans. 7000 ticks in microseconds is the acceptance criterion AND the proof the liftability law is being honoured.

### step 3 — replay (the save format, prototyped)

> [!card] the command log belongs to the client
The sim's log holds *events*; the *command* history is the client's to keep — the REPL grows `history: Vec<Vec<Command>>`, pushing the queued batch every tick (empty batches included — the timing IS the data). `Command` and `ModuleId` gain Serialize/Deserialize. REPL verbs: `save <file>` (RON of the history) · `replay <file>` (fresh world from the same fixture, fold every batch through `tick`). Add a sim helper `replay(fixture_text, script) -> Result<World, LoadError>` so tests don't go through the REPL. **World derives PartialEq** (ChaCha8Rng supports it) — replay equality is `assert_eq!(original, refolded)`, whole-universe, one line. ==savegames, bug reports, multiplayer sync, and the history engine: all this one trick in different hats. U(seed, log) stops being a slogan today.==

### step 4 — the tests that define done

> [!card] two new, one sacred, one promoted
**(a) the_battery_dies:** testudo, power up, close all (23A on a 144_000 anchor) → `tick 7000` → assert `SourceDepleted` at exactly **tick 6260**, PowerChanged{DARK} for every load the tick after, cause chain from a dark light back through depletion to a None. **(b) anchors_dont_drift:** advance in one `tick 5000` vs five `tick 1000`s on twin worlds → identical logs and states (proves charge is derived, not accumulated — THE liftability regression test). **(c) canary v3 (p3t3, the sacred one):** same fixture + same script incl. depletion and a trip, twice → byte-identical logs; wire into CI if not already; never deleted, ever. **(d) replay_refolds (p3t4):** interactive-style script → save → replay onto fresh world → `assert_eq!` on the Worlds themselves. **p3t2 audit:** grep the crate for rng use — one ChaCha8Rng, seeded from the fixture, no other randomness; tick the box on inspection, it was built right in phase 1.

### rails

> [!card] do not — phase 3 edition
No real event queue (the `depletion_tick` field is the embryo; the BTreeMap-of-events queue is the phase 8+ lift). No floats (amp-ticks are integers; decimal years arrive when history spans years, not seconds). No recharge, no solar, no second battery logic beyond what the tables already give free. No condition effects (phase 4 owns degradation). No REPL polish beyond `save`/`replay`. Done = four tests green in CI, boxes ticked, one decision-log line for anything unforeseen — and the phase locked, never reopened, as advertised.

##### sys://phase4 — the build sheet — condition. the fault chase, gate one

## phase 4: the ship learns to lie

Goal state: *a fault seeded into one module makes something three hops away misbehave, `scan` reports the misbehaviour without ever naming the cause, and I can find it from the deck alone.* Everything in phases 2–3 was mechanism. This is the first phase whose output is a question the player has to answer — and the first that can fail on taste rather than on tests.

> [!card] PHASE 4 COMPLETE — gate one passed, 2026-08-15
**All six boxes built** (step 3 cut — see the log), 26 tests green, and the honest part done: five hulls seeded blind into /tmp with the assignment shuffled at generation so nobody knew which was which. **Gate one passes.** The loop works — sweep with `scan`, notice the pattern, form a hypothesis, confirm with `inspect` — and the control hull with nothing wrong was correctly read as healthy, which is the result that mattered most: an instrument that always finds something is one you learn to ignore. **Two findings, both logged:** a dead consumer produces no symptom at all, and diagnosis took 3–4 commands on a fourteen-module hull. ==next: phase 5, history — owner verbs, the dumb owner bot, and gate two.==

### step 1 — capacity, on the suppliers

> [!card] maximums belong to the part
`Bus { max_a: u32 }` (currently a unit variant) and `BatteryBank { capacity: u64, max_draw_a: u32 }` — **capacity is amp-ticks** (how much is left), **max_draw_a is amps** (how fast it can leave). Nothing has limited discharge rate until now. **Overdraw trips breakers:** a supplier over its maximum emits `CapacityExceeded { id, load_a, rating_a, degraded_rating_a }` and schedules trips for every closed breaker *anywhere* downstream — not just direct children, since a battery feeds the bus which feeds the breakers. Reuse `pending_trips`, so the thermal delay and the cause chain come free. A bus giving out blacks out both halves of a testudo; that is the intended drama, and it reads very differently in the log from one breaker tripping. **Testudo numbers:** VK-D12 bus at **40A**, VK-B440 at **40A** max draw — comfortably above both hulls at full health (29A and 33A), so the supply fault is reachable *only* through condition. ==an earlier draft put both at 30A, which would have made the 2155 hull fail at the bus before BRK-01 could trip — quietly killing the galley trap. suppliers are checked before breakers: the bus failing is the more upstream truth, and blaming a breaker for a bus fault is a lie rather than a puzzle. pending_trips is a BTreeMap, so or_insert makes first-writer-wins structural rather than remembered.==

### step 2 — condition, on the suppliers only

> [!card] one threshold, one ramp, one float multiply
A consumer's "capacity" is what it delivers into thermal or fluid, and neither network exists yet — so in phase 4 **a consumer is either drawing or dead**, and the ladder belongs to buses, breakers and batteries. `>= 0.70` **nominal**, full rating · `0.00–0.70` **ramp**, `rating × (condition / 0.70)` truncated · `0.00` **dead**, which falls out of the ramp for free. **The ramp normalises the band onto 0–1 rather than scaling flat**, so at exactly 0.70 it yields full rating and meets the branch above it — continuous, no cliff. A flat `rating × condition` would jump 25A→17A across a hair's difference in condition. **A degraded breaker is the best fault in the game:** it trips below its nameplate, so the symptom is "the galley keeps going dark" and the cause is a part whose printed rating says it should be fine. **Derating is one `f32` multiply and one truncation** — IEEE 754 single multiplication is exactly-rounded and deterministic across machines; the purity card's danger is transcendentals and fused operations, not one `mul`. **Dead consumers draw nothing but stay energised** — a dead lamp still has power at its terminals, and modelling it as unenergised would make the breaker upstream look like it isolated something it did not. ==no powf, no accumulation, back to integer amps immediately. the threshold is one named const used by both the branch and the divisor, so they cannot drift apart. phase 4 is where floats enter the sim and this is the only shape they get until libm arrives.==

### step 3 — intermittence — ~~the first real RNG draw~~ CUT

> [!card] cut, 2026-08-08 — and the spec didn't survive contact anyway
**The spec as written could not work.** A coin flip inside `settle_power` that faults the supplier puts its victims on `pending_trips`, the breakers open next tick, and breakers stay open until someone closes them. So the "intermittent" fault takes the ship down permanently on its first bad roll — indistinguishable from a hard failure, and the **dropped and returned** symptom step 5 wanted could never occur. A real flicker has to live in `energised_set`, not the fault path; and since nothing re-settles without an event, it would have needed scheduling like `depletion_tick` — the anchor + next-event-time triple again.

**Cut anyway, for a better reason:** a fault you cannot reproduce is a fault you cannot diagnose. If `scan` reports something different each run, the instrument becomes a coin flip and the puzzle gets noisier, not harder. And the texture the sheet was reaching for — *"works fine until demand peaks"* — already exists deterministically: a derated breaker holds at 17A and gives out at 23A, so switching the heater on trips it and leaving it off doesn't. Load-dependent, reproducible, diagnosable by experiment. ==the RNG stays unexercised until phase 5's owner bot, which is a real customer with a real reason. revisit only if gate one finds the fault chase too easy — then it is a lever pulled to fix a measured problem, not because a sheet said so.==

### step 4 — seed-fault, through the boundary

> [!card] the dev command is still a command
Built as `Command::SetCondition { id, new_condition: Condition }` → `EventKind::ConditionChanged { id, from, to }`, cause `None`, because the paracausal did it. Then `settle_power` with that event as cause, so the fault takes hold at once instead of waiting for the next actuator. **Named Changed, not Degraded**, because the command can also raise a value and an event that logs a repair as a degradation is a lie in a permanent record. **It carries a `Condition`, not an `f32`** — so an out-of-range value cannot be put into the command at all, and the sim needs no rejection for bad values, only the existing `NoSuchModule`. Validation happens once, at the deck's parse, where `"abc"` and `"5.0"` are different mistakes with different messages. **"Hidden from normal output" means `scan` never mentions condition** — not that the event is hidden. The log is truth and `inspect` already shows condition; the deck's *instrument* is what must stay ignorant. ==it goes through the boundary, so it lands in the log, so it replays: a bug report is a script file. from/to matters because condition has no other record — once overwritten, the log is the only place the old value survives, and phase 5's excavation test reads exactly this.==

### step 5 — scan, the deliberate gap

> [!card] symptoms. never causes.
`scan` reports per-module observable state and never a cause. Three symptoms now that intermittence is cut: **nominal** — energised, drawing its nameplate · **dark** — not energised · **starved** — energised, but the supply feeding it is running close to its limit. **A symptom must be observable at the module being reported.** "Starved" defined as *"my supplier is over limit"* would be a fact about the supplier wearing this module's name, and it would last exactly one tick before the breakers opened. Defined as voltage sag it persists, and it is what an instrument at that module would genuinely see. **The point is that the symptom is distributed:** six modules report sagging supply, not one of them knows why, and what they have in common is the bus. Spotting that intersection is the fault tree. ==the vocabulary lives in the sim as a Symptom enum with a World::symptom_of query, so the REPL only formats it and cannot quietly invent a symptom that leaks a cause — and survey can reuse it in phase 5. the moment scan says "LT-01 dark because BRK-01 is degraded", phase 4 has failed and the game is a debugger. if the chase feels flat, the fix is almost always symptoms named too directly — widen the gap, then test again.==

### step 6 — the tests, then the honest part

> [!card] seven green, then one question tests can't answer
Built, in `crates/sim/tests/phase4.rs`: **(a) a_tired_breaker_trips_early** — BRK-01 at 0.5 trips on `{23, 25, 17}`, with a control proving an undegraded ship carrying the same 23A does not · **(b) the_bus_gives_out** — PWR-02 at 0.4 gives 22A effective against 29A, no breaker over its own rating, both open the tick after · **(c) dead_modules_draw_nothing** — HTR-01 dead drops the ship to 17A and the tired breaker that trips at 23A holds at 11A · **(d) suppliers_outrank_breakers** — 2155 hull with the bus degraded, both faults live, asserts `CapacityExceeded` and the *absence* of `BreakerTripped` · **(e) faults_have_causes** — walks a dark lamp back to a None and asserts the chain never passes through `ConditionChanged` · **(f) seeded_faults_replay** — a script with `SetCondition` refolds to an identical World, proving Condition round-trips through serde · **(g) seeding_a_ghost_is_history_not_an_error** — `SetCondition` on nothing aboard rejects rather than panicking. **Then:** seed a fault blind, chase it through scan / status / inspect, and decide whether it has the Hail Mary pull. ==assert whole events, not just that something tripped — the triple {23, 25, 17} is the only place the ramp's arithmetic is written down as a fact. twenty-two green tests cannot answer the last question.==

### rails

> [!card] do not — phase 4 edition
No thermal, no fluid (consumers stay two-state until phase 6). No automatic degradation, no wear, no owner bot — condition moves only when commanded; decay is phase 5's landlord. No transcendentals, no `powf`, no accumulating floats: one multiply, one truncation, back to integers. No polling — and now no RNG either, so nothing in this phase can produce two different answers from the same state. No scan colours or polish beyond function. No new module kinds; ten done properly beats two hundred done shallowly. Done = seven tests green, one honest playtest written up, and a decision-log line for anything this sheet didn't foresee. ==float comparisons want `> 0.0` not `== 0.0` — float_cmp is pedantic and CI denies warnings. the cast allows on the derating function are signed off: truncation to whole amps IS the intent.==

##### sys://phase4.5 — the build sheet — scale. the bet changes shape

## phase 4.5: the universe learns to skip

Goal state: *a fleet of a thousand hulls, advanced a decade in one call, costing only what actually happened — and "give me the state at tick T" answered without stepping to it.* Gate one asked whether chasing a fault in a terminal is fun, and passed on a fourteen-module ship. This phase asks the other question: **does the architecture survive 10⁶ objects**. It is the bet that would hurt most to discover late, and every phase 5 line written before it would assume a single hull.

> [!card] PHASE 4.5 COMPLETE — the architecture survives, 2026-08-25
**10⁶ objects — 2000 hulls × 500 modules — advanced 20,000 ticks in 23.9ms.** 34 tests green. The control series is the finding, because it isolates the variable: with **0** hulls powered that same jump costs **0.3ms**, with **200** it costs **25.3ms**, with **2000** it costs **273ms**. Linear in what is *awake*, flat in what merely exists — 1800 dark hulls cost **148ns each** across a twenty-thousand-tick horizon. ==the sheet's own extrapolation was right, and so was its diagnosis: settle never needed scoping. measured after the layer landed, one hull-settle is 478µs at 0.955µs per module — the same per-module cost as before, only never again asked to do the whole universe at once.==

> [!card] 622 → 260 bytes a module, and one thing that cannot be interned
Three changes, each measured on its own. **`as_built` deleted** — a `.clone()` of every connection that nothing ever read: 622 → 317 B/module. **`PortName` interned** — a million connections held two heap strings apiece where the entire universe has about thirty distinct port names. **`run` resolved** against the run table the fixture always declared and the loader always threw away, which also recovers `cap` for the day a trunk can be the fault instead of a breaker. 317 → 260 B/module; **594 MiB → 248 MiB** for the full 10⁶. ==labels look like the next win and are not: they are unique WITHIN a hull and repeat only ACROSS hulls, so a per-world table saves exactly nothing. capturing them needs the table on Universe, which trades away the world independence the two-ship canary just proved. serial is inspect-only — nothing in the sim reads it — and could leave resident memory entirely. the step-4 card said ModuleMeta holds four strings including maker; maker had already moved to the part catalogue by then, so the advice was right about interning and wrong about where.==

> [!card] what the sheet did not foresee
**`tick()` did not become a wrapper.** Step 2 planned `advance_to(world, t, commands)` with `tick` thin on top; what shipped is `advance_to(world, t)` taking no commands at all, and `tick` unchanged. What a command batch *means* across a jump is a real design question, and answering this phase's question did not require it. The deck still says `tick n`. **`Universe::advance_to` does not pop the earliest world.** Finding the earliest means asking every world, so a plain loop over all of them is the same O(worlds) with less machinery — the queue earns its keep only when an incrementally-maintained heap replaces the scan, and at 2000 hulls the scan is nowhere near the ceiling. **Settle runs once per accepted command, not once per batch** — 25 commands to wake a hull is 25 full settles, 2.56s across 200 hulls. ==coalescing to one settle per tick is a ~25× and it is a cause-chain decision wearing a performance costume: settle-per-command is exactly what lets each event name the specific command that caused it. it also only shows up on mass power-up, which is a loading scenario, not steady state.==

> [!card] measured, not guessed — 2026-08-15
Synthetic hulls at 952 / 10,002 / 50,002 modules, release build. **One settle: 2ms / 22ms / 119ms** — linear in N, ~2.4µs per module. **An idle tick: 36ns at 1k, 163ns at 50k** — near-flat, growing only with cache pressure. ==extrapolated to 10⁶ modules in one World, a single settle is 2.4 SECONDS. every command, every trip, every depletion. that is the wall, and it is not the tick loop.==

### the diagnosis

> [!card] World is one ship, and nothing is smaller than everything
`settle_power` re-derives the entire world on every event. At 500 modules that is 1.2ms and fine; at 10⁶ it is fatal. **But settle does not need scoping internally** — the doc already said it: *"ships have fixpoints, the universe has a queue"*, and *"world = region of simultaneity; universe = event queue over analytic orbits."* `World` is already correctly named. What is missing is the layer above it. ==no rename, no restructuring of settle_power. a layer is added, not a system rebuilt.==

### step 1 — expose the next event time

> [!card] it is already computed; nothing can see it
`World::next_event_at() -> Option<u64>` — the minimum of every source's `depletion_tick` and anything in `pending_trips`. A pure query, no new state, no behaviour change. **Writing it first forces the audit**: every scheduled thing in the sim must be discoverable from one place, which is the precondition for a queue. If something is scheduled and cannot be seen here, it is a poll in disguise.

### step 2 — delete the poll

> [!card] the change that makes "state at T" real
`advance_to(world, t, commands)`: process everything due at or before `t`, then set `world.tick = t`. `tick()` becomes a thin wrapper for `advance_to(tick + 1)`. A quiet century costs the same as a quiet second. **Protected by the canary, generalised:** the same script advanced tick-by-tick and in single jumps must produce byte-identical logs — `anchors_dont_drift` applied to the whole sim rather than to charge alone. ==one design consequence: commands arrive AT a time now. the deck's `tick n` becomes `advance n`, and batches carry timestamps. this is the step with the most risk and the strongest safety net.==

### step 3 — the universe above the world

> [!card] a fleet where nothing happens costs one comparison
`Universe { worlds: BTreeMap<ShipId, World> }`, with `advance_to(t)` popping the world whose `next_event_at` is earliest, advancing it, and rescheduling. **Undocked hulls share nothing**, so cross-world events are zero until docking exists — which is exactly the lookahead that licenses parallelism later (Chandy–Misra: speedup ≈ 1/(s + (1−s)/k), where s is the fraction of events crossing hulls). ==canary across two ships: same scripts, interleaved advancing versus sequential, identical logs per ship.==

### step 4 — measure at 10⁶

> [!card] 2000 hulls × 500 modules
Numbers wanted: resident memory, one settle, and events/sec under a synthetic owner-bot load. **Do the string interning first or the measurement lies.** `ModuleMeta` holds four `String`s — maker, part, serial, label — ≈200 bytes each module, and `maker: "Vostok"` is identical across hundreds of thousands of them. Interned to a catalogue id that is ~16 bytes. ==serials stay strings: genuinely unique, genuinely needed. measure with them in, or the 10⁶ number describes the allocator rather than the architecture.==

### rails

> [!card] do not — phase 4.5 edition
No geometry. No positions, no KDS, no frontend — those need the app era and there is nothing to index yet. No parallelism: partition first, prove the logs identical, and only then reach for rayon. No real event-queue optimisation — a BTreeMap keyed by time is correct; the calendar queue's O(1) is a 20× on a hot path that does not exist yet. No owner bot, no history: phase 5 waits, and is better for it, because a bot that can drive a fleet is more interesting than one driving a hull. **Done** = next_event_at exposed, the poll deleted, the jump-vs-step canary green, a Universe holding many worlds, and a measured number at 10⁶ written into this sheet.

##### sys://vocabulary — events & verbs

## the event vocabulary

The log can only contain what the sim can do, and forensics can only read what leaves residue. So every event earns its place by answering: **what physical trace does this leave?** If the answer is "none", it's telemetry, not history.

| event | emitted by | residue it leaves |
|---|---|---|
| PowerChanged | sim | none — telemetry. stays out of forensics |
| BreakerTripped | sim | none directly, but repeated trips → owner reacts → residue one hop later |
| ConditionDegraded | sim | the condition scalar itself — visible on inspect. neglect reads as a pattern of these, unanswered |
| ModuleReplaced | owner / player | serial + date discontinuity — the loudest witness on the ship |
| ConnectionRerouted | owner / player | deviation from the as-built schematic |
| RepairDeferred | owner policy | implicit — low condition on old serials, the fingerprint of a cheap decade |
| ModuleSalvagedIn | owner / player | used serial, wrong manufacturer for the hull class |

> [!card] the test for any new verb
Could a stranger with `survey` and no log *notice* that this happened? Yes → it's history. No → it's plumbing. Both are fine, but only one goes in the forensics view. **And every event carries a cause reference** — one field, the triggering event's id, sworn at birth in phase 2. It makes cascades walkable, the flight recorder a story instead of a list, and timeline diffs narratable chains instead of bare counts.

##### sys://rules — datalog, for future me

## how the crepe thing works

Datalog is a tiny logic language: **facts** (things that are true), **rules** (head `<-` body: "the head is true whenever the body is"), and an engine that applies the rules over and over until nothing new appears — the **fixpoint**. That's the entire language. It's SQL's expressive core plus recursion, minus the misery of recursive CTEs. A rule body is a JOIN, the head is a SELECT INTO, and I already write those for a living.

The killer feature for this project is that **recursion is free and safe**. "Energised spreads along wires through closed breakers" is one line, and the engine handles the traversal, the visited set, the cycles, and termination. The moment a question contains the word *"through"*, it's a Datalog question.

```rust
`// the whole of power reachability. no BFS, no stack, no visited set.
Energised(m) <- Source(m);
Energised(b) <- Energised(a), Wire(a, b), ClosedBreaker(b);

// stratified negation: "not" is allowed, just not inside its own recursion
Starved(m)   <- Consumer(m), !Pressurised(m);`
```

### the sandwich

Crepe embeds this in Rust as a macro that compiles the rules to plain fast code. It never touches `World` directly — every use is the same four-layer sandwich, and the rules engine only ever replaces the *query* in the middle, never plan/apply:

| layer | what happens | where the meaning lives |
|---|---|---|
| 1 project | `&World` → facts, via one exhaustive `match` | semantics. an open breaker isn't an `if` — it's a fact I decline to assert. absent = non-conducting |
| 2 solve | engine runs rules to fixpoint | pure physics-as-sentences. deterministic: same facts in, same set out |
| 3 plan | conclusions → `Vec<Change>` | datalog has no verb for "drain the battery" — mutation gets *described* here |
| 4 apply | changes land, events emit | the one mutation site, shared by every system |

**Coupling between networks is one membership test in the projection.** A pump asserts `RunningPump` only if power's solution says it's energised — the fluid rules never mention electricity, and an unpowered pump is indistinguishable from a switched-off one. Solve power once per resolution — per tick in the scaffold, per dirtying event under DES — and pass the result down.

> [!card] when to reach for it (and when not)
Not by default. Power's four roles are a five-line flood-fill — the walk wins. The rules engine earns its ~25 lines of projection ceremony when the propagation itself grows clauses: the data network's "deck sees sensor if a powered repeater chain connects them and no fault masks the bus" is the likely first customer, and `survey`'s forensics the second. The tell, both times: the walk fills with special cases, or the question contains "through". **Under DES, solves are scoped, triggered, and memoised:** per-ship-per-network, run only when an event dirties that scope (there is no tick to solve on), cached on unchanged fact sets — so most events never touch the rules engine and c stays in the light regime. Never a global solve: ships have fixpoints, the universe has a queue. If profiling ever wants true incrementality, the door is differential dataflow / DBSP — datalog with O(delta) updates. **And the ECS verdict, post-DES: keep its nouns, discard its verbs** — per-aspect tables are the right storage for formulas and projections alike, but the every-system-every-tick schedule is polling, and nothing here polls. ECS storage, DES control flow; bevy's full ECS stays correct in the app, which is a materialised view.
> And the accretion payoff: one rule at the bottom becomes a story at the top. `DeadHead(p) <- RunningPump(p), !Pressurised(p)` is one line → pump wear → the owner bot causes it by valving a line off → residue on the serial nobody replaced → survey flags it → I get to diagnose it. ==new symptom, new residue, new decision. passes the cdda clause three for three.==

**Learning path, one evening each:** learndatalogtoday.org for the mental model (it's the Datomic dialect — take the concepts, not the syntax) · Percival for a zero-install playground · the crepe docs.rs front page, which IS the tutorial · then the toy: twenty hard-coded facts about an imaginary ship, five rules, and that file becomes the first draft of `forensics.rs`. If the rules ever want counts or maximums, that's **ascent**'s aggregates — or derive the set in Datalog and fold it in an iterator after. Rules for logic, iterators for arithmetic.

##### sys://law — the laws of the universe — current, consolidated

## reference, for cold tuesdays

### repl grammar so far

```rust
`list · inspect <id> · status <id> · scan · survey
power on|off <id> · breaker open|close <id>
advance <n> · seed-fault · replace <id> · reroute <a> <b>
run-years <n> [policy] · seek <year> · save-log · replay <file>`
```

> [!card] the cost model — what the function charges for
**Pay per decision, per cascade, per message, per burn, per query — never per second, never per object, never per observer.** Cost(Δt) ≈ Σ events(Δt) × c_resolve, with resolution in microseconds. The variables, in dominance order: **agency** (deciding agents × wake frequency — population of minds, not matter; idle minds schedule sparse wake-ups), **cascade density** (self-limiting; hysteresis and coalescing are cost optimisations), **interaction channels** (the between-worlds emergence dial and the compute bill are the same knob — richness is priced in events), and **trajectory changes** (burns screen against the population; coasting paupers are cheaper than the torch-rich, which is thematically perfect). Free or nearly: elapsed time, existence (promises are O(1), positions closed-form), and attention (renderer never writes). Query costs: position O(1); biography O(its life's events); seek O(events since snapshot) — snapshot spacing is the knob; cold start O(spine ticks × cast), so the cast budget is a performance parameter. Parallelism: rayon across worlds, near-perfect; the serial floor is the busiest single world — and **merges accumulate**: docking and portals weld worlds, so undock splits promptly, and "how big can one world get" is the question to watch. ==time is free. existence is memory. agency is the currency.== **And the whole eager↔lazy spectrum is legal:** purity means evaluation order can't change values, so eager-one-world and universe-as-a-thunk produce identical bytes — eager pays per event as the clock advances, lazy pays per observation (causal cone forced at query time, memoised). Rule for choosing a point: **force what entangles, defer what's lonely.**And analytics have their own ledger:** interior facts are computed at event time, so galaxy-wide questions are retrieval — ad-hoc = a table scan (seconds over 10⁶ ships), anticipated = a standing query maintained O(delta) per event (microseconds at ask-time; incremental view maintenance, the DBSP door), lazy+naive = the ρ→1 catastrophe (hours), lazy+planner = predicate pushdown through the eager traffic layer (minutes). Four orders of magnitude, decided by when you told the universe you'd care. ==derelicts are the loneliest objects in the universe — the subject matter is why laziness fits this game so well.==

> [!card] time — one exact stream
**The destination is discrete-event simulation.** State between events is analytic; events (threshold crossings, decisions, commands, intercepts) are *solved for*, never discovered by stepping. Tick rate is a sampling resolution of exact state — there is no coarse model, so nothing can drift, and the inner life of the world is fully real at all times, everywhere: crises unfold unwatched, walk-ins are genuine, global queries have true answers. **Warp is a render ratio** — same event stream at any speed, the renderer never writes, warp auto-drops on queue events involving me. Cost scales with what happens, not with time: a quiet hulk's decade is microseconds at full fidelity, and the expensive thing is agents deciding, not objects existing. **Movement is never sampled, so nothing can tunnel** — trajectories are continuous functions (powered = flip-and-burn piecewise quadratics, unpowered = conics), intercepts are exact roots, events carry exact timestamps, ticks snap to events. The ticked sim of phases 1–5 is scaffolding under the liftability law. ==make the living work like the dead, and the montage stops existing because everything is always exact.==

> [!card] deep time — the macro spine
Cold starts run a **civilisational tick** — stations, traffic, factions, shipyards as aggregates at year/decade resolution — so 500 years of causal history costs milliseconds, and populations at any epoch sample from its tables. Individuals are promises: (fixture, policy, seed, duration), materialised sub-second on first observation, permanent archival state after (`pristine`, one-way). Biographies are bounded by object lifetime, with the spine as gazetteer. The cast of **named entities is budgeted** — hundreds of flagships, founders, notorious hulls simulated individually inside the spine, never thousands: df is slow because legends mode materialises everyone; this history is read through hulls, so full detail only ever needs to exist at the object being excavated. Coherence comes from three mechanisms: **pins** (a cast member's deeds are hard facts on specific objects — ten hulls she touched, excavated in any order, tell one life), **idea diffusion** (techniques and superstitions spread along the traffic graph; survey dates a reroute by which yard's trick it uses), and the **canonical-owner law** (rosters belong to hulls; careers are built by querying rosters — one derivation, many views, order-independent agreement). Any random NPC is thus a completed mystery the universe has already written. **And mobile entities carry their logs** — a traded gun is a tiny ship: trades, discharges, and condition ticks append at event time and travel with the item, so provenance queries are one-record reads (eager: milliseconds) or a bounded cone-crawl along the custody chain (lazy: seconds). Places aggregate; movers carry. Registries are the standing-query version maintained by institutions and are permitted to lie; wear is the log's physical shadow, so survey-versus-record works at item scale — and every traded item's chain begins, paracausally, with me. **Queries walk records, not counterfactuals:** a room's history is the events that reference it (one spine construction fact + a sparse log), not everything that could have mattered — builders aren't upstream, they're a deeper question, forced one stratum at a time (crew roster fn → assignment templates → demographics), each bounded, memoised, pinned. The regress has a floor: below the spine's grain, "why" stops being an event and becomes a distribution wearing a name — which is exactly where real archives bottom out too. **The spine is the historiography**, and the floor is permanently self-consistent under interrogation, because pure functions don't develop contradictions. ==the universe never tells stories. it leaves evidence, and the evidence agrees because reality is one function.==

> [!card] saves — the clock, the seeds, and my fingerprints
A save is: universe seed + clock + touched worlds (checksummed snapshot + short command tail, snapshotted at save time so the tail is ~zero) + the event queue. Loading: read the seed (zero work), evaluate positions from the closed forms at T (arithmetic — nothing was ever "behind"), deserialise snapshots + fold tails (milliseconds), restore the queue. Full replay-from-zero exists for two customers only: the CI test (snapshot+replay ≡ straight run) and the diegetic replays (flight recorder, year-5 scrub). Keep snapshot[n-1] as fallback — replay exactness makes the longer tail free insurance. Snapshots carry a schema version; new fields get explicit defaults; old logs never run under new sim code except through the versioned door. ==accretion changes the future's rules, never the past's records.==

> [!card] contact & collisions — settled, app-layer, not yet built
Detection is geometry, so it lives with Avian; the sim only ever hears `Command::Impact` or `Command::Cut` — one boundary crossing, deterministic and replayable from the log. Damage is deletions; connected components decides how many pieces exist (never the physics engine); debris is decorative until proven otherwise. Contact damage is **a per-cell stress accumulator, one mechanism at three tempos**: below threshold nothing (pushing — docking is the gentle end of the same pipeline), crossing it fails cells incrementally (crushing — a stream of small impacts, systems dying in sequence as the nose advances), all-at-once is every cell failing on the same tick (impact). Failures batch to one command per region per tick; splits must persist a few ticks before spawning a second body. Cuts commit at discrete points with an emissive capped edge — a laser scar and a crush scar are different shapes in the event log, so `survey` tells them apart. **Projectiles are trivial EDMD:** one leg, one solved contact, no zeno (bullets don't bounce to rest, they arrive) — the struck thing owns the scar at cell grade, the victim owns the wound, the shooter's log holds a reference, and flight paths are never stored because the tracer is a formula, re-derivable exactly for any replay. ==the sim decides what exists, physics decides where it drifts, rendering decides how clean it looks — and the three never wait on each other.==

> [!card] gravity — settled, fiction dial at hail mary
**Gravity is a derived field, never simulated.** Interiors compute in the co-rotating frame; `g(cell) = ω²r + thrust`, computed on demand like survey, stored nowhere. Regimes gate networks through the projection (a gravity-fed FluidLink simply isn't asserted in zero-g) and rated-g envelopes in the RON make regime change a fault axis — broken *under which gravity* is a diagnostic dimension nothing else produces. Fiction dial: torch drives, sustained ~1g burns. **Ships have two downs** — aft under burn (tower layout), none on coast — so every voyage exercises the regime faults twice, at flip. **Small ships:** the regime collapses to one down-vector — `None | Thrust(g)`; gravity is a phase of the trip, not a place on the ship, and the worksite stays zero-g (protecting the grab-and-brace bet). Magboots are EVA equipment, never a gravity substitute. **Centrifuge is the third variant:** `Spin { axis, omega }` — and **ω is itself derived, never authored**: the anchor is angular momentum L (conserved between torque events — thruster firings are linear L-legs, geometry gives τ = Σ r×F), inertia I comes from the cell mass distribution already computed for components, and **ω = L/I(configuration) is a formula**. Commands are only thruster firings; "spun up" is a solved threshold crossing. The skater effect is therefore free — winching the tether out grows I and drops ω automatically, so payout changes gravity through both channels of ω²r at once, exactly as the real rig would. Asymmetric thruster failure makes spin-up induce wobble: a symptom whose cause is a propellant valve three hops away. The propellant ledger is the spin history, readable by strangers. Canonical spin is about the design principal axis; past a wobble threshold the ship enters a `Tumbling` fault regime (an event state to diagnose, not euler dynamics to integrate — dzhanibekov stays in the appendix). Nothing physically spins in the engine (the app rotates the mesh and starfield, the interior simulates co-rotating). Spin motors, bearings and tether cables are ordinary modules with condition, so a neglected bearing IS partial gravity and a diagnosis; hub-to-rim gradient emerges from cell radius; despin-to-dock kills gravity-fed plumbing through the projection, zero new code. Spin+thrust mutually exclusive by procedure (skippable — the tilted vector exists as a punishment); the hub is the seam, smooth by construction. Scale honesty: partial g (0.1–0.3g) is the shipboard maximum, full g is station geography, thrust g is the voyage. **And fuel poverty is where kepler lives:** torch burn priced high means the early game routes on impulsive burns + conic coasts — literally KSP's patched-conic model, same piecewise-leg machinery, still exact — with transfer windows and phase angles as the poor salvager's puzzle, and brachistochrone straight lines as what affluence buys. Orbital mechanics is a progression axis, not a casualty of the torch. ==someone builds a ring hoping it works, and it does, because ω²r doesn't care about hope. a struggling salvager moves like the dead do — coasting the same conics as the hulks she hunts — and climbing the economy is migrating to the trajectory family of the living.==

##### sys://maths — the cost model, derived

## the algebra of the universe

Let **P** = objects (~**m** modules each), **A** = deciding agents at mean wake frequency **f**, **B(T)** = burns over horizon T, **c** = per-event resolution (µs-scale), queue size **Q ≈ A + P**. Total events:

```rust
`E(T) = A·f·T·(1+β)  +  γ·P·T  +  B(T)·(1+K)
agency + cascades   ambient decay   burns + conjunctions

// no Δt, no per-object term — that's DES.
// the ticked scaffold costs O((T/Δt)·P·m); DES wins by the universe's duty cycle.`
```

### the two poles

```rust
`EAGER:  Cost(T) = O(E(T)·(log(A+P) + m))     Query = O(1)
LAZY:   Cost(T) = O(1)                        Query = |⋃ᵢ C(qᵢ)| · (c + h)
// C(q) = causal cone of the query; h = thunk-graph bookkeeping ≈ c

ρ = |⋃C(qᵢ)| / E(T)          // fraction of causal history the player observes
Cost_lazy / Cost_eager = ρ·(1 + h/c)
lazy wins  ⟺  ρ < c/(c+h)   // with h ≈ c: iff you observe under half the universe`
```

### optimal snapshot spacing

```rust
`J(σ) = (T/σ)·C_s + n_seek·c·ē·σ/2      // storage + expected replay-to-seek
dJ/dσ = 0  ⟹  σ* = √(2·T·C_s / (n_seek·c·ē))   // spacing ∝ √(rarity of seeking)`
```

### the phase transition

Cone size is component size in the interaction graph. With **κ = ι·t** expected interactions per object (erdős–rényi), mean degree 1 is critical:

```rust
`κ < 1:  E[cone] = 1/(1−κ)         // O(1) in P — laziness asymptotically free
κ = 1:  cones go power-law, largest ~ P^(2/3)   // unstable
κ > 1:  giant component θ·P,  θ = 1 − e^(−κθ)   // one query forces ~everything;
// lazy degenerates to eager + overhead`
```

**"force what entangles, defer what's lonely" has a critical exponent.** The emergence dial (interaction density) is the temperature. Derelicts sit at κ ≈ 0 — cone = own biography, independent of P. The spine is the giant component, kept eager on purpose. The promise architecture is, formally, renormalisation: eager inside the giant component, lazy on the subcritical fringe. And **interface design is cone surgery** — cone size is set by the dependency min-cut, so pilots decide off coarse summaries (never full interiors) or one map query forces the world.

### worked numbers

```rust
`P = 10⁴ · A = 10³ @ f = 10⁻² Hz · T = 1 yr · c = 2µs · h = c
E(T) ≈ 3.15×10⁸ events                    // agency dominates, always
eager year-warp   ≈ 13 core-minutes
lazy year-skip    = 0, then salvager session (ρ ≈ 1%) ≈ 13 seconds
lazy + bad map query (θ ≈ 0.8) ≈ 17 core-min   // WORSE than eager. the transition, live
break-even ρ* = 50%   // completionists get the eager universe; hermits the lazy one`
```

> [!card] corollary — minds are DES-shaped too (the thousand-npc station)
A crowded station is only expensive if minds tick like game AI. They don't get to: **anchor + formula applies to behaviour** — an NPC is scheduled activity blocks (shift 0900–1700, market, quarters), events at boundaries and interactions only, position between decisions an analytic leg like any ship. f is a *design variable spanning four orders of magnitude*: per-step wandering (~1 Hz) vs activity blocks (~15 events/day ≈ 2×10⁻⁴ Hz) is a ~6000× swing. At activity grain, 10³ NPCs ≈ 5.5×10⁶ events/year ≈ **tens of core-seconds to warp** — even as the serial floor of one world. The economy couples through **narrow cuts**: NPC purchases feed an aggregate market node; the market emits price events at hysteresis thresholds, never per transaction; other stations' cones pull the *summary*, not a thousand biographies. The station is locally supercritical by design — internally one dense component — but its cut to the rest of the universe stays thin, so the density doesn't leak. ==the bustle is real. the footsteps are theatre. and hysteresis on the market ticker is a cost optimisation, same as the thermostat.== **And minds never poll:** "tick once per second" is ticked-world grammar — 1 Hz is fine at 1× (0.2%/core per thousand) but 17 core-hours per station-year under warp and ~1.5GB of log per agent-year, almost all failing the residue test. DES minds are **interrupt-driven**: wake on activity boundaries + subscribed events in perception scope — reaction latency ~zero, better than any poll, free while idle. Wake density escalates honestly with situation (idle ≈ 15/day, engaged ≈ minutes, crisis ≈ 1 Hz for its bounded duration): not a fidelity tier, just behaviour — a panicking engineer genuinely decides more. Subscriptions are β donations (a breach waking forty minds is aliveness), so they carry dampers: panic saturates, responders hand off. **Occupancy is a maintained view:** activity blocks are canonical room-grade positions, so every block-boundary event updates two room-sets (O(log n)) and "who's in the room" is a pre-paid read — even naive is 10k formula evals in microseconds. Sub-room position is theatre and the renderer never writes, so **incident victims are seeded draws over the canonical occupancy set**, weighted by canonical involvement (participants ≫ bystanders); witnessed incidents run as crisis episodes with real geometry, unwitnessed ones resolve at room grade and the theatre is staged to match the record — resolution grade logged like tempo, so replays never contradict. And the occupancy set at incident time IS the witness list: every unwitnessed firefight ships with a canonical roster of people who know something. **Positions upgrade to metre grade when rules consume them:** movement as waypoint legs through the corridor graph (~5× event rate, still trivial) makes position(t) a formula, exact at any precision — adopted the day line-of-sight, cover, alibis, or CCTV reconstruction exist to read it, and not before. The general depth law: **anything (anchor, formula, next-event) can join U at arbitrary precision; continuous chaos never fully can** — the floor is dynamical class, not compute, and precision that no rule consumes is dead weight wearing a decimal point.

> [!card] why any of this is legal
U is a pure fold; evaluation strategies are reduction orders; **church–rosser** says a confluent system has one normal form — order changes cost, never value. Eager, lazy, partitioned-parallel (independent events commute) are reduction strategies over the same term. ==the entire performance design space is choosing a reduction order for one expression.==

> [!card] the feasibility envelope (16 cores · 64GB · c=2µs · activity-grain minds · logs on disk · ±1 order everywhere)
Real-time 1× is never the constraint, anywhere — the binding axis differs per approach. **Eager one-world:** serial year-skip binds → ~3×10⁴ agents at a 5-minute montage, ~10⁵–10⁶ resident objects → **3–10 systems**: exactly the single-system game actually designed, which is why one-world-v1 is defensible. **Eager partitioned:** ×cores on warp, RAM binds → ~4×10⁵ agents, **~30–100 systems**. **Promise/hybrid:** compute stops binding at all — 10⁸ objects are 3GB of seeds, the spine carries 10⁵ stations as aggregate nodes, the touched set after 1000 played hours is a rounding error → **10³–10⁴ systems**, capped only by keeping κ subcritical: aggregate spine, thin inter-system cuts, summary interfaces on the map query. **Fully lazy:** 2⁶⁴ addressable and uncomputed; converges on the hybrid's practical numbers with worse constants and salsa-grade implementation cost. ==where objects are cheap, minds are the ceiling; where minds are deferred, my discipline is. the galaxy scales exactly as far as i can keep it lonely.==

> [!card] the accretion law — five years of rules, one by one
**Aliveness and the performance wall are the same threshold: β = 1.** Cascades multiply cost by 1/(1−β) — graceful until near-critical, divergent at criticality, where the universe breaks as physics before it breaks as performance (runaway avalanches, every spark a firestorm). The alive feeling is the *near-critical band from below*: power-law event avalanches, mostly small, occasionally magnificent — where df lives. Rule count never walls: evaluation is fact-driven, and ~1,500 rules across partitioned crepe programs is comfortable. Budget: ~1–3 rules/session ⇒ five years ≈ **500–1,500 rules**; ~100–300 makes a ship feel alive (the paper dossier did it with 17), ~500–1,500 makes a world — the five-year cadence lands in the world band. The monotone contract (add forever, never revisit) holds under one discipline: **every rule ships with its damper** — consumes something finite, saturates, hysteresis, cooldown, or dead-ends into residue — so each rule caps its own β donation locally and global criticality stays approached, never crossed. Content grows ~R² (interactions) while authoring grows R, so the marginal evening buys more universe every year. Taxes: curate the fact vocabulary (rule 900 must find what rule 200 produces), and let CI define "unhappy" — fixture fleet, rate-consistency distributions, and an avalanche-size histogram standing watch on β. ==the thermostat mod was the second law of thermodynamics, arriving early. tune toward criticality; stop where avalanches are stories, not weather.==

##### sys://data — learning ledger

## what i'm learning & when

### now, in context

- **Ownership via graph modelling** — IDs over references, split borrows (borrow two fields, not `&mut World` at everything), compute-then-apply. Will feel like pointless ceremony for a week. Then it won't.
- **Enums + exhaustive `match`** as the whole behaviour system. The pattern-matching instinct from Elixir, transplanted more or less intact.
- **`serde` + `ron`** for fixtures and saves. Savegame = one derive on one struct, because everything lives in `World`. That's the payoff for the box being a box.
- **Hand-rolled REPL parser** — more fun than clap, and it *is* the deck's command grammar, so it's not even a detour.
- **Property testing** over fixtures: solver completes, no orphaned modules, every symptom traces to a seeded cause.
### later, when it's earned

- **Datalog** via crepe (then ascent if aggregates call) — see sys://rules for the whole path. One evening for the toy, adopted in anger the week `survey.rs` hits its third nested loop.
- **Bevy** — pin the version, upgrade at milestones only, never mid-feature, no exceptions — plus **Avian** for the zero-g body and **big_space** for coordinates. Ship interior lives in a nested grid; the sim never finds out where the ship is, and that's the point.
- **Patched conics + flip-and-burn legs** — universal-variable Kepler for the dead and the broke, piecewise quadratics for the flush. Analytic, so time warp is free and conjunctions are a calculation, not a discovery.
- **rayon** across worlds. **`Vec`-indexed tables** if a profiler ever asks nicely. Not one second before.
### deliberately not learning yet

Bevy's query API, shaders, netcode, trait-object plugin architectures, anything n-body, DES event-queue machinery (the liftability law keeps the door open; walking through it is a phase 8+ decision). All of it is either `app/`-layer or nobody's problem until the bets pay out.

##### sys://forensics — decision log

## the log gets excavated, so keep one

One Obsidian note per decision, with the *why* attached. Six months in, the danger isn't a hard problem — it's me re-arguing a settled one because I forgot how the argument went. The ships get event logs; so does the project. The record so far, amendments preserved because the past doesn't get edited:

- **2026-07-30** DECIDED — sim crate has zero bevy deps; the REPL is the deck, and the deck comes first
- **2026-07-30** DECIDED — module-level abstraction, four coupled networks — depth from count, not resolution
- **2026-07-30** DECIDED — runs routed by hand through finite conduit space; derelicts are never greenfield; reach-in subfloor by default
- **2026-07-30** DECIDED — ecs shape, no ecs framework; determinism outranks parallelism inside a world
- **2026-07-30** DECIDED — ship-ness derived from connectivity — bisection is deletions, not a feature
- **2026-07-30** DECIDED — world = region of simultaneity; universe = event queue over analytic orbits
- **2026-07-30** DECIDED — history = the same sim run by a dumb owner bot; richness is capped by the sim's verb vocabulary, so replace / reroute / neglect come forward
- **2026-07-31** DECIDED — contact = per-cell stress accumulator; pushing / crushing / impact are one mechanism at three tempos; sim decides what exists, avian decides where it drifts, rendering decides how clean the cut looks
- **2026-07-31** DECIDED — gravity = derived field, never simulated; regimes None | Thrust | Spin{ω}; spin motors/bearings/tethers are ordinary modules, so gravity faults ARE condition faults; spin+thrust exclusive by procedure, hub is the seam
- **2026-07-31** DECIDED — tempo follows attention; the tempo that ran is the canon; saves = clock + seeds + fingerprints
- **2026-07-31** AMENDED — tempo default inverted: fine tick everywhere at 1× — the world's inner life when nobody's there is a load-bearing value; costs accepted knowingly
- **2026-07-31** AMENDED — no coarse model, ever: the destination is discrete-event — analytic state between solved events; every "tempo" is a sampling of one exact stream, so drift is impossible; fixed tick stays as scaffolding under the liftability law
- **2026-07-31** DECIDED — fuel poverty is where kepler lives: transfer windows as the poor salvager's puzzle, straight lines as what affluence buys — orbital mechanics is a progression axis, not a casualty of the torch
- **2026-07-31** DECIDED — docs merged: the plan is the law, the log is the history, one file holds both
- **2026-08-01** DECIDED — ω is derived, never authored: L is the anchor (conserved between torque legs), ω = L/I(configuration) — the skater effect, wobble faults, and the propellant ledger all emerge
- **2026-08-01** DECIDED — minds never poll: interrupt-driven, wake on boundaries + subscriptions; wake density escalates honestly with situation; occupancy is a maintained view; incident victims are seeded draws over canonical rosters
- **2026-08-01** DECIDED — movers carry their logs, places aggregate; queries walk records, not counterfactuals; the regress floor is the spine's distributions — the spine is the historiography
- **2026-08-01** DECIDED — every event carries a cause reference — cascades walkable, diffs narratable, sworn at events' birth in phase 2
- **2026-08-01** DECIDED — the client is a subscriber: perception sphere renders, consequence sphere is canonical, cascades resolve in the queue — rendering cannot break what it is forbidden to touch
- **2026-08-04** DECIDED — settle per accepted command, not once per batch — same events in the same order, but every PowerChanged then names the flip that actually caused it instead of whichever command was last in the queue. the phase 2 sheet's "apply commands, then propagate" left batch attribution undefined; this is the smallest reading that keeps the cause law true
- **2026-08-04** DECIDED — tick_power and tick_depletion run BEFORE apply_commands — consequences scheduled earlier fire at the top of the tick. the delay is what stops a trip feeding back into its own cause, and a switch flipped on the death tick is honestly too late to save her
- **2026-08-04** DECIDED — the sheet's tick 6260 is unreachable with the sheet's own draw table: 23A is BRK-01 alone, but the battery carries both breakers at 29A, so 144_000/29 = 4965. the phase 3 blurb grabbed the wrong number off its own step 1 — the arithmetic, not the code, was wrong
- **2026-08-04** DECIDED — depletion darkens the ship in the SAME tick, not the tick after — the sheet's test blurb contradicts its step 2, and unlike a thermal breaker there is no physical reason a flat battery holds the lights up for another second
- **2026-08-04** DECIDED — kirin house style is invented, not canon: KR-YYYY-NNNN, aftermarket, arrives used. the 2155 heater is a 2151 unit fitted in 2155 at condition 0.71, and its galley reroute leaves a 3-way run carrying four — over cap, silently, until run caps exist to notice
- **2026-08-04** DECIDED — adjacency order follows fixture line order, and that is accepted — deterministic per file, but reordering two connection lines describing the SAME ship yields a different event log and both canaries pass. "same ship" is a stronger claim than "same file"; only the weaker one is tested. revisit if a fixture ever gets tidied
- **2026-08-08** DECIDED — derating ramps rather than scales: rating × (condition / 0.70), normalising the band onto 0–1 so it is continuous at the threshold. flat rating × condition would drop a 25A breaker to 17A across a hair's difference in condition. one named const feeds both the branch and the divisor so they cannot drift
- **2026-08-08** DECIDED — intermittence CUT. the spec could not produce a flicker anyway — a faulted supplier opens breakers, breakers stay open, so the first bad roll is permanent. and a fault you cannot reproduce is a fault you cannot diagnose: scan would report differently each run, making the instrument unreliable rather than the puzzle harder. "fine until demand peaks" already exists deterministically in load-dependent trips. revisit only if gate one finds diagnosis too easy
- **2026-08-08** DECIDED — fault events carry BOTH the nameplate and the effective rating. with one figure you cannot tell later whether 17A was a small breaker or a tired one — and condition moves in phase 5, so the log must stay self-describing without knowing what condition was at the time
- **2026-08-08** DECIDED — cause chains deliberately do NOT name the seeded fault. the cascade names the load that pushed the breaker over, because that is what physically happened; noticing that 25 and 17 disagree is the player's job. asserted by faults_have_causes so nobody "fixes" it into a debugger later
- **2026-08-08** DECIDED — a breaker opening emits BreakerSet{closed:false} caused by the trip — closed=false was the only state change in the sim that did not announce itself, and a half-dark ship had nothing in the log explaining why. no new variant: cause None means the player threw it, Some means the sim did. the chain gains its missing hop — dark ← opened ← tripped ← load ← switch ← None
- **2026-08-08** DECIDED — dead consumers draw nothing but stay ENERGISED. a dead lamp still has power at its terminals; modelling it as unenergised would be a lie status repeats, and would make the breaker upstream look like it isolated something it did not. a failed heater is not heating AND not eating, so it stops draining the battery too — that falls out of downstream_load for free
- **2026-08-08** DECIDED — SetCondition carries a Condition, not an f32 — an out-of-range value is unrepresentable in the command, so the sim needs no rejection for bad values, only NoSuchModule. errors unrepresentable where possible, applied to a command payload. validation happens once, at the deck's parse, where "abc" and "5.0" are different mistakes
- **2026-08-15** DECIDED — GATE ONE PASSES. five hulls seeded blind (shuffled at generation, so neither of us knew the mapping); the deck led the diagnosis rather than code knowledge, and the healthy control hull was correctly read as healthy — the instrument does not cry wolf. the loop is scan → pattern → hypothesis → inspect → confirm, and it holds
- **2026-08-15** NOTED — known hole: a dead consumer is undiagnosable. it draws nothing, nothing trips, every other module reads nominal, and the only trace is the battery outlasting its expected burn. no symptom in the vocabulary means "drawing under its nameplate". deferred to phase 6, where a dead scrubber stops scrubbing and the consequence becomes visible in another network — fixing it now would mean an instrument reading amps at a module, which edges toward naming the fault
- **2026-08-15** NOTED — 3–4 commands per diagnosis, which is fast — but a testudo is 14 modules, one bus, two breakers, so the search space is tiny and that number describes the fixture more than the mechanism. re-measure on a bigger hull before concluding the chase is too easy. if it is still 3–4 on forty modules, THEN widen the gap
- **2026-08-15** DECIDED — list drops the condition column; inspect keeps it. one command printing every module's condition made the blind playtest unwinnable-as-designed — read the column, done, scan never used. the sheet always said the seeded fault is hidden from normal output; list IS normal output. condition now costs one command per module, so you must suspect before you can check, which is the loop

##### sys://later — filed, not forgotten

## later.md — the good ones

None of these touch the path to gate two. All of them are enabled by decisions already made, which is why they're written down now — each constrains nothing and inherits everything.

- **The zero-g movement toy.** Bevy + Avian, a capsule, joint-based grab-and-brace, a grey-box corridor, evenings of pure tuning. The first-person bet lives or dies here — the sim can't answer it, and it doesn't get to pick the camera by default
- **Quote-before-you-know contracts · orbital-decay deadlines · the home ship · the mystery layer over fault trees.** The career loop, filed whole. wants: gate two passed and an economy
- **Sound as a sim output.** Modules emit signatures scaled by condition; sound travels through pressurised rooms or structure (muffled), never across vacuum. The ship coming alive is *heard*, faults are diagnosable by ear, helmet-to-bulkhead listening in vacuum is free. wants: the room graph, which is being built anyway
- **The flight recorder.** A findable module holding a partial command log of the ship's final hours — power it and replay the death on the deck, tick by tick. Damaged by default; the truncation IS the mystery. wants: the determinism already sworn. it's the replay system pointed backwards
- **The manual that's wrong.** Per-class in-fiction manuals generated from the RON catalogue — but the derelict's manual describes its class and year, not this hull with its decade of bodges. The diff between manual and ship is the history, readable. wants: the catalogue + as-built, both already planned
- **Fuel that is also coolant.** The astrophage move: torch fuel is the primary heat sink, so thermal load spends delta-v and a long job in sunlight has a fuel cost. One substance asserting facts in two networks; the economy knob and the thermal endgame become the same knob. wants: the network solvers, already designed
- **The ship's cat.** A life-support consumer, a zero-g physics object with opinions, and a residue generator — claw marks, a hair-clogged filter, a dish zip-tied to a rail in the one compartment that held pressure. Finding out the derelict had a cat, and then wondering, is the most human beat the generator can produce. wants: a handful of RON entries and nerve. df precedent: the cats are load-bearing
- **The procedural catalogue — brands, product lines, every universe slightly different.** Archetypes stay hand-authored as bounded templates; the seed generates *manufacturers* as spine entities — foundings, product lines per era, market share by region, recalls, bankruptcies — and each brand gets house traits within the archetype's envelope: vostok overbuilds and eats filters, someone's 4400 series has a bearing that dies young. Era-dating by product line, regional dating by market share, and a *recall* is a mystery generator — every hull carrying that part has the same latent fault, galaxy-wide, discoverable. Canonical-owner law keeps it coherent; the shareable-seed culture gets folklore: *in this universe, never buy kirin pumps.* wants: the spine + the RON templates, both already planned. guardrail: generation perturbs within hand-authored envelopes — generate-working-then-break, applied to industry
- **Physics inside U — episodes, or full event-driven.** Default stance: loose items are event-grade truth, positions theatre. Tier one upgrade: deterministic physics *as episodes* (step only during burn/impact/shove, sleep aggressively — a sleeping object is anchor + constant, already DES-compliant). **Tier two, the principled target: event-driven molecular dynamics** (alder & wainwright 1959 — older than stepped game physics). Bodies fly ballistic legs (linear in zero-g, parabolic under thrust, both closed-form); contact times are *solved* (sphere–sphere = quadratic root — conjunction screening, miniaturised); nothing steps, ever; a drifting spanner costs zero events until its predicted arrival. Physics becomes warp-proof and canonical — the last poll dies. The three classical pains each meet an existing law: tumbling boxes go transcendental → canonical clutter is capsules, rotation is theatre; the zeno bounce-chatter is a restitution cascade at criticality → the rest threshold is its damper (legislated settling, filed early, was this all along); piles are constraint misery → resting groups freeze to compounds, and zero-g interiors barely pile. Player hands arrive as commands; a hard burn is a bounded burst of new legs + solved bulkhead rendezvous — the dent-pattern generator, made exact. wants: nothing before the app era. tier one adoptable cheap; tier two is the beautiful version — capsules, cell lists over the room grid, and nerve
- **The geometry function — appearance is history, rendered.** `crates/geom`: meshes as a pure function of canonical state (cell grid + RON shape templates + cut planes), zero bevy deps, canonical vertices per the hardware-boundary law — voxel-engine chunk meshing wearing this project's vocabulary. The catalogue grows a visual axis: brand house-styles as mesh grammars (a vostok pump LOOKS overbuilt), so era-dating works by eye. And damage derives from the log — dents at impact cells, scorch along thermal events, patina from condition, the shortcut visibly the wrong gauge — **no artist paints damage; templates are authored, the specific is derived, and survey and the eyeball read the same function**. Raymarching pixels straight from data (dreams/claybook lineage) is viable but buys no canon — gpu floats are theatre regardless. The deterministic exception: a tiny software raytracer (libm, 256×256, offline) for the deck's survey photographs — **canonical evidence images**, bit-identical from the same state on any machine: photographs that can be proven. wants: shape templates in the RON, one pure crate, and — for the evidence camera — a forensics game's sense of humour about frame rates
- **Stations: grown, not authored.** A station is a ship that accreted: section templates (ring, spar, hab block, dock arm — the catalogue law, authored once) instantiated by the spine's construction record, so geometry is a fold over economic history and stations have literal **growth rings** — module cohorts and idioms dating each era. Population is an indexed roster function `person(station, i)` over spine demographics; schedules are role templates × the place graph — ten thousand coherent lives, none stored. Occupancy resolves in three strata forced by consequence: aggregate table (role mix × counts, nearly free) → seeded index draw for incidents (same moment always re-derives the same people) → materialise only the drawn dozen. An unwitnessed firefight in a 10k station forces ~12 biographies, ever. The visual view is the same laziness wearing meshes: exterior renders from the section list (the spine record is LOD-0), interiors force by proximity inside docking and airlocks, crowds render from aggregates — correct in count and mix, individually theatre — and **talking to a stranger IS the materialisation event**: every extra is a promise, and curiosity is the forcing function. wants: section + role templates in RON, the spine, and the roster function. the game's thesis, operating at eye level
- **Planets: sites, not surfaces.** Already in as celestial anchors (conics need something to orbit and decay into). As places: *a planet base is a ship that doesn't move* — full World machinery plus the easiest regime yet, `Surface(g)` — and the room graph gains an **infinite reservoir node**: a planet is a room graph where one room is infinite. Terrain is a site graph (settlements, wrecks, pads as nodes; travel as solved legs; kilometres of regolith are theatre); weather is a seeded event stream, damper-shaped by nature; crash sites are the contact pipeline's output as geography — archaeology with strata. Descent: torch ships land propulsively, so entry is a quadratic leg — the fiction dial deleted aerobraking, the one non-analytic manoeuvre. In the spine a planet is a very entangled station: populations as aggregates, budgeted cast, and **the spaceport is the min-cut** — thin by law and by realism. Refused, permanently: geology, ecosystems, voxels — that's df's other half and a different five-year project. wants: one regime variant, one reservoir node, one site-graph edge type. everything else is a skybox
- **Multiplayer = lockstep, already mostly built.** U(seed, logs[]) — the player was the one non-derivable process; multiplayer makes it N. Deterministic lockstep (the factorio / "1500 archers on a 28.8k modem" model): exchange commands only, everyone folds the same ordered stream. The constitution is accidentally the readiness checklist — libm is cross-machine sync, checksummed snapshots are desync detection, the save format IS the late-join protocol, replays are spectating. Real costs: total ordering + input delay (or rollback, which cheap snapshots + exact replay make unusually viable), and time becomes negotiated — warp is a group decision, the montage needs consensus. Async co-op across branches is already legal under the wormhole laws: a friend's log folds as a divergent world, their ghost working their jobs one intrusion away. wants: the determinism already sworn, a network layer as one more client of the boundary, and friends
- **Wormholes: branching + the timeline diff.** The past is derivable, so a viewing wormhole is free; travelling back is inserting a command at an earlier timestamp and re-folding — a new branch, multiverse-style, because two logs literally ARE two universes. Paradoxes are unrepresentable; the payoff is diffing branches event-by-event and watching causality ripple — and **divergence is causal, not numerical:** confluence keeps everything outside the intervention's cone bit-identical across branches, so the diff IS the lightcone of the change, finite and queryable ("who exists here that died there" returns names), while seed-indexed identity means the same souls live different fates in each branch. The butterfly effect, with a paper trail. And the diff pays only for the lightcone — identity outside the cone is proven by confluence, never checked — so the full moral ledger of an intervention ("saved 10,412; killed 37 downstream, including the pirate you created") costs O(cone), reads as a walkable chain via cause references, and is **counterfactual causal inference made executable** — pearl's third rung as a game mechanic, attribution clean by construction because exactly one input differs. Only the player can run it: everyone in-fiction lives one timeline, so perfect knowledge of what choices cost is the paracausal being's private burden. Meeting yourself: a divergent counterpart — same log to the branch point, then their own person — who leaves YOUR residue on hulls you never touched, which is an ayla case where the culprit is the detective, one branch over. wants: the pure function, already sworn. cost: a worldgen bar per branch. novikov loops: never
- **Portals (spatial, in-universe).** The sim was never euclidean — every system runs on connectivity, so a portal is an edge and the sim cannot tell a wormhole from a hatch: air, sound, cables, and pathfinding cross it for free. Trajectories treat mouths as leg boundaries; an open portal merges worlds, a closed one splits them; two hulls joined only by a portal are two bodies sharing one atmosphere. Newtonian fiction means no accidental time machine. Real cost is all app-layer (render-through, cloned colliders mid-mouth). Forensics dividend: survey detects REMOVED portals — as-built paths shorter than physically possible. wants: one edge type and nerve. the topology-first instinct pays its whole debt here

##### sys://glossary — the vocabulary, for month eighteen

## acronyms & symbols

### the theory

| DES | **Discrete-Event Simulation** — the destination: state analytic between events, events solved and processed from a queue in timestamp order. Nothing steps; cost ∝ happenings, not time. |
|---|---|
| EDMD | **Event-Driven Molecular Dynamics** — DES applied to physics (alder & wainwright, 1959): ballistic legs, collision times solved as roots, contacts as queue events. Tier-two loose-object physics. |
| ER | **Erdős–Rényi** random graphs — mean degree κ < 1 → small components; κ > 1 → a giant component. Governs when laziness stops being cheap. |
| β | cascade **branching factor** — consequences per event. Cost × 1/(1−β); β = 1 is the wall AND the aliveness threshold. |
| κ | expected interactions per object over a horizon — the ER mean degree; the entanglement/emergence dial. |
| ρ | fraction of causal history the player's queries force — lazy beats eager iff ρ < c/(c+h). |
| U(seed, logs) | the universe function. everything else is a corollary. |

### the engineering

| ECS | **Entity Component System** — IDs + per-aspect tables + systems as functions. Taking the shape, not the framework. |
|---|---|
| REPL | **Read-Eval-Print Loop** — the terminal. Here, diegetic: the REPL IS the deck. |
| RON | **Rusty Object Notation** — the fixture/catalogue/policy data format. |
| DAG | **Directed Acyclic Graph** — the network coupling order, kept acyclic by couplings-cross-tick-boundaries. |
| CI | **Continuous Integration** — test-on-push; the formal definition of "unhappy". |
| RNG | **Random Number Generator** — one, seeded, ChaCha8, the only randomness in the crate. |
| BVH / CCD | **Bounding Volume Hierarchy** (contact-candidate pruning) / **Continuous Collision Detection** (app-layer anti-tunnelling; the sim never needs it — movement is solved). |
| GGPO | the rollback-netcode lineage — the alternative to input-delay lockstep, unusually viable here (cheap snapshots + exact replay). |
| IEEE 754 / FMA / SIMD | the float standard / fused multiply-add / vectorised maths — the three horsemen of cross-machine divergence, quarantined by the libm law. |
| PHOLD / UPS | the standard DES throughput benchmark (millions of events/sec/core is real) / updates-per-second, factorio's sim-rate term. |
| IVM / DBSP | **Incremental View Maintenance** — standing queries updated O(delta) per event (occupancy sets, death censuses, provenance indexes) / **DBSP**, the differential-dataflow lineage: datalog under insertions and deletions — the door if crepe's batch re-solves ever need true incrementality. |

### the prior art

| DF · CDDA · GTNH | dwarf fortress (history as sim output) · cataclysm: dark days ahead (accrete depth forever) · gregtech: new horizons (the routing puzzle is non-negotiable). |
|---|---|
| KSP · PHM | kerbal (patched conics, the thrift-mode trajectories, the warp interlock) · project hail mary (the fiction dial: torches, the tether rig, competence as gameplay). |
| SE · X4 | space engineers (the destruction-performance cautionary tale) · x4: foundations (never ship the a/b). |
| MD · AoC · RCS | molecular dynamics · advent of code (phase 0 — it knows) · reaction control system, the spin thrusters. |

### reading paths — one evening each, adopt in anger later

- **DES:** SimPy (python) for hands-on first contact — processes, events, queues in an afternoon · the event-scheduling chapter of Law, *Simulation Modeling and Analysis* (skip the statistics half) · Jefferson's *Virtual Time* (1985) — time warp / optimistic rollback, which is the multiplayer-rollback idea in academic form · Fujimoto, *Parallel and Distributed Simulation Systems* when partitioning gets real · rust crates to read: `nexosim` (née asynchronix).
- **EDMD:** Lubachevsky, *"How to simulate billiards and similar systems"* (1991) — THE practical algorithm paper: event queue, cell lists, per-particle event trees, directly implementable · Pöschel & Schwager, *Computational Granular Dynamics* part II — includes **inelastic collapse**, which is the zeno bounce problem's real name and treatment · DynamO (bannerman et al., 2011) — open-source general EDMD engine with readable source · Alder & Wainwright (1959) for the pleasure of reading the origin.
- **ER & criticality:** Barabási, *Network Science* — free online, chapter 3 is ER + the giant component, very readable · Newman, *Networks: An Introduction* for rigour · the Galton–Watson **branching process** (any lecture notes) — the actual maths of β · Bak, *How Nature Works* (pop-sci, self-organised criticality) for the aliveness-at-the-edge worldview, sandpiles and all.

##### sys://ritual — between sessions

## session log
