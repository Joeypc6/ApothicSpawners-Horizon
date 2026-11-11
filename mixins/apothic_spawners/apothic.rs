// Apothic Spawners 2.0.1 for Horizon: the spawner module of Apotheosis (MIT, Shadows_of_Fire), rewritten for
// Horizon's hooks. Spawner modifiers change a spawner's vanilla fields and its own ten stats, kept here per spawner;
// its tick runs their rules; Silk Touch picks a spawner up with them; Capturing drops spawn eggs; Echoing repeats loot.
// Everything that changes the world is decided on the server, from what the server holds.
use super::Game;
use horizon_client::containers::BlockEntity;
use horizon_client::items::ItemStack;
use horizon_client::mobs::spawner::{Spawner, SpawnerExt};
use horizon_world::nbt::{Compound, JavaString, Tag};
use horizon_core::random::RandomSource;
use std::cell::RefCell;
use std::collections::BTreeMap;

const NS: &str = "apothic_spawners";
const SPAWNER: &str = "minecraft:spawner";
const DATA: &str = "apothic_spawners:spawner";

type Pos = (i32, i32, i32);

fn java(s: &str) -> JavaString {
    JavaString::from(s)
}

// ---- The config (ASConfig), from config/apotheosis/apothic_spawners.cfg, written with its defaults when missing.

#[derive(Clone, Copy)]
struct Config {
    silk_level: i32,
    silk_damage: i32,
    drop_empty: bool,
    despawn_delay: i32,
}

fn config() -> Config {
    static CFG: std::sync::OnceLock<Config> = std::sync::OnceLock::new();
    *CFG.get_or_init(|| {
        let dir = std::env::current_dir().unwrap_or_default().join("config").join("apotheosis");
        let file = dir.join("apothic_spawners.cfg");
        let text = std::fs::read_to_string(&file).unwrap_or_default();
        let get = |key: &str, default: i64, lo: i64, hi: i64| -> i64 {
            text.lines()
                .find_map(|l| l.trim().strip_prefix(&format!("I:\"{key}\"=")).or_else(|| l.trim().strip_prefix(&format!("B:\"{key}\"="))).map(str::trim).map(str::to_owned))
                .and_then(|v| if v == "true" { Some(1) } else if v == "false" { Some(0) } else { v.parse().ok() })
                .unwrap_or(default)
                .clamp(lo, hi)
        };
        let cfg = Config {
            silk_level: get("Spawner Silk Level", 1, -1, 127) as i32,
            silk_damage: get("Spawner Silk Damage", 100, 0, 100000) as i32,
            drop_empty: get("Spawners Drop Empty", 0, 0, 1) == 1,
            despawn_delay: get("Entity Despawn Delay", 600, 0, 24000) as i32,
        };
        if text.is_empty() {
            let body = format!(
                "# Apotheosis Spawner Module Configuration\n\ngeneral {{\n    # The level of silk touch needed to harvest a spawner.  Set to -1 to disable, 0 to always drop.\n    I:\"Spawner Silk Level\"={}\n\n    # The durability damage dealt to an item that silk touches a spawner.\n    I:\"Spawner Silk Damage\"={}\n\n    # If spawners should clear their contained entity when broken.\n    B:\"Spawners Drop Empty\"={}\n\n    # The time, in ticks, that spawner-spawned mobs will be prevented from despawning for after they have spawned.\n    I:\"Entity Despawn Delay\"={}\n}}\n",
                cfg.silk_level, cfg.silk_damage, cfg.drop_empty, cfg.despawn_delay
            );
            let _ = std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(&file, body));
        }
        cfg
    })
}

// ---- The ten stats of Apothic's own (SpawnerStats), per spawner.

#[derive(Clone, Debug, PartialEq)]
pub struct Stats {
    initial_health: f32,
    ignore_players: bool,
    ignore_conditions: bool,
    redstone_control: bool,
    ignore_light: bool,
    no_ai: bool,
    silent: bool,
    youthful: bool,
    burning: bool,
    echoing: i32,
    /// The activation range the player set: kept here while ignore_players stands it at -1 (vanilla's "any").
    range: i32,
    modified: bool,
    unstable: i32,
}

impl Default for Stats {
    fn default() -> Self {
        Stats { initial_health: 1.0, ignore_players: false, ignore_conditions: false, redstone_control: false, ignore_light: false, no_ai: false, silent: false, youthful: false, burning: false, echoing: 0, range: 16, modified: false, unstable: -1 }
    }
}

impl Stats {
    fn is_plain(&self) -> bool {
        Stats { range: self.range, modified: self.modified, ..Stats::default() } == *self
    }
    fn to_tag(&self) -> Compound {
        let mut c = Compound::new();
        let mut put = |k: &str, t: Tag| {
            c.insert(java(k), t);
        };
        put("apothic_spawners:initial_health", Tag::Float(self.initial_health.to_bits()));
        for (k, v) in [("ignore_players", self.ignore_players), ("ignore_conditions", self.ignore_conditions), ("redstone_control", self.redstone_control), ("ignore_light", self.ignore_light), ("no_ai", self.no_ai), ("silent", self.silent), ("youthful", self.youthful), ("burning", self.burning)] {
            put(&format!("apothic_spawners:{k}"), Tag::Byte(i8::from(v)));
        }
        put("apothic_spawners:echoing", Tag::Int(self.echoing));
        put("hz_range", Tag::Int(self.range));
        put("modified", Tag::Byte(i8::from(self.modified)));
        put("unstable_ticks", Tag::Int(self.unstable));
        c
    }
    fn from_tag(c: &Compound) -> Stats {
        let b = |k: &str| matches!(c.get(&java(&format!("apothic_spawners:{k}"))), Some(Tag::Byte(1)));
        let i = |k: &str, d: i32| match c.get(&java(k)) {
            Some(Tag::Int(v)) => *v,
            _ => d,
        };
        Stats {
            initial_health: match c.get(&java("apothic_spawners:initial_health")) {
                Some(Tag::Float(v)) => f32::from_bits(*v).clamp(0.0, 1.0),
                _ => 1.0,
            },
            ignore_players: b("ignore_players"),
            ignore_conditions: b("ignore_conditions"),
            redstone_control: b("redstone_control"),
            ignore_light: b("ignore_light"),
            no_ai: b("no_ai"),
            silent: b("silent"),
            youthful: b("youthful"),
            burning: b("burning"),
            echoing: i("apothic_spawners:echoing", 0).max(0),
            range: i("hz_range", 16),
            modified: matches!(c.get(&java("modified")), Some(Tag::Byte(1))),
            unstable: i("unstable_ticks", -1),
        }
    }
}

