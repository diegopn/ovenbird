# Bundled LaTeX templates

The ZIP files in `vendor/` are the complete, unmodified packages distributed by
their upstream projects. Ovenbird copies and extracts these local archives when
creating a template project; project creation does not contact the network.

| Archive | Upstream package | License/source |
| --- | --- | --- |
| `abntex2.zip` | abnTeX2 1.9.7 | LPPL 1.3, [CTAN](https://ctan.org/pkg/abntex2) |
| `ieeetran.zip` | IEEEtran 1.8b | LPPL 1.3, [CTAN](https://ctan.org/pkg/ieeetran) |
| `acmart.zip` | acmart 2.20 | LPPL 1.3, [CTAN](https://ctan.org/pkg/acmart) |
| `elsarticle.zip` | elsarticle 3.4 | LPPL 1.3, [CTAN](https://ctan.org/pkg/elsarticle) |
| `springer-nature.zip` | Springer Nature journal template, December 2024 | [Official package](https://www.springernature.com/gp/authors/campaigns/latex-author-support); original archive retained |

The archives retain their original notices and documentation. `overlays/`
provides class/style files at the project root when their upstream archive keeps
them in a nested directory or supplies only the documented source (`.dtx`). It
also supplies Ovenbird starter documents for ABNT, IEEE, ACM, and Elsevier, with
an active bibliography and a local `references.bib` file. The Springer Nature
entry document is copied unchanged to the project root with its required class
and bibliography styles. Generated class files keep their upstream copyright
and LPPL notices.

SHA-256 checksums for the bundled upstream archives:

```text
abntex2.zip       2e4931c7336456083e10748bfb9b5cba855f81899ec84ea92d35a9aa04fc7f85
acmart.zip        93933ce58fbeffa13e23398bb523fcb68275ecc73369c6bc73b421eeef2c10de
elsarticle.zip    0b093093e84db49f99bcc9a7c3f69ed1fb61b0147c6d296427aff7963e7f50f6
ieeetran.zip      e0cd4f5afbd42c8076092280e72b3e0a5111efe501d35de9f715cfb8da313cb4
springer-nature.zip 812e76dcaa9c28dc1bff1fb6065d51729b67d4ea140552a05088317414a3ecae
```
