fn apothic_spawners_changed(level: &crate::game::GameLevel<'_>, pos: horizon_core::position::BlockPos, old: horizon_world::registry::BlockStateId, new: horizon_world::registry::BlockStateId) {
    let spawner = |s: horizon_world::registry::BlockStateId| &*level.facts.get(s).name == "minecraft:spawner";
    if spawner(old) != spawner(new) {
        crate::game::apothic_spawners::block_changed(&format!("{:?}", level.dimension), (pos.x, pos.y, pos.z));
    }
}
