# Obsidian Wiki Maintenance

## Purpose

This file defines the policy for maintaining the Reinhardt project knowledge base in the Obsidian wiki vault. The wiki captures architecture decisions, development patterns, troubleshooting solutions, and lessons learned that would otherwise be lost between conversations.

---

## Vault Reference

**Access Method:** Obsidian MCP server (`obsidian-vault`). The vault path is resolved automatically by the MCP server — do not hardcode it.
**Connectivity Check:** Call `obsidian_list_files_in_vault` to verify the MCP server is available and the vault is accessible.
**Vault CLAUDE.md:** Contains structure, conventions, and operation instructions

---

## When to Update (OW-1)

Update the Obsidian wiki **frequently and proactively**. The goal is a steadily growing, well-distributed knowledge base, so **err on the side of capturing**: whenever you learn, decide, or discover something a future contributor would want to know, write a page. When in doubt, write it — a short, focused page is far better than a lost insight.

Update at the **end of every meaningful work unit** (after committing or completing a logical chunk) and, in addition, the moment any trigger below occurs — even mid-task, as soon as the knowledge is stable enough to record. A single work unit frequently yields **several** pages spread across different categories (see OW-2 and OW-7); capture each one rather than collapsing them into a single note.

| Trigger | Example | Typical Category |
|---------|---------|------------------|
| Architectural decision made | Chose `Arc<dyn Trait>` over a generic parameter for the DI container | `decisions/` |
| New pattern / idiom discovered | Dead-code typecheck pattern for type-erased closures | `knowledge/patterns/` |
| Troubleshooting solution found | CI OOM caused by arm64 runner memory limits | `knowledge/troubleshooting/` |
| Lesson learned from an incident | Partial release failure recovery procedure | `knowledge/learnings/` |
| Crate / module understood in depth | Responsibilities and public surface of `reinhardt-query` | `modules/` |
| Reusable component studied | How the `#[model]` macro expands and what it generates | `components/` |
| External dependency assessed | Why `reinhardt-query` replaced direct SeaQuery usage | `dependencies/` |
| Data / control flow traced | Request → router → handler → response path | `flows/` |
| Concept / idea clarified | What "CoC is a right, not an obligation" means in practice | `concepts/` |
| Entity worth tracking identified | An upstream repo, tool, crate, or external project | `entities/` |
| Topic area mapped | Overview of the authentication subsystem | `domains/` |
| Options compared | `Arc<dyn Trait>` vs. generics for DI, with trade-offs | `comparisons/` |
| Non-trivial question answered | "Why are there two `DatabaseConnection` types?" | `questions/` |

**DO NOT create a page for:**
- Purely mechanical changes that produce no reusable knowledge (typo fixes, formatting, import reordering)
- Work still in progress (uncommitted, untested) — wait until it is stable, then capture it
- Knowledge already captured in the wiki — **update** the existing page instead (check `wiki/hot.md` and `wiki/index.md` first)
- Operational records (PR creation logs, CI re-run triggers, review thread replies)

Everything else is fair game. "It feels small" is **not** a reason to skip — record it as a short page and cross-link it.

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

Create new pages or update existing ones under the **most specific** category. The vault provides a rich category set — use its full range instead of defaulting everything to `knowledge/` (see OW-7):

| Knowledge Type | Wiki Location |
|---------------|---------------|
| Crate / module overview | `wiki/modules/` |
| Reusable component (macro, trait, abstraction) | `wiki/components/` |
| Code pattern / idiom / convention | `wiki/knowledge/patterns/` |
| Bug fix / workaround / recurring issue | `wiki/knowledge/troubleshooting/` |
| Lesson learned / post-mortem | `wiki/knowledge/learnings/` |
| Architecture decision (ADR) | `wiki/decisions/` |
| External dependency assessment | `wiki/dependencies/` |
| Data flow / request path / macro expansion | `wiki/flows/` |
| Concept / idea / framework | `wiki/concepts/` |
| Entity (person, org, product, repo) | `wiki/entities/` |
| Top-level topic area / domain overview | `wiki/domains/` |
| Side-by-side comparison | `wiki/comparisons/` |
| Filed answer to a user query | `wiki/questions/` |

If the target category directory does not exist yet, create it alongside the page — populating under-used categories is encouraged (OW-7). When a category has an `_index.md`, add the new page to it.

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

## Distribute Across Categories (OW-7)

Knowledge MUST be spread across the full category set, never concentrated in a single directory. Historically, pages piled up in `wiki/knowledge/troubleshooting/` while `concepts/`, `entities/`, `components/`, `flows/`, and `comparisons/` stayed empty. Counter this actively:

- **Pick the most specific category.** `knowledge/troubleshooting/` is for bug fixes and recurring issues only — it is **not** a catch-all. A reusable technique belongs in `patterns/`, a design rationale in `decisions/`, an explanation of an idea in `concepts/`.
- **One work unit → multiple pages.** A single fix often produces a `troubleshooting/` page (symptom + fix), a `patterns/` page (the reusable technique), and sometimes a `concepts/` or `components/` page. Create them all and cross-link with `[[Wikilink]]`.
- **Seed under-used categories.** Touching a crate → a `modules/` page; studying a macro or trait → a `components/` page; tracing a request → a `flows/` page; weighing options → a `comparisons/` page; answering a "why" question → a `questions/` page.
- **Prefer many small, focused, cross-linked pages** over one large catch-all page.
- **Check balance periodically.** When reviewing `wiki/index.md`, if one category dominates, look for knowledge that belongs in the empty ones and split it out.

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
- Do NOT record user interactions or conversation details
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
- Create pages frequently and proactively — when in doubt, write a short focused page rather than skip it (OW-1)
- Route each page to the **most specific** category, using the full category set rather than just `knowledge/` (OW-2, OW-7)
- Split a single work unit into multiple cross-linked pages across categories when applicable (OW-7)
- Proactively populate under-used categories (`modules/`, `components/`, `dependencies/`, `flows/`, `concepts/`, `entities/`, `comparisons/`, `questions/`) (OW-7)

### NEVER DO
- Block primary work due to Obsidian MCP unavailability (OW-3)
- Default every page to `wiki/knowledge/troubleshooting/`, or let one category dominate the vault (OW-7)
- Skip capturing reusable knowledge merely because it seems small (OW-1)
- Create wiki pages for trivial changes or operational records (OW-1)
- Duplicate information already in CLAUDE.md or `instructions/` (OW-4)
- Record user interactions or conversation details in the wiki (OW-4)
- Perform partial meta-page updates (update all three or none) (OW-2)
- Save knowledge to a memory system without simultaneously writing to the wiki (OW-6), unless Obsidian MCP is unavailable
