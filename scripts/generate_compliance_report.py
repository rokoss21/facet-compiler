#!/usr/bin/env python3
"""Generate a markdown compliance summary from the v2.1.3 migration checklist."""

from __future__ import annotations

import argparse
import datetime as dt
import pathlib
import re
import subprocess
import sys
from collections import OrderedDict

CHECKBOX_RE = re.compile(r"^\s*-\s+\[(?P<state>[xX ])\]\s+(?P<text>.+?)\s*$")
SECTION_RE = re.compile(r"^###\s+(?P<title>.+?)\s*$")


def git_value(args: list[str], default: str = "unknown") -> str:
    try:
        return (
            subprocess.check_output(["git", *args], text=True, stderr=subprocess.DEVNULL)
            .strip()
            or default
        )
    except Exception:
        return default


def parse_checklist(checklist_path: pathlib.Path) -> tuple[OrderedDict[str, dict], list[tuple[str, str]]]:
    text = checklist_path.read_text(encoding="utf-8")
    lines = text.splitlines()

    in_spec_section = False
    current_subsection = "Uncategorized"
    sections: OrderedDict[str, dict] = OrderedDict()
    open_items: list[tuple[str, str]] = []

    for line in lines:
        if line.startswith("## 3) Spec Compliance Checklist"):
            in_spec_section = True
            continue
        if in_spec_section and line.startswith("## 4)"):
            break
        if not in_spec_section:
            continue

        section_match = SECTION_RE.match(line)
        if section_match:
            current_subsection = section_match.group("title")
            sections.setdefault(current_subsection, {"done": 0, "total": 0})
            continue

        item_match = CHECKBOX_RE.match(line)
        if not item_match:
            continue

        sections.setdefault(current_subsection, {"done": 0, "total": 0})
        sections[current_subsection]["total"] += 1

        is_done = item_match.group("state").lower() == "x"
        if is_done:
            sections[current_subsection]["done"] += 1
        else:
            open_items.append((current_subsection, item_match.group("text")))

    return sections, open_items


def pct(done: int, total: int) -> float:
    return 100.0 if total == 0 else (done / total) * 100.0


def render_markdown(
    checklist_path: pathlib.Path,
    sections: OrderedDict[str, dict],
    open_items: list[tuple[str, str]],
) -> str:
    total_done = sum(v["done"] for v in sections.values())
    total_all = sum(v["total"] for v in sections.values())
    total_open = total_all - total_done

    now = dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%d %H:%M:%SZ")
    commit = git_value(["rev-parse", "--short", "HEAD"])
    branch = git_value(["rev-parse", "--abbrev-ref", "HEAD"])

    lines: list[str] = []
    lines.append("# FACET v2.1.3 Compliance Report")
    lines.append("")
    lines.append(f"- Generated (UTC): `{now}`")
    lines.append(f"- Commit: `{commit}`")
    lines.append(f"- Branch: `{branch}`")
    lines.append(f"- Checklist: `{checklist_path.as_posix()}`")
    lines.append("")
    lines.append("## Summary")
    lines.append("")
    lines.append(f"- Completed: `{total_done}`")
    lines.append(f"- Open: `{total_open}`")
    lines.append(f"- Total: `{total_all}`")
    lines.append(f"- Completion: `{pct(total_done, total_all):.2f}%`")
    lines.append("")
    lines.append("## By Section")
    lines.append("")
    lines.append("| Section | Done | Total | Completion |")
    lines.append("|---|---:|---:|---:|")
    for title, stat in sections.items():
        lines.append(
            f"| {title} | {stat['done']} | {stat['total']} | {pct(stat['done'], stat['total']):.2f}% |"
        )
    lines.append("")
    lines.append("## Open Items")
    lines.append("")
    if open_items:
        for section, item in open_items:
            lines.append(f"- [{section}] {item}")
    else:
        lines.append("- None")
    lines.append("")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--checklist",
        default="docs/14-v2.1.3-migration-checklist.md",
        help="Path to migration checklist markdown.",
    )
    parser.add_argument(
        "--output",
        default="compliance-report.md",
        help="Output markdown file path.",
    )
    args = parser.parse_args()

    checklist_path = pathlib.Path(args.checklist)
    if not checklist_path.exists():
        print(f"Checklist not found: {checklist_path}", file=sys.stderr)
        return 1

    sections, open_items = parse_checklist(checklist_path)
    if not sections:
        print("No compliance sections parsed from checklist.", file=sys.stderr)
        return 1

    output = render_markdown(checklist_path, sections, open_items)
    out_path = pathlib.Path(args.output)
    out_path.write_text(output, encoding="utf-8")
    print(f"Wrote compliance report: {out_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
