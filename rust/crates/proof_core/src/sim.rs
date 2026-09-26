//! A simple autoplay bot that drives [`GameState`] through the same actions as the UI.
//! Used for balance checks, invariants in tests, and the Godot `--autoplay` smoke run.

use crate::bake::CrustLevel;
use crate::content::{Flour, Pattern, Recipe, Shape};
use crate::customer::{self, Want};
use crate::geom::{V2, resample, v2};
use crate::rng::Rng;
use crate::scoring::template;
use crate::state::{Action, Event, GameState, OVEN_CAPACITY, Phase, Reject};

/// Rough count of player gestures an action represents (budget checks).
pub fn gestures(a: &Action) -> u32 {
    match a {
        Action::Dress { stencil, topping, .. } => stencil.is_some() as u32 + topping.is_some() as u32,
        Action::Score { cuts, .. } => cuts.len() as u32 + 1,
        Action::Bake { .. } => 2,
        Action::MakeTray { .. } => 2,
        Action::Feed { .. } => 2,
        Action::Mix { .. } => 6,
        _ => 1,
    }
}

#[derive(Clone, Debug, Default)]
pub struct DayLog {
    pub day: u32,
    pub gestures: u32,
    pub coins_end: u32,
    pub earned: u32,
    pub level: u32,
    pub served: u32,
    pub loved: u32,
    pub visitors: u32,
    pub baked: u32,
    pub rejects: u32,
}

pub struct Bot {
    pub rng: Rng,
    pub jitter: f32,
}

impl Bot {
    pub fn new(seed: u64) -> Bot {
        Bot { rng: Rng::new(seed ^ 0xB07), jitter: 0.04 }
    }

    fn cuts(&mut self, p: Pattern, shape: Shape) -> Vec<Vec<V2>> {
        let j = self.jitter;
        template(p, shape)
            .iter()
            .map(|s| {
                resample(s, 14)
                    .into_iter()
                    .map(|q| q + v2(self.rng.range(-j, j), self.rng.range(-j, j)))
                    .collect()
            })
            .collect()
    }

