//! Block registry: ids, textures and gameplay properties.
#![allow(dead_code)]

pub type BlockId = u8;

pub const AIR: u8 = 0;
pub const STONE: u8 = 1;
pub const GRASS: u8 = 2;
pub const DIRT: u8 = 3;
pub const COBBLESTONE: u8 = 4;
pub const OAK_PLANKS: u8 = 5;
pub const BIRCH_PLANKS: u8 = 6;
pub const SPRUCE_PLANKS: u8 = 7;
pub const JUNGLE_PLANKS: u8 = 8;
pub const ACACIA_PLANKS: u8 = 9;
pub const BEDROCK: u8 = 10;
pub const WATER: u8 = 11;
pub const LAVA: u8 = 12;
pub const SAND: u8 = 13;
pub const GRAVEL: u8 = 14;
pub const GOLD_ORE: u8 = 15;
pub const IRON_ORE: u8 = 16;
pub const COAL_ORE: u8 = 17;
pub const DIAMOND_ORE: u8 = 18;
pub const REDSTONE_ORE: u8 = 19;
pub const LAPIS_ORE: u8 = 20;
pub const OAK_LOG: u8 = 21;
pub const BIRCH_LOG: u8 = 22;
pub const SPRUCE_LOG: u8 = 23;
pub const JUNGLE_LOG: u8 = 24;
pub const ACACIA_LOG: u8 = 25;
pub const OAK_LEAVES: u8 = 26;
pub const BIRCH_LEAVES: u8 = 27;
pub const SPRUCE_LEAVES: u8 = 28;
pub const JUNGLE_LEAVES: u8 = 29;
pub const ACACIA_LEAVES: u8 = 30;
pub const GLASS: u8 = 31;
pub const SANDSTONE: u8 = 32;
pub const WOOL: u8 = 33;
pub const GOLD_BLOCK: u8 = 34;
pub const IRON_BLOCK: u8 = 35;
pub const DIAMOND_BLOCK: u8 = 36;
pub const COAL_BLOCK: u8 = 37;
pub const BRICKS: u8 = 38;
pub const TNT: u8 = 39;
pub const BOOKSHELF: u8 = 40;
pub const MOSSY_COBBLESTONE: u8 = 41;
pub const OBSIDIAN: u8 = 42;
pub const TORCH: u8 = 43;
pub const CRAFTING_TABLE: u8 = 44;
pub const FURNACE: u8 = 45;
pub const FURNACE_LIT: u8 = 46;
pub const CHEST: u8 = 47;
pub const FARMLAND: u8 = 48;
pub const WHEAT: u8 = 49;
pub const SHORT_GRASS: u8 = 50;
pub const FERN: u8 = 51;
pub const DEAD_BUSH: u8 = 52;
pub const DANDELION: u8 = 53;
pub const POPPY: u8 = 54;
pub const BLUE_ORCHID: u8 = 55;
pub const CACTUS: u8 = 56;
pub const SUGAR_CANE: u8 = 57;
pub const SNOW_LAYER: u8 = 58;
pub const SNOW_BLOCK: u8 = 59;
pub const ICE: u8 = 60;
pub const CLAY: u8 = 61;
pub const PUMPKIN: u8 = 62;
pub const NUM_BLOCKS: usize = 63;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shape {
    None,
    Cube,
    Cross,
    Torch,
    Crop,
    Liquid,
    /// Snow layer: 2/16 high slab.
    Layer,
    /// Cactus: sides inset by 1/16.
    Cactus,
    /// Farmland: 15/16 high.
    Farmland,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Layer {
    Opaque,
    Cutout,
    Translucent,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    None,
    Pickaxe,
    Axe,
    Shovel,
    Hoe,
    Sword,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tint {
    None,
    Grass,
    Foliage,
    Fixed(u32),
}

#[derive(Clone, Copy, Debug)]
pub struct BlockDef {
    pub name: &'static str,
    /// Texture names: top, bottom, side
    pub tex: [&'static str; 3],
    /// Optional front texture (furnace / crafting table / pumpkin / chest), faces the player when placed
    pub front: Option<&'static str>,
    pub shape: Shape,
    pub layer: Layer,
    pub solid: bool,
    pub opaque: bool,
    pub light_emit: u8,
    pub light_opacity: u8,
    pub hardness: f32,
    pub tool: Tool,
    /// 0 = drops with anything; 1 = needs wooden+ pickaxe; 2 stone+; 3 iron+; 4 diamond
    pub tier: u8,
    pub tint: Tint,
    pub replaceable: bool,
}

const fn cube(name: &'static str, t: &'static str, hardness: f32, tool: Tool, tier: u8) -> BlockDef {
    BlockDef {
        name,
        tex: [t, t, t],
        front: None,
        shape: Shape::Cube,
        layer: Layer::Opaque,
        solid: true,
        opaque: true,
        light_emit: 0,
        light_opacity: 15,
        hardness,
        tool,
        tier,
        tint: Tint::None,
        replaceable: false,
    }
}
const fn cube3(name: &'static str, top: &'static str, bottom: &'static str, side: &'static str, hardness: f32, tool: Tool, tier: u8) -> BlockDef {
    let mut b = cube(name, side, hardness, tool, tier);
    b.tex = [top, bottom, side];
    b
}
const fn plant(name: &'static str, t: &'static str, tint: Tint) -> BlockDef {
    BlockDef {
        name,
        tex: [t, t, t],
        front: None,
        shape: Shape::Cross,
        layer: Layer::Cutout,
        solid: false,
        opaque: false,
        light_emit: 0,
        light_opacity: 0,
        hardness: 0.0,
        tool: Tool::None,
        tier: 0,
        tint,
        replaceable: false,
    }
}
const fn leaves(name: &'static str, t: &'static str, tint: Tint) -> BlockDef {
    let mut b = cube(name, t, 0.2, Tool::Hoe, 0);
    b.layer = Layer::Cutout;
    b.opaque = false;
    b.light_opacity = 1;
    b.tint = tint;
    b
}

use Tool::*;

pub static BLOCKS: [BlockDef; NUM_BLOCKS] = {
    let mut air = cube("Air", "stone", 0.0, None, 0);
    air.shape = Shape::None;
    air.solid = false;
    air.opaque = false;
    air.light_opacity = 0;
    air.replaceable = true;

    let mut grass = cube3("Grass Block", "grass_top", "dirt", "dirt", 0.6, Shovel, 0);
    grass.tint = Tint::Grass;

    let mut water = cube("Water", "water_still", 100.0, None, 0);
    water.shape = Shape::Liquid;
    water.layer = Layer::Translucent;
    water.solid = false;
    water.opaque = false;
    water.light_opacity = 2;
    water.replaceable = true;
    water.hardness = -1.0;
    let mut lava = water;
    lava.name = "Lava";
    lava.tex = ["lava_still", "lava_still", "lava_still"];
    lava.layer = Layer::Opaque;
    lava.light_emit = 15;
    lava.light_opacity = 15;

    let mut glass = cube("Glass", "glass", 0.3, None, 0);
    glass.layer = Layer::Cutout;
    glass.opaque = false;
    glass.light_opacity = 0;

    let mut torch = plant("Torch", "torch", Tint::None);
    torch.shape = Shape::Torch;
    torch.light_emit = 14;

    let mut table = cube3("Crafting Table", "crafting_table_top", "oak_planks", "crafting_table_side", 2.5, Axe, 0);
    table.front = Some("crafting_table_front");
    let mut furnace = cube3("Furnace", "furnace_top", "furnace_top", "furnace_side", 3.5, Pickaxe, 1);
    furnace.front = Some("furnace_front");
    let mut furnace_lit = furnace;
    furnace_lit.front = Some("furnace_front_on");
    furnace_lit.light_emit = 13;
    let mut chest = cube3("Chest", "chest_top", "chest_top", "chest_side", 2.5, Axe, 0);
    chest.front = Some("chest_front");
    chest.opaque = false;
    chest.light_opacity = 0;
    chest.layer = Layer::Cutout;

    let mut farmland = cube3("Farmland", "farmland", "dirt", "dirt", 0.6, Shovel, 0);
    farmland.shape = Shape::Farmland;
    farmland.opaque = false;
    farmland.light_opacity = 0;

    let mut wheat = plant("Wheat Crops", "wheat_stage7", Tint::None);
    wheat.shape = Shape::Crop;

    let mut short_grass = plant("Grass", "short_grass", Tint::Grass);
    short_grass.replaceable = true;
    let mut fern = plant("Fern", "fern", Tint::Grass);
    fern.replaceable = true;
    let mut dead_bush = plant("Dead Bush", "dead_bush", Tint::None);
    dead_bush.replaceable = true;

    let mut cactus = cube3("Cactus", "cactus_top", "cactus_bottom", "cactus_side", 0.4, None, 0);
    cactus.shape = Shape::Cactus;
    cactus.opaque = false;
    cactus.light_opacity = 0;
    cactus.layer = Layer::Cutout;

    let mut snow_layer = cube("Snow", "snow", 0.1, Shovel, 0);
    snow_layer.shape = Shape::Layer;
    snow_layer.opaque = false;
    snow_layer.light_opacity = 0;
    snow_layer.replaceable = true;
    snow_layer.solid = false;

    let mut ice = cube("Ice", "ice", 0.5, Pickaxe, 0);
    ice.layer = Layer::Translucent;
    ice.opaque = false;
    ice.light_opacity = 2;

    let mut pumpkin = cube3("Pumpkin", "pumpkin_top", "pumpkin_top", "pumpkin_side", 1.0, Axe, 0);
    pumpkin.front = Some("pumpkin_side");

    let mut bedrock = cube("Bedrock", "bedrock", -1.0, None, 0);
    bedrock.hardness = -1.0;

    [
        air,
        cube("Stone", "stone", 1.5, Pickaxe, 1),
        grass,
        cube("Dirt", "dirt", 0.5, Shovel, 0),
        cube("Cobblestone", "cobblestone", 2.0, Pickaxe, 1),
        cube("Oak Planks", "oak_planks", 2.0, Axe, 0),
        cube("Birch Planks", "birch_planks", 2.0, Axe, 0),
        cube("Spruce Planks", "spruce_planks", 2.0, Axe, 0),
        cube("Jungle Planks", "jungle_planks", 2.0, Axe, 0),
        cube("Acacia Planks", "acacia_planks", 2.0, Axe, 0),
        bedrock,
        water,
        lava,
        cube("Sand", "sand", 0.5, Shovel, 0),
        cube("Gravel", "gravel", 0.6, Shovel, 0),
        cube("Gold Ore", "gold_ore", 3.0, Pickaxe, 3),
        cube("Iron Ore", "iron_ore", 3.0, Pickaxe, 2),
        cube("Coal Ore", "coal_ore", 3.0, Pickaxe, 1),
        cube("Diamond Ore", "diamond_ore", 3.0, Pickaxe, 3),
        cube("Redstone Ore", "redstone_ore", 3.0, Pickaxe, 3),
        cube("Lapis Lazuli Ore", "lapis_ore", 3.0, Pickaxe, 2),
        cube3("Oak Log", "oak_log_top", "oak_log_top", "oak_log", 2.0, Axe, 0),
        cube3("Birch Log", "birch_log_top", "birch_log_top", "birch_log", 2.0, Axe, 0),
        cube3("Spruce Log", "spruce_log_top", "spruce_log_top", "spruce_log", 2.0, Axe, 0),
        cube3("Jungle Log", "jungle_log_top", "jungle_log_top", "jungle_log", 2.0, Axe, 0),
        cube3("Acacia Log", "acacia_log_top", "acacia_log_top", "acacia_log", 2.0, Axe, 0),
        leaves("Oak Leaves", "oak_leaves", Tint::Foliage),
        leaves("Birch Leaves", "birch_leaves", Tint::Fixed(0x80A755)),
        leaves("Spruce Leaves", "spruce_leaves", Tint::Fixed(0x619961)),
        leaves("Jungle Leaves", "jungle_leaves", Tint::Foliage),
        leaves("Acacia Leaves", "acacia_leaves", Tint::Foliage),
        glass,
        cube3("Sandstone", "sandstone_top", "sandstone_bottom", "sandstone_side", 0.8, Pickaxe, 1),
        cube("White Wool", "white_wool", 0.8, None, 0),
        cube("Block of Gold", "gold_block", 3.0, Pickaxe, 3),
        cube("Block of Iron", "iron_block", 5.0, Pickaxe, 2),
        cube("Block of Diamond", "diamond_block", 5.0, Pickaxe, 3),
        cube("Block of Coal", "coal_block", 5.0, Pickaxe, 1),
        cube("Bricks", "bricks", 2.0, Pickaxe, 1),
        cube3("TNT", "tnt_top", "tnt_bottom", "tnt_side", 0.0, None, 0),
        cube3("Bookshelf", "oak_planks", "oak_planks", "bookshelf", 1.5, Axe, 0),
        cube("Mossy Cobblestone", "mossy_cobblestone", 2.0, Pickaxe, 1),
        cube("Obsidian", "obsidian", 50.0, Pickaxe, 4),
        torch,
        table,
        furnace,
        furnace_lit,
        chest,
        farmland,
        wheat,
        short_grass,
        fern,
        dead_bush,
        plant("Dandelion", "dandelion", Tint::None),
        plant("Poppy", "poppy", Tint::None),
        plant("Blue Orchid", "blue_orchid", Tint::None),
        cactus,
        plant("Sugar Cane", "sugar_cane", Tint::None),
        snow_layer,
        cube("Snow Block", "snow", 0.2, Shovel, 0),
        ice,
        cube("Clay", "clay", 0.6, Shovel, 0),
        pumpkin,
    ]
};

#[inline]
pub fn def(id: BlockId) -> &'static BlockDef {
    &BLOCKS[id as usize]
}
#[inline]
pub fn is_opaque(id: BlockId) -> bool {
    BLOCKS[id as usize].opaque
}
#[inline]
pub fn is_solid(id: BlockId) -> bool {
    BLOCKS[id as usize].solid
}

pub fn is_log(id: BlockId) -> bool {
    (OAK_LOG..=ACACIA_LOG).contains(&id)
}
pub fn is_leaves(id: BlockId) -> bool {
    (OAK_LEAVES..=ACACIA_LEAVES).contains(&id)
}
pub fn is_plant(id: BlockId) -> bool {
    matches!(def(id).shape, Shape::Cross | Shape::Crop)
}

/// Does this block need a supporting block below it?
pub fn needs_support(id: BlockId) -> bool {
    matches!(id, WHEAT | SHORT_GRASS | FERN | DEAD_BUSH | DANDELION | POPPY | BLUE_ORCHID | SUGAR_CANE | CACTUS | SNOW_LAYER)
}

/// Collision box(es) in local block coords (min, max) or None if not solid.
pub fn collision_box(id: BlockId) -> Option<([f32; 3], [f32; 3])> {
    let d = def(id);
    if !d.solid {
        return None;
    }
    Some(match d.shape {
        Shape::Cactus => ([1.0 / 16.0, 0.0, 1.0 / 16.0], [15.0 / 16.0, 15.0 / 16.0, 15.0 / 16.0]),
        Shape::Farmland => ([0.0; 3], [1.0, 15.0 / 16.0, 1.0]),
        Shape::Cube => {
            if id == CHEST {
                ([1.0 / 16.0, 0.0, 1.0 / 16.0], [15.0 / 16.0, 14.0 / 16.0, 15.0 / 16.0])
            } else {
                ([0.0; 3], [1.0; 3])
            }
        }
        _ => return None,
    })
}

/// Selection/outline box in local block coords.
pub fn selection_box(id: BlockId, meta: u8) -> ([f32; 3], [f32; 3]) {
    let d = def(id);
    let p = 1.0 / 16.0;
    match d.shape {
        Shape::Cross => ([2.0 * p, 0.0, 2.0 * p], [14.0 * p, 13.0 * p, 14.0 * p]),
        Shape::Crop => ([0.0, 0.0, 0.0], [1.0, (meta as f32 + 1.0) * 2.0 * p, 1.0]),
        Shape::Torch => match meta {
            1 => ([5.5 * p, 3.0 * p, 0.0], [10.5 * p, 13.0 * p, 5.0 * p]),          // on north wall, pointing south... approximate
            2 => ([5.5 * p, 3.0 * p, 11.0 * p], [10.5 * p, 13.0 * p, 1.0]),
            3 => ([0.0, 3.0 * p, 5.5 * p], [5.0 * p, 13.0 * p, 10.5 * p]),
            4 => ([11.0 * p, 3.0 * p, 5.5 * p], [1.0, 13.0 * p, 10.5 * p]),
            _ => ([6.0 * p, 0.0, 6.0 * p], [10.0 * p, 10.0 * p, 10.0 * p]),
        },
        Shape::Layer => ([0.0; 3], [1.0, 2.0 * p, 1.0]),
        Shape::Cactus => ([p, 0.0, p], [15.0 * p, 1.0, 15.0 * p]),
        Shape::Farmland => ([0.0; 3], [1.0, 15.0 * p, 1.0]),
        _ => {
            if id == CHEST {
                ([p, 0.0, p], [15.0 * p, 14.0 * p, 15.0 * p])
            } else {
                ([0.0; 3], [1.0; 3])
            }
        }
    }
}
