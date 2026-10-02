//! Item registry. Item ids < 256 are block items (id == block id);
//! ids >= 256 are pure items.
#![allow(dead_code)]

use crate::block::{self, Tool};

pub type ItemId = u16;

macro_rules! items {
    ($($c:ident = $n:expr, $name:expr, $tex:expr;)*) => {
        $(pub const $c: ItemId = $n;)*
        pub static ITEM_LIST: &[(ItemId, &str, &str)] = &[$(($n, $name, $tex)),*];
    };
}

items! {
    STICK = 256, "Stick", "stick";
    COAL = 257, "Coal", "coal";
    CHARCOAL = 258, "Charcoal", "charcoal";
    IRON_INGOT = 259, "Iron Ingot", "iron_ingot";
    GOLD_INGOT = 260, "Gold Ingot", "gold_ingot";
    DIAMOND = 261, "Diamond", "diamond";
    WHEAT_SEEDS = 262, "Wheat Seeds", "wheat_seeds";
    WHEAT = 263, "Wheat", "wheat";
    BREAD = 264, "Bread", "bread";
    APPLE = 265, "Apple", "apple";
    PORKCHOP = 266, "Raw Porkchop", "porkchop";
    COOKED_PORKCHOP = 267, "Cooked Porkchop", "cooked_porkchop";
    BEEF = 268, "Raw Beef", "beef";
    COOKED_BEEF = 269, "Steak", "cooked_beef";
    CHICKEN = 270, "Raw Chicken", "chicken";
    COOKED_CHICKEN = 271, "Cooked Chicken", "cooked_chicken";
    ROTTEN_FLESH = 272, "Rotten Flesh", "rotten_flesh";
    LEATHER = 273, "Leather", "leather";
    FEATHER = 274, "Feather", "feather";
    BONE = 275, "Bone", "bone";
    STRING = 276, "String", "string";
    GUNPOWDER = 277, "Gunpowder", "gunpowder";
    FLINT = 278, "Flint", "flint";
    ARROW = 279, "Arrow", "arrow";
    BOW = 280, "Bow", "bow";
    BOWL = 281, "Bowl", "bowl";
    SUGAR = 282, "Sugar", "sugar";
    EGG = 283, "Egg", "egg";
    BUCKET = 284, "Bucket", "bucket";
    WATER_BUCKET = 285, "Water Bucket", "water_bucket";
    LAVA_BUCKET = 286, "Lava Bucket", "lava_bucket";
    PAPER = 287, "Paper", "paper";
    OAK_DOOR_ITEM = 290, "Oak Door", "oak_door";
    BED_ITEM = 291, "Bed", "red_bed";
    WOODEN_PICKAXE = 300, "Wooden Pickaxe", "wooden_pickaxe";
    WOODEN_AXE = 301, "Wooden Axe", "wooden_axe";
    WOODEN_SHOVEL = 302, "Wooden Shovel", "wooden_shovel";
    WOODEN_SWORD = 303, "Wooden Sword", "wooden_sword";
    WOODEN_HOE = 304, "Wooden Hoe", "wooden_hoe";
    STONE_PICKAXE = 305, "Stone Pickaxe", "stone_pickaxe";
    STONE_AXE = 306, "Stone Axe", "stone_axe";
    STONE_SHOVEL = 307, "Stone Shovel", "stone_shovel";
    STONE_SWORD = 308, "Stone Sword", "stone_sword";
    STONE_HOE = 309, "Stone Hoe", "stone_hoe";
    IRON_PICKAXE = 310, "Iron Pickaxe", "iron_pickaxe";
    IRON_AXE = 311, "Iron Axe", "iron_axe";
    IRON_SHOVEL = 312, "Iron Shovel", "iron_shovel";
    IRON_SWORD = 313, "Iron Sword", "iron_sword";
    IRON_HOE = 314, "Iron Hoe", "iron_hoe";
    GOLDEN_PICKAXE = 315, "Golden Pickaxe", "golden_pickaxe";
    GOLDEN_AXE = 316, "Golden Axe", "golden_axe";
    GOLDEN_SHOVEL = 317, "Golden Shovel", "golden_shovel";
    GOLDEN_SWORD = 318, "Golden Sword", "golden_sword";
    GOLDEN_HOE = 319, "Golden Hoe", "golden_hoe";
    DIAMOND_PICKAXE = 320, "Diamond Pickaxe", "diamond_pickaxe";
    DIAMOND_AXE = 321, "Diamond Axe", "diamond_axe";
    DIAMOND_SHOVEL = 322, "Diamond Shovel", "diamond_shovel";
    DIAMOND_SWORD = 323, "Diamond Sword", "diamond_sword";
    DIAMOND_HOE = 324, "Diamond Hoe", "diamond_hoe";
}

pub const MAX_ITEM: usize = 325;

pub fn is_block(id: ItemId) -> bool {
    (id as usize) < block::NUM_BLOCKS && id != 0
}

pub fn name(id: ItemId) -> &'static str {
    if id < 256 {
        return block::def(id as u8).name;
    }
    ITEM_LIST.iter().find(|e| e.0 == id).map(|e| e.1).unwrap_or("?")
}

