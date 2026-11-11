"""Writes the Apothic Spawners bundle's horizon-mixin.toml and its one-line hook helpers, and packs the content.

Usage: python make_bundle.py
"""

import json
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
BUNDLE = ROOT / "mixins" / "apothic_spawners"
BUILD = ROOT.parent / "native-build" / "main" / "apothic_spawners"
JAR = ROOT.parent / "ApothicSpawners-26.1.2-2.0.1.jar"
if not JAR.exists():
    JAR = ROOT.parent / "Ported" / JAR.name
HORIZON = "fdc87aecdbd116bf1e08adcd062fcbaea8c2e2ca"
MIXINS = "5a64af938ec86b00d53d16a408aabc0038fb3e57"
VERSION = "2.0.1"
M = "crate::game::apothic_spawners"

GAME_MUT = ("game", "&mut crate::game::Game<'_>", "&mut *self")
GAME = ("game", "&crate::game::Game<'_>", "&*self")

# (id, hook, bindings, body)
HOOKS = [
    ("use-on", "use_item_on", [GAME_MUT, ("stack", "&ItemStack", "stack"), ("pos", "[i32; 3]", "pos"), ("claimed", "&mut bool", "&mut *claimed")], "use_item_on(game, stack, pos, claimed)"),
    ("placed", "place_block", [GAME_MUT, ("pos", "[i32; 3]", "pos")], "placed(game, pos)"),
    ("destroy", "destroy_block", [GAME_MUT, ("pos", "[i32; 3]", "pos")], "destroyed(game, pos)"),
    ("loot", "loot_drops", [GAME_MUT, ("context", "&crate::mod_api::NativeLootDrops<'_>", "context"), ("drops", "&mut Vec<ItemStack>", "&mut *drops")], "loot_drops(game, context, drops)"),
    ("spawned", "mob_spawned", [GAME_MUT, ("dimension", "&str", "dimension"), ("kind", "&str", "kind"), ("pos", "[f64; 3]", "pos"), ("natural", "bool", "natural")], "mob_spawned(game, dimension, kind, pos, natural)"),
    ("explosion", "explosion_detonate", [GAME_MUT, ("dimension", "&str", "dimension"), ("center", "[f64; 3]", "center"), ("blocks", "&mut Vec<[i32; 3]>", "&mut *blocks")], "explosion(game, dimension, center, blocks)"),
    ("net-from-client", "net_from_client", [GAME_MUT, ("player", "u128", "player"), ("channel", "&str", "channel"), ("data", "&[u8]", "data")], "net_from_client(game, player, channel, data)"),
    ("net-from-server", "net_from_server", [("channel", "&str", "channel"), ("data", "&[u8]", "data")], "net_from_server(channel, data)"),
    ("level-tick", "level_tick_post", [GAME_MUT, ("dimension", "&str", "dimension")], "level_tick(game, dimension)"),
    ("inventory-tick", "inventory_tick", [GAME_MUT, ("slot", "i32", "slot"), ("selected", "bool", "selected")], "inventory_tick(game, slot, selected)"),
    ("tooltip", "tooltip", [GAME, ("stack", "&ItemStack", "stack"), ("lines", "&mut Vec<(Text, u32)>", "&mut *lines")], "tooltip(game, stack, lines)"),
    ("kit-frame", "frame", [GAME_MUT], "kit_frame(game)"),
    ("joined", "entity_join_level", [GAME, ("entity", "u32", "entity")], "entity_joined(game, entity)"),
    ("left", "entity_leave_level", [("entity", "u32", "entity")], "entity_left(entity)"),
    ("stopping", "server_stopping", [("dedicated", "bool", "dedicated")], "server_stopping()"),
]

# ASEvents.tickDumbMobs (EntityTickEvent.Pre): a no-AI mob marked `apotheosis:movable` still travels, with its AI
# turned on only for the call. HEAD of the mob step's `pre_tick`: the first part of `Mob.tick`, after the old position
# and tickCount are taken, as the event fires in `ServerLevel.tickNonPassenger`. Any other mob leaves at `no_ai`.
DUMB_MOBS = """fn apothic_spawners_dumb_mobs(m: &mut crate::mobs::Mob, w: &crate::mobs::MobWorld<'_>, events: &mut Vec<crate::mobs::MobEvent>) {
    if m.no_ai && apothic_spawners_movable::is_movable(m.id) {
        m.no_ai = false;
        let input = horizon_core::geometry::Vec3::new(f64::from(m.ai.xxa), f64::from(m.ai.zza), f64::from(m.ai.yya));
        m.travel(w, input, events);
        m.no_ai = true;
    }
}
"""

