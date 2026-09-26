//! Doughs in the fridge, the oven, and the loaves & treats that come out.

use crate::art::bread::{CutView, LoafView};
use crate::content::{Pattern, Recipe, Shape, Stencil, Topping, Treat};
use crate::geom::{V2, polyline_len};
use crate::scoring::{self, ScoreReport};
use serde::{Deserialize, Serialize};

/// A shaped dough resting in a banneton in the fridge.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Dough {
    pub id: u32,
    pub recipe: Recipe,
    pub shape: Shape,
    /// 0..1 from the starter's pep at mix time (minus waiting too long).
    pub strength: f32,
    /// 0..1 flavour tang.
    pub tang: f32,
    /// 0..1 stretch & fold quality.
    pub fold: f32,
    pub starter: String,
    pub made_day: u32,
    pub stencil: Option<Stencil>,
    pub topping: Option<Topping>,
    pub cuts: Vec<Vec<V2>>,
    pub guide: Option<Pattern>,
    pub score: Option<ScoreReport>,
    pub seed: u32,
}

impl Dough {
    pub fn view(&self, r: f32) -> LoafView {
        LoafView {
            shape: self.shape,
            recipe: self.recipe,
            r,
            bake: 0.0,
            spring: 0.0,
            crust: 0.0,
            cuts: self.cuts.iter().map(|pts| CutView { pts: pts.clone(), bloom: 0.0, ear: 0.0 }).collect(),
            stencil: self.stencil,
            topping: self.topping,
            seed: self.seed,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CrustLevel {
    Blonde,
    Golden,
    Bold,
}

impl CrustLevel {
    pub const ALL: [CrustLevel; 3] = [CrustLevel::Blonde, CrustLevel::Golden, CrustLevel::Bold];
    pub fn of(shade: f32) -> CrustLevel {
        if shade < 0.38 {
            CrustLevel::Blonde
        } else if shade < 0.7 {
            CrustLevel::Golden
        } else {
            CrustLevel::Bold
        }
    }
    /// Representative shade for icons.
    pub fn shade(self) -> f32 {
        match self {
            CrustLevel::Blonde => 0.2,
            CrustLevel::Golden => 0.55,
            CrustLevel::Bold => 0.9,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            CrustLevel::Blonde => "Blonde",
            CrustLevel::Golden => "Golden",
            CrustLevel::Bold => "Bold",
        }
    }
}

/// One baked cut: geometry + how it opened.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cut {
    pub pts: Vec<V2>,
    pub bloom: f32,
    pub ear: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Loaf {
    pub id: u32,
    pub recipe: Recipe,
    pub shape: Shape,
    /// Oven spring 0..1.
    pub spring: f32,
    /// Crust shade 0..1.
    pub crust: f32,
    pub tang: f32,
    /// Crumb openness 0..1 (from starter pep).
    pub openness: f32,
    pub cuts: Vec<Cut>,
    pub stencil: Option<Stencil>,
    pub topping: Option<Topping>,
    pub pattern: Option<Pattern>,
    pub pattern_match: f32,
    pub looks: f32,
    /// 0..1 overall.
    pub quality: f32,
    pub stars: u8,
    pub starter: String,
    pub day: u32,
    pub seed: u32,
}

impl Loaf {
    pub fn crust_level(&self) -> CrustLevel {
        CrustLevel::of(self.crust)
    }
    pub fn big_ear(&self) -> f32 {
        let ear = self.cuts.iter().map(|c| c.ear * c.bloom).fold(0.0, f32::max);
        (self.spring * 0.5 + ear * 0.7).clamp(0.0, 1.0)
    }
    pub fn view(&self, r: f32) -> LoafView {
        LoafView {
            shape: self.shape,
            recipe: self.recipe,
            r,
            bake: 1.0,
            spring: self.spring,
            crust: self.crust,
            cuts: self
                .cuts
                .iter()
                .map(|c| CutView { pts: c.pts.clone(), bloom: c.bloom, ear: c.ear })
                .collect(),
            stencil: self.stencil,
            topping: self.topping,
            seed: self.seed,
        }
    }
    /// A short celebratory title for the reveal card.
    pub fn title(&self) -> String {
        match self.pattern {
            Some(p) if self.pattern_match > 0.55 => {
                format!("{} · {}", self.recipe.name(), p.name())
            }
            _ => self.recipe.name().to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TreatItem {
    pub kind: Treat,
    pub quality: f32,
    pub seed: u32,
}

/// Anything on the display shelf.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Good {
    Loaf(Loaf),
    Treat(TreatItem),
}

impl Good {
    pub fn price(&self) -> u32 {
        match self {
            Good::Loaf(l) => l.recipe.price(),
            Good::Treat(t) => t.kind.price(),
        }
    }
    pub fn quality(&self) -> f32 {
        match self {
            Good::Loaf(l) => l.quality,
            Good::Treat(t) => t.quality,
        }
    }
    pub fn name(&self) -> String {
        match self {
            Good::Loaf(l) => l.title(),
            Good::Treat(t) => t.kind.name().to_string(),
        }
    }
}

/// Bake a (dressed & scored) dough. `crust` is where the player pulled it (0..1),
/// `lid` is the Dutch-oven upgrade (more steam → more spring).
pub fn bake(dough: &Dough, crust: f32, lid: bool, day: u32) -> Loaf {
    let report =
        dough.score.clone().unwrap_or_else(|| scoring::evaluate(&dough.cuts, dough.shape, dough.guide));
    let spring =
        (0.18 + 0.52 * dough.strength + 0.22 * dough.fold + if lid { 0.12 } else { 0.0 }).clamp(0.0, 1.0);
    let n = dough.cuts.len().max(1) as f32;
    let cuts: Vec<Cut> = dough
        .cuts
        .iter()
        .map(|pts| {
            let clean = scoring::cleanliness(pts);
            let len = polyline_len(pts);
            // Long confident cuts open wide and lift an ear; many small cuts share the spring.
            let share = (1.6 / n).clamp(0.45, 1.0);
            let bloom = (spring * (0.55 + 0.45 * clean) * (0.6 + 0.4 * share)).clamp(0.15, 1.0);
            let ear = (clean * (len / 1.2).clamp(0.0, 1.0) * share * (0.4 + 0.6 * spring)).clamp(0.0, 1.0);
            Cut { pts: pts.clone(), bloom, ear }
        })
        .collect();
    let crust = crust.clamp(0.0, 1.0);
    // A lovely bake sits between pale-blonde and dark-bold; the extremes are still fine.
    let bake_ok = 1.0 - ((crust - 0.55).abs() - 0.3).max(0.0) * 2.5;
    let unscored_penalty = if cuts.is_empty() { 0.15 } else { 0.0 };
    let looks = if cuts.is_empty() {
        0.35 + if dough.stencil.is_some() { 0.25 } else { 0.0 }
    } else {
        (report.looks + if dough.stencil.is_some() || dough.topping.is_some() { 0.08 } else { 0.0 }).min(1.0)
    };
    let quality =
        (0.38 * spring + 0.4 * looks + 0.22 * bake_ok.clamp(0.0, 1.0) - unscored_penalty).clamp(0.0, 1.0);
    let stars = if quality >= 0.74 {
        3
    } else if quality >= 0.5 {
        2
    } else {
        1
    };
    Loaf {
        id: dough.id,
        recipe: dough.recipe,
        shape: dough.shape,
        spring,
        crust,
        tang: dough.tang,
        openness: (0.25 + 0.6 * dough.strength + 0.15 * dough.fold).clamp(0.0, 1.0),
        cuts,
        stencil: dough.stencil,
        topping: dough.topping,
        pattern: report.pattern,
        pattern_match: report.pattern_match,
        looks,
        quality,
        stars,
        starter: dough.starter.clone(),
        day,
        seed: dough.seed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scoring::template;

    fn dough(strength: f32) -> Dough {
        Dough {
            id: 1,
            recipe: Recipe::Country,
            shape: Shape::Boule,
            strength,
            tang: 0.3,
            fold: 0.9,
            starter: "Bubbles".into(),
            made_day: 1,
            stencil: None,
            topping: None,
            cuts: template(Pattern::Wheat, Shape::Boule),
            guide: None,
            score: None,
            seed: 3,
        }
    }

    #[test]
    fn peppy_starter_springs_more() {
        let weak = bake(&dough(0.2), 0.55, false, 1);
        let strong = bake(&dough(0.95), 0.55, false, 1);
        assert!(strong.spring > weak.spring + 0.3);
        assert!(strong.openness > weak.openness);
        assert!(strong.quality > weak.quality);
        assert_eq!(strong.pattern, Some(Pattern::Wheat));
    }

    #[test]
    fn crust_levels_are_preferences_not_failures() {
        let d = dough(0.8);
        for c in [0.2, 0.55, 0.9] {
            let l = bake(&d, c, false, 1);
            assert!(l.stars >= 2, "crust {c} → {} stars", l.stars);
        }
        assert_eq!(CrustLevel::of(0.2), CrustLevel::Blonde);
        assert_eq!(CrustLevel::of(0.55), CrustLevel::Golden);
        assert_eq!(CrustLevel::of(0.9), CrustLevel::Bold);
    }

    #[test]
    fn single_long_cut_lifts_an_ear() {
        let mut d = dough(0.9);
        d.cuts = template(Pattern::Ear, Shape::Batard);
        d.shape = Shape::Batard;
        let l = bake(&d, 0.6, true, 1);
        assert!(l.cuts[0].ear > 0.5, "{:?}", l.cuts[0]);
        assert!(l.big_ear() > 0.7);
    }
}
