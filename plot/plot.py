#!/usr/bin/env python3

import argparse

import pandas as pd
import matplotlib.pyplot as plt


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("csv", help="Path to the benchmark CSV")
    parser.add_argument(
        "--output",
        default="benchmark.png",
        help="Output image path (default: benchmark.png)",
    )
    parser.add_argument(
        "--log",
        action="store_true",
        help="Use a logarithmic Y axis",
    )
    args = parser.parse_args()

    df = pd.read_csv(
        args.csv,
        names=[
            "variant",
            "program",
            "engine",
            "duration",
            "unit",
        ],
    )

    # Convert nanoseconds to milliseconds for readability.
    df["duration_ms"] = df["duration"] / 1_000_000

    # Aggregate repeated runs.
    summary = (
        df.groupby(["program", "variant", "engine"])["duration_ms"]
        .agg(["mean", "std"])
        .reset_index()
    )

    # Create one figure per engine.
    engines = summary["engine"].unique()

    for engine in engines:
        engine_df = summary[summary["engine"] == engine]

        programs = engine_df["program"].unique()
        variants = engine_df["variant"].unique()

        fig, ax = plt.subplots(figsize=(12, 7))

        # Grouped bars:
        #   x = input program
        #   color/group = variant
        x = range(len(programs))
        width = 0.8 / len(variants)

        for i, variant in enumerate(variants):
            data = (
                engine_df[engine_df["variant"] == variant]
                .set_index("program")
                .reindex(programs)
            )

            positions = [
                p + (i - (len(variants) - 1) / 2) * width
                for p in x
            ]

            ax.bar(
                positions,
                data["mean"],
                width,
                yerr=data["std"],
                capsize=4,
                label=variant,
            )

        ax.set_xticks(list(x))
        ax.set_xticklabels(programs, rotation=45, ha="right")

        ax.set_xlabel("Input program")
        ax.set_ylabel("Execution time (ms)")
        ax.set_title(f"Benchmark results — {engine}")
        ax.legend(title="Variant")

        if args.log:
            ax.set_yscale("log")

        ax.grid(axis="y", alpha=0.3)

        fig.tight_layout()

        output = args.output

        # If there are multiple engines, append the engine name.
        if len(engines) > 1:
            from pathlib import Path

            path = Path(args.output)
            output = path.with_stem(
                f"{path.stem}_{engine}"
            )

        fig.savefig(output, dpi=200)
        plt.close(fig)

        print(f"Wrote {output}")


if __name__ == "__main__":
    main()