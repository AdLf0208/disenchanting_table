// The Dis-Enchanting Table's pure engine: which item may be dis-enchanted,
// what the output book takes, what each menu slot accepts, and the experience
// cost model.
//
// Game-free: the build script's block entity, menu rules, interaction and
// command are thin adapters over these functions, and the same file is the
// standalone library's `cargo test` target.
//
// Read from Dis-Enchanting Table 5.0.2 (`DisEnchantingHelper`,
// `DisEnchantingStrategy`, `DisEnchantingMenu`, `ExperienceHelper`) on the
// upstream 1.21.6 branch; see `../../FINDINGS.md`. The experience thresholds are
// vanilla's, not upstream's half-dropping `n / 2` sum.

/// The item the extra slot takes (`SlotPredicates.VALID_EXTRA`).
pub const BOOK_ITEM: &str = "minecraft:book";

/// The item whose enchantments are stored, not applied.
pub const ENCHANTED_BOOK_ITEM: &str = "minecraft:enchanted_book";

/// The block this port registers (`ModBlocks` uses `mod_id` for its own path).
pub const TABLE_BLOCK: &str = "disenchanting_table:disenchanting_table";

/// The menu type the content mod declares (`data/<id>/horizon/menu/<name>.json`).
pub const TABLE_MENU: &str = "disenchanting_table:disenchanting_table";

/// `TileEntityConstants.INPUT_SLOT` / `EXTRA_SLOT` / `OUTPUT_SLOT`.
pub const INPUT_SLOT: usize = 0;
pub const BOOK_SLOT: usize = 1;
pub const OUTPUT_SLOT: usize = 2;
pub const SLOT_COUNT: usize = 3;

/// `disenchanting_table-server.toml`'s defaults.
pub const DEFAULT_SETTINGS: Settings = Settings {
    automatic_disenchanting: false,
    automatic_disenchanting_ticks: 10,
    resets_repair_cost: true,
    requires_experience: true,
    uses_points: true,
    experience_cost: 25,
};

/// The server config this slice bakes in (no config API at the pin: a gap).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    /// `automatic_disenchanting`: a 10-tick auto mode; not ported (gap).
    pub automatic_disenchanting: bool,
    /// `automatic_disenchanting_ticks`.
    pub automatic_disenchanting_ticks: u32,
    /// `resets_repair_cost`: clear the input's prior-work penalty.
    pub resets_repair_cost: bool,
    /// `requires_experience`.
    pub requires_experience: bool,
    /// `uses_points`: charge points (true) or levels (false).
    pub uses_points: bool,
    /// `experience_cost`: points or levels, by `uses_points`.
    pub experience_cost: u32,
}

/// Whether `block` is this mod's table (never vanilla's enchanting table).
#[must_use]
pub fn is_table(block: &str) -> bool {
    block == TABLE_BLOCK
}

/// Whether `item` is the extra slot's book (`Items.BOOK`, not a writable one).
#[must_use]
pub fn is_book(item: &str) -> bool {
    item == BOOK_ITEM
}

/// Whether `item` stores its enchantments rather than applying them.
#[must_use]
pub fn is_enchanted_book(item: &str) -> bool {
    item == ENCHANTED_BOOK_ITEM
}

/// `DisEnchantingHelper.canDisenchant`: `enchantments` are the stack's
/// crafting enchantments (`ItemStack.crafting_enchantments`: the stored ones
/// for an enchanted book, the applied ones otherwise).
#[must_use]
pub fn can_disenchant(item: &str, enchantments: &[(&str, u32)]) -> bool {
    if enchantments.is_empty() {
        return false;
    }
    // A one-enchantment book would leave nothing behind, so it is refused.
    !is_enchanted_book(item) || enchantments.len() >= 2
}

/// What a dis-enchant would produce and what would stay behind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Plan {
    /// The input was an ordinary enchanted item: the output book takes every
    /// enchantment and the item keeps none.
    Whole { moved: Vec<(String, u32)> },
    /// The input was an enchanted book: it keeps its first enchantment and the
    /// output book takes the rest.
    Split {
        kept: (String, u32),
        moved: Vec<(String, u32)>,
    },
}

