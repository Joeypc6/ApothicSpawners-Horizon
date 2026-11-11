// `frame`: the layer's work while JEI is on; its window shut when JEI is turned off.
fn kit_layer_frame_apothic_spawners(game: &mut crate::game::Game<'_>) {
    apothic_spawners_optional::if_kit_jei!("apothic_spawners", {
    if game.native_on(crate::game::jei::ID) {
        crate::game::hz_kit_layer_apothic_spawners::frame(game);
    } else if crate::game::hz_kit_window_apothic_spawners::is_open() {
        // JEI turned off with its window open: nothing would close it, and it would keep taking the screen's input.
        crate::game::hz_kit_window_apothic_spawners::close();
        let _ = crate::game::hz_kit_window_apothic_spawners::take_acts();
    }
} else { let _ = game; () })
}
