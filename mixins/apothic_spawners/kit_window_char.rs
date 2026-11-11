fn jei_window_char(game: &crate::game::Game<'_>, c: char, canceled: &mut bool) {
    apothic_spawners_optional::if_kit_jei!("apothic_spawners", {
    if game.player.is_none() || *canceled {
        return;
    }
    // The panel's search box takes it first, when it has the focus.
    crate::game::hz_kit_creative_apothic_spawners::char_typed(c, canceled);
    // Over the window the keys are the window's: their characters never reach the screen behind it (the creative
    // search would take a recipe key's letter).
    if !*canceled && !c.is_control() && crate::game::hz_kit_window_apothic_spawners::takes_keys(game) {
        *canceled = true;
    }
} else { let _ = (game, c, canceled); () })
}
