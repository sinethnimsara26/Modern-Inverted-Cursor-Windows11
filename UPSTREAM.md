# Upstream and licensing notes

The cursor artwork/assets used by the installer come from:

**Modern Inverted Mouse Cursors** by `emvaized`
https://github.com/emvaized/modern_inverted_mouse_cursors

Pinned upstream version: **v3.6.2**

The final Rust installer does **not** contain or redistribute the author's Patreon download package. It downloads the public source cursor assets from the pinned GitHub tag at runtime.

The installer verifies those public files using the exact byte sizes and Git blob SHA-1 identifiers recorded for the v3.6.2 tree before applying them.

The upstream repository states that its source is MIT licensed. Refer to the upstream repository for the authoritative license text and attribution requirements.

The installer source in this folder was created independently during troubleshooting on 2026-10-03.
