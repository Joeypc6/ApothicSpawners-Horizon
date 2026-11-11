fn apothic_spawners_use_on(game: &mut crate::game::Game<'_>, stack: &ItemStack, pos: [i32; 3], claimed: &mut bool) {
    crate::game::apothic_spawners::use_item_on(game, stack, pos, claimed);
}
