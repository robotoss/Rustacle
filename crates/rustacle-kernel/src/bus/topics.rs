use serde::{Deserialize, Serialize};

/// Well-known bus topic names.
pub struct BusTopics;

impl BusTopics {
    /// PTY output bytes (high throughput, `DropOldest`).
    pub const TERMINAL_OUTPUT: &str = "terminal.output";

    /// Working directory changed (`CoalesceLatest`).
    pub const TERMINAL_CWD: &str = "terminal.cwd";

    /// Agent reasoning step (`BlockPublisher`).
    pub const AGENT_REASONING: &str = "agent.reasoning";

    /// Agent cost sample (`CoalesceLatest`).
    pub const AGENT_COST: &str = "agent.cost";

    /// Permission ask from kernel to UI (`BlockPublisher`).
    pub const PERMISSION_ASK: &str = "permission.ask";
}

/// Terminal output chunk published on `terminal.output`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalChunk {
    pub tab_id: String,
    pub data: Vec<u8>,
    pub seq: u64,
}

/// Working directory change published on `terminal.cwd`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CwdChange {
    pub tab_id: String,
    pub cwd: String,
}
