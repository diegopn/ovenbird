# Ovenbird icons

The editable source is `../io.github.diegopn.ovenbird.svg`: real vector
paths and gradients, without embedded raster images, external resources,
fonts, filters, or an exterior shadow. The opaque sage rounded-square base
fills the entire 128-unit canvas. Only its rounded corners are transparent;
there is no exterior padding. The bird was enlarged proportionally with
the base to preserve its original composition.

The PNGs in this directory are rendered directly from that SVG at 32, 48,
64, 128, 256, and 512 pixels. Regenerate them with librsvg or Inkscape after
editing the SVG; do not upscale a smaller PNG. The separate
`../io.github.diegopn.ovenbird-symbolic.svg` uses a simplified 16-unit,
single-color silhouette with transparent cutouts for the eye and writing.
GTK recolors this symbolic asset for the current interface context.

`../meson.build` installs the SVG to
`share/icons/hicolor/scalable/apps/`, the symbolic SVG to
`share/icons/hicolor/symbolic/apps/`, and each PNG to its corresponding
`share/icons/hicolor/SIZExSIZE/apps/` directory. The existing desktop
file's `Icon=io.github.diegopn.ovenbird` and MetaInfo `launchable` entry
identify the installed application icon; no additional MetaInfo icon
tag is needed for this desktop application.

Design: an ivory folded-paper bird with two document lines on a sage base.
The colors and silhouette are shared by the light and dark presentations.
The export kit also contains a 1024-pixel PNG for design use, which is not
installed by Meson, plus a contact sheet for visual inspection.

## Provenance

These assets were created with AI assistance in Codex on 2026-10-08:
ImageGen generated the selected visual concept and a transparent reference;
Codex recreated that concept as editable SVG paths, simplified the symbolic
version, rendered the PNGs with librsvg, and added the Meson installation
entries. The icon files follow the repository's GPL-3.0-or-later license.
This provenance concerns this icon change only, not other repository content.

Flathub's current requirements ask submitters to disclose included
AI-generated material. Its separate restrictions on Flathub manifests and
automated submission/review interactions also apply; this change does not
modify the Flatpak manifest or submit the application.

References:

- https://docs.flathub.org/docs/for-app-authors/metainfo-guidelines#icons
- https://docs.flathub.org/docs/for-app-authors/metainfo-guidelines/quality-guidelines#app-icon
- https://docs.flatpak.org/en/latest/conventions.html#application-icons
- https://developer.gnome.org/hig/guidelines/app-icons.html
- https://docs.flathub.org/docs/for-app-authors/requirements#generative-ai-policy
