/// Backpressure policy for an event bus topic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackpressurePolicy {
    /// Publisher awaits until subscriber has room.
    /// Use for must-not-lose events (agent.reasoning, permission.ask).
    BlockPublisher,

    /// Drop oldest buffered item when full.
    /// Use for high-throughput idempotent streams (terminal.output).
    DropOldest,

    /// Keep only the latest value per subscriber.
    /// Use for state-summary topics (terminal.cwd, agent.cost).
    CoalesceLatest,
}