fn owned(levels: &[(&str, u32)]) -> Vec<(String, u32)> {
    levels
        .iter()
        .map(|(id, level)| ((*id).to_owned(), *level))
        .collect()
}

/// `DisEnchantingMenu.createResult`: the plan, or none when the input may not
/// be dis-enchanted. The output is a new enchanted book of `moved`.
#[must_use]
pub fn plan(item: &str, enchantments: &[(&str, u32)]) -> Option<Plan> {
    if !can_disenchant(item, enchantments) {
        return None;
    }
    if !is_enchanted_book(item) {
        return Some(Plan::Whole {
            moved: owned(enchantments),
        });
    }
    let ((id, level), rest) = enchantments.split_first()?;
    Some(Plan::Split {
        kept: ((*id).to_owned(), *level),
        moved: owned(rest),
    })
}

/// What a take costs, by `DisEnchantingStrategy.hasEnoughExperience`'s rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Charge {
    /// Free: experience is not required, the cost is zero, or creative.
    Nothing,
    /// `Player.giveExperiencePoints(-cost)`.
    Points(u32),
    /// `Player.giveExperienceLevels(-cost)`.
    Levels(u32),
}

/// The charge a take at the player's side needs. Creative is free because
/// vanilla's `hasEnoughExperience` answers true there and the engine clamps the
/// points at 0.
#[must_use]
pub fn charge(settings: Settings, creative: bool) -> Charge {
    if creative || !settings.requires_experience || settings.experience_cost == 0 {
        return Charge::Nothing;
    }
    if settings.uses_points {
        Charge::Points(settings.experience_cost)
    } else {
        Charge::Levels(settings.experience_cost)
    }
}

/// `Player.getXpNeededForNextLevel`: the points from `level` to `level + 1`.
#[must_use]
pub fn points_to_next_level(level: u32) -> u32 {
    if level < 15 {
        7 + level * 2
    } else if level < 30 {
        37 + (level - 15) * 5
    } else {
        112 + (level - 30) * 9
    }
}

/// The points a player has banked once they reach `level` with no progress.
#[must_use]
pub fn points_at_level(level: u32) -> u64 {
    (0..level)
        .map(|each| u64::from(points_to_next_level(each)))
        .sum()
}

/// A player's total points from their own `experienceLevel` and
/// `experienceProgress` (`ExperienceHelper.getTotalPlayerExperiencePoints`,
/// with vanilla's thresholds).
#[must_use]
pub fn total_points(level: i32, progress: f32) -> u64 {
    let level = u32::try_from(level).unwrap_or(0);
    let next = u64::from(points_to_next_level(level));
    let extra = (f64::from(progress.clamp(0.0, 1.0)) * next as f64).round() as u64;
    points_at_level(level) + extra
}

/// The level and progress `points` buys (`ExperienceHelper.getLevelFromExperiencePoints`,
/// with vanilla's thresholds).
#[must_use]
pub fn level_after_points(points: u64) -> (u32, f32) {
    let mut level = 0_u32;
    let mut rest = points;
    loop {
        let next = u64::from(points_to_next_level(level));
        if rest < next {
            return (level, rest as f32 / next as f32);
        }
        rest -= next;
        level += 1;
    }
}

/// Whether the player's own `experienceLevel` / `experienceProgress` pay
/// `charge`.
#[must_use]
pub fn affordable(charge: Charge, xp_level: i32, xp_progress: f32) -> bool {
    match charge {
        Charge::Nothing => true,
        Charge::Points(cost) => total_points(xp_level, xp_progress) >= u64::from(cost),
        Charge::Levels(cost) => u32::try_from(xp_level).unwrap_or(0) >= cost,
    }
}