#[derive(Default)]
struct State {
    /// Each spawner's stats by dimension and position, as saved in the world's data/apothic_spawners.json.
    spawners: Option<BTreeMap<(String, Pos), Stats>>,
    /// The spawner whose tick is under way: the mobs it adds take its stats (`mob_spawned`).
    ticking: Option<(String, Pos, Stats, u64, i32)>,
    /// Mobs already given a spawner's stats.
    done_mobs: std::collections::HashSet<u32>,
    /// A spawner a player is about to place from an item, with what it held (`use_item_on`, then `place_block`).
    placing: Vec<(u128, Compound)>,
    /// Spawned mobs with Echoing, by entity id.
    echoing: BTreeMap<u32, i32>,
    /// Places a spawner appeared or went at (`block_changed`), by the level's dimension as it printed.
    cleared: Vec<(String, Pos)>,
    /// A spawner broken: its fields and stats, kept for its drop (`destroy_block`, then `loot_drops`).
    broken: Option<((String, Pos), Compound, Stats)>,
    /// Silk Touch's extra wear owed by a player's held tool (ApothSpawnerBlock.playerDestroy), paid on their next
    /// inventory tick, where the server may change what they carry.
    wear: Vec<(u128, u32)>,
    /// Spawner mobs kept from despawning (ASEvents.onDespawn, Entity Despawn Delay): dimension, mob id and the game
    /// time they may go from. They were made persistent when they came, and are let go then.
    despawn_hold: Vec<(String, u32, u64)>,
    /// Commands to run at the next level tick (granting advancements), outside the player's action.
    commands: Vec<String>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

fn save_path(game: &Game<'_>) -> std::path::PathBuf {
    game.world_root.join("data").join("apothic_spawners.json")
}

fn with_spawners<R>(game: &Game<'_>, f: impl FnOnce(&mut BTreeMap<(String, Pos), Stats>) -> R) -> R {
    STATE.with_borrow_mut(|s| {
        let map = s.spawners.get_or_insert_with(|| load(game));
        f(map)
    })
}

fn load(game: &Game<'_>) -> BTreeMap<(String, Pos), Stats> {
    let mut out = BTreeMap::new();
    let Ok(bytes) = std::fs::read(save_path(game)) else { return out };
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(&bytes) else { return out };
    for e in v["spawners"].as_array().into_iter().flatten() {
        let (Some(dim), Some(p)) = (e["dim"].as_str(), e["pos"].as_array()) else { continue };
        let p: Vec<i32> = p.iter().filter_map(|x| x.as_i64()).map(|x| x as i32).collect();
        let [x, y, z] = p[..] else { continue };
        let g = |k: &str| e[k].as_bool().unwrap_or(false);
        out.insert((dim.to_owned(), (x, y, z)), Stats {
            initial_health: e["initial_health"].as_f64().unwrap_or(1.0).clamp(0.0, 1.0) as f32,
            ignore_players: g("ignore_players"),
            ignore_conditions: g("ignore_conditions"),
            redstone_control: g("redstone_control"),
            ignore_light: g("ignore_light"),
            no_ai: g("no_ai"),
            silent: g("silent"),
            youthful: g("youthful"),
            burning: g("burning"),
            echoing: e["echoing"].as_i64().unwrap_or(0).clamp(0, 1 << 20) as i32,
            range: e["range"].as_i64().unwrap_or(16) as i32,
            modified: g("modified"),
            unstable: e["unstable"].as_i64().unwrap_or(-1) as i32,
        });
    }
    out
}

fn save(game: &Game<'_>) {
    if game.world_root.as_os_str().is_empty() {
        return;
    }
    let list: Vec<serde_json::Value> = with_spawners(game, |m| {
        m.iter()
            .map(|((dim, p), s)| {
                serde_json::json!({
                    "dim": dim, "pos": [p.0, p.1, p.2], "initial_health": s.initial_health, "ignore_players": s.ignore_players,
                    "ignore_conditions": s.ignore_conditions, "redstone_control": s.redstone_control, "ignore_light": s.ignore_light,
                    "no_ai": s.no_ai, "silent": s.silent, "youthful": s.youthful, "burning": s.burning, "echoing": s.echoing,
                    "range": s.range, "modified": s.modified, "unstable": s.unstable
                })
            })
            .collect()
    });
    let path = save_path(game);
    let _ = std::fs::create_dir_all(path.parent().unwrap_or(&path)).and_then(|()| std::fs::write(&path, serde_json::to_vec(&serde_json::json!({ "spawners": list })).unwrap_or_default()));
}

fn stats_at(game: &Game<'_>, dim: &str, pos: Pos) -> Option<Stats> {
    let cleared = STATE.with_borrow_mut(|s| {
        let before = s.cleared.len();
        s.cleared.retain(|(d, k)| !(*k == pos && same_dimension(d, dim)));
        before != s.cleared.len()
    });
    if cleared {
        set_stats(game, dim, pos, None);
        return None;
    }
    with_spawners(game, |m| m.get(&(dim.to_owned(), pos)).cloned())
}

fn set_stats(game: &Game<'_>, dim: &str, pos: Pos, stats: Option<Stats>) {
    with_spawners(game, |m| match stats {
        Some(s) => {
            m.insert((dim.to_owned(), pos), s);
        }
        None => {
            m.remove(&(dim.to_owned(), pos));
        }
    });
    save(game);
}

fn spawner_mut<'a>(game: &'a mut Game<'_>, pos: Pos) -> Option<&'a mut Spawner> {
    match game.world.block_entities.get_mut(&pos) {
        Some(BlockEntity::Spawner(s)) => Some(s),
        _ => None,
    }
}

// ---- Spawner modifiers (SpawnerModifier, StatModifier), from the recipe files, checked on the server.

#[derive(Clone, Debug)]
struct Change {
    stat: String,
    value: serde_json::Value,
    min: Option<f64>,
    max: Option<f64>,
    set: bool,
}

/// The vanilla stats' bounds in Apothic: VanillaStat clamps to its own min/max and never past what a short holds.
fn vanilla_get(s: &Spawner, st: &Stats, stat: &str) -> Option<i32> {
    Some(match stat {
        "min_delay" => s.min_spawn_delay,
        "max_delay" => s.max_spawn_delay,
        "spawn_count" => s.spawn_count,
        "max_nearby_entities" => s.max_nearby_entities,
        "req_player_range" => st.range,
        "spawn_range" => s.spawn_range,
        _ => return None,
    })
}

fn vanilla_set(s: &mut Spawner, st: &mut Stats, stat: &str, v: i32) {
    let v = v.clamp(0, i32::from(i16::MAX));
    match stat {
        "min_delay" => s.min_spawn_delay = v,
        "max_delay" => s.max_spawn_delay = v,
        "spawn_count" => s.spawn_count = v,
        "max_nearby_entities" => s.max_nearby_entities = v,
        "req_player_range" => st.range = v,
        "spawn_range" => s.spawn_range = v,
        _ => {}
    }
}

/// Applies one change; true when the stat changed (StatModifier.apply).
fn apply_change(s: &mut Spawner, st: &mut Stats, c: &Change) -> bool {
    let num = c.value.as_f64();
    if let Some(have) = vanilla_get(s, st, &c.stat) {
        let Some(v) = num else { return false };
        let want = if c.set { v as i64 } else { i64::from(have) + v as i64 };
        let lo = c.min.map_or(i64::MIN, |m| m as i64);
        let hi = c.max.map_or(i64::MAX, |m| m as i64);
        // An add stops at its bound; a value already past the bound in its own direction is left as it is.
        let want = if c.set { want } else if v >= 0.0 { want.min(hi.max(i64::from(have))) } else { want.max(lo.min(i64::from(have))) };
        let want = want.clamp(0, i64::from(i16::MAX)) as i32;
        if want == have {
            return false;
        }
        vanilla_set(s, st, &c.stat, want);
        return true;
    }
    match c.stat.as_str() {
        "initial_health" => {
            let Some(v) = num else { return false };
            let have = st.initial_health;
            let want = if c.set { v as f32 } else { have + v as f32 };
            let want = want.clamp(c.min.map_or(0.0, |m| m as f32), c.max.map_or(1.0, |m| m as f32)).clamp(0.0, 1.0);
            if (want - have).abs() < 1e-6 {
                return false;
            }
            st.initial_health = want;
            true
        }
        "echoing" => {
            let Some(v) = num else { return false };
            let have = st.echoing;
            let want = if c.set { v as i64 } else { i64::from(have) + v as i64 };
            let want = want.clamp(c.min.map_or(0, |m| m as i64), c.max.map_or(i64::from(i32::MAX), |m| m as i64)).max(0) as i32;
            if want == have {
                return false;
            }
            st.echoing = want;
            true
        }
        b => {
            let Some(v) = c.value.as_bool() else { return false };
            let slot = match b {
                "ignore_players" => &mut st.ignore_players,
                "ignore_conditions" => &mut st.ignore_conditions,
                "redstone_control" => &mut st.redstone_control,
                "ignore_light" => &mut st.ignore_light,
                "no_ai" => &mut st.no_ai,
                "silent" => &mut st.silent,
                "youthful" => &mut st.youthful,
                "burning" => &mut st.burning,
                _ => return false,
            };
            // BooleanStat: add flips it, set sets it.
            let want = if c.set { v } else { !*slot };
            if want == *slot {
                return false;
            }
            *slot = want;
            true
        }
    }
}

struct Modifier {
    main: serde_json::Value,
    off: Option<serde_json::Value>,
    consumes_off: bool,
    changes: Vec<Change>,
}

fn modifiers(game: &Game<'_>) -> Vec<Modifier> {
    game.native_recipes_of_type("apothic_spawners:spawner_modifier")
        .iter()
        .filter_map(|r| {
            let j = &r.json;
            let main = j.get("mainhand")?.clone();
            let changes = j["stat_changes"]
                .as_array()?
                .iter()
                .filter_map(|c| {
                    Some(Change {
                        stat: c["type"].as_str()?.trim_start_matches("apothic_spawners:").to_owned(),
                        value: c["value"].clone(),
                        min: c["min"].as_f64(),
                        max: c["max"].as_f64(),
                        set: c["mode"].as_str() == Some("set"),
                    })
                })
                .collect();
            Some(Modifier { main, off: j.get("offhand").cloned().filter(|o| !o.is_null()), consumes_off: j["consumes_offhand"].as_bool().unwrap_or(true), changes })
        })
        .collect()
}

fn ingredient_test(game: &Game<'_>, json: &serde_json::Value, stack: Option<&ItemStack>) -> bool {
    let Some(stack) = stack.filter(|s| s.count > 0) else { return false };
    // 26.3's ingredient forms: "minecraft:x", "#minecraft:tag", a list, or the older {"item"} / {"tag"}. The form as
    // written goes to the recipe reader first (it takes 26.3's "#tag" string, not always the old {"tag"} object).
    if game.native_recipe_ingredient(json).is_some_and(|i| i.test(stack)) {
        return true;
    }
    let json = match json {
        serde_json::Value::String(s) => match s.strip_prefix('#') {
            Some(tag) => serde_json::json!({ "tag": tag }),
            None => serde_json::json!({ "item": s }),
        },
        other => other.clone(),
    };
    game.native_recipe_ingredient(&json).is_some_and(|i| i.test(stack)) || json["item"].as_str() == Some(&*stack.item)
}

/// ModifierTrigger.trigger: after a modifier, every advancement whose conditions the spawner now meets is granted to
/// the player (the conditions as the content mod kept them; MinMaxBounds.Ints for numbers, a boolean for a flag).
fn modifier_advancements(game: &mut Game<'_>, player: &str, spawner: &Spawner, stats: &Stats) {
    thread_local! {
        static TABLE: RefCell<Option<Vec<(String, serde_json::Value)>>> = const { RefCell::new(None) };
    }
    let table = TABLE.with_borrow_mut(|t| {
        t.get_or_insert_with(|| {
            game.native_data("data/apothic_spawners/hz/modifier_advancements.json")
                .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
                .and_then(|v| v.as_object().cloned())
                .map(|o| o.into_iter().map(|(k, v)| (k, v["conditions"].clone())).collect())
                .unwrap_or_default()
        })
        .clone()
    });
    let int_ok = |c: &serde_json::Value, v: i64| match c {
        serde_json::Value::Null => true,
        serde_json::Value::Number(n) => n.as_i64() == Some(v),
        o => o["min"].as_i64().is_none_or(|m| v >= m) && o["max"].as_i64().is_none_or(|m| v <= m),
    };
    let flag_ok = |c: &serde_json::Value, v: bool| c.as_bool().is_none_or(|want| want == v);
    for (id, c) in table {
        let ok = int_ok(&c["min_delay"], i64::from(spawner.min_spawn_delay))
            && int_ok(&c["max_delay"], i64::from(spawner.max_spawn_delay))
            && int_ok(&c["spawn_count"], i64::from(spawner.spawn_count))
            && int_ok(&c["max_nearby_entities"], i64::from(spawner.max_nearby_entities))
            && int_ok(&c["req_player_range"], i64::from(spawner.required_player_range))
            && int_ok(&c["spawn_range"], i64::from(spawner.spawn_range))
            && flag_ok(&c["ignore_players"], stats.ignore_players)
            && flag_ok(&c["ignore_conditions"], stats.ignore_conditions)
            && flag_ok(&c["redstone_control"], stats.redstone_control)
            && flag_ok(&c["ignore_light"], stats.ignore_light)
            && flag_ok(&c["no_ai"], stats.no_ai)
            && flag_ok(&c["silent"], stats.silent)
            && flag_ok(&c["youthful"], stats.youthful);
        // Commands don't run while a player's action is handled (`native_run_command` refuses): the level tick runs it.
        if ok {
            let line = format!("advancement grant {player} only {id}");
            STATE.with_borrow_mut(|s| s.commands.push(line));
        }
    }
}

/// `use_item_on`, server side: a modifier on a spawner, or a spawner about to be placed from its item.
pub(in crate::game) fn use_item_on(game: &mut Game<'_>, stack: &ItemStack, pos: [i32; 3], claimed: &mut bool) {
    if !game.native_on(NS) {
        return;
    }
    let Some(player) = game.native_acting_player() else { return };
    if &*stack.item == SPAWNER {
        if let Some(Tag::Compound(c)) = custom(stack).and_then(|c| c.get(&java(DATA))).cloned() {
            STATE.with_borrow_mut(|s| {
                s.placing.retain(|(u, _)| *u != player.uuid);
                s.placing.push((player.uuid, c));
            });
        }
        return;
    }
    if game.native_block(pos) != SPAWNER {
        return;
    }
    let key = (pos[0], pos[1], pos[2]);
    let dim = game.native_dimension().to_owned();
    // A blacklisted spawn egg is refused (handleUseItem).
    if stack.item.ends_with("_spawn_egg") && blacklisted(game, &stack.item.replace("_spawn_egg", "")) {
        *claimed = true;
        return;
    }
    let off = game.native_carried_stack(-106).cloned();
    // SpawnerModifier.findMatch: one that also asks for the off hand's item wins over one that doesn't (the inverse
    // modifiers share their main hand item with the plain ones and add quartz in the off hand).
    let mut all = modifiers(game);
    all.sort_by_key(|m| m.off.is_none());
    let Some(m) = all.into_iter().find(|m| ingredient_test(game, &m.main, Some(stack)) && m.off.as_ref().is_none_or(|o| ingredient_test(game, o, off.as_ref()))) else { return };
    let mut stats = stats_at(game, &dim, key).unwrap_or_else(|| Stats { range: spawner_mut(game, key).map_or(16, |s| if s.required_player_range < 0 { 16 } else { s.required_player_range }), ..Stats::default() });
    let Some(spawner) = spawner_mut(game, key) else { return };
    if stats.unstable >= 0 {
        return;
    }
    let mut changed = false;
    let mut range_touched = false;
    for c in &m.changes {
        let did = apply_change(spawner, &mut stats, c);
        changed |= did;
        range_touched |= did && matches!(c.stat.as_str(), "req_player_range" | "ignore_players");
    }
    if !changed {
        return;
    }
    *claimed = true;
    stats.modified = true;
    // ignore_players stands the vanilla range at -1, vanilla's "any distance"; the set range comes back without it.
    if range_touched {
        spawner.required_player_range = if stats.ignore_players { -1 } else { stats.range };
    }
    // What the advancements test, before `game` is used again (the spawner is borrowed from it).
    let after = spawner.clone();
    set_stats(game, &dim, key, Some(stats.clone()));
    modifier_advancements(game, &player.name, &after, &stats);
    if player.game_mode != 1 {
        let selected = game.selected as i32;
        if let Some(s) = game.native_carried_stack(selected) {
            s.count = s.count.saturating_sub(1);
        }
        if m.off.is_some() && m.consumes_off
            && let Some(s) = game.native_carried_stack(-106)
        {
            s.count = s.count.saturating_sub(1);
        }
    }
    game.native_block_entity_sync(pos);
}

fn custom(stack: &ItemStack) -> Option<&Compound> {
    match stack.extra.as_ref()?.other.iter().find(|(k, _)| k == "minecraft:custom_data").map(|(_, t)| t) {
        Some(Tag::Compound(c)) => Some(c),
        _ => None,
    }
}

/// The `apothic_spawners:blacklisted_from_spawners` entity tag, read once.
fn blacklisted(game: &Game<'_>, entity: &str) -> bool {
    thread_local! {
        static BLACKLIST: RefCell<Option<std::collections::HashSet<String>>> = const { RefCell::new(None) };
    }
    BLACKLIST.with_borrow_mut(|list| {
        list.get_or_insert_with(|| {
            let mut out = std::collections::HashSet::new();
            entity_tag(game, "apothic_spawners:blacklisted_from_spawners", &mut out, 0);
            out
        })
        .contains(entity)
    })
}

/// An entity type tag's members, nested tags included. `c:bosses`
/// (a convention tag vanilla doesn't ship) is NeoForge's Tags.EntityTypes.BOSSES when nothing gives it.
fn entity_tag(game: &Game<'_>, tag: &str, out: &mut std::collections::HashSet<String>, depth: u32) {
    if depth > 8 {
        return;
    }
    let (ns, path) = tag.split_once(':').unwrap_or(("minecraft", tag));
    let file = format!("data/{ns}/tags/entity_type/{path}.json");
    // The game's resources, content mods' included.
    let bytes = game.native_data(&file);
    let values = bytes.and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok()).and_then(|v| v["values"].as_array().cloned());
    let Some(values) = values else {
        if tag == "c:bosses" {
            out.extend(["minecraft:ender_dragon".to_owned(), "minecraft:wither".to_owned()]);
        }
        return;
    };
    for v in values {
        let Some(id) = v.as_str().or_else(|| v["id"].as_str()) else { continue };
        match id.strip_prefix('#') {
            Some(inner) => entity_tag(game, inner, out, depth + 1),
            None => {
                out.insert(id.to_owned());
            }
        }
    }
}

