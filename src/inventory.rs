//! Player inventory, furnace state and crafting recipes.

use crate::block;
use crate::item::{self, ItemId, ItemStack};

/// 36 main slots: 0..9 hotbar, 9..36 main inventory.
#[derive(Clone)]
pub struct Inventory {
    pub slots: [ItemStack; 36],
    pub selected: usize,
}

impl Inventory {
    pub fn new() -> Inventory {
        Inventory { slots: [ItemStack::EMPTY; 36], selected: 0 }
    }
    pub fn held(&self) -> ItemStack {
        self.slots[self.selected]
    }
    pub fn held_mut(&mut self) -> &mut ItemStack {
        &mut self.slots[self.selected]
    }
    /// Try to add a stack; returns the leftover count.
    pub fn add(&mut self, mut st: ItemStack) -> u8 {
        let max = item::max_stack(st.id);
        // merge into existing stacks (hotbar first, then main)
        for i in 0..36 {
            let s = &mut self.slots[i];
            if !s.is_empty() && s.can_stack_with(&st) && s.count < max {
                let n = (max - s.count).min(st.count);
                s.count += n;
                st.count -= n;
                if st.count == 0 {
                    return 0;
                }
            }
        }
        for i in 0..36 {
            if self.slots[i].is_empty() {
                let n = st.count.min(max);
                self.slots[i] = ItemStack { id: st.id, count: n, damage: st.damage };
                st.count -= n;
                if st.count == 0 {
                    return 0;
                }
            }
        }
        st.count
    }
    pub fn consume_held(&mut self, n: u8) {
        let s = self.held_mut();
        s.count = s.count.saturating_sub(n);
        if s.count == 0 {
            *s = ItemStack::EMPTY;
        }
    }
    /// Damage the held tool; returns true if it broke.
    pub fn damage_held(&mut self, amount: u16) -> bool {
        let s = self.held_mut();
        let max = item::max_damage(s.id);
        if max == 0 || s.is_empty() {
            return false;
        }
        s.damage += amount;
        if s.damage >= max {
            *s = ItemStack::EMPTY;
            return true;
        }
        false
    }
}

#[derive(Clone, Debug)]
pub struct FurnaceState {
    pub input: ItemStack,
    pub fuel: ItemStack,
    pub output: ItemStack,
    pub burn_time: i32,
    pub burn_total: i32,
    pub cook_time: i32,
}

pub const COOK_TOTAL: i32 = 200;

impl FurnaceState {
    pub fn new() -> FurnaceState {
        FurnaceState { input: ItemStack::EMPTY, fuel: ItemStack::EMPTY, output: ItemStack::EMPTY, burn_time: 0, burn_total: 0, cook_time: 0 }
    }
    fn can_smelt(&self) -> Option<ItemId> {
        if self.input.is_empty() {
            return None;
        }
        let r = item::smelt_result(self.input.id)?;
        if self.output.is_empty() || (self.output.id == r && self.output.count < item::max_stack(r)) {
            Some(r)
        } else {
            None
        }
    }
    /// Advance one game tick. Returns whether the furnace is burning.
    pub fn tick(&mut self) -> bool {
        if self.burn_time > 0 {
            self.burn_time -= 1;
        }
        let res = self.can_smelt();
        if self.burn_time == 0 && res.is_some() && !self.fuel.is_empty() {
            let ft = item::fuel_time(self.fuel.id);
            if ft > 0 {
                self.burn_time = ft;
                self.burn_total = ft;
                if self.fuel.id == item::LAVA_BUCKET {
                    self.fuel = ItemStack::new(item::BUCKET, 1);
                } else {
                    self.fuel.count -= 1;
                    if self.fuel.count == 0 {
                        self.fuel = ItemStack::EMPTY;
                    }
                }
            }
        }
        if self.burn_time > 0 {
            if let Some(r) = res {
                self.cook_time += 1;
                if self.cook_time >= COOK_TOTAL {
                    self.cook_time = 0;
                    if self.output.is_empty() {
                        self.output = ItemStack::new(r, 1);
                    } else {
                        self.output.count += 1;
                    }
                    self.input.count -= 1;
                    if self.input.count == 0 {
                        self.input = ItemStack::EMPTY;
                    }
                }
            } else {
                self.cook_time = 0;
            }
        } else if self.cook_time > 0 {
            self.cook_time = (self.cook_time - 2).max(0);
        }
        self.burn_time > 0
    }
}

// ---------------- Crafting ----------------

/// Recipe ingredient matcher.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Ing {
    Item(ItemId),
    AnyPlanks,
    AnyLog,
    AnyCobble,
}

