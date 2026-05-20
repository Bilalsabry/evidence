# Paper — ACL LaTeX build

This directory contains the ACL-formatted LaTeX source for the paper
*Three Rules for a Citation: Decomposing Faithfulness in
Retrieval-Augmented Generation*. The content is ported faithfully from
`docs/paper/paper-draft.md` (sections 1, 3–7) and
`docs/paper/related-work.md` (section 2); every numbered result, CI,
`[OPEN]` marker, limitation bullet, the v1→v2 audit, and the
AI-assisted authoring disclosure are preserved verbatim.

## Files

- `main.tex` — camera-ready / non-anonymous driver. Loads
  `acl` with the `final` option and sets the author block to
  *Bilal Sabry, Krux AI*.
- `anon-main.tex` — anonymous-submission driver. Loads `acl` with
  `review` (line numbers + page numbers + anonymized header) and sets
  the author block to *Anonymous / Anonymous Institution*.
- `body.tex` — the abstract through the authoring disclosure. Shared
  between the two drivers; do not add `\documentclass` or
  `\begin{document}` here.
- `related.tex` — section 2 (Related Work), `\input` from `body.tex`.
- `refs.bib` — bibliography. Every `\cite{...}` in `body.tex` and
  `related.tex` resolves to an entry here.
- `acl.sty`, `acl_natbib.bst` — official ACL style files from
  [acl-org/acl-style-files](https://github.com/acl-org/acl-style-files),
  pinned to commit
  `2353f3ea58abf3fc4d2ee08e8d0cd2fe749f25a9`.

## Compiling locally (macOS)

A full TeX install is required (≈ 4 GB). Install once with:

```bash
brew install --cask mactex-no-gui
```

Then from this directory:

```bash
# Camera-ready (final, non-anonymous)
latexmk -pdf main.tex

# Anonymous submission (review mode: line numbers, anonymized header)
latexmk -pdf anon-main.tex
```

Outputs land at `main.pdf` and `anon-main.pdf`. To clean intermediate
files: `latexmk -C`.

If you prefer not to install LaTeX, see the Overleaf instructions
below.

## Compiling on Overleaf

1. Create a new blank project on https://www.overleaf.com.
2. Upload the contents of this `paper/` directory: `main.tex`,
   `anon-main.tex`, `body.tex`, `related.tex`, `refs.bib`, `acl.sty`,
   `acl_natbib.bst`, and this `README.md`.
3. In the Overleaf project menu, set the *Main document* to
   `main.tex` (camera-ready) or `anon-main.tex` (anonymous).
4. Make sure the compiler is set to `pdfLaTeX` and the TeX Live
   version is `2023` or newer.
5. Click *Recompile*. The PDF appears in the right-hand pane.

To switch between camera-ready and anonymous on Overleaf, just change
the main document setting — no source edits required.

## Style file provenance

`acl.sty` and `acl_natbib.bst` are copied verbatim from the upstream
[acl-org/acl-style-files](https://github.com/acl-org/acl-style-files)
repository at commit
`2353f3ea58abf3fc4d2ee08e8d0cd2fe749f25a9`. The upstream `README.md`
in that repository is the authoritative reference for which workshop
CFPs the style is current for; check it if submitting to a venue with
a custom style overlay.
