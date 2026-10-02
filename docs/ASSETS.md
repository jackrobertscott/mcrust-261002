# Asset specification

All art is generated in Rust code (no files are loaded at runtime, zero crates).
Each module in `src/assets/` exposes:

```rust
pub const NAMES: &[&str];                  // every asset this module provides
pub fn get(name: &str) -> Option<Image>;  // build the image for `name`
```

`Image` (src/image.rs) is RGBA8, top row first. Helpers available in `crate::image`:
`Image::new/filled/get/set/put/fill_rect/blit/blend/scaled/save_png`, `Rng` (deterministic),
`rgb(0xRRGGBB)`.

Preview everything with `cargo run -- --dump-assets asset_dump` (writes PNGs).

Style goal: be as faithful as possible to the look of vanilla Minecraft (Java Edition 1.14+
"Programmer Art"-era / modern default textures are both fine, but be consistent within a
module). Pixel art only: 16x16 for blocks/items, hard pixels, limited palettes, no blur.
Hand-authoring pixel grids as string literals (one char per pixel + palette) is encouraged
for anything iconic (tools, food, ores, torch, flowers, wheat, font glyphs, logo). Noise-based
generation with a palette is fine for stone/dirt/sand/gravel etc.

## blocks (src/assets/blocks.rs) - all 16x16
Tinted textures (marked *) must be GRAYSCALE (the engine multiplies by a biome colour);
everything else is in final colour.

stone, dirt, grass_top*, grass_side_overlay* (grayscale grass fringe on the top ~3-4 rows
with drips, fully transparent elsewhere - drawn over `dirt`), grass_side_snowed, snow, ice
(semi-transparent alpha ~160-200), sand, sandstone_top, sandstone_side, sandstone_bottom,
gravel, clay, bedrock, cobblestone, mossy_cobblestone, bricks, obsidian,
oak_log, oak_log_top, oak_planks, oak_leaves*,
birch_log, birch_log_top, birch_planks, birch_leaves*,
spruce_log, spruce_log_top, spruce_planks, spruce_leaves*,
jungle_log, jungle_log_top, jungle_planks, jungle_leaves*,
acacia_log, acacia_log_top, acacia_planks, acacia_leaves*,
(leaves: opaque leaf pixels + fully transparent holes, "fancy" style)
coal_ore, iron_ore, gold_ore, diamond_ore, redstone_ore, lapis_ore,
coal_block, iron_block, gold_block, diamond_block,
glass (transparent centre, frame + a couple of glint streaks),
crafting_table_top, crafting_table_side, crafting_table_front,
furnace_front, furnace_front_on, furnace_side, furnace_top,
chest_front, chest_side, chest_top,
torch (the vanilla 16x16 torch texture: 2px wide stick centred at x=7..8, rows 6..15, flame/
glowing tip at the top rows 6..7(+), transparent elsewhere),
water_still (blue, alpha ~180), lava_still (opaque orange/yellow),
farmland, farmland_moist,
wheat_stage0 .. wheat_stage7 (8 textures, transparent background, cross-plant style; stage7
is ripe golden wheat),
short_grass*, fern*, dandelion, poppy, blue_orchid, dead_bush, sugar_cane* (sugar cane is
tinted in vanilla; make it grayscale-ish light green... actually: provide it in final green
colour, NOT tinted), cactus_side, cactus_top, cactus_bottom,
white_wool, bookshelf, pumpkin_side, pumpkin_top, tnt_side, tnt_top, tnt_bottom,
destroy_stage_0 .. destroy_stage_9 (crack overlays: dark gray/black crack pixels with alpha,
transparent elsewhere; progressively more cracks).

(sugar_cane is NOT tinted: give it final colours.)

## items (src/assets/items.rs) - all 16x16, transparent background
Inventory icon style with dark outlines exactly like vanilla item sprites.

stick, coal, charcoal, iron_ingot, gold_ingot, diamond, wheat_seeds, wheat, bread, apple,
porkchop, cooked_porkchop, beef, cooked_beef, chicken, cooked_chicken, rotten_flesh,
leather, feather, bone, string, gunpowder, flint, arrow, bow, bowl, mushroom_stew, sugar,
egg, bucket, water_bucket, lava_bucket, paper,
and for each material in [wooden, stone, iron, golden, diamond] and each tool in
[pickaxe, axe, shovel, sword, hoe]: `<material>_<tool>` e.g. wooden_pickaxe, golden_hoe
(25 tools; vanilla diagonal orientation: handle bottom-left, head top-right).

