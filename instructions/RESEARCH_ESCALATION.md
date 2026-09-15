# Research Escalation Policy

## RE-1 (MUST): Reassess After Two Failed Attempts

After two distinct failed repairs or disproved hypotheses, stop speculative
edits and reassess the evidence before another attempt. Read-only inspection and
diagnostic commands are not repair attempts. Reproduce the symptom, identify
what each result ruled out, and seek authoritative documentation for uncertain
external behavior.

## RE-2 (MUST): Use Available Authoritative Sources

Start with the relevant local implementation, configuration, dependency version,
and official documentation. Use a callable documentation lookup or web search
when current external facts are needed. Open the original source before relying
on a search summary.

Context7, Fetch, Perplexity, Tavily, and Brave are optional capabilities, not a
mandatory chain. Use a working equivalent when one is absent or fails. Missing
optional tooling does not require installation or a pause in independent work.
Respect task-specific source restrictions and avoid exposing secrets or private
repository content in external queries.

If external access is unavailable, state the unresolved fact and use additional
local evidence where it can decide the issue. Do not repeat the same ineffective
repair or claim an unverified hypothesis as the root cause.

## RE-3 (MUST): Ask a Discriminating Research Question

Include the exact symptom, relevant version/target/environment, attempted
hypotheses, and evidence that ruled them out. Identify what new fact would change
the next implementation decision. Query only the context necessary to answer it.

## RE-4 (SHOULD): Preserve Durable Findings

Record the technical cause, supporting source, and working fix in the relevant
project documentation when needed. Use OBSIDIAN_WIKI.md for non-duplicated durable
knowledge. Follow the active memory system's write permissions; research does
not independently authorize writing private memory or changing user settings.

## RE-5 (SHOULD): Keep Reasoning Useful

Use a concise hypothesis/evidence summary for complex failures. A separate
reasoning tool or another agent is optional and must satisfy the task's tool and
delegation rules. Communicate conclusions and evidence, not a transcript of
internal reasoning.

## Related Documentation

- [Agent Workflow](AGENT_WORKFLOW.md)
- [Issue Handling](ISSUE_HANDLING.md)
- [Upstream Issue Reporting](UPSTREAM_ISSUE_REPORTING.md)
- [GitHub Interaction](GITHUB_INTERACTION.md)
