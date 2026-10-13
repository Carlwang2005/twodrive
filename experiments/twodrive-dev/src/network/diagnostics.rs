//! Bounded safe transport diagnostics plus persistent connection route status.
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};
#[derive(Default)]
pub struct Diagnostics {
    lines: Mutex<Vec<String>>,
    fallback: AtomicBool,
}
impl Diagnostics {
    pub fn record(&self, line: &str) {
        if !line.starts_with("network:") {
            return;
        }
        // The owned peer CLI emits this exact protocol marker. Keep the route
        // flag independently of the bounded, human-readable diagnostic history.
        if line.starts_with("network: route=system/proxy fallback;") {
            self.fallback.store(true, Ordering::Relaxed);
        }
        let mut lines = self.lines.lock().unwrap();
        if lines.len() >= 24 {
            lines.remove(0);
        }
        lines.push(line.chars().take(512).collect());
    }
    pub fn snapshot(&self) -> Vec<String> {
        self.lines.lock().unwrap().clone()
    }
    pub fn fallback(&self) -> bool {
        self.fallback.load(Ordering::Relaxed)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn route_warning_survives_diagnostic_rotation_and_resets_per_connection() {
        let d = Diagnostics::default();
        d.record("network: unrelated text route=system/proxy fallback");
        assert!(!d.fallback());
        d.record("network: route=system/proxy fallback; physical probe failed");
        for _ in 0..100 {
            d.record("network: route details");
        }
        assert!(d.fallback());
        assert_eq!(d.snapshot().len(), 24);
        assert!(!d.snapshot().iter().any(|s| s.contains("fallback")));
        assert!(!Diagnostics::default().fallback());
    }
}
