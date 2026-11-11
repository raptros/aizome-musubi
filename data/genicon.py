"""Draw Musubi's knot on Aizome's 64-unit app tile.

The glyph uses two interleaved square ribbons on the 2-unit grid. The tile
matches Sekka's genicons.py: kon face, one-unit bevel, kachi edge and hard
four-unit tile shadow. The knot itself has a hard three-unit dye shadow.

    python3 data/genicon.py
"""

from pathlib import Path

PALE, MID, DEEP = "#a9c4ff", "#7aa2f7", "#4a67b3"
AMBER, KON, KACHI = "#e0af68", "#24283b", "#0b0c12"


def rect(x, y, width, height, fill, extra=""):
    return f'<rect x="{x}" y="{y}" width="{width}" height="{height}" fill="{fill}"{extra}/>'


def ribbon(horizontal, fill):
    if horizontal:
        parts = ((12, 20, 36, 6), (12, 34, 36, 6), (12, 20, 6, 20), (42, 20, 6, 20))
    else:
        parts = ((22, 12, 6, 36), (36, 12, 6, 36), (22, 12, 20, 6), (22, 42, 20, 6))
    return "".join(rect(*part, fill) for part in parts)


def icon():
    knot = (
        ribbon(True, PALE)
        + ribbon(False, MID)
        # Alternate the overpasses: the pale ribbon passes above the blue
        # at the upper-left and lower-right crossings.
        + rect(22, 20, 6, 6, PALE)
        + rect(36, 34, 6, 6, PALE)
        + rect(29, 29, 6, 6, AMBER)
    )
    shadow = ribbon(True, DEEP) + ribbon(False, DEEP)
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" width="64" height="64" '
        'role="img" aria-label="Musubi knot">'
        '<title>Musubi knot</title>'
        + rect(4, 4, 60, 60, KACHI)
        + rect(0, 0, 60, 60, KON)
        + rect(1, 1, 58, 1, "#ffffff", ' fill-opacity=".08"')
        + '<rect x=".5" y=".5" width="59" height="59" fill="none" stroke="' + KACHI + '"/>'
        + '<g transform="translate(3 3)">' + shadow + '</g>'
        + knot
        + '</svg>\n'
    )


if __name__ == "__main__":
    Path(__file__).with_name("org.aizome.Musubi.svg").write_text(icon())