/// `Slot.mayPlace` for this mod's menu (`native_menu_slot_rules`): the input
/// takes only an item that may be dis-enchanted, the extra slot only a book,
/// the output nothing. Another mod's menu is left alone.
pub fn slot_rule(
    menu: &str,
    slot: usize,
    item: &str,
    enchantments: &[(&str, u32)],
    may_place: &mut bool,
    _max_stack: &mut u32,
) {
    if menu != TABLE_MENU {
        return;
    }
    match slot {
        INPUT_SLOT => *may_place = can_disenchant(item, enchantments),
        BOOK_SLOT => *may_place = is_book(item),
        OUTPUT_SLOT => *may_place = false,
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHARPNESS: &str = "minecraft:sharpness";
    const UNBREAKING: &str = "minecraft:unbreaking";

    #[test]
    fn only_an_item_with_enchantments_may_be_disenchanted() {
        assert!(!can_disenchant("minecraft:diamond_sword", &[]));
        assert!(can_disenchant("minecraft:diamond_sword", &[(SHARPNESS, 3)]));
        assert!(can_disenchant("minecraft:stick", &[(SHARPNESS, 1)]));
    }

    #[test]
    fn an_enchanted_book_needs_two_enchantments() {
        // Upstream refuses a one-enchantment book: nothing would be left.
        assert!(!can_disenchant(ENCHANTED_BOOK_ITEM, &[(SHARPNESS, 3)]));
        assert!(!can_disenchant(ENCHANTED_BOOK_ITEM, &[]));
        assert!(can_disenchant(
            ENCHANTED_BOOK_ITEM,
            &[(SHARPNESS, 3), (UNBREAKING, 2)]
        ));
    }

    #[test]
    fn an_item_gives_the_output_every_enchantment_and_keeps_none() {
        let plan = plan(
            "minecraft:diamond_sword",
            &[(SHARPNESS, 3), (UNBREAKING, 2)],
        );
        assert_eq!(
            plan,
            Some(Plan::Whole {
                moved: vec![(SHARPNESS.to_owned(), 3), (UNBREAKING.to_owned(), 2)],
            })
        );
    }

    #[test]
    fn a_book_keeps_its_first_enchantment_and_gives_the_rest() {
        let plan = plan(
            ENCHANTED_BOOK_ITEM,
            &[(SHARPNESS, 3), (UNBREAKING, 2), ("minecraft:mending", 1)],
        );
        assert_eq!(
            plan,
            Some(Plan::Split {
                kept: (SHARPNESS.to_owned(), 3),
                moved: vec![
                    (UNBREAKING.to_owned(), 2),
                    ("minecraft:mending".to_owned(), 1)
                ],
            })
        );
    }

    #[test]
    fn an_ineligible_input_has_no_plan() {
        assert_eq!(plan("minecraft:diamond_sword", &[]), None);
        assert_eq!(plan(ENCHANTED_BOOK_ITEM, &[(SHARPNESS, 1)]), None);
    }

    #[test]
    fn the_default_cost_is_twenty_five_points() {
        assert_eq!(charge(DEFAULT_SETTINGS, false), Charge::Points(25));
    }

    #[test]
    fn a_creative_player_pays_nothing() {
        // Upstream's `hasEnoughExperience` is always true in creative, and the
        // engine clamps the points at 0, so the take is free there.
        assert_eq!(charge(DEFAULT_SETTINGS, true), Charge::Nothing);
    }

    #[test]
    fn experience_can_be_switched_off_or_charged_in_levels() {
        let free = Settings {
            requires_experience: false,
            ..DEFAULT_SETTINGS
        };
        assert_eq!(charge(free, false), Charge::Nothing);
        let levels = Settings {
            uses_points: false,
            ..DEFAULT_SETTINGS
        };
        assert_eq!(charge(levels, false), Charge::Levels(25));
    }

    #[test]
    fn vanilla_level_thresholds_are_cumulative() {
        assert_eq!(points_to_next_level(0), 7);
        assert_eq!(points_to_next_level(14), 35);
        assert_eq!(points_to_next_level(15), 37);
        assert_eq!(points_to_next_level(29), 107);
        assert_eq!(points_to_next_level(30), 112);
        assert_eq!(points_at_level(0), 0);
        assert_eq!(points_at_level(1), 7);
        assert_eq!(points_at_level(9), 135);
        assert_eq!(points_at_level(15), 315);
        assert_eq!(points_at_level(16), 352);
        assert_eq!(points_at_level(30), 1395);
    }

    #[test]
    fn a_players_points_round_trip_through_the_level() {
        assert_eq!(total_points(0, 0.0), 0);
        assert_eq!(total_points(9, 0.0), 135);
        // 0.6 of the first level's 7 points is 4 points, rounded.
        assert_eq!(total_points(0, 0.6), 4);
        // 110 points is level 7 with 19 of its 21 points.
        let (level, progress) = level_after_points(110);
        assert_eq!(level, 7);
        assert_eq!(total_points(level as i32, progress), 110);
    }

    #[test]
    fn affordability_follows_points_or_levels() {
        let points = Charge::Points(25);
        assert!(affordable(points, 9, 0.0));
        assert!(!affordable(points, 0, 0.0));
        // 27 points at level 3, 16 at level 2.
        assert!(affordable(points, 3, 0.0));
        assert!(!affordable(points, 2, 0.0));
        assert!(affordable(Charge::Nothing, 0, 0.0));
        assert!(!affordable(Charge::Levels(25), 24, 0.9));
        assert!(affordable(Charge::Levels(25), 25, 0.0));
        // A negative level (a broken save) reads as zero.
        assert!(!affordable(points, -3, 0.0));
        assert!(!affordable(Charge::Levels(1), -3, 0.0));
    }

    #[test]
    fn the_input_slot_takes_only_something_worth_disenchanting() {
        let mut may_place = true;
        let mut max_stack = 99;
        slot_rule(
            TABLE_MENU,
            INPUT_SLOT,
            "minecraft:diamond_sword",
            &[(SHARPNESS, 3)],
            &mut may_place,
            &mut max_stack,
        );
        assert!(may_place);
        let mut may_place = true;
        slot_rule(
            TABLE_MENU,
            INPUT_SLOT,
            "minecraft:diamond_sword",
            &[],
            &mut may_place,
            &mut max_stack,
        );
        assert!(!may_place);
        // A one-enchantment book would leave nothing behind, so it is refused.
        assert!(!can_disenchant(ENCHANTED_BOOK_ITEM, &[(SHARPNESS, 1)]));
        assert_eq!(max_stack, 99);
    }

    #[test]
    fn the_book_slot_takes_only_books() {
        let mut may_place = true;
        let mut max_stack = 99;
        slot_rule(
            TABLE_MENU,
            BOOK_SLOT,
            BOOK_ITEM,
            &[],
            &mut may_place,
            &mut max_stack,
        );
        assert!(may_place);
        let mut may_place = true;
        slot_rule(
            TABLE_MENU,
            BOOK_SLOT,
            "minecraft:writable_book",
            &[],
            &mut may_place,
            &mut max_stack,
        );
        assert!(!may_place);
    }

    #[test]
    fn the_output_slot_takes_nothing_and_another_menu_is_untouched() {
        let mut may_place = true;
        let mut max_stack = 99;
        slot_rule(
            TABLE_MENU,
            OUTPUT_SLOT,
            BOOK_ITEM,
            &[],
            &mut may_place,
            &mut max_stack,
        );
        assert!(!may_place);
        let mut may_place = true;
        slot_rule(
            "botanypots:basic_pot_menu",
            INPUT_SLOT,
            "minecraft:stick",
            &[],
            &mut may_place,
            &mut max_stack,
        );
        assert!(may_place, "another mod's menu must be left alone");
    }

    #[test]
    fn the_block_and_menu_ids_are_the_jar_s_own() {
        assert!(is_table(TABLE_BLOCK));
        assert!(!is_table("minecraft:enchanting_table"));
        assert!(!is_table("disenchanting_table:something_else"));
    }
}
