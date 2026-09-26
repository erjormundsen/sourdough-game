//! Customers, their orders ("wants") and how much they love what you hand them.

use crate::bake::{CrustLevel, Good};
use crate::content::{Flour, Pattern, Recipe, Shape, Species, Stencil, Topping, Treat};
use crate::economy::Unlock;
use crate::rng::Rng;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Want {
    Recipe(Recipe),
    Shape(Shape),
    Tangy,
    Mild,
    Crust(CrustLevel),
    Pattern(Pattern),
    Stencil(Stencil),
    Topping(Topping),
    Treat(Treat),
    BigEar,
    /// Something they've never had from you.
    Surprise,
    AnyLoaf,
}

impl Want {
    pub fn label(&self) -> String {
        match self {
            Want::Recipe(r) => r.name().to_string(),
            Want::Shape(s) => format!("A {}", s.name().to_lowercase()),
            Want::Tangy => "Something tangy!".into(),
            Want::Mild => "Something mild".into(),
            Want::Crust(c) => format!("{} crust", c.name()),
            Want::Pattern(p) => format!("{} score", p.name()),
            Want::Stencil(s) => format!("{} stencil", s.name()),
            Want::Topping(t) => format!("{} on top", t.name()),
            Want::Treat(t) => format!("A {}", t.name().to_lowercase()),
            Want::BigEar => "A big proud ear".into(),
            Want::Surprise => "Surprise me!".into(),
            Want::AnyLoaf => "Any loaf, please".into(),
        }
    }

    /// The unlock this want depends on, if any.
    pub fn needs(&self) -> Option<Unlock> {
        match self {
            Want::Recipe(r) => Some(Unlock::Recipe(*r)),
            Want::Pattern(p) => Some(Unlock::Pattern(*p)),
            Want::Stencil(s) => Some(Unlock::Stencil(*s)),
            Want::Topping(t) => Some(Unlock::Topping(*t)),
            Want::Treat(t) => Some(Unlock::Treat(*t)),
            _ => None,
        }
    }

    pub fn wants_treat(&self) -> bool {
        matches!(self, Want::Treat(_))
    }
}

pub struct Profile {
    pub species: Species,
    pub name: &'static str,
    pub likes: &'static [(Want, u32)],
    pub hello: &'static [&'static str],
    /// Picky critics always ask for two things and tip more.
    pub critic: bool,
    pub gifts: [Unlock; 2],
}

