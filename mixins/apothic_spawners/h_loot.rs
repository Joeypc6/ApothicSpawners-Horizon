fn apothic_spawners_loot(game: &mut crate::game::Game<'_>, context: &crate::mod_api::NativeLootDrops<'_>, drops: &mut Vec<ItemStack>) {
    crate::game::apothic_spawners::loot_drops(game, context, drops);
}
