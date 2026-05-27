# Obsidian Wiki Maintenance

## Purpose

This file defines the policy for maintaining the Reinhardt project knowledge base in the Obsidian wiki vault. The wiki captures architecture decisions, development patterns, troubleshooting solutions, and lessons learned that would otherwise be lost between conversations.

---

## Vault Reference

**Vault Path:** `~/obsidian/reinhardt-wiki`
**Access Method:** Obsidian MCP server (`obsidian-vault`)
**Vault CLAUDE.md:** Contains structure, conventions, and operation instructions

---

## When to Update (OW-1)

Update the Obsidian wiki at the **end of a meaningful work unit** — after committing or completing a logical chunk of work. A "meaningful work unit" includes:

| Trigger | Example |
|---------|---------|
| Architectural decision made | Chose `Arc<dyn Trait>` over generic parameter for DI container |
| New pattern discovered | Dead-code typecheck pattern for type-erased closures |
| Troubleshooting solution found | CI OOM caused by arm64 runner memory limits |
| Lesson learned from incident | Partial release failure recovery procedure |
| Cross-cutting knowledge gained | Semgrep false-positive triggers on comment continuation lines |

**DO NOT update for:**
- Trivial changes (typo fixes, formatting, import reordering)
- Work still in progress (uncommitted, untested)
- Information already captured in the wiki (check `wiki/hot.md` first)
- Operational records (PR creation logs, CI re-run triggers, review thread replies)

---

## Update Procedure (OW-2)

### Step 1: Check Availability

Attempt to call an Obsidian MCP tool (e.g., `obsidian_list_files_in_vault`).

- **If available:** proceed to Step 2
- **If unavailable (MCP error, connection refused, etc.):** skip the entire wiki update — do NOT block primary work

### Step 2: Read Current Context

1. Read `wiki/hot.md` for recent context
2. Read `wiki/index.md` for the master catalog
3. Determine whether the new knowledge is already captured

### Step 3: Create or Update Pages

Create new pages or update existing ones under the appropriate category:

| Knowledge Type | Wiki Location | Template |
|---------------|---------------|----------|
| Code pattern / idiom | `wiki/knowledge/patterns/` | Pattern template |
| Bug fix / workaround | `wiki/knowledge/troubleshooting/` | Troubleshooting template |
| Lesson learned | `wiki/knowledge/learnings/` | Learning template |
| Architecture decision | `wiki/decisions/` | ADR template |
| Domain overview | `wiki/domains/` | Domain template |

**Page Requirements:**
- YAML frontmatter: `type`, `status`, `created`, `updated`, `tags` (minimum)
- Use `[[Wikilink]]` format for cross-references
- Content in English

### Step 4: Update Meta Pages

After creating or updating pages, update these meta pages:

1. **`wiki/index.md`** — Add new page entries under the appropriate section
2. **`wiki/hot.md`** — Refresh the "Recent Changes" and "Key Recent Facts" sections
3. **`wiki/log.md`** — Prepend a new entry at the TOP with: date, mode, pages created/updated, sources

---

## Skip Conditions (OW-3)

The wiki update MUST be skipped entirely (no partial updates) when ANY of these conditions are true:

- Obsidian MCP server is unavailable or returns connection errors
- The current work is trivial (no new knowledge generated)
- The session is focused on emergency/hotfix work where speed is critical
- Wiki updates are explicitly disabled via configuration or skip flag

When skipping, do NOT:
- Report the skip as an error or warning
- Retry the MCP connection
- Block or delay primary work

---

## Wiki Query and Knowledge Retrieval (OW-5)

Before answering questions or making decisions that may benefit from prior knowledge, query the wiki to improve response quality:

1. Use `/wiki-query` to search for relevant existing knowledge in the vault
2. Incorporate retrieved context into the current response or decision

This enables accumulated project knowledge to inform ongoing work, reducing repeated investigation and improving consistency.

---

## Wiki Ingest — Synchronized Knowledge Capture (OW-6)

When saving knowledge to any memory system (claude-mem, Claude Code auto-memory, Serena, or equivalent), **simultaneously** invoke `/wiki-ingest` to persist the same knowledge in the Obsidian wiki.

**Trigger conditions (save to wiki at the same time as memory):**
- claude-mem `memory_add` or `observation_add` — also `/wiki-ingest`
- Claude Code auto-memory write (`Write` to `~/.claude/projects/.../memory/`) — also `/wiki-ingest`
- Serena `write_memory` — also `/wiki-ingest`
- Any other memory persistence operation

**Rationale:** Memory systems are conversation-scoped or tool-scoped. The Obsidian wiki provides a durable, cross-session, human-browsable knowledge base. Dual-writing ensures knowledge is never siloed in a single system.

**Skip conditions:** Same as OW-3 (MCP unavailable, trivial, emergency, or explicitly disabled). If Obsidian MCP is unavailable, proceed with the memory save alone — do NOT block it.

---

## Quality Standards (OW-4)

### Content Quality
- Each page must provide actionable knowledge (not just a record of what happened)
- Include the **why** — what constraint or context led to the decision/pattern
- Include the **how** — concrete code examples or commands when applicable
- Reference GitHub Issues/PRs for traceability (e.g., `#4624`, `PR #4627`)

### Exclusions
- Do NOT record operational details (which agent ran what, session IDs, timestamps of tool calls)
- Do NOT duplicate information already in CLAUDE.md, AGENTS.md, or `instructions/`
- Do NOT record conversation details or session-specific interactions
- Do NOT create pages for knowledge that is obvious from reading the code

---

## Quick Reference

### MUST DO
- Check Obsidian MCP availability before attempting wiki updates (OW-2)
- Skip wiki update entirely if MCP is unavailable (OW-3)
- Read `wiki/hot.md` before creating new pages to avoid duplicates (OW-2)
- Update `wiki/index.md`, `wiki/hot.md`, and `wiki/log.md` after every page creation or update (OW-2)
- Include YAML frontmatter on all wiki pages (OW-2)
- Focus on actionable knowledge with "why" and "how" (OW-4)
- Reference GitHub Issues/PRs for traceability (OW-4)
- Use `/wiki-query` to retrieve existing knowledge before answering questions or making decisions (OW-5)
- Dual-write: when saving to any memory system (claude-mem, auto-memory, Serena), simultaneously `/wiki-ingest` to the Obsidian wiki (OW-6)

### NEVER DO
- Block primary work due to Obsidian MCP unavailability (OW-3)
- Create wiki pages for trivial changes or operational records (OW-1)
- Duplicate information already in CLAUDE.md or `instructions/` (OW-4)
- Record conversation details or session-specific interactions in the wiki (OW-4)
- Perform partial meta-page updates (update all three or none) (OW-2)
- Save knowledge to a memory system without simultaneously writing to the wiki (OW-6), unless Obsidian MCP is unavailable