/// `block_changed` (any `Level.setBlock` on the server, commands included): a spawner that appears or goes leaves no
/// stats behind at its place, as a fresh `ApothSpawnerTile` has none; they go when next looked up. A picked-up one
/// gets its own back in `placed`, after this.
pub fn block_changed(dimension: &str, key: Pos) {
    STATE.with_borrow_mut(|s| s.cleared.push((dimension.to_owned(), key)));
}

/// The dimension's id as `block_changed` names it (the level's `Dimension` by its variant).
fn same_dimension(debug: &str, id: &str) -> bool {
    let d = debug.to_ascii_lowercase();
    match id {
        "minecraft:overworld" => d.contains("overworld"),
        "minecraft:the_nether" => d.contains("nether"),
        "minecraft:the_end" => d.contains("end"),
        other => d.contains(other.rsplit(':').next().unwrap_or(other)),
    }
}

/// `explosion_detonate` (ApothSpawnerBlock.onBlockExploded): an explosion doesn't break a spawner; it makes it
/// unstable, sixty ticks to its burst, unless it already is.
pub(in crate::game) fn explosion(game: &mut Game<'_>, dimension: &str, _center: [f64; 3], blocks: &mut Vec<[i32; 3]>) {
    if !game.native_on(NS) {
        return;
    }
    let spawners: Vec<[i32; 3]> = blocks.iter().copied().filter(|p| game.native_block(*p) == SPAWNER).collect();
    if spawners.is_empty() {
        return;
    }
    blocks.retain(|p| !spawners.contains(p));
    for p in spawners {
        let key = (p[0], p[1], p[2]);
        let mut stats = stats_at(game, dimension, key).unwrap_or_else(|| Stats { range: spawner_mut(game, key).map_or(16, |s| if s.required_player_range < 0 { 16 } else { s.required_player_range }), ..Stats::default() });
        if stats.unstable < 0 {
            stats.unstable = 60;
            set_stats(game, dimension, key, Some(stats));
            game.native_block_entity_sync(p);
        }
    }
}

