#!/usr/bin/env python3
"""Writes the feature templates the chunker experiments compare.

Usage:
    python3 -I -B make_templates.py <output directory>

cabocha.template  The template of CaboCha's chunker: column 0 is the surface
                  and column 1 the part of speech.
extra.template    That, and 13 lines for a third column, as the template of
                  CaboCha for named entities has them: the column alone two
                  morphemes to each side, pairs and triples of neighbours,
                  and the pair with the part of speech.
light.template    CaboCha's, and 5 lines for a third column: alone for the
                  morpheme, the one before and the one after, and the pair
                  with the part of speech for the morpheme and the one
                  before. The default template of lindera-dependency.
both.template     extra.template, and 4 lines for a fourth column.

The candidates of #1464, each light.template with the combinations J.DepP's
chunker has and the template lacks: two kinds of column across the place
where a bunsetsu may start, before the morpheme the label is for.

cross-pos.template   The surface before with the part of speech of the
                     morpheme, and the part of speech before with its
                     surface.
cross-form.template  The conjugation form before with the part of speech
                     and with the surface of the morpheme.
cross.template       Both, and the conjugation form two morphemes before.
"""

import os
import sys

CABOCHA = """U00:%x[-2,0]
U01:%x[-1,0]
U02:%x[0,0]
U03:%x[1,0]
U04:%x[2,0]
U05:%x[-1,0]/%x[0,0]
U06:%x[0,0]/%x[1,0]
U10:%x[-2,1]
U11:%x[-1,1]
U12:%x[0,1]
U13:%x[1,1]
U14:%x[2,1]
U15:%x[-2,1]/%x[-1,1]
U16:%x[-1,1]/%x[0,1]
U17:%x[0,1]/%x[1,1]
U18:%x[1,1]/%x[2,1]
U20:%x[-2,1]/%x[-1,1]/%x[0,1]
U21:%x[-1,1]/%x[0,1]/%x[1,1]
U22:%x[0,1]/%x[1,1]/%x[2,1]
"""

EXTRA = """U30:%x[-2,2]
U31:%x[-1,2]
U32:%x[0,2]
U33:%x[1,2]
U34:%x[2,2]
U35:%x[-2,2]/%x[-1,2]
U36:%x[-1,2]/%x[0,2]
U37:%x[0,2]/%x[1,2]
U38:%x[1,2]/%x[2,2]
U40:%x[-2,2]/%x[-1,2]/%x[0,2]
U41:%x[-1,2]/%x[0,2]/%x[1,2]
U42:%x[0,2]/%x[1,2]/%x[2,2]
U50:%x[0,1]/%x[0,2]
"""

LIGHT = """U31:%x[-1,2]
U32:%x[0,2]
U33:%x[1,2]
U50:%x[0,1]/%x[0,2]
U51:%x[-1,1]/%x[-1,2]
"""

FOURTH = """U61:%x[-1,3]
U62:%x[0,3]
U63:%x[1,3]
U70:%x[0,1]/%x[0,3]
"""

CROSS_POS = """U80:%x[-1,0]/%x[0,1]
U81:%x[-1,1]/%x[0,0]
"""

CROSS_FORM = """U82:%x[-1,2]/%x[0,1]
U83:%x[-1,2]/%x[0,0]
"""

TWO_BEFORE = """U30:%x[-2,2]
"""

TEMPLATES = [
    ("cabocha", CABOCHA),
    ("extra", CABOCHA + EXTRA),
    ("light", CABOCHA + LIGHT),
    ("both", CABOCHA + EXTRA + FOURTH),
    ("cross-pos", CABOCHA + LIGHT + CROSS_POS),
    ("cross-form", CABOCHA + LIGHT + CROSS_FORM),
    ("cross", CABOCHA + LIGHT + CROSS_POS + CROSS_FORM + TWO_BEFORE),
]

out = sys.argv[1]
os.makedirs(out, exist_ok=True)
for name, text in TEMPLATES:
    with open(os.path.join(out, f"{name}.template"), "w", encoding="utf-8") as f:
        f.write(text + "B\n")