    /// Play the current phase to its end, returning (actions taken, events).
    pub fn play_phase(&mut self, g: &mut GameState, log: &mut DayLog) -> Vec<Event> {
        let mut events = Vec::new();
        let act =
            |g: &mut GameState, a: Action, log: &mut DayLog, events: &mut Vec<Event>| -> Result<(), Reject> {
                log.gestures += gestures(&a);
                match g.apply(a) {
                    Ok(mut e) => {
                        events.append(&mut e);
                        Ok(())
                    }
                    Err(r) => {
                        log.rejects += 1;
                        Err(r)
                    }
                }
            };
        match g.phase {
            Phase::Morning => {
                let wants: Vec<Want> = g.tomorrow.iter().flat_map(|v| v.order.wants.clone()).collect();
                let stencils = g.unlocked_stencils();
                let toppings = g.unlocked_toppings();
                let patterns = g.unlocked_patterns();
                let ids = g.bakeable();
                for (i, id) in ids.iter().enumerate() {
                    let target = wants.get(i).copied();
                    let stencil = match target {
                        Some(Want::Stencil(s)) => Some(s),
                        _ if self.rng.chance(0.4) => self.rng.pick(&stencils).copied(),
                        _ => None,
                    };
                    let topping = match target {
                        Some(Want::Topping(t)) => Some(t),
                        _ if self.rng.chance(0.3) => self.rng.pick(&toppings).copied(),
                        _ => None,
                    };
                    let _ = act(g, Action::Dress { dough: *id, stencil, topping }, log, &mut events);
                    let pattern = match target {
                        Some(Want::Pattern(p)) => p,
                        _ => *self.rng.pick(&patterns).unwrap_or(&Pattern::Ear),
                    };
                    let shape = g.fridge_dough(*id).map(|d| d.shape).unwrap_or(Shape::Boule);
                    let cuts = self.cuts(pattern, shape);
                    let _ =
                        act(g, Action::Score { dough: *id, cuts, guide: Some(pattern) }, log, &mut events);
                }
                let crust_for = |w: Option<&Want>| match w {
                    Some(Want::Crust(c)) => c.shade(),
                    _ => CrustLevel::Golden.shade(),
                };
                for (k, pair) in ids.chunks(OVEN_CAPACITY).enumerate() {
                    let crust = crust_for(wants.get(k * 2));
                    let _ = act(g, Action::Bake { doughs: pair.to_vec(), crust }, log, &mut events);
                }
                log.baked += ids.len() as u32;
                let treats = g.unlocked_treats();
                if let Some(t) = self.rng.pick(&treats).copied()
                    && g.discard >= 2
                {
                    let _ = act(g, Action::MakeTray { treat: t, quality: 0.8 }, log, &mut events);
                }
                let _ = act(g, Action::OpenShop, log, &mut events);
                log.visitors = g.visitors.len() as u32;
            }
            Phase::Shop => {
                while let Some(v) = g.current_visitor().cloned() {
                    if g.shelf.is_empty() {
                        let _ = act(g, Action::Skip, log, &mut events);
                        continue;
                    }
                    let hist = g.friends.get(&v.species).map(|f| f.history.clone()).unwrap_or_default();
                    let best = (0..g.shelf.len())
                        .max_by(|a, b| {
                            let sa = customer::satisfaction(&v.order, &g.shelf[*a], &hist);
                            let sb = customer::satisfaction(&v.order, &g.shelf[*b], &hist);
                            sa.total_cmp(&sb)
                        })
                        .unwrap();
                    let _ = act(g, Action::Serve { shelf_idx: best }, log, &mut events);
                }
                let _ = act(g, Action::CloseShop, log, &mut events);
            }
            Phase::Evening => {
                let wants: Vec<Want> = g.tomorrow.iter().flat_map(|v| v.order.wants.clone()).collect();
                let tangy = wants.contains(&Want::Tangy);
                let mild = wants.contains(&Want::Mild);
                let flours = g.unlocked_flours();
                for jar in 0..g.starters.len() {
                    let flour = if tangy && jar == 0 {
                        Flour::Rye
                    } else if mild {
                        Flour::White
                    } else if flours.contains(&Flour::Wheat) {
                        Flour::Wheat
                    } else {
                        *self.rng.pick(&flours).unwrap()
                    };
                    let _ = act(g, Action::Feed { jar, flour, stir: 0.9 }, log, &mut events);
                }
                let recipes = g.unlocked_recipes();
                let wanted: Vec<Recipe> = wants
                    .iter()
                    .filter_map(|w| if let Want::Recipe(r) = w { Some(*r) } else { None })
                    .collect();
                let mut k = 0;
                while g.fridge_free() > 0 {
                    let recipe = wanted.get(k).copied().unwrap_or_else(|| *self.rng.pick(&recipes).unwrap());
                    let shape = if wants.contains(&Want::Shape(Shape::Batard)) && k == 0 {
                        Shape::Batard
                    } else {
                        Shape::Boule
                    };
                    let jar = (0..g.starters.len())
                        .max_by(|a, b| g.starters[*a].pep.total_cmp(&g.starters[*b].pep))
                        .unwrap_or(0);
                    if act(g, Action::Mix { recipe, jar, shape, fold: 0.85 }, log, &mut events).is_err() {
                        break;
                    }
                    k += 1;
                }
                // Shop the catalog: cheapest first, fridges and jars favoured.
                let mut items = g.shop_items();
                items.sort_by_key(|c| match c.unlock {
                    crate::economy::Unlock::Fridge(_) => 0,
                    crate::economy::Unlock::Jar(_) => 1,
                    _ => 2 + c.cost,
                });
                for it in items {
                    if g.coins >= it.cost + 5 {
                        let _ = act(g, Action::Buy(it.unlock), log, &mut events);
                    }
                }
                let _ = act(g, Action::Sleep, log, &mut events);
            }
        }
        events
    }

    /// Play one whole day starting from Morning.
    pub fn play_day(&mut self, g: &mut GameState) -> DayLog {
        let mut log = DayLog { day: g.day, ..DayLog::default() };
        let coins0 = g.coins;
        for _ in 0..3 {
            self.play_phase(g, &mut log);
        }
        log.coins_end = g.coins;
        log.earned = g.coins.saturating_sub(coins0);
        log.level = g.level();
        let served = g
            .visitors
            .iter()
            .filter(|v| matches!(v.outcome, Some(r) if r != customer::Reaction::Sorry))
            .count();
        log.served = served as u32;
        log.loved = g.visitors.iter().filter(|v| v.outcome == Some(customer::Reaction::Love)).count() as u32;
        log
    }
}

/// Convenience: all goods on the shelf matching a want.
pub fn shelf_matches(g: &GameState, w: &Want) -> usize {
    g.shelf.iter().filter(|s| customer::want_score(w, s, &[]) > 0.9).count()
}