/// Texture name for flat item sprites (None for block items rendered as 3D blocks).
pub fn texture(id: ItemId) -> Option<&'static str> {
    if id < 256 {
        // Some blocks are shown as flat sprites in the inventory
        return match id as u8 {
            block::TORCH => Some("b:torch"),
            block::SHORT_GRASS => Some("b:short_grass"),
            block::FERN => Some("b:fern"),
            block::DEAD_BUSH => Some("b:dead_bush"),
            block::DANDELION => Some("b:dandelion"),
            block::POPPY => Some("b:poppy"),
            block::BLUE_ORCHID => Some("b:blue_orchid"),
            block::SUGAR_CANE => Some("b:sugar_cane"),
            block::WHEAT => Some("b:wheat_stage7"),
            block::OAK_SAPLING => Some("b:oak_sapling"),
            block::LADDER => Some("b:ladder"),
            block::BIRCH_SAPLING => Some("b:birch_sapling"),
            block::SPRUCE_SAPLING => Some("b:spruce_sapling"),
            block::JUNGLE_SAPLING => Some("b:jungle_sapling"),
            block::ACACIA_SAPLING => Some("b:acacia_sapling"),
            _ => None,
        };
    }
    ITEM_LIST.iter().find(|e| e.0 == id).map(|e| e.2)
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ToolInfo {
    pub kind: Tool,
    /// 0 wood, 1 stone, 2 iron, 3 gold, 4 diamond
    pub material: u8,
}

impl ToolInfo {
    pub fn speed(&self) -> f32 {
        [2.0, 4.0, 6.0, 12.0, 8.0][self.material as usize]
    }
    /// Harvest level using block tiers: wood/gold 1, stone 2, iron 3, diamond 4
    pub fn level(&self) -> u8 {
        [1, 2, 3, 1, 4][self.material as usize]
    }
    pub fn durability(&self) -> u16 {
        [59, 131, 250, 32, 1561][self.material as usize]
    }
    pub fn attack_damage(&self) -> f32 {
        let base = [0.0, 1.0, 2.0, 0.0, 3.0][self.material as usize];
        match self.kind {
            Tool::Sword => 4.0 + base,
            Tool::Axe => 3.0 + base,
            Tool::Pickaxe => 2.0 + base,
            Tool::Shovel => 1.5 + base,
            _ => 1.0,
        }
    }
}

pub fn tool_info(id: ItemId) -> Option<ToolInfo> {
    if !(300..325).contains(&id) {
        return None;
    }
    let n = id - 300;
    let kind = match n % 5 {
        0 => Tool::Pickaxe,
        1 => Tool::Axe,
        2 => Tool::Shovel,
        3 => Tool::Sword,
        _ => Tool::Hoe,
    };
    Some(ToolInfo { kind, material: (n / 5) as u8 })
}

pub fn max_stack(id: ItemId) -> u8 {
    if tool_info(id).is_some() || matches!(id, BOW | WATER_BUCKET | LAVA_BUCKET | BED_ITEM) {
        1
    } else if matches!(id, EGG | BUCKET) {
        16
    } else {
        64
    }
}

pub fn max_damage(id: ItemId) -> u16 {
    if let Some(t) = tool_info(id) { t.durability() } else if id == BOW { 384 } else { 0 }
}

/// (hunger points, saturation) restored when eaten.
pub fn food(id: ItemId) -> Option<(i32, f32)> {
    Some(match id {
        APPLE => (4, 2.4),
        BREAD => (5, 6.0),
        PORKCHOP => (3, 1.8),
        COOKED_PORKCHOP => (8, 12.8),
        BEEF => (3, 1.8),
        COOKED_BEEF => (8, 12.8),
        CHICKEN => (2, 1.2),
        COOKED_CHICKEN => (6, 7.2),
        ROTTEN_FLESH => (4, 0.8),
        _ => return None,
    })
}

/// Burn time in ticks when used as furnace fuel.
pub fn fuel_time(id: ItemId) -> i32 {
    if id < 256 {
        let b = id as u8;
        if block::is_sapling(b) {
            return 100;
        }
        if (block::OAK_PLANKS..=block::ACACIA_PLANKS).contains(&b) || block::is_log(b) || matches!(b, block::CRAFTING_TABLE | block::CHEST | block::BOOKSHELF | block::OAK_FENCE | block::LADDER) {
            return 300;
        }
        if b == block::COAL_BLOCK {
            return 16000;
        }
        return 0;
    }
    match id {
        COAL | CHARCOAL => 1600,
        STICK => 100,
        LAVA_BUCKET => 20000,
        BOW => 300,
        BOWL => 100,
        OAK_DOOR_ITEM => 200,
        _ => {
            if let Some(t) = tool_info(id) {
                if t.material == 0 {
                    return 200;
                }
            }
            0
        }
    }
}

/// Furnace smelting result.
pub fn smelt_result(id: ItemId) -> Option<ItemId> {
    let b = |x: u8| x as ItemId;
    Some(match id {
        x if x == b(block::IRON_ORE) => IRON_INGOT,
        x if x == b(block::GOLD_ORE) => GOLD_INGOT,
        x if x == b(block::SAND) => b(block::GLASS),
        x if x == b(block::COBBLESTONE) => b(block::STONE),
        x if x == b(block::CLAY) => b(block::BRICKS),
        x if x == b(block::DIAMOND_ORE) => DIAMOND,
        x if x == b(block::COAL_ORE) => COAL,
        x if x < 256 && block::is_log(x as u8) => CHARCOAL,
        PORKCHOP => COOKED_PORKCHOP,
        BEEF => COOKED_BEEF,
        CHICKEN => COOKED_CHICKEN,
        _ => return None,
    })
}

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct ItemStack {
    pub id: ItemId,
    pub count: u8,
    pub damage: u16,
}

impl ItemStack {
    pub const EMPTY: ItemStack = ItemStack { id: 0, count: 0, damage: 0 };
    pub fn new(id: ItemId, count: u8) -> ItemStack {
        ItemStack { id, count, damage: 0 }
    }
    pub fn is_empty(&self) -> bool {
        self.count == 0 || self.id == 0
    }
    pub fn can_stack_with(&self, o: &ItemStack) -> bool {
        self.id == o.id && max_stack(self.id) > 1 && self.damage == o.damage
    }
}
