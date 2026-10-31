# Server Pack & SMP Fixer (`server_pack_fixer`)

A native Horizon mod (`horizon-mixins` bundle) by **CayMac** that fixes multi-pack SMP server compatibility in Horizon (such as **HugoSMP** / `hugosmp.net`, ItemsAdder, Oraxen, and Nexo servers):

1. **Obfuscated PNG & Bad CRC Fixer (Fast Loading & No Missing Glyphs)**:
   - Configures the PNG texture and font decoder to ignore checksums (`ignore_checksums(true)`), exactly matching vanilla Java Minecraft behavior.
   - Prevents tens of thousands of `font sheet ... could not be read` (`CrcMismatch`) warnings on obfuscated server packs (e.g. HugoSMP, ItemsAdder, Oraxen).
   - Eliminates the 40–90s "Loading assets" freeze/timeout when joining servers and fixes missing character boxes (`[] []`) in pause menus, scoreboards, bossbars, and holograms.
   - Automatically sanitizes and repairs corrupted chunk CRCs in the disk cache overlay.
2. **Reliable Multi-Pack Server Downloads & Cache**:
   - Retries failed/stalled HTTP resource pack downloads up to 3 times with explicit timeouts and browser-friendly fallback headers.
   - Expands the server resource pack cache limit from `20` to `256` files so multi-pack servers never evict their own active packs on startup.
   - Sanitizes quirky `pack.mcmeta` files (UTF-8 BOMs, trailing commas, or missing root metadata) so one sub-pack never aborts the entire server pack stack.
3. **1.21+ Pack Overlay & Legacy `custom_model_data` Bridge**:
   - Activates modern 1.21+ pack overlay directories (`format >= 34`) when a server pack specifies a `max_format` below Horizon's `75`.
   - Automatically bridges legacy `assets/minecraft/models/item/*.json` `overrides` (`custom_model_data`) into 1.21.4+ `assets/minecraft/items/*.json` `range_dispatch` definitions.
4. **Custom Pause / Options Buttons & Font Glyph Region**:
   - Stretches custom GUI button sprites that omit `.png.mcmeta` instead of tiling them with a 3px border (`GuiSpriteScaling.DEFAULT`).
   - Preserves Private Use Area (`\u{E000}..=\u{F8FF}`) and negative-space banner glyphs in button labels without character-clipping.
   - Wraps the dynamic font glyph atlas region cleanly instead of permanently returning missing-glyph boxes when full.
5. **Leaderboard Holograms (`TextDisplay`)**:
   - Invalidates cached `TextDisplay` line splits whenever a server pack finishes loading or the font epoch advances.
   - Resolves `Translate` spans before line splitting and prevents mid-line breaks on PUA / negative-space hologram banners.
   - Applies a subtle per-glyph depth step in 3D world text to eliminate Z-fighting between background banners and overlaid text, and resolves inline player head skins on holograms.
6. **Multiplayer Chunk Hole Fixer**:
   - Accepts server chunks arriving before `SetChunkCacheCenter` or near the player during spawn/teleportation.
   - Meshes lit remote server chunks immediately even when 1 of their 8 neighbour columns has not been sent by the server yet, and automatically remeshes borders as neighbours arrive.