pub const PROFILES: [Profile; 8] = [
    Profile {
        species: Species::Bunny,
        name: "Mimi",
        likes: &[
            (Want::Stencil(Stencil::Heart), 3),
            (Want::Stencil(Stencil::Bunny), 3),
            (Want::Mild, 2),
            (Want::Topping(Topping::Sesame), 2),
            (Want::Treat(Treat::Muffin), 2),
            (Want::Crust(CrustLevel::Blonde), 1),
        ],
        hello: &[
            "Hi hi! Something sweet and gentle?",
            "Ooh, it smells like clouds in here!",
            "Hello! I brought my basket!",
        ],
        critic: false,
        gifts: [
            Unlock::Stencil(Stencil::Bunny),
            Unlock::Topping(Topping::Poppy),
        ],
    },
    Profile {
        species: Species::Bear,
        name: "Bruno",
        likes: &[
            (Want::Crust(CrustLevel::Bold), 3),
            (Want::BigEar, 3),
            (Want::Recipe(Recipe::DarkRye), 2),
            (Want::Treat(Treat::CinnamonBun), 2),
            (Want::Tangy, 1),
        ],
        hello: &[
            "Hrrm. The crustier, the better.",
            "Morning! Something hearty?",
            "I could eat a whole loaf. Two, even.",
        ],
        critic: false,
        gifts: [Unlock::Lid, Unlock::Recipe(Recipe::DarkRye)],
    },
    Profile {
        species: Species::Duck,
        name: "Pip",
        likes: &[
            (Want::Tangy, 3),
            (Want::Shape(Shape::Boule), 2),
            (Want::Topping(Topping::Oats), 2),
            (Want::Recipe(Recipe::Country), 2),
            (Want::Pattern(Pattern::Cross), 1),
        ],
        hello: &[
            "Quack! Is it sour? I love sour!",
            "Hi! Puddle-hopping made me hungry.",
            "Do you have the tangy stuff?",
        ],
        critic: false,
        gifts: [Unlock::Topping(Topping::Oats), Unlock::Flour(Flour::Wheat)],
    },
    Profile {
        species: Species::Cat,
        name: "Sir Whiskers",
        likes: &[
            (Want::Pattern(Pattern::Wheat), 3),
            (Want::Pattern(Pattern::Leaf), 3),
            (Want::Pattern(Pattern::Ear), 2),
            (Want::BigEar, 2),
            (Want::Crust(CrustLevel::Golden), 2),
        ],
        hello: &[
            "I review bakeries. Impress me.",
            "Hmm. Let us see your blade work.",
            "One loaf. Exquisite, if you please.",
        ],
        critic: true,
        gifts: [
            Unlock::Pattern(Pattern::Wheat),
            Unlock::Pattern(Pattern::Leaf),
        ],
    },
    Profile {
        species: Species::Hedgehog,
        name: "Hazel",
        likes: &[
            (Want::Recipe(Recipe::WholeWheat), 3),
            (Want::Topping(Topping::Poppy), 2),
            (Want::Topping(Topping::Sesame), 2),
            (Want::Pattern(Pattern::Wheat), 2),
            (Want::Stencil(Stencil::Star), 2),
        ],
        hello: &[
            "Hello dear! Anything seedy today?",
            "My prickles are for show, I promise.",
            "Wholesome and crunchy, please!",
        ],
        critic: false,
        gifts: [
            Unlock::Stencil(Stencil::Star),
            Unlock::Recipe(Recipe::WholeWheat),
        ],
    },
    Profile {
        species: Species::Frog,
        name: "Momo",
        likes: &[
            (Want::Surprise, 4),
            (Want::Stencil(Stencil::Sun), 2),
            (Want::Recipe(Recipe::Olive), 2),
            (Want::Tangy, 1),
        ],
        hello: &[
            "Ribbit! Surprise me!",
            "Something I've never had!",
            "I'm feeling adventurous today!",
        ],
        critic: false,
        gifts: [Unlock::Stencil(Stencil::Sun), Unlock::Recipe(Recipe::Olive)],
    },
    Profile {
        species: Species::Sheep,
        name: "Clover",
        likes: &[
            (Want::Crust(CrustLevel::Blonde), 3),
            (Want::Mild, 3),
            (Want::Recipe(Recipe::Cheddar), 2),
            (Want::Shape(Shape::Boule), 1),
            (Want::Stencil(Stencil::Heart), 1),
        ],
        hello: &[
            "Baa... something soft, please.",
            "Hello! Nothing too crunchy, hehe.",
            "Mild and fluffy, like me!",
        ],
        critic: false,
        gifts: [
            Unlock::Recipe(Recipe::Cheddar),
            Unlock::Treat(Treat::Muffin),
        ],
    },
    Profile {
        species: Species::Otter,
        name: "Otto",
        likes: &[
            (Want::Treat(Treat::Bagel), 3),
            (Want::Treat(Treat::CinnamonBun), 2),
            (Want::Recipe(Recipe::CranberryWalnut), 2),
            (Want::Shape(Shape::Batard), 2),
            (Want::Tangy, 1),
        ],
        hello: &[
            "Hey hey! Got anything with a hole in it?",
            "Snack time! Snack time!",
            "I'll float home with a treat.",
        ],
        critic: false,
        gifts: [
            Unlock::Treat(Treat::Bagel),
            Unlock::Recipe(Recipe::CranberryWalnut),
        ],
    },
];

