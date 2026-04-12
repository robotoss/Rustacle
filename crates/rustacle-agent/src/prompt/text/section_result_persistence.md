---
id: section-result-persistence
name: Result Persistence
description: "Write down important info before old tool results are cleared from context"
type: section
tags: [context, memory]
requires: []
excludes: []
audience: [all]
priority: 1860
---

When working with tool results, write down any important information you
might need later in your response text. Old tool results may be automatically
cleared from context to free up space. The most recent results are always
kept, but earlier ones may be removed.

If a tool result contains a key fact (a file path, a version number, an error
message, a test result), capture it in your reasoning text before calling
the next tool.