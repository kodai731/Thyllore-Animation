---
name: plan-fix
description: Takes an IssueHistory MD file as input, investigates the root cause deeply using bug-investigator and expert-explore agents, then appends a detailed implementation plan to the file.
user-invocable: true
allowed-tools: Read, Grep, Glob, Task, Write, Edit
---

# Plan Fix — Detailed Fix Planning from Issue Reports

Takes an investigated issue MD file and appends a detailed, actionable fix plan by leveraging specialized agents for deep investigation.

## Usage

```
/plan-fix .claude/local/IssueHistory/SomeIssue.md
```

## Execution Flow

### Step 1: Read the Issue File

Read the MD file specified in `$ARGUMENTS` and extract:

- Symptom
- Root Cause (current understanding)
- Existing Fix description (if any)
- Related file paths and line numbers

If `$ARGUMENTS` is empty, display an error message and stop.

### Step 2: Parallel Agent Investigation

Launch the following 2 agents **in parallel** using the Task tool.

#### expert-explore Agent

Deep investigation of the code surrounding the issue. Instructions to the agent:

- Trace the data flow through all related files mentioned in the issue
- Identify all callers and callees of the affected functions
- Check whether similar patterns exist elsewhere in the codebase
- List all locations where the fix could cause side effects
- Do NOT modify any code. Research and report only.

#### bug-investigator Agent

Root cause verification and fix proposal. Instructions to the agent:

- Form multiple hypotheses (at least 3) about the root cause and verify each against the code
- List at least 2-3 fix approaches
- Score each approach on Correctness / Minimal Impact / Robustness / Maintainability (1-5 scale)
- Select the recommended approach and describe the fix steps at concrete code-change level
- Do NOT modify any code. Research and planning only.

### Step 3: Integrate Results and Build the Plan

Merge findings from both agents into a structured fix plan following this format:

```markdown
## Detailed Fix Plan

### Investigation Summary
- Data flow analysis results
- Impact scope listing

### Approach Comparison
| Approach | Correctness | Minimal Impact | Robustness | Maintainability | Total |
|----------|-------------|----------------|------------|-----------------|-------|
| A: ...   | ?           | ?              | ?          | ?               | ?     |
| B: ...   | ?           | ?              | ?          | ?               | ?     |

### Recommended Approach
- Selection rationale

### Implementation Steps
1. File path and concrete change description for each step
2. Expected result after each step
3. Verification method

### Verification Plan
- Build check
- Test check
- Visual verification steps (if applicable)

### Risk Assessment
- Potential side effects
- Additional tests needed
```

### Step 4: Append to the MD File

Append the completed fix plan to the end of the original issue MD file.
Do NOT modify existing content in the file.

## Important Rules

- Agents must NOT modify code. Investigation and planning only.
- The plan must include concrete file paths and line numbers.
- The plan must respect coding conventions defined in CLAUDE.md.
- Respond to the user in Japanese. Write the plan content in English.
