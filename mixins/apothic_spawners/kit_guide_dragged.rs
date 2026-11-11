fn kit_guide_dragged_apothic_spawners(delta: [f32; 2], canceled: &mut bool) {
    apothic_spawners_optional::if_kit!("apothic_spawners", {
    crate::game::hz_kit_guide_apothic_spawners::drag(delta, canceled);
} else { let _ = (delta, canceled); () })
}