# The spawner's server tick: Apothic's rules first (`BaseSpawner.serverTick` in `SpawnerLogicExt`). Cancellable.
TICK = """fn apothic_spawners_tick(game: &mut crate::game::Game<'_>, key: (i32, i32, i32), spawner: &mut horizon_client::mobs::spawner::Spawner, cart: Option<u32>) -> core::ops::ControlFlow<Result<()>> {
    crate::game::apothic_spawners::spawner_tick(game, key, spawner, cart)
}
"""

SPECIAL_HELPERS = {
    "embedded_content.rs": """// Apothic Spawners is one file on the registry: its data and textures are the content mod in this bundle's data/
// (made by tools/make_content.py from the jar). It is written out once and loaded as a dev mod is.
fn apothic_spawners_embedded_content(root: &std::path::Path, enabled: &std::collections::BTreeSet<String>, dev: &[std::path::PathBuf]) -> core::ops::ControlFlow<std::path::PathBuf> {
    static CONTENT: &[u8] = horizon_client::native_data!("apothic_spawners", "apothic_spawners-content.hmod");
    let dir = std::env::current_dir().unwrap_or_default().join(".horizon-cache");
    let file = dir.join("apothic_spawners-embedded-content.hmod");
    if dev.contains(&file) {
        return core::ops::ControlFlow::Continue(());
    }
    let same = std::fs::read(&file).is_ok_and(|on_disk| on_disk == CONTENT);
    if !same && std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(&file, CONTENT)).is_err() {
        log::warn!("Horizon: Apothic Spawners' content could not be written to {}", file.display());
        return core::ops::ControlFlow::Continue(());
    }
    let mut with = dev.to_vec();
    with.push(file);
    core::ops::ControlFlow::Break(content_root(root, enabled, &with))
}
""",
    "list.rs": """fn apothic_spawners_list(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    mods.push(crate::game::native_hooks::NativeMod {
        id: "apothic_spawners".into(),
        title: "Apothic Spawners 2.0.1".into(),
        description: "Spawners you can pick up with Silk Touch and change with items, as in Apotheosis.".into(),
        keys: Vec::new(),
    });
}
""",
    "h_tick.rs": TICK,
    "h_dumb_mobs.rs": DUMB_MOBS,
    "h_changed.rs": """fn apothic_spawners_changed(level: &crate::game::GameLevel<'_>, pos: horizon_core::position::BlockPos, old: horizon_world::registry::BlockStateId, new: horizon_world::registry::BlockStateId) {
    let spawner = |s: horizon_world::registry::BlockStateId| &*level.facts.get(s).name == "minecraft:spawner";
    if spawner(old) != spawner(new) {
        crate::game::apothic_spawners::block_changed(&format!("{:?}", level.dimension), (pos.x, pos.y, pos.z));
    }
}
""",
}


def binding_lines(bindings):
    return "".join(f'  {{ name = "{n}", ty = "{t}", value = "{v}" }},\n' for n, t, v in bindings)


