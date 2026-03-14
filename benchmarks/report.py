#!/usr/bin/env python3
"""Generate benchmark report from results.csv."""

import csv
import os
import sys

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
CSV_PATH = os.path.join(SCRIPT_DIR, "results.csv")
README_PATH = os.path.join(SCRIPT_DIR, "README.md")
CHARTS_DIR = os.path.join(SCRIPT_DIR, "charts")

LANG_ORDER = ["sans", "python", "go", "node", "rust"]
LANG_DISPLAY = {"sans": "Sans", "python": "Python", "go": "Go", "node": "Node.js", "rust": "Rust"}


def read_results():
    """Read CSV into {benchmark: {language: {mean, stddev, min, max}}}."""
    results = {}
    with open(CSV_PATH) as f:
        reader = csv.DictReader(f)
        for row in reader:
            bench = row["benchmark"]
            lang = row["language"]
            if bench not in results:
                results[bench] = {}
            results[bench][lang] = {
                "mean": float(row["mean_ms"]),
                "stddev": float(row["stddev_ms"]),
                "min": float(row["min_ms"]),
                "max": float(row["max_ms"]),
            }
    return results


def generate_markdown(results):
    """Generate README.md with results tables."""
    langs = [l for l in LANG_ORDER if any(l in results[b] for b in results)]
    benchmarks = list(results.keys())

    lines = []
    lines.append("# Sans Benchmark Results\n")
    lines.append("Comparing Sans against Python, Go, Node.js, and Rust across 5 workloads.\n")
    lines.append("All times in milliseconds (lower is better). Speedup relative to Python.\n")

    # Main results table
    lines.append("## Results\n")
    header = "| Benchmark |"
    sep = "|-----------|"
    for lang in langs:
        header += f" {LANG_DISPLAY[lang]} (ms) |"
        sep += "---:|"
    lines.append(header)
    lines.append(sep)

    for bench in benchmarks:
        row = f"| {bench} |"
        for lang in langs:
            if lang in results[bench]:
                mean = results[bench][lang]["mean"]
                row += f" {mean:.1f} |"
            else:
                row += " - |"
        lines.append(row)

    lines.append("")

    # Speedup table (relative to Python)
    lines.append("## Speedup vs Python\n")
    lines.append("| Benchmark |" + "".join(f" {LANG_DISPLAY[l]} |" for l in langs))
    lines.append("|-----------|" + "".join("---:|" for _ in langs))

    for bench in benchmarks:
        row = f"| {bench} |"
        python_mean = results[bench].get("python", {}).get("mean")
        for lang in langs:
            if lang in results[bench] and python_mean:
                mean = results[bench][lang]["mean"]
                if mean > 0:
                    speedup = python_mean / mean
                    row += f" {speedup:.1f}x |"
                else:
                    row += " - |"
            else:
                row += " - |"
        lines.append(row)

    lines.append("")

    # Per-benchmark detail
    lines.append("## Details\n")
    for bench in benchmarks:
        lines.append(f"### {bench}\n")
        lines.append("| Language | Mean (ms) | Std Dev | Min | Max |")
        lines.append("|----------|--------:|--------:|----:|----:|")
        for lang in langs:
            if lang in results[bench]:
                d = results[bench][lang]
                lines.append(
                    f"| {LANG_DISPLAY[lang]} | {d['mean']:.1f} | {d['stddev']:.1f} | {d['min']:.1f} | {d['max']:.1f} |"
                )
        lines.append("")

    # Methodology
    lines.append("## Methodology\n")
    lines.append("- Each benchmark runs 10 times (1 warmup + 10 timed)")
    lines.append("- Wall-clock time measured via Python `time.perf_counter()`")
    lines.append("- Sans, Go, and Rust are compiled ahead of time with optimizations")
    lines.append("- Python uses CPython, Node.js uses V8")
    lines.append("- All programs produce identical output for correctness verification")
    lines.append("")

    # Benchmarks description
    lines.append("## Workloads\n")
    lines.append("| Benchmark | Description |")
    lines.append("|-----------|-------------|")
    lines.append("| fib | Recursive fibonacci(35), no memoization |")
    lines.append("| loop_sum | Sum integers 1 to 1,000,000 |")
    lines.append("| array_ops | Build 100k array, map (*2), filter (even), sum |")
    lines.append("| string_concat | Concatenate 5 strings per iteration, 100k iterations |")
    lines.append("| json_roundtrip | Build/stringify/parse JSON with 1k keys, 100 iterations |")
    lines.append("")

    return "\n".join(lines)


def generate_charts(results):
    """Generate bar charts if matplotlib available."""
    try:
        import matplotlib
        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
    except ImportError:
        print("matplotlib not available, skipping charts")
        return

    os.makedirs(CHARTS_DIR, exist_ok=True)
    langs = [l for l in LANG_ORDER if any(l in results[b] for b in results)]
    colors = {"sans": "#FF6B35", "python": "#3776AB", "go": "#00ADD8", "node": "#339933", "rust": "#DEA584"}

    for bench in results:
        fig, ax = plt.subplots(figsize=(8, 4))
        lang_names = []
        means = []
        errs = []
        bar_colors = []

        for lang in langs:
            if lang in results[bench]:
                lang_names.append(LANG_DISPLAY[lang])
                means.append(results[bench][lang]["mean"])
                errs.append(results[bench][lang]["stddev"])
                bar_colors.append(colors.get(lang, "#888"))

        bars = ax.barh(lang_names, means, xerr=errs, color=bar_colors, height=0.6)
        ax.set_xlabel("Time (ms)")
        ax.set_title(f"{bench}")
        ax.invert_yaxis()
        plt.tight_layout()
        plt.savefig(os.path.join(CHARTS_DIR, f"{bench}.png"), dpi=150)
        plt.close()

    print(f"Charts saved to {CHARTS_DIR}/")


def main():
    if not os.path.exists(CSV_PATH):
        print(f"Error: {CSV_PATH} not found. Run bench.sh first.")
        sys.exit(1)

    results = read_results()
    md = generate_markdown(results)

    with open(README_PATH, "w") as f:
        f.write(md)
    print(f"Report written to {README_PATH}")

    generate_charts(results)


if __name__ == "__main__":
    main()
