//! Process-local cancel registry for in-flight Agent Engine turns.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

fn map() -> &'static Mutex<HashMap<String, Arc<AtomicBool>>> {
    static MAP: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();
    MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn register_turn(turn_id: &str) -> Arc<AtomicBool> {
    let flag = Arc::new(AtomicBool::new(false));
    if let Ok(mut g) = map().lock() {
        g.insert(turn_id.to_string(), flag.clone());
    }
    flag
}

pub fn cancel_turn(turn_id: &str) -> bool {
    if let Ok(g) = map().lock() {
        if let Some(f) = g.get(turn_id) {
            f.store(true, Ordering::SeqCst);
            return true;
        }
    }
    false
}

pub fn remove_turn(turn_id: &str) {
    if let Ok(mut g) = map().lock() {
        g.remove(turn_id);
    }
}

pub fn is_cancelled(flag: &AtomicBool) -> bool {
    flag.load(Ordering::SeqCst)
}
