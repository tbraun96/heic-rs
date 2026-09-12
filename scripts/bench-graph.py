#!/usr/bin/env python3
"""Draw the README's performance charts from benches/results.json.

Every number the README shows lives in that one file. This script renders it as SVG -- once for a
light ground and once for a dark one, because GitHub serves whichever the reader's theme asks for --
and, with --check-readme, asserts that the tables in README.md still say what the JSON says. A chart
that disagrees with the table beside it is worse than no chart, so the check is the point.

    python3 scripts/bench-graph.py --write --check-readme
"""

import argparse
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
DATA = ROOT / "benches" / "results.json"
OUT = ROOT / "assets"

# CertifiedCopy brand tokens (assets/brand/tokens/brand.css in the certified.sh tree).
THEMES = {
    "light": dict(bg="#FFFFFF", edge="#E9E6F0", ink="#17141F", muted="#6E6880",
                  ours="#12B981", theirs="#CFCAD9", track="#F5F3F9", gold="#EFB338"),
    "dark": dict(bg="#0F0D14", edge="#272231", ink="#F4F2F9", muted="#9A94AE",
                 ours="#22C993", theirs="#3B3549", track="#17141F", gold="#EFB338"),
}
FONT = "ui-monospace, SFMono-Regular, 'IBM Plex Mono', Menlo, monospace"
SANS = "'IBM Plex Sans', -apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, sans-serif"

W, LEFT, BAR_X, BAR_W, ROW_H, HEAD = 900, 24, 250, 470, 76, 96


def esc(s):
    return s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


def px(m):
    return f"{m:.2f} Mpx" if m >= 1 else f"{m*1000:.0f} kpx"


def ms(v):
    return f"{v:.3f} ms" if v < 1 else f"{v:.2f} ms"


def chart(title, note, rows, legend, t, label_key="label"):
    """rows: [{label, sub, a, b, badge, badge_good}] where `a` is ours and `b` the comparison."""
    c = THEMES[t]
    h = HEAD + ROW_H * len(rows) + 20
    o = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{h}" '
         f'viewBox="0 0 {W} {h}" role="img" aria-label="{esc(title)}">',
         f'<rect x="0.5" y="0.5" width="{W-1}" height="{h-1}" rx="14" fill="{c["bg"]}" '
         f'stroke="{c["edge"]}"/>',
         f'<text x="{LEFT}" y="40" font-family="{SANS}" font-size="19" font-weight="600" '
         f'fill="{c["ink"]}">{esc(title)}</text>',
         f'<text x="{LEFT}" y="63" font-family="{SANS}" font-size="13" fill="{c["muted"]}">'
         f'{esc(note)}</text>']
    # legend
    lx = W - LEFT
    for name, col in reversed(legend):
        o.append(f'<text x="{lx}" y="41" font-family="{SANS}" font-size="12.5" '
                 f'fill="{c["muted"]}" text-anchor="end">{esc(name)}</text>')
        lx -= 8 + 7 * len(name)
        o.append(f'<rect x="{lx-11}" y="32" width="11" height="11" rx="3" fill="{c[col]}"/>')
        lx -= 24
    for i, r in enumerate(rows):
        y = HEAD + i * ROW_H
        peak = max(r["a"], r["b"]) or 1
        o.append(f'<text x="{LEFT}" y="{y+20}" font-family="{FONT}" font-size="13.5" '
                 f'font-weight="500" fill="{c["ink"]}">{esc(r[label_key])}</text>')
        o.append(f'<text x="{LEFT}" y="{y+38}" font-family="{SANS}" font-size="12" '
                 f'fill="{c["muted"]}">{esc(r["sub"])}</text>')
        for j, (val, col) in enumerate(((r["a"], "ours"), (r["b"], "theirs"))):
            by = y + 2 + j * 26
            o.append(f'<rect x="{BAR_X}" y="{by}" width="{BAR_W}" height="20" rx="5" '
                     f'fill="{c["track"]}"/>')
            w = max(3, round(BAR_W * val / peak))
            o.append(f'<rect x="{BAR_X}" y="{by}" width="{w}" height="20" rx="5" fill="{c[col]}"/>')
            inside = c["bg"] if col == "ours" else c["ink"]
            tx, anchor, fill = (BAR_X + w - 9, "end", inside) if w > 96 else (
                BAR_X + w + 9, "start", c["muted"])
            o.append(f'<text x="{tx}" y="{by+14}" font-family="{FONT}" font-size="12.5" '
                     f'font-weight="600" fill="{fill}" text-anchor="{anchor}">{ms(val)}</text>')
        good = r["badge_good"]
        col = c["ours"] if good else c["gold"]
        o.append(f'<text x="{W-LEFT}" y="{y+30}" font-family="{FONT}" font-size="17" '
                 f'font-weight="700" fill="{col}" text-anchor="end">{esc(r["badge"])}</text>')
    o.append("</svg>")
    return "\n".join(o) + "\n"


