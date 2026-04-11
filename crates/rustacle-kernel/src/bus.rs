/// Event bus stub. Real implementation arrives in Sprint 2.
pub struct Bus;

impl Bus {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for Bus {
    fn default() -> Self {
        Self::new()
    }
}
