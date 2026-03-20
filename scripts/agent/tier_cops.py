#!/usr/bin/env python3
from __future__ import annotations
"""Classify diverging cops into tiers by FP+FN count.

Reads corpus-results.json and outputs a tiered list of cops suitable for
remote agent dispatch.

Usage:
    python3 scripts/agent/tier_cops.py                    # summary
    python3 scripts/agent/tier_cops.py --extended          # use extended corpus
    python3 scripts/agent/tier_cops.py --tier 1            # list only tier 1 cops
    python3 scripts/agent/tier_cops.py --tier 1 --names    # just cop names (for scripting)
    python3 scripts/agent/tier_cops.py --input results.json
"""

import argparse
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from corpus_download import download_corpus_results as _download_corpus

TIER_THRESHOLDS = {
    1: (1, 50),       # 1-50 FP+FN: easy, best for automated agents
    2: (51, 1000),    # 51-1000: medium, may need stronger model
    3: (1001, 999999),  # 1001+: hard, needs manual investigation
}


def main():
    parser = argparse.ArgumentParser(
        description="Classify diverging cops into tiers by FP+FN count")
    parser.add_argument("--input", type=Path,
                        help="Path to corpus-results.json")
    parser.add_argument("--extended", action="store_true",
                        help="Use extended corpus")
    parser.add_argument("--tier", type=int, choices=[1, 2, 3],
                        help="Show only this tier")
    parser.add_argument("--names", action="store_true",
                        help="Output just cop names (one per line, for scripting)")
    args = parser.parse_args()

    if args.input:
        input_path = args.input
    else:
        prefer = "extended" if args.extended else "standard"
        input_path, _, _ = _download_corpus(prefer=prefer)

    data = json.loads(input_path.read_text())
    by_cop = data.get("by_cop", [])

    # Collect diverging cops
    cops = []
    for entry in by_cop:
        fp = entry.get("fp", 0)
        fn = entry.get("fn", 0)
        total = fp + fn
        if total == 0:
            continue
        cops.append({
            "cop": entry["cop"],
            "fp": fp,
            "fn": fn,
            "total": total,
            "matches": entry.get("matches", 0),
            "match_rate": entry.get("match_rate", 0),
        })

    cops.sort(key=lambda c: c["total"])

    # Classify into tiers
    tiers = {1: [], 2: [], 3: []}
    for cop in cops:
        for tier, (lo, hi) in TIER_THRESHOLDS.items():
            if lo <= cop["total"] <= hi:
                tiers[tier].append(cop)
                break

    if args.names:
        target = tiers[args.tier] if args.tier else cops
        for c in target:
            print(c["cop"])
        return

    if args.tier:
        tier_cops = tiers[args.tier]
        lo, hi = TIER_THRESHOLDS[args.tier]
        print(f"Tier {args.tier} ({lo}-{hi} FP+FN): {len(tier_cops)} cops\n")
        print(f"{'Cop':<50} {'FP':>6} {'FN':>6} {'Total':>6} {'Match%':>7}")
        print(f"{'-'*50} {'-'*6} {'-'*6} {'-'*6} {'-'*7}")
        for c in tier_cops:
            pct = f"{c['match_rate']*100:.1f}%" if c['match_rate'] else "?"
            print(f"{c['cop']:<50} {c['fp']:>6} {c['fn']:>6} {c['total']:>6} {pct:>7}")
        return

    # Summary
    print(f"Total diverging cops: {len(cops)}\n")
    for tier in [1, 2, 3]:
        tier_cops = tiers[tier]
        lo, hi = TIER_THRESHOLDS[tier]
        total_fpfn = sum(c["total"] for c in tier_cops)
        print(f"Tier {tier} ({lo}-{hi} FP+FN): {len(tier_cops)} cops, "
              f"{total_fpfn:,} total FP+FN")

    print(f"\nTotal FP+FN: {sum(c['total'] for c in cops):,}")
    print(f"\nUse --tier N to see individual cops in a tier.")
    print(f"Use --tier N --names for scripting (one cop name per line).")


if __name__ == "__main__":
    main()