def main():
    BUNDLE.mkdir(parents=True, exist_ok=True)
    out = [
        "[mod]",
        'id = "apothic_spawners"',
        f'version = "{VERSION}"',
        "",
        "# The Horizon commit this bundle was built and tested on.",
        "[compatibility]",
        f'horizon_revision = "{HORIZON}"',
        "",
        "[[module]]",
        'id = "apothic_spawners"',
        'file = "crates/horizon-client/src/game.rs"',
        'name = "apothic_spawners"',
        'source = "apothic.rs"',
        "",
        "[[module]]",
        'id = "apothic_spawners_movable"',
        'file = "crates/horizon-client/src/mobs/ai.rs"',
        'name = "apothic_spawners_movable"',
        'source = "movable.rs"',
        "",
    ]
    for hook_id, hook, bindings, body in HOOKS:
        helper = f"h_{hook_id.replace('-', '_')}.rs"
        args = ", ".join(f"{n}: {t}" for n, t, _ in bindings)
        (BUNDLE / helper).write_text(f"fn apothic_spawners_{hook_id.replace('-', '_')}({args}) {{\n    {M}::{body};\n}}\n", encoding="utf8")
        out += ["[[injection]]", f'id = "as-{hook_id}"', 'kind = "HEAD"', f'hook = "{hook}"', "hook_version = 1", f'helper = "{helper}"', "bindings = [", binding_lines(bindings).rstrip("\n"), "]", ""]
    for name, text in SPECIAL_HELPERS.items():
        (BUNDLE / name).write_text(text, encoding="utf8")
    out += ["[[injection]]", 'id = "as-tick"', 'kind = "HEAD"', "cancellable = true", 'target = "horizon_client::game::spawner::Game<\'_>::server_tick_spawner_of"', 'helper = "h_tick.rs"', "bindings = [",
            binding_lines([GAME_MUT, ("key", "(i32, i32, i32)", "key"), ("spawner", "&mut horizon_client::mobs::spawner::Spawner", "&mut *spawner"), ("cart", "Option<u32>", "cart")]).rstrip("\n"), "]", ""]
    out += ["[[injection]]", 'id = "as-dumb-mobs"', 'kind = "HEAD"', 'target = "horizon_client::mobs::ai::pre_tick"', 'helper = "h_dumb_mobs.rs"', "bindings = [",
            binding_lines([("m", "&mut crate::mobs::Mob", "&mut *m"), ("w", "&crate::mobs::MobWorld<'_>", "w"), ("events", "&mut Vec<crate::mobs::MobEvent>", "&mut *events")]).rstrip("\n"), "]", ""]
    out += ["[[injection]]", 'id = "as-changed"', 'kind = "HEAD"', 'hook = "block_changed"', "hook_version = 1", 'helper = "h_changed.rs"', "bindings = [",
            binding_lines([("level", "&crate::game::GameLevel<'_>", "&*self"), ("pos", "horizon_core::position::BlockPos", "pos"), ("old", "horizon_world::registry::BlockStateId", "old"), ("new", "horizon_world::registry::BlockStateId", "new")]).rstrip("\n"), "]", ""]
    out += ["[[injection]]", 'id = "as-list"', 'kind = "HEAD"', 'hook = "mods"', "hook_version = 1", 'helper = "list.rs"', "bindings = [", binding_lines([("mods", "&mut Vec<crate::game::native_hooks::NativeMod>", "&mut *mods")]).rstrip("\n"), "]", ""]
    out += ["[[injection]]", 'id = "as-embedded-content"', 'kind = "HEAD"', "cancellable = true", 'target = "horizon_client::game::mods::content_root"', 'helper = "embedded_content.rs"', "bindings = [", binding_lines([("root", "&std::path::Path", "root"), ("enabled", "&std::collections::BTreeSet<String>", "enabled"), ("dev", "&[std::path::PathBuf]", "dev")]).rstrip("\n"), "]", ""]
    (BUNDLE / "horizon-mixin.toml").write_text("\n".join(out), encoding="utf8")
    subprocess.run([sys.executable, str(ROOT.parent / "jei-kit" / "apply.py"), "apothic_spawners", str(BUNDLE)], check=True)
    # The registry checks with -D warnings: helpers' unused parameters, and those of the `else` a missing mod leaves.
    for fixer in ("unused_helper_params.py", "use_helper_params.py"):
        subprocess.run([sys.executable, str(ROOT.parent / "tools" / fixer), str(BUNDLE)], check=True)
    (BUNDLE / "data").mkdir(exist_ok=True)
    subprocess.run([sys.executable, str(ROOT / "tools" / "make_content.py"), str(JAR), str(BUNDLE / "data" / "apothic_spawners-content.hmod")], check=True)
    target = BUILD / "mixins" / "apothic_spawners"
    target.mkdir(parents=True, exist_ok=True)
    for old in target.rglob("*"):
        if old.is_file():
            old.unlink()
    for f in BUNDLE.rglob("*"):
        if f.is_file():
            to = target / f.relative_to(BUNDLE)
            to.parent.mkdir(parents=True, exist_ok=True)
            to.write_bytes(f.read_bytes())
    (BUILD / "horizon-native.json").write_text(json.dumps({"game": "26.3", "horizon": HORIZON, "id": "apothic_spawners", "mixins": MIXINS, "schema": 1, "version": VERSION}, indent=2), encoding="utf8")
    print("bundle written:", len(HOOKS) + 4, "injections")


if __name__ == "__main__":
    main()