## mobs (src/assets/mobs.rs)
Entity skins using vanilla Java model UV layouts. Box unwrap rule for a box with texture
offset (u,v) and size (w,h,d) (w=x size, h=y size, d=z size):
- top    : x=u+d,       y=v,   size w x d
- bottom : x=u+d+w,     y=v,   size w x d
- right  : x=u,         y=v+d, size d x h   (entity's right side)
- front  : x=u+d,       y=v+d, size w x h   (the face / front)
- left   : x=u+d+w,     y=v+d, size d x h
- back   : x=u+d+w+d,   y=v+d, size w x h
Unused texture area should be transparent.

Skins and boxes (u,v : w x h x d):
- `pig` 64x32: head (0,0: 8x8x8), snout (16,16: 4x3x1), body (28,8: 10x16x8), leg (0,16: 4x6x4)
  NOTE quadruped bodies are modelled standing up then rotated 90deg: in the body box, the
  "front" face (w x h = 10x16) is the BACK/top of the animal (the spine side), "top" face (w x d)
  is the animal's rear end... just paint the body box all pink with subtle shading; the
  unwrapped "front" (10x16) is the animal's back, "back" (10x16) is its belly.
- `cow` 64x32: head (0,0: 8x8x6), horn (22,0: 1x3x1), body (18,4: 12x18x10), udder (52,0: 4x6x1),
  leg (0,16: 4x12x4). Black and white patches, pink udder, face with eyes, light nose.
- `sheep` 64x32 (shorn skin): head (0,0: 6x6x8), body (28,8: 8x16x6), leg (0,16: 4x12x4)
- `sheep_fur` 64x32 (wool layer, white/light gray): head (0,0: 6x6x6), body (28,8: 8x16x6), leg (0,16: 4x6x4)
- `chicken` 64x32: head (0,0: 4x6x3), beak (14,0: 4x2x2), wattle/red chin (14,4: 2x2x2),
  body (0,9: 6x8x6), leg (26,0: 3x5x3), wing (24,13: 1x4x6)
- `zombie` 64x64: head (0,0: 8x8x8), body (16,16: 8x12x4), right arm (40,16: 4x12x4),
  right leg (0,16: 4x12x4), left arm (32,48: 4x12x4), left leg (16,48: 4x12x4)
- `steve` 64x64: same layout as zombie (classic Steve: brown hair, cyan shirt, blue pants)
- `skeleton` 64x32: head (0,0: 8x8x8), body (16,16: 8x12x4), arm (40,16: 2x12x2), leg (0,16: 2x12x2)
- `creeper` 64x32: head (0,0: 8x8x8), body (16,16: 8x12x4), leg (0,16: 4x6x4)
- `spider` 64x32: head (32,4: 8x8x8), neck (0,0: 6x6x6), body (0,12: 10x8x12), leg (18,0: 16x2x2)
Also environment textures in this module:
- `sun` 32x32 (bright yellow-white square with glow, on transparent/black)
- `moon` 32x32 (full moon, gray craters, transparent bg)
- `clouds` 256x256 (white opaque pixels for cloud, fully transparent elsewhere; blocky clusters
  like vanilla clouds.png - clouds cover ~30-40%)

## gui (src/assets/gui.rs)
- `font` 128x128: ASCII bitmap font, 16x16 grid of 8x8 cells, glyph for char code c at
  cell (c%16, c/16). White (255,255,255,255) glyph pixels on transparent. Recreate the
  vanilla Minecraft default font (ascii.png) as faithfully as possible (glyph heights 7px with
  baseline at row 7, descenders for g j p q y into row 8? no - vanilla fits in 8 rows). Also
  provide `pub fn glyph_width(c: u8) -> u32` = advance width in pixels INCLUDING 1px spacing
  (vanilla: most chars 6, 'i' '!' '.' ',' ':' ';' '|' 2, 'l' 3, 't' 'I' '[' ']' 4, space 4,
  'f' 'k' '<' '>' '{' '}' 5, '@' '~' 7). Glyph pixels start at x=0 in the cell.
- `widgets_button` 200x20, `widgets_button_hover` 200x20, `widgets_button_disabled` 200x20
  (vanilla gray stone button with light top-left bevel, dark bottom-right, black outline;
  hover version has blue/lighter tint & white outline as in vanilla)
- `hotbar` 182x22 (vanilla hotbar strip: 9 slots), `hotbar_selection` 24x24
- `crosshair` 15x15 (white plus, 1px thick; engine draws it with invert blending)
- `heart_container` 9x9 (black outlined empty heart), `heart_full` 9x9, `heart_half` 9x9
- `food_empty` 9x9, `food_full` 9x9, `food_half` 9x9 (drumstick/shank icons)
- `armor_empty`.. skip; `air_bubble` 9x9
- `inventory` 176x166 (vanilla survival inventory background: light gray #C6C6C6 panel with
  rounded bevelled border, slots 18x18 (dark gray #8B8B8B inset with dark top-left #373737 and
  white bottom-right edges). Item slot top-left (16x16 item area) positions:
  armor (8, 8+i*18) i=0..3, crafting 2x2 at (98,18),(116,18),(98,36),(116,36), result (154,28),
  main inventory (8+c*18, 84+r*18) r=0..2, hotbar (8+c*18, 142). The slot frame is the 18x18
  around the item area (starting at item_pos - 1). Player preview area: black box at
  (26,8)-(75,78). Arrow from 2x2 grid to result like vanilla. Leave space at text locations
  (the engine draws "Crafting" text at (97,8)).
- `crafting_table` 176x166: 3x3 grid at (30+c*18, 17+r*18), result slot (124,35) drawn as the
  bigger 26x26 result slot, arrow between them at ~(90,35); inventory (8+c*18, 84+r*18) and
  hotbar (8+c*18,142). Text "Crafting" drawn by engine at (28,6), "Inventory" at (8,72).
- `furnace` 176x166: input slot (56,17), fuel slot (56,53), result (116,35) big slot,
  empty flame outline at (56,36) and empty arrow at (79,34) as in vanilla; inventory as above.
- `chest` 176x166: 3 rows of 9 slots at (8+c*18, 18+r*18), inventory at (8+c*18, 84+r*18), hotbar at 142.
- `furnace_flame` 14x14 (lit flame sprite), `furnace_arrow` 24x17 (filled white progress arrow)
- `logo` approx 274x44: the "MINECRAFT" title logo: big blocky letters with stone (gray
  cobble-ish) texture fill, dark outline and 3D extruded darker underside/right side.
- `title_edition` optional skip.
- `slot_highlight` not needed (engine draws white translucent rect).
- `background_tile` 16x16 dirt (dark-ish) for the options/menu background (engine darkens it).
- `panorama` not needed (engine renders live world behind title).
