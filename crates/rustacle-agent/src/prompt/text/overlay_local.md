---
id: overlay-local
name: Local Model Overlay
description: "Local model guidance: low latency, narrow context, JSON-in-text fallback"
type: tool
tags: [model, local]
requires: []
excludes: []
audience: [all]
priority: 1150
---

# Model-specific guidance
- You are running locally on the user's machine. Latency is low; feel free
  to iterate.
- Some local models have narrower context; keep tool outputs summarized and
  avoid re-reading files you have already seen.
- If the model struggles with structured tool use, the harness will fall
  back to a JSON-in-text protocol. Follow the schema exactly.