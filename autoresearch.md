# Autoresearch Rules

- Optimize for the configured primary metric while preserving correctness.
- Do not overfit benchmarks and do not cheat benchmarks.
- Every behavior fix must include thorough unit/fixture tests.
- Guard runtime performance with benchmark checks when changing hot paths.
- Capture promising deferred ideas in `autoresearch.ideas.md`.
- **When implementing autocorrect for a cop, always check `~/Dev/rubocop` for the corresponding RuboCop autocorrect logic and mirror it as closely as practical.**
