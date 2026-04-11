//! Partial thought flushing: sentence boundary or timeout, whichever first.

use std::time::Duration;

/// Configuration for thought flushing behavior.
#[derive(Debug, Clone)]
pub struct FlushConfig {
    /// Flush after this many milliseconds of buffered text.
    pub flush_ms: u64,
    /// Minimum chars before considering a sentence-boundary flush.
    pub min_chars: usize,
}

impl Default for FlushConfig {
    fn default() -> Self {
        Self {
            flush_ms: 80,
            min_chars: 10,
        }
    }
}

impl FlushConfig {
    /// Get flush timeout as a `Duration`.
    #[must_use]
    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.flush_ms)
    }
}

/// Buffer for accumulating partial thought text.
#[derive(Debug, Default)]
pub struct ThoughtBuffer {
    text: String,
    config: FlushConfig,
}

impl ThoughtBuffer {
    #[must_use]
    pub fn new(config: FlushConfig) -> Self {
        Self {
            text: String::new(),
            config,
        }
    }

    /// Append text to the buffer.
    pub fn push(&mut self, chunk: &str) {
        self.text.push_str(chunk);
    }

    /// Check if the buffer should flush based on sentence boundary.
    #[must_use]
    pub fn should_flush_sentence(&self) -> bool {
        if self.text.len() < self.config.min_chars {
            return false;
        }
        // Flush on sentence-ending punctuation followed by space/newline, or newline
        self.text.ends_with(". ")
            || self.text.ends_with(".\n")
            || self.text.ends_with("? ")
            || self.text.ends_with("?\n")
            || self.text.ends_with("! ")
            || self.text.ends_with("!\n")
            || self.text.ends_with('\n')
    }

    /// Take the buffered text, clearing the buffer.
    pub fn take(&mut self) -> String {
        std::mem::take(&mut self.text)
    }

    /// Check if buffer is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Get the flush timeout.
    #[must_use]
    #[allow(dead_code)]
    pub fn timeout(&self) -> std::time::Duration {
        self.config.timeout()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flush_on_sentence_end() {
        let mut buf = ThoughtBuffer::new(FlushConfig::default());
        buf.push("Hello world. ");
        assert!(buf.should_flush_sentence());
    }

    #[test]
    fn no_flush_on_short_text() {
        let mut buf = ThoughtBuffer::new(FlushConfig::default());
        buf.push("Hi. ");
        assert!(!buf.should_flush_sentence()); // under min_chars
    }

    #[test]
    fn flush_on_newline() {
        let mut buf = ThoughtBuffer::new(FlushConfig::default());
        buf.push("A longer line of text\n");
        assert!(buf.should_flush_sentence());
    }

    #[test]
    fn take_clears_buffer() {
        let mut buf = ThoughtBuffer::new(FlushConfig::default());
        buf.push("hello");
        let text = buf.take();
        assert_eq!(text, "hello");
        assert!(buf.is_empty());
    }
}