/// `place_block`: a spawner placed from a picked-up one takes its fields and stats back.
pub(in crate::game) fn placed(game: &mut Game<'_>, pos: [i32; 3]) {
    if game.native_block(pos) != SPAWNER {
        return;
    }
    let Some(player) = game.native_acting_player() else { return };
    let Some(data) = STATE.with_borrow_mut(|s| s.placing.iter().position(|(u, _)| *u == player.uuid).map(|i| s.placing.remove(i).1)) else { return };
    let key = (pos[0], pos[1], pos[2]);
    let dim = game.native_dimension().to_owned();
    if let Some(Tag::Compound(fields)) = data.get(&java("spawner")) {
        let loaded = Spawner::load(&Tag::Compound(fields.clone()));
        game.world.block_entities.insert(key, BlockEntity::Spawner(Box::new(loaded)));
    }
    if let Some(Tag::Compound(st)) = data.get(&java("stats")) {
        let mut stats = Stats::from_tag(st);
        stats.unstable = -1;
        if let Some(s) = spawner_mut(game, key) {
            s.required_player_range = if stats.ignore_players { -1 } else { stats.range };
        }
        set_stats(game, &dim, key, (!stats.is_plain() || stats.modified).then_some(stats));
    }
    game.native_block_entity_sync(pos);
}

/// `destroy_block`: what a spawner held, kept for its drop; its stats leave the world either way.
pub(in crate::game) fn destroyed(game: &mut Game<'_>, pos: [i32; 3]) {
    if game.native_block(pos) != SPAWNER {
        return;
    }
    let key = (pos[0], pos[1], pos[2]);
    let dim = game.native_dimension().to_owned();
    let stats = stats_at(game, &dim, key).unwrap_or_default();
    let fields = match game.world.block_entities.get(&key) {
        Some(BlockEntity::Spawner(s)) => {
            let mut c = Compound::new();
            for (k, v) in s.save() {
                c.insert(java(k), v);
            }
            c
        }
        _ => Compound::new(),
    };
    STATE.with_borrow_mut(|s| s.broken = Some(((dim.clone(), key), fields, stats)));
    set_stats(game, &dim, key, None);
}

/// `loot_drops`: Silk Touch drops the spawner with its data (ApothSpawnerBlock.getDrops); Capturing drops a spawn
/// egg; Echoing repeats a spawner mob's drops.
pub(in crate::game) fn loot_drops(game: &mut Game<'_>, context: &crate::mod_api::NativeLootDrops<'_>, drops: &mut Vec<ItemStack>) {
    if !game.native_on(NS) {
        return;
    }
    if let Some((pos, _)) = context.block {
        let key = (pos[0], pos[1], pos[2]);
        let Some(((_, at), mut fields, mut stats)) = STATE.with_borrow_mut(|s| s.broken.take()) else { return };
        if at != key {
            return;
        }
        // ApothSpawnerBlock.onBlockExploded: an explosion doesn't break a spawner, it makes it unstable (sixty ticks,
        // then it bursts). Horizon's explosion has already taken it: it is put back as it was, counting down.
        if context.explosion.is_some() {
            drops.clear();
            if stats.unstable < 0 {
                let dim = context.dimension.to_owned();
                game.native_run_command(&format!("execute in {dim} run setblock {} {} {} {SPAWNER}", key.0, key.1, key.2));
                let loaded = Spawner::load(&Tag::Compound(fields));
                game.world.block_entities.insert(key, BlockEntity::Spawner(Box::new(loaded)));
                stats.unstable = 60;
                set_stats(game, &dim, key, Some(stats));
                game.native_block_entity_sync(pos);
            }
            return;
        }
        let silk = context.tool.map_or(0, |t| t.enchantment_level("minecraft:silk_touch"));
        let level = config().silk_level;
        if level < 0 || (silk as i32) < level {
            return;
        }
        stats.modified = true;
        stats.unstable = -1;
        if config().drop_empty {
            fields.remove(&java("SpawnPotentials"));
            fields.remove(&java("SpawnData"));
            fields.insert(java("Delay"), Tag::Short(0));
        }
        let mut item = ItemStack::new(SPAWNER, 1);
        let mut data = Compound::new();
        data.insert(java("spawner"), Tag::Compound(fields));
        data.insert(java("stats"), Tag::Compound(stats.to_tag()));
        let mut c = custom(&item).cloned().unwrap_or_default();
        c.insert(java(DATA), Tag::Compound(data));
        set_custom(&mut item, c);
        drops.clear();
        drops.push(item);
        // playerDestroy: the tool takes Spawner Silk Damage - 1 more (hurtAndBreak, so Unbreaking still counts).
        if let Some(player) = context.player
            && config().silk_damage > 1
        {
            STATE.with_borrow_mut(|s| s.wear.push((player, (config().silk_damage - 1) as u32)));
        }
        return;
    }
    let Some((id, kind)) = context.entity else { return };
    // ASEvents.echoing: the mob's loot table rolled `echoes` more times, with the same killer and weapon (Looting
    // counts). (Its experience x (1 + echoes) waits for a mob experience hook.)
    if let Some(echoes) = STATE.with_borrow_mut(|s| s.echoing.remove(&id)) {
        let weapon = context.player.and_then(|p| game.native_held_stack(p));
        let by_player = context.player.is_some();
        for _ in 0..echoes.min(64) {
            let rolled = game.res.items().loot_table(context.table, by_player, weapon.as_ref(), &mut game.world.random);
            drops.extend(rolled);
        }
    }
    // Capturing on the killer's weapon: level x 0.5% per level (the enchantment's own chance, see capturing.json).
    let Some(player) = context.player else { return };
    let Some(weapon) = game.native_held_stack(player) else { return };
    let level = weapon.enchantment_level("apothic_spawners:capturing");
    if level == 0 || blacklisted(game, kind) {
        return;
    }
    let chance = capturing_chance(game) * level as f32;
    let roll = (game.world.random.next_float()).min(1.0);
    let egg = format!("{kind}_spawn_egg");
    if roll < chance && game.res.items().get(&egg).is_some() {
        drops.push(ItemStack::new(&egg, 1));
    }
}

