// Apothic Spawners' movable no-AI mobs (ASEvents.tickDumbMobs), in the mob step's own module: which mobs carry
// `apotheosis:movable` in their NeoForgeData. The game side (apothic.rs) keeps this in step with the saved data:
// set when a no-AI spawner spawns a mob, read again whenever an entity joins the level, dropped when it leaves.
// Only the server's hooks fill it, so a client's mob step never matches.
use std::collections::BTreeSet;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

static MOVABLE: Mutex<BTreeSet<u32>> = Mutex::new(BTreeSet::new());
/// Any id in MOVABLE: a no-AI mob checks the set only then.
static ANY: AtomicBool = AtomicBool::new(false);

pub(super) fn is_movable(id: u32) -> bool {
    ANY.load(Ordering::Relaxed) && MOVABLE.lock().unwrap_or_else(|e| e.into_inner()).contains(&id)
}

impl crate::mobs::Mob {
    /// Marks (or unmarks) mob `id` as one of Apothic Spawners' movable no-AI mobs.
    pub fn apothic_spawners_set_movable(id: u32, on: bool) {
        if !on && !ANY.load(Ordering::Relaxed) {
            return;
        }
        let mut set = MOVABLE.lock().unwrap_or_else(|e| e.into_inner());
        if on {
            set.insert(id);
        } else {
            set.remove(&id);
        }
        ANY.store(!set.is_empty(), Ordering::Relaxed);
    }

    /// Forgets every mark: the server stopped, and the next world's ids start again.
    pub fn apothic_spawners_clear_movable() {
        let mut set = MOVABLE.lock().unwrap_or_else(|e| e.into_inner());
        set.clear();
        ANY.store(false, Ordering::Relaxed);
    }
}
