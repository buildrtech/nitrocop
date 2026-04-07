# Deferred optimization ideas

- Rework `Layout/SpaceAroundOperators` as a two-tier pipeline (cheap byte pre-scan to detect operator presence + conditional AST/text passes) to skip heavy collector/visitor setup on files that cannot produce offenses.
- Apply the same “defer config parsing/allocations until after fast name gates” pattern used in `Rails/DynamicFindBy` to other hot AST cops (especially `Rails/FindByOrAssignmentMemoization`, `Rails/RedundantActiveRecordAllMethod`, and similar call-name-driven cops).
- Cache ActiveRecord-class ancestry lookups for receiverless `find_by_*` calls within a file (currently each candidate call walks class nodes via a fresh visitor).
- For `Lint/UselessAssignment`, experiment with variable-name interning (ID-based liveness maps/sets) to reduce repeated `String` allocations in read/write tracking.
- Fix `NITROCOP_COP_PROFILE` single-cop AST guard (`cop.name().contains('/')` fallback) to use registry capabilities only; current profiling over-attributes AST time and can mislead optimization targeting.
