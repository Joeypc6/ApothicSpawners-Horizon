fn apothic_spawners_dumb_mobs(m: &mut crate::mobs::Mob, w: &crate::mobs::MobWorld<'_>, events: &mut Vec<crate::mobs::MobEvent>) {
    if m.no_ai && apothic_spawners_movable::is_movable(m.id) {
        m.no_ai = false;
        let input = horizon_core::geometry::Vec3::new(f64::from(m.ai.xxa), f64::from(m.ai.zza), f64::from(m.ai.yya));
        m.travel(w, input, events);
        m.no_ai = true;
    }
}
