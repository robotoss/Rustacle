---
id: overlay-anthropic
name: Anthropic Model Overlay
description: "Anthropic-specific tool_use blocks, no custom XML, lean prose"
type: tool
tags: [model, anthropic]
requires: []
excludes: []
audience: [all]
priority: 1150
---

# Model-specific guidance
- Use the `tool_use` blocks. Do not wrap tool calls in <function_calls> or
  any custom XML; the harness already handles the dialect.
- Inline <thinking> blocks are not needed — your reasoning is already visible
  via the streaming Thought events. Keep prose lean.
- For long turns, you may call up to 5 read-only tools in parallel.