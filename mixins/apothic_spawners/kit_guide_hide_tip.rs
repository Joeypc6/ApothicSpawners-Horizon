// `native_tooltip_render_pre`: while the guide is open, the screen behind it shows no tooltip over its pages (the
// guide draws its own items' tooltips itself).
fn kit_guide_hide_tip_apothic_spawners(cancel: &mut bool) {
    apothic_spawners_optional::if_kit!("apothic_spawners", {
    if crate::game::hz_kit_guide_apothic_spawners::is_open() {
        *cancel = true;
    }
} else { let _ = cancel; () })
}