fn capturing_chance(game: &Game<'_>) -> f32 {
    // The enchantment's effect value per level, from capturing.json ("apothic_spawners:capturing" component), else 0.5%.
    game.native_data("data/apothic_spawners/enchantment/capturing.json")
        .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
        .and_then(|v| v.pointer("/effects/apothic_spawners:capturing").and_then(|x| x.as_f64()))
        .unwrap_or(0.005) as f32
}

fn set_custom(stack: &mut ItemStack, c: Compound) {
    let extra = stack.extra.get_or_insert_with(Default::default);
    extra.other.retain(|(k, _)| k != "minecraft:custom_data");
    extra.other.push(("minecraft:custom_data".into(), Tag::Compound(c)));
}

// ---- The tick (SpawnerLogicExt.serverTick's rules ahead of vanilla's).

/// The spawner's server tick at `key`: Continue lets vanilla's run (with the range already standing at -1 for
/// ignore_players); Break(Ok) skips it (no redstone signal, or an unstable spawner counting down).
pub(in crate::game) fn spawner_tick(game: &mut Game<'_>, key: Pos, spawner: &mut Spawner, cart: Option<u32>) -> core::ops::ControlFlow<horizon_client::Result<()>> {
    // The tick this hook runs again itself (ignore_conditions, below): the original body, untouched.
    if RERUN.with(std::cell::Cell::get) {
        return core::ops::ControlFlow::Continue(());
    }
    let dim = game.native_dimension().to_owned();
    let Some(mut stats) = stats_at(game, &dim, key) else {
        // A plain spawner: its mobs only get the despawn delay (mob_spawned).
        if config().despawn_delay > 0 {
            let (tick, range) = (game.world.game_time as u64, spawner.spawn_range);
            STATE.with_borrow_mut(|s| s.ticking = Some((dim, key, Stats::default(), tick, range)));
        }
        return core::ops::ControlFlow::Continue(());
    };
    let pos = [key.0, key.1, key.2];
    if stats.unstable >= 0 {
        unstable_particles(game, key);
        stats.unstable -= 1;
        if stats.unstable < 0 {
            detonate(game, key, spawner);
            set_stats(game, &dim, key, None);
        } else {
            with_spawners(game, |m| m.insert((dim.clone(), key), stats.clone()));
        }
        return core::ops::ControlFlow::Break(Ok(()));
    }
    if stats.redstone_control && game.native_neighbor_signal(&dim, pos) <= 0 {
        return core::ops::ControlFlow::Break(Ok(()));
    }
    if stats.ignore_players {
        spawner.required_player_range = -1;
    }
    let tick = game.world.game_time as u64;
    let range = spawner.spawn_range;
    let ignore_conditions = stats.ignore_conditions;
    STATE.with_borrow_mut(|s| s.ticking = Some((dim, key, stats, tick, range)));
    if !ignore_conditions {
        return core::ops::ControlFlow::Continue(());
    }
    // IGNORE_CONDITIONS: the spawn loop skips checkSpawnRules, light included. Horizon's spawner does the same for a
    // SpawnData with custom_spawn_rules (it checks only their light ranges), so for this one tick every SpawnData
    // without them gets light 0..15 both ways, and loses it again after.
    use horizon_client::chunk::block_entity::Next;
    use horizon_client::mobs::spawner::CustomSpawnRules;
    const ANY: CustomSpawnRules = CustomSpawnRules { block: (0, 15), sky: (0, 15) };
    let mut lent = Vec::new();
    for (i, (data, _)) in spawner.potentials.iter_mut().enumerate() {
        if data.custom_spawn_rules.is_none() {
            data.custom_spawn_rules = Some(ANY);
            lent.push(Some(i));
        }
    }
    if let Some(Next::Own(data)) = spawner.next.as_mut()
        && data.custom_spawn_rules.is_none()
    {
        data.custom_spawn_rules = Some(ANY);
        lent.push(None);
    }
    RERUN.with(|r| r.set(true));
    let result = game.server_tick_spawner_of(key, spawner, cart);
    RERUN.with(|r| r.set(false));
    for i in lent {
        match i {
            Some(i) => {
                if let Some((data, _)) = spawner.potentials.get_mut(i) {
                    data.custom_spawn_rules = None;
                }
            }
            None => {
                if let Some(Next::Own(data)) = spawner.next.as_mut()
                    && data.custom_spawn_rules == Some(ANY)
                {
                    data.custom_spawn_rules = None;
                }
            }
        }
    }
    core::ops::ControlFlow::Break(result)
}

thread_local! {
    /// The spawner tick is being run again by `spawner_tick` itself.
    static RERUN: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// ApothSpawnerTile's client tick while unstable: eight flames and eight large smokes a tick, scattered 2.5 blocks
/// round it and thrown each its own way (`nextGaussian`). Sent by the server here, to players within 32 blocks.
fn unstable_particles(game: &mut Game<'_>, key: Pos) {
    use horizon_client::particles::{ParticleOptions, ParticleType};
    // A small generator of its own (the level's isn't the mod's to draw from), seeded by the tick and place.
    struct Rand(u64);
    impl Rand {
        fn next(&mut self) -> f64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            (self.0 >> 11) as f64 / (1u64 << 53) as f64
        }
        fn gauss(&mut self) -> f64 {
            let (u, v) = (self.next().max(1e-12), self.next());
            (-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * v).cos()
        }
    }
    let mut r = Rand((game.world.game_time as u64) ^ ((key.0 as u64) << 40) ^ ((key.1 as u64) << 20) ^ (key.2 as u64) ^ 0x9E37_79B9_7F4A_7C15);
    let dim = game.native_dimension().to_owned();
    let Some(level) = game.native_level(&dim) else { return };
    let flame = ParticleOptions::simple(ParticleType::Flame);
    let smoke = ParticleOptions::simple(ParticleType::LargeSmoke);
    for _ in 0..8 {
        let x = f64::from(key.0) + 0.5 + (r.next() - r.next()) * 2.5;
        let y = f64::from(key.1) + r.next();
        let z = f64::from(key.2) + 0.5 + (r.next() - r.next()) * 2.5;
        let d = [r.gauss(), r.next() * 0.01, r.gauss()];
        // A count of 0 sends one particle moving along `d`.
        game.native_level_send_particles(level, &flame, None, [x, y, z], d, 1.0, 0);
        game.native_level_send_particles(level, &smoke, None, [x, y, z], d, 1.0, 0);
    }
}

/// ApothSpawnerTile.detonateUnstable: the block goes, an explosion of 8, twelve of its mob.
fn detonate(game: &mut Game<'_>, key: Pos, spawner: &Spawner) {
    let pos = [key.0, key.1, key.2];
    let kind = spawner.next_data().and_then(|d| match d.entity.get(&java("id")) {
        Some(Tag::String(s)) => s.to_string().ok(),
        _ => None,
    });
    game.native_run_command(&format!("setblock {} {} {} air", pos[0], pos[1], pos[2]));
    // level.explode(null, centre, 8, BLOCK) at once, and the mobs after it, as detonateUnstable does.
    let dim = game.native_dimension().to_owned();
    let centre = [f64::from(pos[0]) + 0.5, f64::from(pos[1]) + 0.5, f64::from(pos[2]) + 0.5];
    game.native_explode(&dim, centre, 8.0, false, super::native_explosions::NativeBlast::Block);
    if let Some(kind) = kind {
        for _ in 0..12 {
            game.native_run_command(&format!("summon {kind} {}.5 {} {}.5", pos[0], pos[1], pos[2]));
        }
    }
    game.native_run_command(&format!("loot spawn {}.5 {}.5 {}.5 loot apothic_spawners:gameplay/unstable_spawner", pos[0], pos[1], pos[2]));
}

