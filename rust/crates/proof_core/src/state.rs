//! The game state machine. Every player intent is an [`Action`]; applying it returns the
//! [`Event`]s the UI should celebrate. Deterministic for a given seed.
//!
//! Day rhythm: **Morning** (dress, score, bake, treats) → **Shop** (serve regulars) →
//! **Evening** (feed starters, mix doughs into the fridge, shop the catalog) → Sleep.

use crate::bake::{self, Dough, Good, Loaf, TreatItem};
use crate::content::{Flour, Pattern, Recipe, Shape, Species, Stencil, Topping, Treat};
use crate::customer::{self, Order, Reaction, Want};
use crate::economy::{self, CATALOG, STARTING, Unlock};
use crate::geom::V2;
use crate::rng::Rng;
use crate::scoring;
use crate::starter::{Cloth, FeedResult, PRESET_NAMES, Starter};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SAVE_VERSION: u32 = 1;
pub const OVEN_CAPACITY: usize = 2;
pub const MAX_DISCARD: u32 = 6;
pub const TRAY_COST: u32 = 2;
pub const MAX_ZINE: usize = 60;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Phase {
    Morning,
    Shop,
    Evening,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Visit {
    pub species: Species,
    pub order: Order,
    pub preorder: bool,
    pub outcome: Option<Reaction>,
    pub hello: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Friend {
    pub hearts: u32,
    pub gifts: u32,
    pub history: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DayStats {
    pub coins: u32,
    pub tips: u32,
    pub hearts: u32,
    pub served: u32,
    pub loved: u32,
    pub baked: u32,
    pub treats: u32,
    pub xp: u32,
    pub best: Option<Loaf>,
    pub leftovers: u32,
}

/// Player preferences (saved with the bakery).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub sound: bool,
    pub haptics: bool,
    /// Reduce motion: gentler print-in, no ink wobble.
    pub calm: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { sound: true, haptics: true, calm: false }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GameState {
    pub version: u32,
    pub seed: u64,
    pub day: u32,
    pub phase: Phase,
    pub coins: u32,
    pub xp: u32,
    pub starters: Vec<Starter>,
    pub discard: u32,
    pub fridge: Vec<Dough>,
    pub fridge_slots: u32,
    pub shelf: Vec<Good>,
    pub unlocked: Vec<Unlock>,
    pub friends: BTreeMap<Species, Friend>,
    pub visitors: Vec<Visit>,
    pub visitor_idx: usize,
    pub tomorrow: Vec<Visit>,
    pub zine: Vec<Loaf>,
    pub discovered: Vec<String>,
    pub today: DayStats,
    pub next_id: u32,
    pub rng: Rng,
    pub tips_seen: Vec<String>,
    #[serde(default)]
    pub settings: Settings,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Action {
    // Morning
    Dress {
        dough: u32,
        stencil: Option<Stencil>,
        topping: Option<Topping>,
    },
    Score {
        dough: u32,
        cuts: Vec<Vec<V2>>,
        guide: Option<Pattern>,
    },
    Bake {
        doughs: Vec<u32>,
        crust: f32,
    },
    MakeTray {
        treat: Treat,
        quality: f32,
    },
    OpenShop,
    // Shop
    Serve {
        shelf_idx: usize,
    },
    Skip,
    CloseShop,
    // Evening
    Feed {
        jar: usize,
        flour: Flour,
        stir: f32,
    },
    Mix {
        recipe: Recipe,
        jar: usize,
        shape: Shape,
        fold: f32,
    },
    Buy(Unlock),
    Rename {
        jar: usize,
        name: String,
    },
    Sleep,
    /// Mark a tutorial note as read (any phase).
    Tip(String),
    /// Change preferences (any phase).
    Settings(Settings),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ServeResult {
    pub species: Species,
    pub reaction: Reaction,
    pub satisfaction: f32,
    pub coins: u32,
    pub tip: u32,
    pub hearts: u32,
    pub good: Good,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NightReport {
    pub day: u32,
    pub hungry: Vec<String>,
    pub fridge: u32,
    pub preorders: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Event {
    Dressed { dough: u32 },
    Scored { dough: u32, report: scoring::ScoreReport },
    Baked { loaves: Vec<Loaf> },
    TrayMade { treat: Treat, count: u32 },
    ShopOpened { visitors: u32 },
    NextVisitor { idx: usize },
    Served(ServeResult),
    Skipped { species: Species, preordered: bool },
    ShopClosed { stats: DayStats },
    Fed { jar: usize, result: FeedResult },
    Mixed { doughs: Vec<u32> },
    Bought(Unlock),
    NewStarter { jar: usize },
    Renamed { jar: usize },
    Slept(NightReport),
    LevelUp { level: u32 },
    Gift { from: Species, unlock: Option<Unlock>, coins: u32 },
    NewPrint { key: String },
    PhaseChanged(Phase),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Reject {
    WrongPhase,
    NoSuchJar,
    NoSuchDough,
    AlreadyFed,
    FridgeFull,
    NotUnlocked,
    AlreadyOwned,
    NotEnoughCoins,
    NotEnoughDiscard,
    NothingToBake,
    OvenFull,
    ShelfEmpty,
    NoVisitor,
    NotReady,
}

impl Reject {
    pub fn message(self) -> &'static str {
        match self {
            Reject::WrongPhase => "Not right now!",
            Reject::NoSuchJar => "That jar is empty.",
            Reject::NoSuchDough => "That dough isn't here.",
            Reject::AlreadyFed => "Already fed — it's full and happy!",
            Reject::FridgeFull => "The fridge is full of bannetons.",
            Reject::NotUnlocked => "Not unlocked yet.",
            Reject::AlreadyOwned => "You already have that.",
            Reject::NotEnoughCoins => "Not enough coins yet.",
            Reject::NotEnoughDiscard => "Need more discard — feed your starters!",
            Reject::NothingToBake => "Nothing to bake.",
            Reject::OvenFull => "Toasty fits two loaves at a time.",
            Reject::ShelfEmpty => "The shelf is empty.",
            Reject::NoVisitor => "Nobody's at the counter.",
            Reject::NotReady => "Not ready yet.",
        }
    }
}

pub type Outcome = Result<Vec<Event>, Reject>;

impl GameState {
    /// A fresh bakery: Grandma's starter on the shelf and her batch proofing in the fridge.
    pub fn new(seed: u64) -> GameState {
        let mut rng = Rng::new(seed);
        let mut bubbles = Starter::new("Bubbles", rng.next_u32(), Cloth::Pink);
        bubbles.pep = 80.0;
        bubbles.tang = 40.0;
        bubbles.rise = 0.85;
        let mut g = GameState {
            version: SAVE_VERSION,
            seed,
            day: 1,
            phase: Phase::Morning,
            coins: 10,
            xp: 0,
            starters: vec![bubbles],
            discard: 1,
            fridge: Vec::new(),
            fridge_slots: 2,
            shelf: Vec::new(),
            unlocked: STARTING.to_vec(),
            friends: BTreeMap::new(),
            visitors: Vec::new(),
            visitor_idx: 0,
            tomorrow: Vec::new(),
            zine: Vec::new(),
            discovered: Vec::new(),
            today: DayStats::default(),
            next_id: 1,
            rng,
            tips_seen: Vec::new(),
            settings: Settings::default(),
        };
        for _ in 0..2 {
            let d = g.new_dough(Recipe::Country, Shape::Boule, 0.8, 0.4, 0.85, "Grandma");
            g.fridge.push(d);
        }
        // Day one is scripted to teach the loop: two loaves, three friends.
        g.tomorrow = vec![
            g.visit(Species::Bunny, Order { wants: vec![Want::Stencil(Stencil::Heart)] }, false),
            g.visit(Species::Bear, Order { wants: vec![Want::Crust(crate::bake::CrustLevel::Bold)] }, false),
            g.visit(Species::Duck, Order { wants: vec![Want::Tangy] }, false),
        ];
        g
    }

    fn visit(&mut self, species: Species, order: Order, preorder: bool) -> Visit {
        let p = customer::profile(species);
        let hello = self.rng.pick(p.hello).copied().unwrap_or("Hello!").to_string();
        Visit { species, order, preorder, outcome: None, hello }
    }

    fn new_dough(
        &mut self,
        recipe: Recipe,
        shape: Shape,
        strength: f32,
        tang: f32,
        fold: f32,
        starter: &str,
    ) -> Dough {
        let id = self.next_id;
        self.next_id += 1;
        Dough {
            id,
            recipe,
            shape,
            strength,
            tang,
            fold,
            starter: starter.to_string(),
            made_day: self.day,
            stencil: None,
            topping: None,
            cuts: Vec::new(),
            guide: None,
            score: None,
            seed: self.rng.next_u32(),
        }
    }

    // ------------------------------------------------------------------------------------
    // Queries
    // ------------------------------------------------------------------------------------

    pub fn has(&self, u: Unlock) -> bool {
        match u {
            Unlock::Fridge(n) => self.fridge_slots >= n,
            Unlock::Jar(n) => self.starters.len() as u32 >= n,
            _ => self.unlocked.contains(&u),
        }
    }

    pub fn level(&self) -> u32 {
        economy::level_for(self.xp)
    }

    pub fn has_lid(&self) -> bool {
        self.unlocked.contains(&Unlock::Lid)
    }

    pub fn can_want(&self, w: &Want) -> bool {
        w.needs().is_none_or(|u| self.has(u))
    }

    pub fn current_visitor(&self) -> Option<&Visit> {
        if self.phase == Phase::Shop { self.visitors.get(self.visitor_idx) } else { None }
    }

    pub fn fridge_free(&self) -> u32 {
        self.fridge_slots.saturating_sub(self.fridge.len() as u32)
    }

    pub fn unlocked_recipes(&self) -> Vec<Recipe> {
        Recipe::ALL.into_iter().filter(|r| self.has(Unlock::Recipe(*r))).collect()
    }
    pub fn unlocked_flours(&self) -> Vec<Flour> {
        Flour::ALL.into_iter().filter(|f| self.has(Unlock::Flour(*f))).collect()
    }
    pub fn unlocked_stencils(&self) -> Vec<Stencil> {
        Stencil::ALL.into_iter().filter(|s| self.has(Unlock::Stencil(*s))).collect()
    }
    pub fn unlocked_toppings(&self) -> Vec<Topping> {
        Topping::ALL.into_iter().filter(|t| self.has(Unlock::Topping(*t))).collect()
    }
    pub fn unlocked_patterns(&self) -> Vec<Pattern> {
        Pattern::ALL.into_iter().filter(|p| self.has(Unlock::Pattern(*p))).collect()
    }
    pub fn unlocked_treats(&self) -> Vec<Treat> {
        Treat::ALL.into_iter().filter(|t| self.has(Unlock::Treat(*t))).collect()
    }
    pub fn customers(&self) -> Vec<Species> {
        let lvl = self.level();
        Species::ALL.into_iter().filter(|s| economy::customer_level(*s) <= lvl).collect()
    }

    /// Catalog items the player can see (owned ones excluded).
    pub fn shop_items(&self) -> Vec<economy::CatalogItem> {
        economy::catalog(self.level()).filter(|c| !self.has(c.unlock)).copied().collect()
    }

    pub fn fridge_dough(&self, id: u32) -> Option<&Dough> {
        self.fridge.iter().find(|d| d.id == id)
    }

    /// Doughs ready for the oven (everything in the fridge in the morning).
    pub fn bakeable(&self) -> Vec<u32> {
        self.fridge.iter().map(|d| d.id).collect()
    }

    pub fn saw_tip(&self, key: &str) -> bool {
        self.tips_seen.iter().any(|k| k == key)
    }

    pub fn mark_tip(&mut self, key: &str) {
        if !self.saw_tip(key) {
            self.tips_seen.push(key.to_string());
        }
    }

    // ------------------------------------------------------------------------------------
    // Actions
    // ------------------------------------------------------------------------------------

    pub fn apply(&mut self, action: Action) -> Outcome {
        use Action as A;
        match (self.phase, action) {
            (Phase::Morning, A::Dress { dough, stencil, topping }) => self.dress(dough, stencil, topping),
            (Phase::Morning, A::Score { dough, cuts, guide }) => self.score(dough, cuts, guide),
            (Phase::Morning, A::Bake { doughs, crust }) => self.bake(doughs, crust),
            (Phase::Morning, A::MakeTray { treat, quality }) => self.make_tray(treat, quality),
            (Phase::Morning, A::OpenShop) => self.open_shop(),
            (Phase::Shop, A::Serve { shelf_idx }) => self.serve(shelf_idx),
            (Phase::Shop, A::Skip) => self.skip(),
            (Phase::Shop, A::CloseShop) => self.close_shop(),
            (Phase::Evening, A::Feed { jar, flour, stir }) => self.feed(jar, flour, stir),
            (Phase::Evening, A::Mix { recipe, jar, shape, fold }) => self.mix(recipe, jar, shape, fold),
            (Phase::Evening, A::Buy(u)) => self.buy(u),
            (_, A::Rename { jar, name }) => self.rename(jar, name),
            (_, A::Settings(st)) => {
                self.settings = st;
                Ok(Vec::new())
            }
            (_, A::Tip(key)) => {
                self.mark_tip(&key);
                Ok(Vec::new())
            }
            (Phase::Evening, A::Sleep) => self.sleep(),
            _ => Err(Reject::WrongPhase),
        }
    }

    fn dough_mut(&mut self, id: u32) -> Result<&mut Dough, Reject> {
        self.fridge.iter_mut().find(|d| d.id == id).ok_or(Reject::NoSuchDough)
    }

    fn dress(&mut self, id: u32, stencil: Option<Stencil>, topping: Option<Topping>) -> Outcome {
        if stencil.is_some_and(|s| !self.has(Unlock::Stencil(s)))
            || topping.is_some_and(|t| !self.has(Unlock::Topping(t)))
        {
            return Err(Reject::NotUnlocked);
        }
        let d = self.dough_mut(id)?;
        d.stencil = stencil;
        d.topping = topping;
        Ok(vec![Event::Dressed { dough: id }])
    }

    fn score(&mut self, id: u32, cuts: Vec<Vec<V2>>, guide: Option<Pattern>) -> Outcome {
        if guide.is_some_and(|p| !self.has(Unlock::Pattern(p))) {
            return Err(Reject::NotUnlocked);
        }
        let d = self.dough_mut(id)?;
        let report = scoring::evaluate(&cuts, d.shape, guide);
        d.cuts = cuts;
        d.guide = guide;
        d.score = Some(report.clone());
        Ok(vec![Event::Scored { dough: id, report }])
    }

    fn bake(&mut self, ids: Vec<u32>, crust: f32) -> Outcome {
        if ids.is_empty() {
            return Err(Reject::NothingToBake);
        }
        if ids.len() > OVEN_CAPACITY {
            return Err(Reject::OvenFull);
        }
        if ids.iter().any(|id| self.fridge_dough(*id).is_none()) {
            return Err(Reject::NoSuchDough);
        }
        let lid = self.has_lid();
        let mut loaves = Vec::new();
        let mut events = Vec::new();
        for id in ids {
            let pos = self.fridge.iter().position(|d| d.id == id).unwrap();
            let dough = self.fridge.remove(pos);
            let loaf = bake::bake(&dough, crust, lid, self.day);
            self.today.baked += 1;
            if self.today.best.as_ref().is_none_or(|b| loaf.quality > b.quality) {
                self.today.best = Some(loaf.clone());
            }
            let key = format!("{:?}/{:?}", loaf.recipe, loaf.pattern);
            if !self.discovered.contains(&key) {
                self.discovered.push(key.clone());
                events.push(Event::NewPrint { key });
            }
            self.zine.push(loaf.clone());
            if self.zine.len() > MAX_ZINE {
                self.zine.remove(0);
            }
            self.shelf.push(Good::Loaf(loaf.clone()));
            loaves.push(loaf);
        }
        events.insert(0, Event::Baked { loaves });
        Ok(events)
    }

    fn make_tray(&mut self, treat: Treat, quality: f32) -> Outcome {
        if !self.has(Unlock::Treat(treat)) {
            return Err(Reject::NotUnlocked);
        }
        if self.discard < TRAY_COST {
            return Err(Reject::NotEnoughDiscard);
        }
        self.discard -= TRAY_COST;
        let n = treat.per_tray();
        for _ in 0..n {
            let seed = self.rng.next_u32();
            self.shelf.push(Good::Treat(TreatItem {
                kind: treat,
                quality: (0.55 + 0.45 * quality).clamp(0.0, 1.0),
                seed,
            }));
        }
        self.today.treats += n;
        Ok(vec![Event::TrayMade { treat, count: n }])
    }

    fn open_shop(&mut self) -> Outcome {
        self.phase = Phase::Shop;
        self.visitors = std::mem::take(&mut self.tomorrow);
        self.visitor_idx = 0;
        Ok(vec![
            Event::PhaseChanged(Phase::Shop),
            Event::ShopOpened { visitors: self.visitors.len() as u32 },
            Event::NextVisitor { idx: 0 },
        ])
    }

    fn gain_xp(&mut self, xp: u32, events: &mut Vec<Event>) {
        let before = self.level();
        self.xp += xp;
        self.today.xp += xp;
        let after = self.level();
        for level in before + 1..=after {
            events.push(Event::LevelUp { level });
        }
    }

    fn serve(&mut self, shelf_idx: usize) -> Outcome {
        let visit = self.visitors.get(self.visitor_idx).cloned().ok_or(Reject::NoVisitor)?;
        if shelf_idx >= self.shelf.len() {
            return Err(Reject::ShelfEmpty);
        }
        let good = self.shelf.remove(shelf_idx);
        let p = customer::profile(visit.species);
        let friend = self.friends.entry(visit.species).or_default();
        let sat = customer::satisfaction(&visit.order, &good, &friend.history);
        let reaction = customer::reaction_for(sat);
        let (coins, tip) = customer::payment(&good, sat, p.critic, visit.preorder);
        let hearts = match reaction {
            Reaction::Love => 2,
            Reaction::Happy => 1,
            _ => 0,
        };
        friend.history.push(customer::signature(&good));
        let before = friend.hearts;
        friend.hearts += hearts;
        let after = friend.hearts;
        let gifts_due = [3u32, 6, 10].iter().filter(|&&t| before < t && after >= t).count() as u32;

        self.coins += coins + tip;
        self.today.coins += coins;
        self.today.tips += tip;
        self.today.hearts += hearts;
        self.today.served += 1;
        if reaction == Reaction::Love {
            self.today.loved += 1;
        }
        self.visitors[self.visitor_idx].outcome = Some(reaction);

        let mut events = vec![Event::Served(ServeResult {
            species: visit.species,
            reaction,
            satisfaction: sat,
            coins,
            tip,
            hearts,
            good,
        })];
        for _ in 0..gifts_due {
            events.push(self.give_gift(visit.species));
        }
        self.gain_xp(2 + hearts * 4, &mut events);
        self.advance_visitor(&mut events);
        Ok(events)
    }

    fn give_gift(&mut self, from: Species) -> Event {
        let friend = self.friends.entry(from).or_default();
        let idx = friend.gifts as usize;
        friend.gifts += 1;
        let options = customer::profile(from).gifts;
        match options.get(idx).copied().filter(|u| !self.has(*u)) {
            Some(u) => {
                self.grant(u);
                Event::Gift { from, unlock: Some(u), coins: 0 }
            }
            None => {
                self.coins += 25;
                Event::Gift { from, unlock: None, coins: 25 }
            }
        }
    }

    fn advance_visitor(&mut self, events: &mut Vec<Event>) {
        self.visitor_idx += 1;
        if self.visitor_idx < self.visitors.len() {
            events.push(Event::NextVisitor { idx: self.visitor_idx });
        }
    }

    fn skip(&mut self) -> Outcome {
        let visit = self.visitors.get(self.visitor_idx).cloned().ok_or(Reject::NoVisitor)?;
        self.visitors[self.visitor_idx].outcome = Some(Reaction::Sorry);
        // Sold out? They leave a pre-order for tomorrow instead of going home sad.
        let preordered = self.shelf.is_empty();
        if preordered {
            let mut v = visit.clone();
            v.preorder = true;
            v.outcome = None;
            self.tomorrow.push(v);
        }
        let mut events = vec![Event::Skipped { species: visit.species, preordered }];
        self.advance_visitor(&mut events);
        Ok(events)
    }

    fn close_shop(&mut self) -> Outcome {
        let mut events = Vec::new();
        while self.visitor_idx < self.visitors.len() {
            let mut ev = self.skip()?;
            events.append(&mut ev);
        }
        // Leftovers go to the neighbours (a little goodwill XP).
        let left = self.shelf.len() as u32;
        self.shelf.clear();
        self.today.leftovers = left;
        self.gain_xp(left, &mut events);
        self.plan_tomorrow();
        self.phase = Phase::Evening;
        events.push(Event::ShopClosed { stats: self.today.clone() });
        events.push(Event::PhaseChanged(Phase::Evening));
        Ok(events)
    }

    /// Pick tomorrow's visitors and pin pre-order tickets.
    fn plan_tomorrow(&mut self) {
        let next = self.day + 1;
        let target = (3 + (next - 1) / 2).min(6) as usize;
        let mut pool: Vec<Species> = self.customers();
        pool.retain(|s| !self.tomorrow.iter().any(|v| v.species == *s));
        self.rng.shuffle(&mut pool);
        let carried = self.tomorrow.len();
        let preorders_wanted = if next >= 5 { 2 } else { 1 };
        let mut new_preorders = 0;
        let owned = self.unlocked.clone();
        for s in pool.into_iter().take(target.saturating_sub(carried)) {
            let can = |w: &Want| w.needs().is_none_or(|u| owned.contains(&u));
            let order = customer::make_order(s, &mut self.rng, can, next);
            let pre = new_preorders + carried < preorders_wanted;
            if pre {
                new_preorders += 1;
            }
            let v = self.visit(s, order, pre);
            self.tomorrow.push(v);
        }
    }

    fn feed(&mut self, jar: usize, flour: Flour, stir: f32) -> Outcome {
        if !self.has(Unlock::Flour(flour)) {
            return Err(Reject::NotUnlocked);
        }
        let s = self.starters.get_mut(jar).ok_or(Reject::NoSuchJar)?;
        if s.fed_today {
            return Err(Reject::AlreadyFed);
        }
        let result = s.feed(flour, stir);
        self.discard = (self.discard + 1).min(MAX_DISCARD);
        Ok(vec![Event::Fed { jar, result }])
    }

    fn mix(&mut self, recipe: Recipe, jar: usize, shape: Shape, fold: f32) -> Outcome {
        if !self.has(Unlock::Recipe(recipe)) {
            return Err(Reject::NotUnlocked);
        }
        let free = self.fridge_free();
        if free == 0 {
            return Err(Reject::FridgeFull);
        }
        let s = self.starters.get(jar).ok_or(Reject::NoSuchJar)?;
        let strength = s.strength();
        let tang = (s.tang / 100.0 + recipe.tang_bias()).clamp(0.0, 1.0);
        let name = s.name.clone();
        let mut ids = Vec::new();
        for _ in 0..free.min(2) {
            let d = self.new_dough(recipe, shape, strength, tang, fold.clamp(0.0, 1.0), &name);
            ids.push(d.id);
            self.fridge.push(d);
        }
        Ok(vec![Event::Mixed { doughs: ids }])
    }

    fn grant(&mut self, u: Unlock) {
        match u {
            Unlock::Fridge(n) => self.fridge_slots = self.fridge_slots.max(n),
            Unlock::Jar(n) => {
                while (self.starters.len() as u32) < n {
                    let i = self.starters.len();
                    let name = PRESET_NAMES[(i * 5 + 1) % PRESET_NAMES.len()];
                    let cloth = [Cloth::Pink, Cloth::Blue, Cloth::Yellow][i % 3];
                    let mut s = Starter::new(name, self.rng.next_u32(), cloth);
                    // Started from a dollop of the first jar: inherits its character.
                    if let Some(first) = self.starters.first() {
                        s.tang = first.tang;
                        s.pep = (first.pep * 0.8).max(40.0);
                    }
                    self.starters.push(s);
                }
            }
            _ => {
                if !self.unlocked.contains(&u) {
                    self.unlocked.push(u);
                }
            }
        }
    }

    fn buy(&mut self, u: Unlock) -> Outcome {
        if self.has(u) {
            return Err(Reject::AlreadyOwned);
        }
        let item = CATALOG.iter().find(|c| c.unlock == u).ok_or(Reject::NotUnlocked)?;
        if item.level > self.level() {
            return Err(Reject::NotUnlocked);
        }
        // Jars and fridges come in order.
        if let Unlock::Jar(n) = u
            && self.starters.len() as u32 + 1 != n
        {
            return Err(Reject::NotUnlocked);
        }
        if self.coins < item.cost {
            return Err(Reject::NotEnoughCoins);
        }
        self.coins -= item.cost;
        let before = self.starters.len();
        self.grant(u);
        let mut events = vec![Event::Bought(u)];
        if self.starters.len() > before {
            events.push(Event::NewStarter { jar: self.starters.len() - 1 });
        }
        Ok(events)
    }

    fn rename(&mut self, jar: usize, name: String) -> Outcome {
        let s = self.starters.get_mut(jar).ok_or(Reject::NoSuchJar)?;
        let clean: String = name.trim().chars().filter(|c| !c.is_control()).take(14).collect();
        if !clean.is_empty() {
            s.name = clean;
        }
        Ok(vec![Event::Renamed { jar }])
    }

    fn sleep(&mut self) -> Outcome {
        let hungry: Vec<String> =
            self.starters.iter().filter(|s| !s.fed_today).map(|s| s.name.clone()).collect();
        for s in &mut self.starters {
            s.overnight();
        }
        // Doughs left from earlier nights over-proof a little.
        let day = self.day;
        for d in &mut self.fridge {
            if d.made_day < day {
                d.strength = (d.strength - 0.12).max(0.1);
            }
        }
        self.day += 1;
        self.phase = Phase::Morning;
        self.today = DayStats::default();
        let report = NightReport {
            day: self.day,
            hungry,
            fridge: self.fridge.len() as u32,
            preorders: self.tomorrow.iter().filter(|v| v.preorder).count() as u32,
        };
        Ok(vec![Event::Slept(report), Event::PhaseChanged(Phase::Morning)])
    }

    // ------------------------------------------------------------------------------------
    // Save / load
    // ------------------------------------------------------------------------------------

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("serialise game")
    }

    pub fn from_json(s: &str) -> Result<GameState, String> {
        let g: GameState = serde_json::from_str(s).map_err(|e| e.to_string())?;
        if g.version > SAVE_VERSION {
            return Err(format!("save from a newer version ({})", g.version));
        }
        Ok(g)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scoring::template;

    #[test]
    fn day_one_loop_runs_end_to_end() {
        let mut g = GameState::new(1);
        assert_eq!(g.phase, Phase::Morning);
        let ids = g.bakeable();
        assert_eq!(ids.len(), 2);
        g.apply(Action::Dress { dough: ids[0], stencil: Some(Stencil::Heart), topping: None }).unwrap();
        g.apply(Action::Score { dough: ids[0], cuts: template(Pattern::Cross, Shape::Boule), guide: None })
            .unwrap();
        g.apply(Action::Score { dough: ids[1], cuts: template(Pattern::Ear, Shape::Boule), guide: None })
            .unwrap();
        let ev = g.apply(Action::Bake { doughs: ids.clone(), crust: 0.85 }).unwrap();
        assert!(matches!(ev[0], Event::Baked { ref loaves } if loaves.len() == 2));
        assert_eq!(g.shelf.len(), 2);
        g.apply(Action::OpenShop).unwrap();
        assert_eq!(g.visitors.len(), 3);
        // Mimi wants the heart loaf.
        let heart = g.shelf.iter().position(|s| matches!(s, Good::Loaf(l) if l.stencil.is_some())).unwrap();
        let ev = g.apply(Action::Serve { shelf_idx: heart }).unwrap();
        match &ev[0] {
            Event::Served(r) => assert_eq!(r.reaction, Reaction::Love),
            e => panic!("{e:?}"),
        }
        g.apply(Action::Serve { shelf_idx: 0 }).unwrap();
        // Pip arrives to an empty shelf: leaves a pre-order.
        let ev = g.apply(Action::Skip).unwrap();
        assert!(matches!(ev[0], Event::Skipped { preordered: true, .. }));
        g.apply(Action::CloseShop).unwrap();
        assert_eq!(g.phase, Phase::Evening);
        assert!(g.tomorrow.iter().any(|v| v.species == Species::Duck && v.preorder));
        assert!(g.coins > 10);
        // Evening: rye makes Bubbles tangy for Pip.
        g.apply(Action::Feed { jar: 0, flour: Flour::Rye, stir: 1.0 }).unwrap();
        assert_eq!(g.apply(Action::Feed { jar: 0, flour: Flour::Rye, stir: 1.0 }), Err(Reject::AlreadyFed));
        g.apply(Action::Mix { recipe: Recipe::Country, jar: 0, shape: Shape::Boule, fold: 0.9 }).unwrap();
        assert_eq!(g.fridge.len(), 2);
        assert_eq!(
            g.apply(Action::Mix { recipe: Recipe::Country, jar: 0, shape: Shape::Boule, fold: 0.9 }),
            Err(Reject::FridgeFull)
        );
        assert!(g.fridge.iter().all(|d| d.tang >= 0.5), "tangy for Pip");
        g.apply(Action::Sleep).unwrap();
        assert_eq!(g.day, 2);
        assert_eq!(g.phase, Phase::Morning);
    }

    #[test]
    fn actions_in_wrong_phase_are_rejected() {
        let mut g = GameState::new(2);
        assert_eq!(g.apply(Action::Sleep), Err(Reject::WrongPhase));
        assert_eq!(g.apply(Action::Serve { shelf_idx: 0 }), Err(Reject::WrongPhase));
    }

    #[test]
    fn buying_needs_coins_and_level() {
        let mut g = GameState::new(3);
        g.phase = Phase::Evening;
        assert_eq!(g.apply(Action::Buy(Unlock::Recipe(Recipe::Cheddar))), Err(Reject::NotUnlocked));
        g.coins = 0;
        assert_eq!(g.apply(Action::Buy(Unlock::Fridge(4))), Err(Reject::NotEnoughCoins));
        g.coins = 100;
        g.apply(Action::Buy(Unlock::Fridge(4))).unwrap();
        assert_eq!(g.fridge_slots, 4);
        assert_eq!(g.apply(Action::Buy(Unlock::Fridge(4))), Err(Reject::AlreadyOwned));
        g.xp = 100;
        g.coins = 100;
        let ev = g.apply(Action::Buy(Unlock::Jar(2))).unwrap();
        assert!(ev.contains(&Event::NewStarter { jar: 1 }));
    }

    #[test]
    fn save_round_trip() {
        let mut g = GameState::new(4);
        let ids = g.bakeable();
        g.apply(Action::Bake { doughs: ids, crust: 0.5 }).unwrap();
        let json = g.to_json();
        let back = GameState::from_json(&json).unwrap();
        assert_eq!(g, back);
    }
}
