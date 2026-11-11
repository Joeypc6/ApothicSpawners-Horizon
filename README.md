# Apothic Spawners for Horizon

An unofficial port of [Apothic Spawners](https://github.com/Shadows-of-Fire/Apothic-Spawners) 2.0.1 (Minecraft
26.1.2, by Shadows_of_Fire, MIT) to [Horizon](https://github.com/horizon-rust/horizon), as one native mod. The Java
mod was rewritten in Rust for Horizon's hooks.

- `mixins/apothic_spawners`: the mod, and its content pack in `mixins/apothic_spawners/data`.
- `tools/`: make the content from the original jar and write the bundle.

## What's there

- Every spawner modifier (items used on a spawner: delays, counts, range, no AI, youthful, silent, ignore players,
  ignore light and conditions, echoing, and the rest), the spawner's saved stats, silk touch with Java's tool wear,
  capturing, banned spawn eggs, unstable spawners, the advancements, Jade and the JEI category.
- No-AI spawner mobs still move as in Java: they fall, get pushed and take knockback, but never walk.

## What differs from the Java mod

- A movable no-AI enderman can still teleport when hurt (no teleport hook in Horizon yet), and jockey passengers
  don't get the spawner's settings.
- Built for Horizon fdc87ae (Nightly 2026-11-08); see `horizon-native.json`.

## Tested

In game on Horizon `fdc87ae` builds (the last, `native-fdc87ae-83d10460ef68`, with the nine other ports): every modifier through a
spawner and its mobs (baby, no AI, silent), the delay rule (min above max), no-AI spawner mobs falling and a `/summon` NoAI mob staying
put, and the W guide. On Horizon `6560641`: silk touch wear, ignore conditions, despawn delay, echoing, unstable spawners, Jade and the
JEI category.
