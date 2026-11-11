fn kit_guide_scrolled_apothic_spawners(game: &crate::game::Game<'_>, scroll: [f64; 2], canceled: &mut bool) {
    apothic_spawners_optional::if_kit!("apothic_spawners", {
    if game.player.is_some() {
        crate::game::hz_kit_guide_apothic_spawners::scroll(scroll[1], canceled);
    }
} else { let _ = (game, scroll, canceled); () })
}