impl Ing {
    fn matches(&self, id: ItemId) -> bool {
        match *self {
            Ing::Item(i) => i == id,
            Ing::AnyPlanks => id < 256 && (block::OAK_PLANKS..=block::ACACIA_PLANKS).contains(&(id as u8)),
            Ing::AnyLog => id < 256 && block::is_log(id as u8),
            Ing::AnyCobble => id == block::COBBLESTONE as ItemId,
        }
    }
}

pub struct Recipe {
    /// Rows of the pattern; each char maps through `key`. ' ' = empty.
    pub pattern: &'static [&'static str],
    pub key: &'static [(char, Ing)],
    pub result: ItemId,
    pub count: u8,
    pub shapeless: bool,
}

const fn b(id: u8) -> ItemId {
    id as ItemId
}

use Ing::*;
pub static RECIPES: &[Recipe] = &[
    Recipe { pattern: &["#"], key: &[('#', Item(b(block::OAK_LOG)))], result: b(block::OAK_PLANKS), count: 4, shapeless: false },
    Recipe { pattern: &["#"], key: &[('#', Item(b(block::BIRCH_LOG)))], result: b(block::BIRCH_PLANKS), count: 4, shapeless: false },
    Recipe { pattern: &["#"], key: &[('#', Item(b(block::SPRUCE_LOG)))], result: b(block::SPRUCE_PLANKS), count: 4, shapeless: false },
    Recipe { pattern: &["#"], key: &[('#', Item(b(block::JUNGLE_LOG)))], result: b(block::JUNGLE_PLANKS), count: 4, shapeless: false },
    Recipe { pattern: &["#"], key: &[('#', Item(b(block::ACACIA_LOG)))], result: b(block::ACACIA_PLANKS), count: 4, shapeless: false },
    Recipe { pattern: &["#", "#"], key: &[('#', AnyPlanks)], result: item::STICK, count: 4, shapeless: false },
    Recipe { pattern: &["##", "##"], key: &[('#', AnyPlanks)], result: b(block::CRAFTING_TABLE), count: 1, shapeless: false },
    Recipe { pattern: &["C", "S"], key: &[('C', Item(item::COAL)), ('S', Item(item::STICK))], result: b(block::TORCH), count: 4, shapeless: false },
    Recipe { pattern: &["C", "S"], key: &[('C', Item(item::CHARCOAL)), ('S', Item(item::STICK))], result: b(block::TORCH), count: 4, shapeless: false },
    Recipe { pattern: &["###", "# #", "###"], key: &[('#', AnyCobble)], result: b(block::FURNACE), count: 1, shapeless: false },
    Recipe { pattern: &["###", "# #", "###"], key: &[('#', AnyPlanks)], result: b(block::CHEST), count: 1, shapeless: false },
    Recipe { pattern: &["###"], key: &[('#', Item(item::WHEAT))], result: item::BREAD, count: 1, shapeless: false },
    Recipe { pattern: &["##", "##"], key: &[('#', Item(b(block::SAND)))], result: b(block::SANDSTONE), count: 1, shapeless: false },
    Recipe { pattern: &["##", "##"], key: &[('#', Item(item::STRING))], result: b(block::WOOL), count: 1, shapeless: false },
    Recipe { pattern: &["##", "##"], key: &[('#', Item(b(block::SNOW_LAYER)))], result: b(block::SNOW_BLOCK), count: 1, shapeless: false },
    Recipe { pattern: &["###", "###", "###"], key: &[('#', Item(item::COAL))], result: b(block::COAL_BLOCK), count: 1, shapeless: false },
    Recipe { pattern: &["###", "###", "###"], key: &[('#', Item(item::IRON_INGOT))], result: b(block::IRON_BLOCK), count: 1, shapeless: false },
    Recipe { pattern: &["###", "###", "###"], key: &[('#', Item(item::GOLD_INGOT))], result: b(block::GOLD_BLOCK), count: 1, shapeless: false },
    Recipe { pattern: &["###", "###", "###"], key: &[('#', Item(item::DIAMOND))], result: b(block::DIAMOND_BLOCK), count: 1, shapeless: false },
    Recipe { pattern: &["#"], key: &[('#', Item(b(block::COAL_BLOCK)))], result: item::COAL, count: 9, shapeless: false },
    Recipe { pattern: &["#"], key: &[('#', Item(b(block::IRON_BLOCK)))], result: item::IRON_INGOT, count: 9, shapeless: false },
    Recipe { pattern: &["#"], key: &[('#', Item(b(block::GOLD_BLOCK)))], result: item::GOLD_INGOT, count: 9, shapeless: false },
    Recipe { pattern: &["#"], key: &[('#', Item(b(block::DIAMOND_BLOCK)))], result: item::DIAMOND, count: 9, shapeless: false },
    Recipe { pattern: &["# #", " # "], key: &[('#', AnyPlanks)], result: item::BOWL, count: 4, shapeless: false },
    Recipe { pattern: &["# #", " # "], key: &[('#', Item(item::IRON_INGOT))], result: item::BUCKET, count: 1, shapeless: false },
    Recipe { pattern: &[" #S", "# S", " #S"], key: &[('#', Item(item::STICK)), ('S', Item(item::STRING))], result: item::BOW, count: 1, shapeless: false },
    Recipe { pattern: &["F", "S", "E"], key: &[('F', Item(item::FLINT)), ('S', Item(item::STICK)), ('E', Item(item::FEATHER))], result: item::ARROW, count: 4, shapeless: false },
    Recipe { pattern: &["###"], key: &[('#', Item(b(block::SUGAR_CANE)))], result: item::PAPER, count: 3, shapeless: false },
    Recipe { pattern: &["#"], key: &[('#', Item(b(block::SUGAR_CANE)))], result: item::SUGAR, count: 1, shapeless: false },
    Recipe { pattern: &["PPP", "BBB", "PPP"], key: &[('P', AnyPlanks), ('B', Item(item::PAPER))], result: b(block::BOOKSHELF), count: 1, shapeless: false },
    Recipe { pattern: &["GSG", "SGS", "GSG"], key: &[('G', Item(item::GUNPOWDER)), ('S', Item(b(block::SAND)))], result: b(block::TNT), count: 1, shapeless: false },
];

