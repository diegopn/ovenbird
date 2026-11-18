# Bundled LaTeX templates

Each ZIP in `vendor/` contains the complete upstream template package together
with the files Ovenbird uses to start a project. The upstream package contents,
documentation, and license notices are retained. Project creation extracts the
local ZIP and does not contact the network.

| Archive | Upstream package | License/source |
| --- | --- | --- |
| `abntex2.zip` | abnTeX2 1.9.7 | LPPL 1.3, [CTAN](https://ctan.org/pkg/abntex2) |
| `ieeetran.zip` | IEEEtran 1.8b | LPPL 1.3, [CTAN](https://ctan.org/pkg/ieeetran) |
| `acmart.zip` | acmart 2.20 | LPPL 1.3, [CTAN](https://ctan.org/pkg/acmart) |
| `elsarticle.zip` | elsarticle 3.4 | LPPL 1.3, [CTAN](https://ctan.org/pkg/elsarticle) |
| `springer-nature.zip` | Springer Nature journal template, December 2024 | [Official package](https://www.springernature.com/gp/authors/campaigns/latex-author-support) |

The ZIPs include the Ovenbird starter documents and example bibliographies at
the project root. Class and bibliography-style files supplied in nested package
directories are also present at the root so the starter document can use them
without extra path configuration. The ACM and Elsevier bundles include their
generated class files. The Springer Nature bundle includes its sample figure.
Generated class files retain their upstream copyright and LPPL notices.

SHA-256 checksums of the bundled ZIP files:

```text
abntex2.zip       247ab346ceefa489a77e0f82e800ba6fa8e0149bcf88ad530bd17f6db202f2ab
acmart.zip        3fcd3161b57ff0c54088a92902e5b2ce6100023e3c9f60a2618e2c53fa9a252a
elsarticle.zip    2254e9b2f208e3fb5c643b00a8895ab973585fa1e22190f000eec9c2d49d427f
ieeetran.zip      142d24e29348275f7a2e8efd1bf448eebafca91d1bcbc682657a95e0e2d318c8
springer-nature.zip ee6c213e7240ee757a85e1a07adc305dd0b16d116566397d275570aaa91d0ff5
```
