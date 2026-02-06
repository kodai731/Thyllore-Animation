---
name: summarize-conversation
description: summarize conversation due to memory it and work to continue while short log.
user-invocable: true
allowed-tools: Read, Grep, Glob
---

# Skill: Conversation Summarizer (Context Memory)

## Purpose
When requested, summarize the current conversation into a compact, reusable context
that can be pasted at the beginning of a new conversation and understood by Claude
without the original chat history.

This summary is intended to:
- Preserve important decisions and assumptions
- Reduce token usage
- Allow the current conversation to be safely cleared
- avoid to long wait for Claude to read past long conversation

## When to Activate
Activate this skill ONLY when the user explicitly requests one of the following:
- "summarize this conversation"
- "create a context memory"
- "prepare a summary for the next conversation"
- "save this conversation in a reusable form"

Do NOT activate automatically.

## Output directory and file name
${PROJECT_ROOT}/.claude/last-conversation.md
Overwrite file everytime.

## Output Format
Output MUST follow this exact structure:

---
# Context Memory

## Project / Topic
- Brief description of the project or discussion topic

## Goals
- What the user is trying to achieve

## Key Decisions
- Explicit decisions already made
- Chosen approaches or rejected alternatives

## Constraints & Rules
- Technical, design, or conceptual constraints
- Non-negotiable rules or preferences

## Terminology
- Definitions of important terms or abbreviations (if any)

## Open Questions
- Unresolved issues or decisions not yet finalized

---

## Style Rules
- Use concise bullet points
- No conversational tone
- No speculation or new ideas
- Do NOT repeat the full conversation
- Do NOT include timestamps or speaker labels
- Prefer factual, stable information only

## Length Limit
- Target: 300–600 characters
- Absolute maximum: 800 characters

## Assumptions
- Assume the summary will be pasted verbatim into a new conversation
- Assume no other context is available
- Optimize for Claude’s comprehension, not human storytelling

## Forbidden Content
- Emojis
- Apologies
- Meta commentary (e.g., "In this conversation we talked about...")
- Suggestions beyond what was already discussed
