//! Debugging and diagnostic tools

pub struct DebugController;

impl DebugController {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DebugController {
    fn default() -> Self {
        Self::new()
    }
}