/// `mob_spawned`: a mob a spawner with stats just added takes them (no AI, silent, youthful, health, burning, echoing).
pub(in crate::game) fn mob_spawned(game: &mut Game<'_>, dimension: &str, kind: &str, pos: [f64; 3], natural: bool) {
    if natural {
        return;
    }
    let tick = game.world.game_time as u64;
    // (The spawner's block entity is out of the world while it ticks: its range came with the tick.)
    let Some((dim, key, stats, at, spawner)) = STATE.with_borrow(|s| s.ticking.clone()) else { return };
    let delay = config().despawn_delay;
    if dim != dimension || at != tick || (stats.is_plain() && delay <= 0) {
        return;
    }
    let near = |p: [f64; 3]| (p[0] - f64::from(key.0) - 0.5).abs() <= f64::from(spawner) + 1.0 && (p[2] - f64::from(key.2) - 0.5).abs() <= f64::from(spawner) + 1.0 && (p[1] - f64::from(key.1)).abs() <= 2.0;
    if !near(pos) {
        return;
    }
    let done = STATE.with_borrow(|s| s.done_mobs.clone());
    let same = |a: &str| a.trim_start_matches("minecraft:") == kind.trim_start_matches("minecraft:");
    // The newest mob of its kind there not yet given them: the one just added.
    let Some(m) = game.mobs.iter_mut().rev().find(|m| same(&m.kind.id) && !done.contains(&m.id) && near([m.position.x, m.position.y, m.position.z])) else { return };
    STATE.with_borrow_mut(|s| {
        if s.done_mobs.len() > 4096 {
            s.done_mobs.clear();
        }
        s.done_mobs.insert(m.id);
    });
    // ASEvents.onDespawn: a spawner's mob stays its first Entity Despawn Delay ticks. (Peaceful still takes the types
    // it doesn't allow: Horizon's checkDespawn discards those before it looks at persistence, as onDespawn lets it.)
    if delay > 0 && !m.persistent {
        m.persistent = true;
        let (id, until) = (m.id, tick + delay as u64);
        STATE.with_borrow_mut(|s| s.despawn_hold.push((dimension.to_owned(), id, until)));
    }
    if stats.is_plain() {
        return;
    }
    if stats.no_ai {
        m.no_ai = true;
    }
    if stats.silent {
        m.silent = true;
    }
    if stats.youthful {
        horizon_client::mobs::finalize::set_baby(m, true);
    }
    if stats.initial_health != 1.0 {
        m.health = (m.health * stats.initial_health).max(0.5);
    }
    if stats.burning && !m.kind.flags.fire_immune {
        m.fire_ticks = i32::MAX;
    }
    let id = m.id;
    // ApothSpawnerTile: a no-AI spawner's mob also gets `apotheosis:movable` in its persistent data (tickDumbMobs).
    if stats.no_ai {
        mark_movable(game, id);
    }
    if stats.echoing > 0 {
        STATE.with_borrow_mut(|s| {
            s.echoing.insert(id, stats.echoing);
        });
    }
}

// ---- Movable no-AI mobs (ASEvents.tickDumbMobs): `apotheosis:movable` in a mob's NeoForge persistent data, saved
// with it as NeoForgeData (Horizon keeps the keys it doesn't model in the mob's loaded compound). The mob step's
// hook (movable.rs, in Horizon's mobs::ai) runs `travel` for a no-AI mob that has it; the mark there follows the data.

const MOVABLE: &str = "apotheosis:movable";

/// `CompoundTag.getBooleanOr("apotheosis:movable", false)` on the mob's persistent data: a number tag's byte value
/// not 0 (`NumericTag.asBoolean`); anything else is the default.
fn raw_movable(game: &Game<'_>, id: u32) -> bool {
    let Some(tag) = game.persist.raw.get(&id).and_then(|t| t.get("NeoForgeData")).and_then(|d| d.get(MOVABLE)) else { return false };
    match tag {
        Tag::Byte(b) => *b != 0,
        Tag::Short(s) => *s as i8 != 0,
        Tag::Int(i) => *i as i8 != 0,
        Tag::Long(l) => *l as i8 != 0,
        Tag::Float(f) => f32::from_bits(*f).floor() as i32 as i8 != 0,
        Tag::Double(d) => f64::from_bits(*d).floor() as i32 as i8 != 0,
        _ => false,
    }
}

/// `getPersistentData().putBoolean("apotheosis:movable", true)`.
fn mark_movable(game: &mut Game<'_>, id: u32) {
    let raw = game.persist.raw.entry(id).or_insert_with(|| Tag::Compound(Compound::new()));
    if let Tag::Compound(c) = raw {
        let data = c.entry(java("NeoForgeData")).or_insert_with(|| Tag::Compound(Compound::new()));
        if !matches!(data, Tag::Compound(_)) {
            *data = Tag::Compound(Compound::new());
        }
        if let Tag::Compound(d) = data {
            d.insert(java(MOVABLE), Tag::Byte(1));
        }
    }
    horizon_client::mobs::Mob::apothic_spawners_set_movable(id, true);
}

/// `entity_join_level`: an entity coming in (from its chunk, a portal, a summon, a spawn) is movable as its data says.
pub(in crate::game) fn entity_joined(game: &Game<'_>, entity: u32) {
    horizon_client::mobs::Mob::apothic_spawners_set_movable(entity, raw_movable(game, entity));
}

/// `entity_leave_level`: its id is let go.
pub(in crate::game) fn entity_left(entity: u32) {
    horizon_client::mobs::Mob::apothic_spawners_set_movable(entity, false);
}

/// `server_stopping`: the next world's entity ids start again.
pub(in crate::game) fn server_stopping() {
    horizon_client::mobs::Mob::apothic_spawners_clear_movable();
}

// ---- What a player sees: the stats in the spawner item's tooltip (ApothSpawnerBlock's), and the guide.

/// `level_tick_post`: spawner mobs whose despawn delay is over may despawn again.
pub(in crate::game) fn level_tick(game: &mut Game<'_>, dimension: &str) {
    for line in STATE.with_borrow_mut(|s| std::mem::take(&mut s.commands)) {
        game.native_run_command(&line);
    }
    if STATE.with_borrow(|s| s.despawn_hold.is_empty()) {
        return;
    }
    let now = game.world.game_time as u64;
    let due: Vec<u32> = STATE.with_borrow_mut(|s| {
        let mut due = Vec::new();
        s.despawn_hold.retain(|(dim, id, until)| {
            let go = dim == dimension && *until <= now;
            if go {
                due.push(*id);
            }
            !go
        });
        due
    });
    for m in game.mobs.iter_mut().filter(|m| due.contains(&m.id)) {
        m.persistent = false;
    }
}

/// `inventory_tick`: the held tool pays the Silk Touch wear its player owes (`State::wear`).
pub(in crate::game) fn inventory_tick(game: &mut Game<'_>, slot: i32, selected: bool) {
    if !selected || STATE.with_borrow(|s| s.wear.is_empty()) {
        return;
    }
    let Some(player) = game.native_acting_player().map(|p| p.uuid) else { return };
    let Some(owed) = STATE.with_borrow_mut(|s| {
        let i = s.wear.iter().position(|(p, _)| *p == player)?;
        Some(s.wear.remove(i).1)
    }) else {
        return;
    };
    let Some(stack) = game.native_carried_stack(slot).cloned() else { return };
    let max = game.res.items().get(&stack.item).map_or(0, |i| i.max_damage);
    if max == 0 {
        return;
    }
    let loss = game.wear(&stack, owed);
    if let Some(held) = game.native_carried_stack(slot) {
        held.damage += loss;
        // hurtAndBreak: worn out, the tool breaks.
        if held.damage >= max {
            held.count = 0;
        }
    }
}