/// Tool recipes are generated: material ingredient per tier.
fn tool_recipes() -> Vec<(Vec<&'static str>, Ing, ItemId)> {
    let mats: [(Ing, ItemId); 5] = [
        (AnyPlanks, item::WOODEN_PICKAXE),
        (AnyCobble, item::STONE_PICKAXE),
        (Item(item::IRON_INGOT), item::IRON_PICKAXE),
        (Item(item::GOLD_INGOT), item::GOLDEN_PICKAXE),
        (Item(item::DIAMOND), item::DIAMOND_PICKAXE),
    ];
    let shapes: [(&[&str], u16); 5] = [
        (&["###", " | ", " | "], 0),
        (&["##", "#|", " |"], 1),
        (&["#", "|", "|"], 2),
        (&["#", "#", "|"], 3),
        (&["##", " |", " |"], 4),
    ];
    let mut v = Vec::new();
    for (m, base) in mats {
        for (s, off) in shapes {
            v.push((s.to_vec(), m, base + off));
        }
    }
    v
}

/// Match a crafting grid (w x h, row-major) against all recipes.
pub fn craft(grid: &[ItemStack], w: usize, h: usize) -> Option<ItemStack> {
    // bounding box of non-empty cells
    let (mut minx, mut miny, mut maxx, mut maxy) = (w, h, 0, 0);
    let mut any = false;
    for y in 0..h {
        for x in 0..w {
            if !grid[y * w + x].is_empty() {
                any = true;
                minx = minx.min(x);
                miny = miny.min(y);
                maxx = maxx.max(x);
                maxy = maxy.max(y);
            }
        }
    }
    if !any {
        return None;
    }
    let (bw, bh) = (maxx - minx + 1, maxy - miny + 1);
    let cell = |x: usize, y: usize| grid[(miny + y) * w + minx + x];

    let try_pattern = |pattern: &[&str], lookup: &dyn Fn(char) -> Option<Ing>| -> bool {
        let ph = pattern.len();
        let pw = pattern.iter().map(|r| r.len()).max().unwrap_or(0);
        if pw != bw || ph != bh {
            return false;
        }
        for mirror in [false, true] {
            let mut ok = true;
            'outer: for y in 0..ph {
                let row: Vec<char> = pattern[y].chars().collect();
                for x in 0..pw {
                    let px = if mirror { pw - 1 - x } else { x };
                    let c = row.get(px).copied().unwrap_or(' ');
                    let st = cell(x, y);
                    if c == ' ' {
                        if !st.is_empty() {
                            ok = false;
                            break 'outer;
                        }
                    } else {
                        match lookup(c) {
                            Some(ing) if !st.is_empty() && ing.matches(st.id) => {}
                            _ => {
                                ok = false;
                                break 'outer;
                            }
                        }
                    }
                }
            }
            if ok {
                return true;
            }
        }
        false
    };

    for r in RECIPES {
        if r.count == 0 {
            continue;
        }
        let lookup = |c: char| r.key.iter().find(|k| k.0 == c).map(|k| k.1);
        if try_pattern(r.pattern, &lookup) {
            return Some(ItemStack::new(r.result, r.count));
        }
    }
    for (pattern, mat, result) in tool_recipes() {
        let lookup = |c: char| match c {
            '#' => Some(mat),
            '|' => Some(Item(item::STICK)),
            _ => None,
        };
        if try_pattern(&pattern, &lookup) {
            return Some(ItemStack::new(result, 1));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::*;

    fn grid(ids: &[u16]) -> Vec<ItemStack> {
        ids.iter().map(|&i| if i == 0 { ItemStack::EMPTY } else { ItemStack::new(i, 1) }).collect()
    }
    const P: u16 = OAK_PLANKS as u16;
    const S: u16 = item::STICK;
    const C: u16 = COBBLESTONE as u16;

    #[test]
    fn planks_from_log_any_position() {
        let g = grid(&[0, 0, 0, OAK_LOG as u16]);
        assert_eq!(craft(&g, 2, 2), Some(ItemStack::new(P, 4)));
        let g = grid(&[0, 0, 0, 0, BIRCH_LOG as u16, 0, 0, 0, 0]);
        assert_eq!(craft(&g, 3, 3), Some(ItemStack::new(BIRCH_PLANKS as u16, 4)));
    }

    #[test]
    fn sticks_table_torches() {
        assert_eq!(craft(&grid(&[P, 0, P, 0]), 2, 2), Some(ItemStack::new(S, 4)));
        assert_eq!(craft(&grid(&[P, P, P, P]), 2, 2), Some(ItemStack::new(CRAFTING_TABLE as u16, 1)));
        assert_eq!(craft(&grid(&[0, item::COAL, 0, S]), 2, 2), Some(ItemStack::new(TORCH as u16, 4)));
        assert_eq!(craft(&grid(&[item::CHARCOAL, 0, S, 0]), 2, 2), Some(ItemStack::new(TORCH as u16, 4)));
    }

    #[test]
    fn tools() {
        assert_eq!(craft(&grid(&[P, P, P, 0, S, 0, 0, S, 0]), 3, 3).unwrap().id, item::WOODEN_PICKAXE);
        assert_eq!(craft(&grid(&[C, C, C, 0, S, 0, 0, S, 0]), 3, 3).unwrap().id, item::STONE_PICKAXE);
        assert_eq!(craft(&grid(&[item::DIAMOND, item::DIAMOND, item::DIAMOND, 0, S, 0, 0, S, 0]), 3, 3).unwrap().id, item::DIAMOND_PICKAXE);
        // axe and mirrored axe
        assert_eq!(craft(&grid(&[P, P, 0, P, S, 0, 0, S, 0]), 3, 3).unwrap().id, item::WOODEN_AXE);
        assert_eq!(craft(&grid(&[0, P, P, 0, S, P, 0, S, 0]), 3, 3).unwrap().id, item::WOODEN_AXE);
        assert_eq!(craft(&grid(&[0, C, 0, 0, S, 0, 0, S, 0]), 3, 3).unwrap().id, item::STONE_SHOVEL);
        assert_eq!(craft(&grid(&[0, item::IRON_INGOT, 0, 0, item::IRON_INGOT, 0, 0, S, 0]), 3, 3).unwrap().id, item::IRON_SWORD);
        assert_eq!(craft(&grid(&[item::GOLD_INGOT, item::GOLD_INGOT, 0, 0, S, 0, 0, S, 0]), 3, 3).unwrap().id, item::GOLDEN_HOE);
        // shovel fits in the 2x2 grid? (no: needs 3 rows)
        assert_eq!(craft(&grid(&[P, 0, S, 0]), 2, 2), None);
    }

    #[test]
    fn furnace_and_bread() {
        assert_eq!(craft(&grid(&[C, C, C, C, 0, C, C, C, C]), 3, 3).unwrap().id, FURNACE as u16);
        assert_eq!(craft(&grid(&[0, 0, 0, item::WHEAT, item::WHEAT, item::WHEAT, 0, 0, 0]), 3, 3).unwrap().id, item::BREAD);
        assert_eq!(craft(&grid(&[P, P, P, P, 0, P, P, P, P]), 3, 3).unwrap().id, CHEST as u16);
    }

    #[test]
    fn smelting() {
        let mut f = FurnaceState::new();
        f.input = ItemStack::new(IRON_ORE as u16, 2);
        f.fuel = ItemStack::new(item::COAL, 1);
        for _ in 0..COOK_TOTAL * 2 + 5 {
            f.tick();
        }
        assert_eq!(f.output, ItemStack::new(item::IRON_INGOT, 2));
        assert!(f.input.is_empty());
        assert!(f.fuel.is_empty());
    }

    #[test]
    fn inventory_add_stacks() {
        let mut inv = Inventory::new();
        assert_eq!(inv.add(ItemStack::new(C, 64)), 0);
        assert_eq!(inv.add(ItemStack::new(C, 10)), 0);
        assert_eq!(inv.slots[0].count, 64);
        assert_eq!(inv.slots[1].count, 10);
        assert_eq!(inv.add(ItemStack::new(item::WOODEN_PICKAXE, 1)), 0);
        assert_eq!(inv.slots[2].id, item::WOODEN_PICKAXE);
    }
}
