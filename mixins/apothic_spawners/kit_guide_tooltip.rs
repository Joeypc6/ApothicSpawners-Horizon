fn kit_guide_tooltip_apothic_spawners(stack: &horizon_client::items::ItemStack, lines: &mut Vec<(horizon_client::text::Text, u32)>) {
    apothic_spawners_optional::if_kit!("apothic_spawners", {
    crate::game::hz_kit_guide_apothic_spawners::tooltip(stack, lines);
} else { let _ = (stack, lines); () })
}