pub(in crate::game) fn tooltip(game: &Game<'_>, stack: &ItemStack, lines: &mut Vec<(horizon_client::text::Text, u32)>) {
    if !game.native_on(NS) {
        return;
    }
    // ASEvents.tooltips: a spawn egg of a type spawners refuse says so.
    if let Some(kind) = stack.item.strip_suffix("_spawn_egg") {
        if blacklisted(game, kind) {
            let text = game.translate("misc.apothic_spawners.banned", "Cannot be used on a Spawner");
            lines.push((horizon_client::text::Text::legacy(&format!("§7{text}")), u32::MAX));
        }
        return;
    }
    if &*stack.item != SPAWNER {
        return;
    }
    let Some(Tag::Compound(data)) = custom(stack).and_then(|c| c.get(&java(DATA))) else { return };
    let Some(Tag::Compound(st)) = data.get(&java("stats")) else { return };
    let stats = Stats::from_tag(st);
    let fields = match data.get(&java("spawner")) {
        Some(Tag::Compound(f)) => Some(Spawner::load(&Tag::Compound(f.clone()))),
        _ => None,
    };
    for s in stat_lines(game, fields.as_ref(), &stats) {
        lines.push((horizon_client::text::Text::legacy(&s), u32::MAX));
    }
}

/// SpawnerStats.generateTooltip: the item's tooltip and Jade's line both.
fn stat_lines(game: &Game<'_>, fields: Option<&Spawner>, stats: &Stats) -> Vec<String> {
    let mut out = Vec::new();
    // SpawnerStat.createTooltip: "name: value", green, the value grey.
    let value = |stat: &str, v: String| format!("§a{}", fill2(game, "value_concat", "%s: %s", &stat_name(game, stat), &format!("§7{v}")));
    if let Some(f) = fields {
        // VanillaStat: always shown.
        for (stat, v) in [("min_delay", f.min_spawn_delay), ("max_delay", f.max_spawn_delay), ("spawn_count", f.spawn_count), ("max_nearby_entities", f.max_nearby_entities), ("req_player_range", stats.range), ("spawn_range", f.spawn_range)] {
            out.push(value(stat, v.to_string()));
        }
    }
    // CustomStat: only off its default. PercentageStat as "#.##%", BooleanStat as its name in dark green, LevelStat in numerals.
    if stats.initial_health != 1.0 {
        out.push(value("initial_health", percent_text(stats.initial_health)));
    }
    for (stat, on) in [("ignore_players", stats.ignore_players), ("ignore_conditions", stats.ignore_conditions), ("redstone_control", stats.redstone_control), ("ignore_light", stats.ignore_light), ("no_ai", stats.no_ai), ("silent", stats.silent), ("youthful", stats.youthful), ("burning", stats.burning)] {
        if on {
            out.push(format!("§2{}", stat_name(game, stat)));
        }
    }
    if stats.echoing != 0 {
        out.push(value("echoing", level_text(game, stats.echoing)));
    }
    out
}

/// A stat's name (SpawnerStat.name).
fn stat_name(game: &Game<'_>, stat: &str) -> String {
    let english: Vec<String> = stat
        .split('_')
        .map(|w| {
            let mut c = w.chars();
            c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
        })
        .collect();
    game.translate(&format!("stat.apothic_spawners.{stat}"), &english.join(" "))
}

/// `misc.apothic_spawners.<key>` with its two `%s` filled.
fn fill2(game: &Game<'_>, key: &str, english: &str, a: &str, b: &str) -> String {
    game.translate(&format!("misc.apothic_spawners.{key}"), english).replacen("%s", a, 1).replacen("%s", b, 1)
}

/// PercentageStat.formatValue: ATTRIBUTE_MODIFIER_FORMAT ("#.##") of the value in percent.
fn percent_text(v: f32) -> String {
    let s = format!("{:.2}", f64::from(v) * 100.0);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    format!("{}%", if s == "-0" { "0" } else { s })
}

/// LevelStat.formatValue: the level as an enchantment's numeral, "-" before it when below 0.
fn level_text(game: &Game<'_>, v: i32) -> String {
    let key = format!("enchantment.level.{}", v.unsigned_abs());
    let numeral = game.translate(&key, &key);
    if v < 0 { format!("-{numeral}") } else { numeral }
}


/// The stats Jade shows, asked of the server while Ctrl is held (SpawnerServerDataProvider sends the tile's data).
const NET_STATS: &str = "apothic_spawners:stats";

thread_local! {
    /// A spawner Jade wants the stats of, sent from the next frame.
    static ASK: std::cell::Cell<Option<Pos>> = const { std::cell::Cell::new(None) };
    /// The client's answer (the spawner, its lines and when they came), and when it last asked.
    static SHOWN: RefCell<(Option<(Pos, Vec<String>, std::time::Instant)>, Option<std::time::Instant>)> = const { RefCell::new((None, None)) };
}

/// `net_from_client`: a player asks for a spawner's stats. Answered only for a spawner within 8 blocks of the player,
/// in their dimension (the host's own client is in reach of everything it sees).
pub(in crate::game) fn net_from_client(game: &mut Game<'_>, player: u128, channel: &str, data: &[u8]) {
    if channel != NET_STATS || !game.native_on(NS) {
        return;
    }
    let Ok(text) = std::str::from_utf8(data) else { return };
    let v: Vec<i32> = text.split(' ').filter_map(|s| s.parse().ok()).collect();
    let [x, y, z] = v.as_slice() else { return };
    let key = (*x, *y, *z);
    if !game.serving.as_ref().is_some_and(|s| s.owner == player) {
        let Some(p) = game.native_server_players().into_iter().find(|p| p.uuid == player) else { return };
        let d = (0..3).map(|i| (p.position[i] - f64::from([key.0, key.1, key.2][i]) - 0.5).powi(2)).sum::<f64>();
        if p.dimension != game.dim.id() || d > 64.0 {
            return;
        }
    }
    if game.native_block([key.0, key.1, key.2]) != SPAWNER {
        return;
    }
    let dim = game.native_dimension().to_owned();
    let stats = stats_at(game, &dim, key).unwrap_or_default();
    let fields = match game.world.block_entities.get(&key) {
        Some(BlockEntity::Spawner(s)) => Some((**s).clone()),
        _ => None,
    };
    let mut out = format!("{} {} {}", key.0, key.1, key.2);
    for l in stat_lines(game, fields.as_ref(), &stats) {
        out.push('\n');
        out.push_str(&l);
    }
    let _ = game.native_net_send_to_player(player, NET_STATS, out.as_bytes());
}

/// `frame`: the stats request Jade's line queued.
fn send_stats_request(game: &mut Game<'_>) {
    if let Some(key) = ASK.with(|a| a.take()) {
        let _ = game.native_net_send_to_server(NET_STATS, format!("{} {} {}", key.0, key.1, key.2).as_bytes());
    }
}

/// `net_from_server`: the stats the client asked for.
pub(in crate::game) fn net_from_server(channel: &str, data: &[u8]) {
    if channel != NET_STATS {
        return;
    }
    let Ok(text) = std::str::from_utf8(data) else { return };
    let mut parts = text.split('\n');
    let v: Vec<i32> = parts.next().unwrap_or("").split(' ').filter_map(|s| s.parse().ok()).collect();
    let [x, y, z] = v.as_slice() else { return };
    let lines = parts.map(str::to_owned).collect();
    SHOWN.with_borrow_mut(|s| s.0 = Some(((*x, *y, *z), lines, std::time::Instant::now())));
}

apothic_spawners_optional::if_mod_items!("jade", {
/// SpawnerClientProvider: "Hold Ctrl for Stats", or with Ctrl held the stats the server sent for this spawner.
fn jade_lines(game: &Game<'_>, target: &crate::game::jade::Target, lines: &mut Vec<crate::game::jade::Line>) {
    let crate::game::jade::Target::Block { pos, block } = target else { return };
    if block != SPAWNER || !game.native_on(NS) {
        return;
    }
    let key = (pos[0], pos[1], pos[2]);
    if !game.has_control_down_with_quirk() {
        lines.push(crate::game::jade::Line::text(game.translate("misc.apothic_spawners.ctrl_stats", "Hold Ctrl for Stats")));
        return;
    }
    let now = std::time::Instant::now();
    let ask = SHOWN.with_borrow_mut(|s| {
        let due = s.1.is_none_or(|t| now.duration_since(t).as_millis() >= 250);
        if due {
            s.1 = Some(now);
        }
        due
    });
    if ask {
        // Jade's providers only read the game: the frame hook sends it (`send_stats_request`).
        ASK.with(|a| a.set(Some(key)));
    }
    SHOWN.with_borrow(|s| {
        if let Some((at, shown, when)) = &s.0
            && *at == key
            && now.duration_since(*when).as_secs() < 2
        {
            for l in shown {
                lines.push(crate::game::jade::Line::text(l.clone()));
            }
        }
    });
}
});

