//! Sourdough starters: the pets. Two hidden stats — Pep (liveliness) and Tang (the
//! microbiome dial) — plus a discard jar that feeds the treat economy.

use crate::art::Expr;
use crate::art::jar::JarView;
use crate::content::Flour;
use crate::ink::Ink;
use serde::{Deserialize, Serialize};

/// Cap colours a jar can wear (cosmetic).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Cloth {
    Pink,
    Blue,
    Yellow,
}

impl Cloth {
    pub fn ink(self) -> Ink {
        match self {
            Cloth::Pink => Ink::Pink,
            Cloth::Blue => Ink::Blue,
            Cloth::Yellow => Ink::Yellow,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Starter {
    pub name: String,
    /// 0..100 liveliness: rise, bubbles, oven spring.
    pub pep: f32,
    /// 0..100 mild → tangy. Yeasties vs Lactos.
    pub tang: f32,
    /// Visual fill level 0..1.
    pub rise: f32,
    /// Rubber band: level right after the last feed.
    pub band: f32,
    pub hooch: bool,
    pub fed_today: bool,
    pub hungry_nights: u32,
    pub last_flour: Option<Flour>,
    pub cloth: Cloth,
    pub seed: u32,
}

/// What happened when a starter was fed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FeedResult {
    pub pep_before: f32,
    pub pep_after: f32,
    pub tang_before: f32,
    pub tang_after: f32,
    pub revived: bool,
}

pub const PRESET_NAMES: [&str; 12] = [
    "Bubbles",
    "Mochi",
    "Doughby",
    "Puddle",
    "Sir Rise",
    "Blossom",
    "Gloop",
    "Pip-squeak",
    "Marsh",
    "Nubbin",
    "Tofu",
    "Clementine",
];

impl Starter {
    pub fn new(name: &str, seed: u32, cloth: Cloth) -> Starter {
        Starter {
            name: name.to_string(),
            pep: 72.0,
            tang: 35.0,
            rise: 0.6,
            band: 0.3,
            hooch: false,
            fed_today: false,
            hungry_nights: 0,
            last_flour: Some(Flour::White),
            cloth,
            seed,
        }
    }

    /// Feed the starter. `stir` 0..1 is how well it was stirred.
    pub fn feed(&mut self, flour: Flour, stir: f32) -> FeedResult {
        let stir = stir.clamp(0.0, 1.0);
        let (pep_before, tang_before) = (self.pep, self.tang);
        let revived = self.hooch || self.hungry_nights > 0;
        let (pep_gain, tang_delta) = match flour {
            Flour::White => (30.0, -12.0),
            Flour::Wheat => (32.0, 3.0),
            Flour::Rye => (38.0, 13.0),
        };
        self.pep = (self.pep + pep_gain + 10.0 * stir).min(100.0);
        self.tang = (self.tang + tang_delta).clamp(0.0, 100.0);
        self.hooch = false;
        self.hungry_nights = 0;
        self.fed_today = true;
        self.last_flour = Some(flour);
        // Discard half, top up: the level drops back to the band.
        self.band = 0.28;
        self.rise = 0.28;
        FeedResult { pep_before, pep_after: self.pep, tang_before, tang_after: self.tang, revived }
    }

    /// Advance one night.
    pub fn overnight(&mut self) {
        if self.fed_today {
            // Rose to a lovely dome overnight, then settled a touch.
            self.rise = (self.band + 0.7 * self.pep / 100.0).clamp(0.2, 1.0);
            self.pep = (self.pep - 8.0).max(0.0);
        } else {
            self.hungry_nights += 1;
            self.pep = (self.pep - 28.0).max(5.0);
            self.tang = (self.tang + 8.0).min(100.0);
            self.hooch = true;
            self.rise = (self.rise - 0.3).max(0.15);
        }
        self.fed_today = false;
    }

    pub fn mood(&self) -> Expr {
        if self.hooch {
            Expr::Sleepy
        } else if self.fed_today {
            Expr::Happy
        } else if self.pep < 35.0 {
            Expr::Hungry
        } else if self.pep > 80.0 {
            Expr::Excited
        } else {
            Expr::Content
        }
    }

    /// 0..1 contribution to dough strength.
    pub fn strength(&self) -> f32 {
        (self.pep / 100.0).clamp(0.0, 1.0)
    }

    pub fn view(&self, t: f32) -> JarView {
        JarView {
            pep: self.pep / 100.0,
            tang: self.tang / 100.0,
            rise: self.rise,
            band: self.band,
            hooch: self.hooch,
            expr: self.mood(),
            t,
            seed: self.seed,
            cloth: self.cloth.ink(),
        }
    }

    /// Short, friendly status line.
    pub fn status(&self) -> &'static str {
        if self.hooch {
            "Sleepy & hungry — feed me!"
        } else if self.fed_today {
            "Fed and bubbling"
        } else if self.pep > 80.0 {
            "Peak pep!"
        } else if self.pep > 50.0 {
            "Happy and lively"
        } else {
            "A bit peckish"
        }
    }

    pub fn tang_word(&self) -> &'static str {
        if self.tang >= 60.0 {
            "tangy"
        } else if self.tang <= 30.0 {
            "mild"
        } else {
            "balanced"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rye_is_tangy_white_is_mild() {
        let mut a = Starter::new("a", 1, Cloth::Pink);
        let mut b = a.clone();
        for _ in 0..4 {
            a.feed(Flour::Rye, 1.0);
            a.overnight();
            b.feed(Flour::White, 1.0);
            b.overnight();
        }
        assert!(a.tang > 70.0, "{}", a.tang);
        assert!(b.tang < 10.0, "{}", b.tang);
    }

    #[test]
    fn neglect_makes_hooch_but_never_kills() {
        let mut s = Starter::new("s", 1, Cloth::Pink);
        for _ in 0..30 {
            s.overnight();
        }
        assert!(s.hooch);
        assert!(s.pep >= 5.0);
        assert_eq!(s.mood(), Expr::Sleepy);
        let r = s.feed(Flour::Rye, 1.0);
        assert!(r.revived);
        assert!(!s.hooch);
        s.overnight();
        s.feed(Flour::Rye, 1.0);
        assert!(s.pep > 60.0, "two feeds revive: {}", s.pep);
    }

    #[test]
    fn daily_feeding_keeps_pep_high_and_rises_overnight() {
        let mut s = Starter::new("s", 1, Cloth::Pink);
        for _ in 0..10 {
            s.feed(Flour::Wheat, 0.8);
            s.overnight();
        }
        assert!(s.pep > 80.0, "{}", s.pep);
        assert!(s.rise > s.band + 0.4);
    }
}
