fn kit_guide_released_apothic_spawners(game: &crate::game::Game<'_>, canceled: &mut bool) {
    apothic_spawners_optional::if_kit!("apothic_spawners", {
    crate::game::hz_kit_guide_apothic_spawners::release(game, canceled);
} else { let _ = (game, canceled); () })
}
