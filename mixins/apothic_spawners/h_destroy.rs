fn apothic_spawners_destroy(game: &mut crate::game::Game<'_>, pos: [i32; 3]) {
    crate::game::apothic_spawners::destroyed(game, pos);
}
