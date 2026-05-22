# Research Escalation Policy

## Purpose

This file defines when and how to escalate from local investigation to external research tools when troubleshooting or designing solutions. The goal is to avoid repeated unproductive attempts and to bring in authoritative external context promptly.

---

## RE-1 (MUST): Escalation Trigger

If a problem is not resolved after **2 or more** unsuccessful improvement attempts (e.g., two distinct fixes that did not resolve the symptom), the agent **MUST** escalate to external research before further trial-and-error attempts.

**What counts as an "attempt":**
- A code change intended to fix the issue that did not resolve it
- A configuration change intended to fix the issue that did not resolve it
- A documented hypothesis tested and disproved

**What does NOT count:**
- Reading code or documentation
- Running diagnostic commands without making changes

---

## RE-2 (MUST): Research Tool Order

When escalating, use the following tool order:

1. **Context7 / Fetch** (primary): Verify the current documentation of any library, framework, or tool involved. Cheaper and lower-latency than search engines.
2. **Perplexity MCP** (preferred for research): Search with reasoning + citations. Use when documentation alone is insufficient or when the issue is not strictly a documentation gap.
3. **Tavily MCP** (alternative): Use when Perplexity is unavailable or when broader coverage is needed.
4. **Brave Search MCP** (fallback): Use when Perplexity and Tavily both fail to surface useful results, or for cross-checking citations.
5. **Fetch MCP**: Always use to retrieve and verify the original text of any URL cited by a search result before relying on it.

---

## RE-3 (MUST): Information to Include in Research Queries

When escalating, the research query MUST include:

- **Symptoms**: The exact error message, failure mode, or unexpected behavior
- **Constraints**: Project tech stack (Rust 2024, SeaQuery via `reinhardt-query`, TestContainers, etc.) and any environmental constraints (macOS, Docker not Podman, etc.)
- **Attempts**: A summary of the 2+ attempts already tried
- **Reasons attempts failed**: What evidence ruled each attempt out

This structure prevents the research tool from suggesting solutions already disproved.

---

## RE-4 (SHOULD): Persist Findings

After a successful escalation:

- Save authoritative findings (root cause, working fix, citations) to **serena memory** when the knowledge will likely apply to future work
- For project-specific decisions (architecture, library trade-offs), prefer GitWhy via `gitwhy_save` so the reasoning is captured alongside the commit
- Verify URLs cited by Perplexity/Tavily/Brave by fetching the original source before relying on them

---

## RE-5 (SHOULD): Use Sequential Thinking for Complex Problems

For multi-step problems where the failure is unclear or the solution space is large, use the `sequentialThinking` MCP tool to externalize reasoning steps before further trial-and-error.

---

## Quick Reference

### ✅ MUST DO
- Escalate to external research after 2 failed improvement attempts
- Include symptoms, constraints, attempts, and failure reasons in research queries
- Verify citation URLs via Fetch before acting on search results
- Try Context7/Fetch first, then Perplexity, then Tavily, then Brave Search

### ❌ NEVER DO
- Continue trial-and-error past 2 failed attempts without external research
- Trust search-tool summaries without verifying the original source
- Skip recording authoritative findings that future sessions will need

---

## Related Documentation

- **Issue Handling**: instructions/ISSUE_HANDLING.md
- **Upstream Issue Reporting**: instructions/UPSTREAM_ISSUE_REPORTING.md
- **GitHub Interaction**: instructions/GITHUB_INTERACTION.md
