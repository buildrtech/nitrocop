# Deferred optimization ideas

- Add an opt-in persistent gem-path cache keyed by `(working_dir, Gemfile.lock mtime, gem_name)` to avoid any Bundler process launch for stable projects; must include robust invalidation and a strict fallback path to avoid stale-plugin correctness drift.
- Investigate selector matching data structures (`args.only`/`args.except` HashSet precompute) for non-default CLI modes so speedups carry beyond default no-selector runs.
