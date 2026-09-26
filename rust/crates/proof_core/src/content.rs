//! Static game content: flours, recipes, decorations, goods and customers.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Flour {
    White,
    Wheat,
    Rye,
}

impl Flour {
    pub const ALL: [Flour; 3] = [Flour::White, Flour::Wheat, Flour::Rye];
    pub fn name(self) -> &'static str {
        match self {
            Flour::White => "White",
            Flour::Wheat => "Whole Wheat",
            Flour::Rye => "Rye",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Inclusion {
    Olive,
    Cranberry,
    Cheddar,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Recipe {
    Country,
    WholeWheat,
    DarkRye,
    Olive,
    CranberryWalnut,
    Cheddar,
}

impl Recipe {
    pub const ALL: [Recipe; 6] = [
        Recipe::Country,
        Recipe::WholeWheat,
        Recipe::DarkRye,
        Recipe::Olive,
        Recipe::CranberryWalnut,
        Recipe::Cheddar,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Recipe::Country => "Country Loaf",
            Recipe::WholeWheat => "Whole Wheat",
            Recipe::DarkRye => "Dark Rye",
            Recipe::Olive => "Olive & Herb",
            Recipe::CranberryWalnut => "Cranberry Walnut",
            Recipe::Cheddar => "Cheddar Crackle",
        }
    }
    pub fn flour(self) -> Flour {
        match self {
            Recipe::WholeWheat => Flour::Wheat,
            Recipe::DarkRye => Flour::Rye,
            _ => Flour::White,
        }
    }
    pub fn inclusion(self) -> Option<Inclusion> {
        match self {
            Recipe::Olive => Some(Inclusion::Olive),
            Recipe::CranberryWalnut => Some(Inclusion::Cranberry),
            Recipe::Cheddar => Some(Inclusion::Cheddar),
            _ => None,
        }
    }
    /// Base price in coins.
    pub fn price(self) -> u32 {
        match self {
            Recipe::Country => 8,
            Recipe::WholeWheat => 9,
            Recipe::DarkRye => 10,
            Recipe::Olive | Recipe::Cheddar => 12,
            Recipe::CranberryWalnut => 13,
        }
    }
    /// Recipe's own contribution to tang (rye is sour, fruit is sweet).
    pub fn tang_bias(self) -> f32 {
        match self {
            Recipe::DarkRye => 0.2,
            Recipe::WholeWheat => 0.08,
            Recipe::CranberryWalnut => -0.1,
            _ => 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Shape {
    Boule,
    Batard,
}

impl Shape {
    /// Radii (x, y) of the loaf outline relative to a unit size.
    pub fn radii(self) -> (f32, f32) {
        match self {
            Shape::Boule => (1.0, 1.0),
            Shape::Batard => (1.28, 0.78),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Shape::Boule => "Boule",
            Shape::Batard => "Bâtard",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Stencil {
    Heart,
    Star,
    Sun,
    Bunny,
}

impl Stencil {
    pub const ALL: [Stencil; 4] = [Stencil::Heart, Stencil::Star, Stencil::Sun, Stencil::Bunny];
    pub fn name(self) -> &'static str {
        match self {
            Stencil::Heart => "Heart",
            Stencil::Star => "Star",
            Stencil::Sun => "Sunny",
            Stencil::Bunny => "Bunny",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Topping {
    Sesame,
    Oats,
    Poppy,
}

impl Topping {
    pub const ALL: [Topping; 3] = [Topping::Sesame, Topping::Oats, Topping::Poppy];
    pub fn name(self) -> &'static str {
        match self {
            Topping::Sesame => "Sesame",
            Topping::Oats => "Oats",
            Topping::Poppy => "Poppy",
        }
    }
}

/// Scoring patterns (faint guides + freehand recognition targets).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Pattern {
    Ear,
    Cross,
    Wheat,
    Leaf,
}

impl Pattern {
    pub const ALL: [Pattern; 4] = [Pattern::Ear, Pattern::Cross, Pattern::Wheat, Pattern::Leaf];
    pub fn name(self) -> &'static str {
        match self {
            Pattern::Ear => "Big Ear",
            Pattern::Cross => "Cross",
            Pattern::Wheat => "Wheat Stalk",
            Pattern::Leaf => "Leaf",
        }
    }
}

/// Secondary goods made by the tray from discard.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Treat {
    Muffin,
    CinnamonBun,
    Bagel,
}

impl Treat {
    pub const ALL: [Treat; 3] = [Treat::Muffin, Treat::CinnamonBun, Treat::Bagel];
    pub fn name(self) -> &'static str {
        match self {
            Treat::Muffin => "Discard Muffin",
            Treat::CinnamonBun => "Cinnamon Bun",
            Treat::Bagel => "Bagel",
        }
    }
    pub fn price(self) -> u32 {
        match self {
            Treat::Muffin => 4,
            Treat::CinnamonBun => 6,
            Treat::Bagel => 5,
        }
    }
    /// How many come out of one tray.
    pub fn per_tray(self) -> u32 {
        match self {
            Treat::Muffin => 4,
            Treat::CinnamonBun => 4,
            Treat::Bagel => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Species {
    Bunny,
    Bear,
    Duck,
    Cat,
    Hedgehog,
    Frog,
    Sheep,
    Otter,
}

impl Species {
    pub const ALL: [Species; 8] = [
        Species::Bunny,
        Species::Bear,
        Species::Duck,
        Species::Cat,
        Species::Hedgehog,
        Species::Frog,
        Species::Sheep,
        Species::Otter,
    ];
}