fn register_jade() {
    apothic_spawners_optional::if_mod!("jade", {
        crate::game::jade::register_client(jade_lines);
    } else { () });
}

apothic_spawners_optional::if_mod_items!("jei", {
apothic_spawners_optional::kit_use!(layer as kit_layer);
apothic_spawners_optional::kit_use!(window as kit_window);

// JEI's Spawner Modifiers category (SpawnerCategory). Each modifier as a recipe: what is held in the main hand and the
// off hand, the spawner it is used on, and its stat changes as the category writes them. The Java category draws
// them on its own texture (gui/spawner_jei.png), as the kit's picture card does here.

/// An ingredient's items, for a recipe's slot.
fn ingredient_stacks(game: &Game<'_>, json: &serde_json::Value) -> Vec<crate::game::jei::Stack> {
    let json = match json {
        serde_json::Value::String(s) => match s.strip_prefix('#') {
            Some(tag) => serde_json::json!({ "tag": tag }),
            None => serde_json::json!({ "item": s }),
        },
        other => other.clone(),
    };
    match game.native_recipe_ingredient(&json) {
        Some(i) => i.order.iter().map(|id| crate::game::jei::Stack::item(id)).collect(),
        None => json["item"].as_str().map(|id| vec![crate::game::jei::Stack::item(id)]).unwrap_or_default(),
    }
}

/// SpawnerStat.formatValue of a modifier's value for `stat`.
fn format_value(game: &Game<'_>, stat: &str, v: &serde_json::Value) -> String {
    match (stat, v) {
        (_, serde_json::Value::Bool(b)) => b.to_string(),
        ("initial_health", v) => percent_text(v.as_f64().unwrap_or(0.0) as f32),
        ("echoing", v) => level_text(game, v.as_f64().unwrap_or(0.0) as i32),
        (_, serde_json::Value::Number(n)) => n.as_i64().map_or_else(|| n.to_string(), |i| i.to_string()),
        (_, other) => other.to_string(),
    }
}

/// SpawnerCategory.getStatMessage.
fn stat_message(game: &Game<'_>, c: &Change) -> String {
    let name = stat_name(game, &c.stat);
    let value = format_value(game, &c.stat, &c.value);
    if !c.set {
        let shown = match value.as_str() {
            "true" => "+".to_owned(),
            "false" => "-".to_owned(),
            v if c.value.as_f64().is_some_and(|n| n as i64 > 0) => format!("+{v}"),
            v => v.to_owned(),
        };
        fill2(game, "concat", "%s %s", &shown, &name)
    } else if c.value.is_number() {
        fill2(game, "value_concat", "%s: %s", &name, &value)
    } else if value == "true" {
        fill2(game, "on", "%s: On", &name, "")
    } else {
        fill2(game, "off", "%s: Off", &name, "")
    }
}

/// SpawnerCategory.getTooltip over a stat line: its name, what it does, and the bounds an added number keeps to.
fn stat_tip(game: &Game<'_>, c: &Change) -> Vec<String> {
    let mut tip = vec![
        format!("§a§n{}", stat_name(game, &c.stat)),
        format!("§7{}", game.translate(&format!("stat.apothic_spawners.{}.desc", c.stat), "")),
    ];
    if c.value.is_number() && !c.set {
        if c.min.is_some() || c.max.is_some() {
            tip.push(" ".to_owned());
        }
        for (bound, key, english) in [(c.min, "min_value", "Min Value: %s"), (c.max, "max_value", "Max Value: %s")] {
            if let Some(b) = bound {
                let v = format_value(game, &c.stat, &serde_json::json!(b));
                tip.push(format!("§7{}", fill2(game, key, english, &v, "")));
            }
        }
    }
    tip
}

fn modifier_card(game: &Game<'_>, m: &Modifier) -> kit_window::Card {
    // SpawnerCategory: spawner_jei.png (169x75); the main hand's slot at 11,11 and the off hand's at 11,48, or, with no
    // off hand, the texture's empty-hand part over that one; the spawner drawn (not a slot) at 31,29; the stat lines
    // right-aligned at 168, 11 apart around 37, blue under the pointer. The "?"s light up and say which hand.
    let misc = |key: &str, english: &str| game.translate(&format!("misc.apothic_spawners.{key}"), english);
    let mut slots = vec![([11.0, 11.0], ingredient_stacks(game, &m.main), Vec::new())];
    let mut covers = Vec::new();
    let mut hovers = vec![kit_window::PictureHover {
        rect: [-1.0, 13.0, 10.0, 12.0],
        tip: vec![misc("mainhand", "Held in Main Hand")],
        cover: Some(([0.0, 75.0, 10.0, 12.0], [-1.0, 13.0])),
    }];
    match &m.off {
        Some(off) => {
            slots.push(([11.0, 48.0], ingredient_stacks(game, off), Vec::new()));
            let mut tip = vec![misc("offhand", "Held in Off Hand")];
            if !m.consumes_off {
                tip.push(format!("§7{}", misc("not_consumed", "This item is not consumed")));
            }
            hovers.push(kit_window::PictureHover { rect: [-1.0, 50.0, 10.0, 12.0], tip, cover: Some(([0.0, 75.0, 10.0, 12.0], [-1.0, 50.0])) });
        }
        None => covers.push(([0.0, 88.0, 28.0, 34.0], [1.0, 31.0])),
    }
    hovers.push(kit_window::PictureHover { rect: [33.0, 30.0, 16.0, 16.0], tip: vec![misc("rclick_spawner", "Right-Click the Spawner")], cover: None });
    let top = 37 - (m.changes.len() as i32 * 11) / 2 + 2;
    let texts = m
        .changes
        .iter()
        .enumerate()
        .map(|(i, c)| kit_window::PictureText {
            at: [168.0, (top + i as i32 * 11) as f32],
            text: stat_message(game, c),
            color: 0xFF33_3333,
            hover: 0xFF80_80FF,
            right: true,
            tip: stat_tip(game, c),
        })
        .collect();
    kit_window::Card::Picture {
        sprite: "apothic_spawners:gui/spawner_jei".to_owned(),
        src: [0.0, 0.0, 169.0, 75.0],
        covers,
        slots,
        items: vec![([31.0, 29.0], SPAWNER.to_owned())],
        texts,
        hovers,
        fluids: Vec::new(),
    }
}

/// The kit's page flow: the Spawner Modifiers tab for a spawner (all of them) or for an item a modifier holds.
fn jei_tabs(game: &Game<'_>, item: &crate::game::jei::Stack, uses: bool) -> Vec<kit_window::Tab> {
    let all = modifiers(game);
    let held = |m: &Modifier| {
        let has = |j: &serde_json::Value| ingredient_stacks(game, j).iter().any(|s| s.item == item.item);
        has(&m.main) || m.off.as_ref().is_some_and(has)
    };
    let cards: Vec<kit_window::Card> = all
        .iter()
        .filter(|m| item.item == SPAWNER || (uses && held(m)))
        .map(|m| modifier_card(game, m))
        .collect();
    if cards.is_empty() {
        return Vec::new();
    }
    let title = game.translate("title.apothic_spawners.spawner", "Spawner Modification");
    vec![kit_window::Tab { icon: SPAWNER.to_owned(), title, catalysts: vec![SPAWNER.to_owned()], cards }]
}

fn jei_register() {
    kit_layer::register(kit_layer::Provider { is_catalyst: |item| item == SPAWNER, tabs: jei_tabs, ..kit_layer::Provider::new(NS) });
}
});

apothic_spawners_optional::kit_use!(guide as kit_guide);

/// `frame`: the guide's pages.
pub(in crate::game) fn kit_frame(game: &mut Game<'_>) {
    if !game.native_on(NS) {
        return;
    }
    kit_guide::register(NS, "Apothic Spawners", "Apothic Spawners guide");
    register_jade();
    send_stats_request(game);
    apothic_spawners_optional::if_mod!("jei", {
        jei_register();
    } else { () });
}
