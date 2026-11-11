fn kit_guide_char_apothic_spawners(game: &crate::game::Game<'_>, c: char, canceled: &mut bool) {
    apothic_spawners_optional::if_kit!("apothic_spawners", {
    let _ = c;
    if game.player.is_some() && crate::game::hz_kit_guide_apothic_spawners::is_open() {
        *canceled = true;
    }
} else { let _ = (game, c, canceled); () })
}
