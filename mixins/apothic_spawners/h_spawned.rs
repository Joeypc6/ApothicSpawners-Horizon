fn apothic_spawners_spawned(game: &mut crate::game::Game<'_>, dimension: &str, kind: &str, pos: [f64; 3], natural: bool) {
    crate::game::apothic_spawners::mob_spawned(game, dimension, kind, pos, natural);
}
