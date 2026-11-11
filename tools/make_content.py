"""Apothic Spawners' data (recipes, advancements, the Capturing enchantment, the unstable spawner's loot, tags),
translations and textures as a Horizon content mod, from the jar. It adds no block or item: it changes the vanilla
spawner. Also the W guide's pages and its 3D scene.

Usage: python make_content.py <ApothicSpawners jar> <out.hmod>
"""
import io
import json
import sys
import zipfile

ID = "apothic_spawners"
VERSION = "2.0.1"


def guide(files):
    import guide_index
    pages = {
        "spawners.md": """---
navigation:
  title: Apothic Spawners
item_ids:
- minecraft:spawner
---

# Spawners

A spawner can be picked up with **Silk Touch** (level 1 by default), which keeps everything it was set to. It costs
the tool 100 durability.

Hold an item and right-click a spawner to change it: each **spawner modifier** names the item for the main hand, and
some an item for the off hand too. Both are used up, unless the modifier says otherwise. The JEI category
**Spawner Modifiers** lists them all.

A spawner caught in an explosion becomes **unstable**: three seconds later it bursts, with twelve of its mob and the
loot of a broken spawner.

**Capturing** on a weapon gives a mob a chance to drop its spawn egg.
""",
    }
    pages["spawners.md"] += "\n" + guide_index.scene(files, ID, "spawner", [
        (0, 0, 0, "minecraft:spawner"), (0, -1, 0, "minecraft:redstone_block"), (2, 0, 0, "minecraft:spawner"), (2, 1, 0, "minecraft:lever", {"face": "floor", "facing": "north"})])
    guide_index.write(files, ID, pages)


def spawn_eggs():
    """Every spawn egg in 26.3's item registry (from the installed Nightly's prepared files), or [] if not found."""
    import glob
    for path in sorted(glob.glob("C:/Users/*/AppData/Roaming/horizon-launcher/versions/nightly-*/.local/horizon-client/26.3/registries.json"), reverse=True):
        try:
            items = json.load(open(path, encoding="utf-8"))["minecraft:item"]["entries"]
        except (OSError, ValueError, KeyError):
            continue
        return sorted(k for k in items if k.endswith("_spawn_egg"))
    return []


def main(jar, out):
    source = zipfile.ZipFile(jar)
    files = {}
    modifier_advancements = {}
    for name in source.namelist():
        if name.endswith("/") or not name.startswith(("assets/", "data/")):
            continue
        if name.endswith(".json") and name.startswith("data/"):
            doc = json.loads(source.read(name).decode("utf-8-sig"))
            if isinstance(doc, dict):
                doc.pop("neoforge:conditions", None)
            # ModifierTrigger is the Java mod's own criterion: the native mod tests its conditions when a modifier is
            # used and grants the advancement (data/apothic_spawners/hz/modifier_advancements.json keeps them). In
            # the advancement it is `impossible`, which only a grant completes.
            if "/advancement/" in name and isinstance(doc, dict):
                for key, c in doc.get("criteria", {}).items():
                    if c.get("trigger") == "apothic_spawners:spawner_modifier":
                        adv = name.split("/advancement/", 1)[1][:-5]
                        modifier_advancements[f"apothic_spawners:{adv}"] = {"criterion": key, "conditions": c.get("conditions", {})}
                        doc["criteria"][key] = {"trigger": "minecraft:impossible"}
                    # ApothicSpawners' item sub-predicate `apothic_spawners:spawn_egg` ("a SpawnEggItem"): Horizon
                    # doesn't read it, and an unread sub-predicate matched every item (any pickup earned "Captured!").
                    # As vanilla's form: the game's spawn eggs, listed from its item registry.
                    elif c.get("trigger") == "minecraft:inventory_changed":
                        for pred in c.get("conditions", {}).get("items", []):
                            subs = pred.get("predicates", {}) if isinstance(pred, dict) else {}
                            if "apothic_spawners:spawn_egg" in subs:
                                del subs["apothic_spawners:spawn_egg"]
                                if not subs:
                                    pred.pop("predicates", None)
                                eggs = spawn_eggs()
                                if eggs:
                                    pred["items"] = eggs
                                else:
                                    doc["criteria"][key] = {"trigger": "minecraft:impossible"}
            files[name] = json.dumps(doc, indent=2).encode()
            continue
        files[name] = source.read(name)
    files["data/apothic_spawners/hz/modifier_advancements.json"] = json.dumps(modifier_advancements, indent=2).encode()
    guide(files)
    files["horizon-mod.toml"] = "\n".join([
        "# Apothic Spawners' data and textures, from ApothicSpawners-26.1.2-2.0.1.jar by apothicspawners-horizon/tools/make_content.py.",
        f'id = "{ID}"',
        'name = "Apothic Spawners (data)"',
        f'version = "{VERSION}"',
        'description = "Apothic Spawners\' modifiers, enchantment, advancements and textures."',
        'authors = ["Shadows_of_Fire"]',
        'api = "0.2"',
        'side = "both"',
        'capabilities = ["registries"]',
        "",
    ]).encode()
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w") as z:
        for name in sorted(files):
            info = zipfile.ZipInfo(name, date_time=(2026, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, files[name])
    open(out, "wb").write(buffer.getvalue())
    print(len(files), "files,", len(buffer.getvalue()), "bytes")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])
