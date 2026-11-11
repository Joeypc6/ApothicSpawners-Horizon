fn jei_window_scrolled(game: &crate::game::Game<'_>, scroll: [f64; 2], canceled: &mut bool) {
    apothic_spawners_optional::if_kit_jei!("apothic_spawners", {
    if game.player.is_none() || *canceled {
        return;
    }
    crate::game::hz_kit_window_apothic_spawners::scroll(scroll[1], canceled);
} else { let _ = (game, scroll, canceled); () })
}
