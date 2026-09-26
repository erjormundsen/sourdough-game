//! Unlocks, the catalog on the night page, and bakery levels.

use crate::content::{Flour, Pattern, Recipe, Species, Stencil, Topping, Treat};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Unlock {
    Recipe(Recipe),
    Flour(Flour),
    Stencil(Stencil),
    Topping(Topping),
    Pattern(Pattern),
    Treat(Treat),
    /// Fridge grows to this many banneton slots.
    Fridge(u32),
    /// Another starter jar (the n-th).
    Jar(u32),
    /// Dutch-oven lid: steam for extra spring.
    Lid,
}

impl Unlock {
    pub fn name(&self) -> String {
        match self {
            Unlock::Recipe(r) => format!("{} recipe", r.name()),
            Unlock::Flour(f) => format!("{} flour", f.name()),
            Unlock::Stencil(s) => format!("{} stencil", s.name()),
            Unlock::Topping(t) => format!("{} topping", t.name()),
            Unlock::Pattern(p) => format!("{} guide", p.name()),
            Unlock::Treat(t) => format!("{} tray", t.name()),
            Unlock::Fridge(n) => format!("Fridge: {n} bannetons"),
            Unlock::Jar(n) => format!("Starter jar #{n}"),
            Unlock::Lid => "Dutch-oven lid".to_string(),
        }
    }
    pub fn blurb(&self) -> &'static str {
        match self {
            Unlock::Recipe(_) => "A new loaf for the shelf.",
            Unlock::Flour(Flour::Rye) => "Feeds the Lactos: tangier starter.",
            Unlock::Flour(Flour::Wheat) => "Balanced feed, lots of pep.",
            Unlock::Flour(Flour::White) => "Feeds the Yeasties: mild starter.",
            Unlock::Stencil(_) => "Dust flour art onto your loaves.",
            Unlock::Topping(_) => "Crunchy seeds for the crust.",
            Unlock::Pattern(_) => "A faint guide for your blade.",
            Unlock::Treat(_) => "Turn discard into treats.",
            Unlock::Fridge(_) => "Prep more loaves each evening.",
            Unlock::Jar(_) => "Adopt another starter pet.",
            Unlock::Lid => "Steam! Bigger ears, taller loaves.",
        }
    }
}

/// Everything unlocked when a new game starts.
pub const STARTING: [Unlock; 8] = [
    Unlock::Recipe(Recipe::Country),
    Unlock::Flour(Flour::White),
    Unlock::Flour(Flour::Rye),
    Unlock::Stencil(Stencil::Heart),
    Unlock::Topping(Topping::Sesame),
    Unlock::Pattern(Pattern::Ear),
    Unlock::Pattern(Pattern::Cross),
    Unlock::Fridge(2),
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CatalogItem {
    pub unlock: Unlock,
    pub cost: u32,
    pub level: u32,
}

const fn item(unlock: Unlock, cost: u32, level: u32) -> CatalogItem {
    CatalogItem { unlock, cost, level }
}

pub const CATALOG: [CatalogItem; 21] = [
    item(Unlock::Recipe(Recipe::WholeWheat), 25, 1),
    item(Unlock::Flour(Flour::Wheat), 15, 1),
    item(Unlock::Topping(Topping::Oats), 15, 1),
    item(Unlock::Fridge(4), 40, 1),
    item(Unlock::Treat(Treat::Muffin), 25, 2),
    item(Unlock::Stencil(Stencil::Star), 20, 2),
    item(Unlock::Pattern(Pattern::Wheat), 25, 2),
    item(Unlock::Recipe(Recipe::DarkRye), 35, 3),
    item(Unlock::Jar(2), 50, 3),
    item(Unlock::Topping(Topping::Poppy), 15, 3),
    item(Unlock::Recipe(Recipe::Olive), 45, 4),
    item(Unlock::Treat(Treat::CinnamonBun), 40, 4),
    item(Unlock::Stencil(Stencil::Sun), 25, 4),
    item(Unlock::Pattern(Pattern::Leaf), 35, 4),
    item(Unlock::Recipe(Recipe::CranberryWalnut), 55, 5),
    item(Unlock::Treat(Treat::Bagel), 40, 5),
    item(Unlock::Fridge(6), 90, 5),
    item(Unlock::Lid, 60, 5),
    item(Unlock::Recipe(Recipe::Cheddar), 55, 6),
    item(Unlock::Stencil(Stencil::Bunny), 30, 6),
    item(Unlock::Jar(3), 110, 6),
];

/// XP needed to reach each level (index = level - 1).
pub const LEVELS: [u32; 8] = [0, 24, 70, 135, 220, 320, 440, 580];

pub fn level_for(xp: u32) -> u32 {
    LEVELS.iter().rposition(|&t| xp >= t).map(|i| i as u32 + 1).unwrap_or(1)
}

/// XP remaining to the next level (None at max).
pub fn xp_to_next(xp: u32) -> Option<(u32, u32)> {
    let lvl = level_for(xp) as usize;
    LEVELS.get(lvl).map(|&next| (xp - LEVELS[lvl - 1], next - LEVELS[lvl - 1]))
}

/// Customers join the regulars as the bakery grows.
pub fn customer_level(s: Species) -> u32 {
    match s {
        Species::Bunny | Species::Bear | Species::Duck => 1,
        Species::Hedgehog => 2,
        Species::Cat | Species::Frog => 3,
        Species::Sheep => 4,
        Species::Otter => 5,
    }
}

/// Catalog entries visible at `level`.
pub fn catalog(level: u32) -> impl Iterator<Item = &'static CatalogItem> {
    CATALOG.iter().filter(move |c| c.level <= level)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_are_monotonic() {
        assert_eq!(level_for(0), 1);
        assert_eq!(level_for(23), 1);
        assert_eq!(level_for(24), 2);
        assert_eq!(level_for(10_000), LEVELS.len() as u32);
        assert!(LEVELS.windows(2).all(|w| w[0] < w[1]));
        assert_eq!(xp_to_next(30), Some((6, 46)));
    }

    #[test]
    fn every_content_item_is_reachable() {
        use std::collections::HashSet;
        let mut all: HashSet<Unlock> = STARTING.iter().copied().collect();
        all.extend(CATALOG.iter().filter(|c| c.level <= LEVELS.len() as u32).map(|c| c.unlock));
        for r in Recipe::ALL {
            assert!(all.contains(&Unlock::Recipe(r)), "{r:?}");
        }
        for s in Stencil::ALL {
            assert!(all.contains(&Unlock::Stencil(s)), "{s:?}");
        }
        for t in Treat::ALL {
            assert!(all.contains(&Unlock::Treat(t)), "{t:?}");
        }
        for p in Pattern::ALL {
            assert!(all.contains(&Unlock::Pattern(p)), "{p:?}");
        }
    }
}