def build(d, t):
    dec = [dict(label=r["file"], sub=r["pixels"] + "  ·  " + px(r["mpx"]),
                a=r["ours_ms"], b=r["theirs_ms"],
                badge=(f'{r["theirs_ms"]/r["ours_ms"]:.2f}× faster'
                       if r["theirs_ms"] >= r["ours_ms"]
                       else f'{r["ours_ms"]/r["theirs_ms"]:.2f}× slower'),
                badge_good=r["theirs_ms"] >= r["ours_ms"]) for r in d["decode"]]
    par = [dict(label=r["file"], sub=f'{r["tiles"]} tile' + ("s" if r["tiles"] != 1 else ""),
                a=r["pooled_ms"], b=r["serial_ms"],
                badge=f'{r["serial_ms"]/r["pooled_ms"]:.1f}×', badge_good=True)
           for r in d["parallel"]]
    col = [dict(label=r["case"], sub="fused upsample + fixed-point matrix",
                a=r["after_ms"], b=r["before_ms"],
                badge=f'{r["before_ms"]/r["after_ms"]:.1f}×', badge_good=True)
           for r in d["color"]]
    return {
        "bench-decode": chart("Decode: whole file in, RGB8 out", d["method"], dec,
                              [("heic-rs", "ours"), ("heic (AGPL-3.0)", "theirs")], t),
        "bench-parallel": chart("What the thread pool bought", "DecodeOptions::threads = Some(1) "
                                "against rayon's default pool. Bars are scaled per row.", par,
                                [("pooled", "ours"), ("one thread", "theirs")], t),
        "bench-color": chart("Colour conversion, one thread", "YUV 4:2:0 to interleaved RGB, "
                             "before and after the fused fixed-point rewrite.", col,
                             [("after", "ours"), ("before", "theirs")], t),
    }


def check_readme(d, text):
    """Every number in the JSON must appear in the README's tables, and vice versa."""
    bad = []
    for r in d["decode"]:
        row = next((ln for ln in text.splitlines()
                    if ln.startswith("| `" + r["file"] + "`")), None)
        if row is None:
            bad.append(f'{r["file"]}: no row in the decode table')
            continue
        for want in (ms(r["ours_ms"]).replace(" ms", ""), ms(r["theirs_ms"]).replace(" ms", "")):
            if want not in row:
                bad.append(f'{r["file"]}: README row does not say {want}')
    for r in d["parallel"]:
        if f'{r["serial_ms"]:.2f} ms' not in text and f'{r["serial_ms"]:.3f} ms' not in text:
            bad.append(f'{r["file"]}: serial {r["serial_ms"]} ms missing from README')
    return bad


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--write", action="store_true", help="write assets/*.svg")
    ap.add_argument("--check-readme", action="store_true")
    a = ap.parse_args()
    d = json.loads(DATA.read_text())
    n = 0
    for t in ("light", "dark"):
        for name, svg in build(d, t).items():
            p = OUT / (f"{name}.svg" if t == "light" else f"{name}-dark.svg")
            if a.write:
                p.write_text(svg)
                n += 1
            print(("wrote " if a.write else "would write ") + str(p.relative_to(ROOT)))
    if a.check_readme:
        bad = check_readme(d, (ROOT / "README.md").read_text())
        for b in bad:
            print("MISMATCH: " + b, file=sys.stderr)
        if bad:
            sys.exit(1)
        print("README tables agree with benches/results.json")
    return n


if __name__ == "__main__":
    main()