pub fn profile(s: Species) -> &'static Profile {
    PROFILES.iter().find(|p| p.species == s).expect("profile")
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Order {
    pub wants: Vec<Want>,
}

impl Order {
    pub fn label(&self) -> String {
        self.wants
            .iter()
            .map(|w| w.label())
            .collect::<Vec<_>>()
            .join(" + ")
    }
}

/// Build an order from a customer's likes, only using wants the player can fulfil.
pub fn make_order(s: Species, rng: &mut Rng, can: impl Fn(&Want) -> bool, day: u32) -> Order {
    let p = profile(s);
    let options: Vec<(Want, u32)> = p.likes.iter().copied().filter(|(w, _)| can(w)).collect();
    let n = if p.critic || (day >= 6 && rng.chance(0.35)) {
        2
    } else {
        1
    };
    let mut wants = Vec::new();
    let mut pool = options;
    for _ in 0..n {
        let total: u32 = pool.iter().map(|(_, w)| *w).sum();
        if total == 0 {
            break;
        }
        let mut roll = rng.below(total);
        let mut pick = 0;
        for (i, (_, w)) in pool.iter().enumerate() {
            if roll < *w {
                pick = i;
                break;
            }
            roll -= w;
        }
        let (w, _) = pool.remove(pick);
        // Never mix a treat with loaf-only wants.
        if wants.iter().any(Want::wants_treat) || (w.wants_treat() && !wants.is_empty()) {
            continue;
        }
        wants.push(w);
    }
    if wants.is_empty() {
        wants.push(Want::AnyLoaf);
    }
    Order { wants }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Reaction {
    /// Over the moon.
    Love,
    Happy,
    /// Polite smile.
    Okay,
    /// Nothing handed over.
    Sorry,
}

/// Loaf signature used for "surprise me".
pub fn signature(g: &Good) -> String {
    match g {
        Good::Loaf(l) => format!(
            "{:?}/{:?}/{:?}/{:?}",
            l.recipe, l.pattern, l.stencil, l.topping
        ),
        Good::Treat(t) => format!("{:?}", t.kind),
    }
}

/// How well `good` satisfies one want (0..1).
pub fn want_score(w: &Want, good: &Good, history: &[String]) -> f32 {
    let loaf = match good {
        Good::Loaf(l) => Some(l),
        Good::Treat(_) => None,
    };
    match (w, good) {
        (Want::Treat(t), Good::Treat(ti)) => {
            if ti.kind == *t {
                1.0
            } else {
                0.35
            }
        }
        (Want::Treat(_), Good::Loaf(_)) => 0.25,
        (_, Good::Treat(_)) => match w {
            Want::Surprise if !history.contains(&signature(good)) => 0.8,
            _ => 0.2,
        },
        _ => {
            let l = loaf.unwrap();
            match w {
                Want::Recipe(r) => (l.recipe == *r) as u8 as f32,
                Want::Shape(s) => (l.shape == *s) as u8 as f32,
                Want::Tangy => {
                    if l.tang >= 0.5 {
                        1.0
                    } else {
                        0.7 * (l.tang / 0.5).powi(2)
                    }
                }
                Want::Mild => {
                    if l.tang <= 0.35 {
                        1.0
                    } else {
                        (1.0 - (l.tang - 0.35) / 0.35).clamp(0.0, 1.0) * 0.7
                    }
                }
                Want::Crust(c) => {
                    let got = l.crust_level();
                    if got == *c {
                        1.0
                    } else if (got as i32 - *c as i32).abs() == 1 {
                        0.4
                    } else {
                        0.1
                    }
                }
                Want::Pattern(p) => {
                    if l.pattern == Some(*p) {
                        0.6 + 0.4 * l.pattern_match
                    } else {
                        0.1
                    }
                }
                Want::Stencil(s) => (l.stencil == Some(*s)) as u8 as f32,
                Want::Topping(t) => (l.topping == Some(*t)) as u8 as f32,
                Want::BigEar => (l.big_ear() / 0.65).clamp(0.0, 1.0),
                Want::Surprise => {
                    if history.contains(&signature(good)) {
                        0.35 + 0.4 * l.looks
                    } else {
                        1.0
                    }
                }
                Want::AnyLoaf => 1.0,
                Want::Treat(_) => unreachable!(),
            }
        }
    }
}

/// Overall satisfaction 0..1: mostly the order, a little the quality.
pub fn satisfaction(order: &Order, good: &Good, history: &[String]) -> f32 {
    let n = order.wants.len().max(1) as f32;
    let fit: f32 = order
        .wants
        .iter()
        .map(|w| want_score(w, good, history))
        .sum::<f32>()
        / n;
    (0.72 * fit + 0.28 * good.quality()).clamp(0.0, 1.0)
}

pub fn reaction_for(sat: f32) -> Reaction {
    if sat >= 0.82 {
        Reaction::Love
    } else if sat >= 0.55 {
        Reaction::Happy
    } else {
        Reaction::Okay
    }
}

/// Coins paid: always at least most of the price, more (plus tips) when delighted.
pub fn payment(good: &Good, sat: f32, critic: bool, preorder: bool) -> (u32, u32) {
    let base = good.price() as f32 * (0.7 + 0.45 * sat);
    let mut tip = match reaction_for(sat) {
        Reaction::Love => 3.0 + 3.0 * sat,
        Reaction::Happy => 1.0,
        _ => 0.0,
    };
    if critic {
        tip *= 2.0;
    }
    if preorder {
        tip += 2.0;
    }
    (base.round() as u32, tip.round() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bake::{Loaf, TreatItem};
    use crate::content::{Recipe, Shape, Stencil, Treat};

    fn loaf() -> Loaf {
        Loaf {
            id: 1,
            recipe: Recipe::Country,
            shape: Shape::Boule,
            spring: 0.8,
            crust: 0.9,
            tang: 0.6,
            openness: 0.7,
            cuts: vec![],
            stencil: Some(Stencil::Heart),
            topping: None,
            pattern: None,
            pattern_match: 0.0,
            looks: 0.7,
            quality: 0.8,
            stars: 3,
            starter: "B".into(),
            day: 1,
            seed: 1,
        }
    }

    #[test]
    fn matching_order_is_loved_and_always_pays() {
        let g = Good::Loaf(loaf());
        let o = Order {
            wants: vec![Want::Crust(CrustLevel::Bold)],
        };
        let s = satisfaction(&o, &g, &[]);
        assert_eq!(reaction_for(s), Reaction::Love);
        let wrong = Order {
            wants: vec![Want::Treat(Treat::Bagel)],
        };
        let s2 = satisfaction(&wrong, &g, &[]);
        assert!(s2 < 0.55);
        let (coins, _) = payment(&g, s2, false, false);
        assert!(coins as f32 >= g.price() as f32 * 0.7 - 0.5);
    }

    #[test]
    fn surprise_prefers_new_things() {
        let g = Good::Loaf(loaf());
        let new = want_score(&Want::Surprise, &g, &[]);
        let old = want_score(&Want::Surprise, &g, &[signature(&g)]);
        assert!(new > old);
        let t = Good::Treat(TreatItem {
            kind: Treat::Bagel,
            quality: 0.8,
            seed: 1,
        });
        assert_eq!(want_score(&Want::Treat(Treat::Bagel), &t, &[]), 1.0);
    }

    #[test]
    fn orders_respect_availability() {
        let mut rng = Rng::new(5);
        for _ in 0..200 {
            let o = make_order(Species::Bunny, &mut rng, |w| w.needs().is_none(), 1);
            assert!(o.wants.iter().all(|w| w.needs().is_none()), "{o:?}");
        }
        let o = make_order(Species::Cat, &mut rng, |_| true, 1);
        assert!(!o.wants.is_empty());
    }
}
