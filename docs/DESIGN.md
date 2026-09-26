# Proof — design

*A tiny, very cute sourdough bakery printed on a risograph.*

"Proof" means two things: dough proofing overnight, and a printer's test print. The whole game rests on that double meaning.

## 1. From first principles: what makes baking fun?

| Real-life joy | How the game delivers it |
|---|---|
| Caring for a living thing (people name their starters!) | Starters are **pets that can't die**, each with a face, a name tape and a tiny visible microbiome. |
| The *anticipation* of tomorrow's bake | **Prep tonight, bake tomorrow.** Doughs you mix in the evening rest in the fridge overnight. |
| The tactile, expressive moment of scoring | You score with **freehand swipes of a blade**. Guides are optional, and clean patterns get recognised and named. |
| The oven reveal ("did I get an ear?!") | The loaf springs up and the cuts **tear open into blooms**, inks slam into register, and a rubber stamp *thunks* down. You also get a crumb shot. |
| Sharing bread with people who love it | **Animal regulars** with personalities, icon orders, friendship hearts and keepsake gifts. |
| A calm daily rhythm | **Compressed days** (Morning → Shop → Evening → Sleep), with no real-world timers and no fail states. |

## 2. Borrowed from the classics

- **Cooking Mama:** every step is a single gesture lasting under ten seconds. Failure is funny, never punishing.
- **Good Pizza, Great Pizza:** scoring is the canvas for self-expression, and orders are vague and full of personality, with a hint button.
- **Papa's ___ria:** each station grades into the customer's reaction and tip, and unlocks come with rank.
- **Tamagotchi, softened like Finch:** the pet needs care, but neglect only makes flatter loaves and a sleepy, hooch-topped jar.
- **Cook, Serve, Delicious 2 holding station:** the fridge holds tonight's prep for tomorrow's service.
- **Stardew / Animal Crossing:** short days, a cozy nightly summary, then "sleep".
- **Neko Atsume:** regulars leave keepsake gifts, and there's a collection zine.
- **Unpacking / A Little to the Left:** tactile juice (squish, blade *shhk*, stamp *thunk*, haptics).

We deliberately avoided stressful timers, lives, energy systems, consumable-resource grind, numbers on screen, and walls of text.

## 3. The loop (one in-game day ≈ 4–9 minutes)

```
☀ Morning (Dawn edition)        🏪 Shop (Daylight edition)       🌙 Evening (Dusk edition)
 wake-up peek at the jars   →   regulars arrive one by one  →   feed starters (flour + stir)
 dress: flour stencil/seeds      icon order + hint               read tomorrow's board
 score: freehand blade           tap/drag a goodie onto them     mix a batch: recipe, starter,
 Toasty: pull at your crust      hearts, coins, tips, gifts        shape, 4 stretch-&-fold swipes
 reveal: bloom, stamp, crumb     sold out? → pre-order ticket    Daily Crumb: catalog, zine, sleep
 treat tray from discard
```

The game opens on **Morning 1**. Grandma's two doughs are already in the fridge and her starter Bubbles is on the shelf, so the first minute is score → oven → reveal. Day one is scripted to teach the loop:
- Mimi wants a heart stencil and Bruno wants a bold crust.
- Pip arrives to an empty shelf and leaves a pre-order: *"something tangy tomorrow"*.
- That evening, feeding Bubbles **rye** makes tomorrow's dough tangy enough for Pip.

## 4. Systems (each produces one thing customers can ask for)

| System | Hidden stats | What customers ask for |
|---|---|---|
| Starter | Pep 0–100, Tang 0–100 | rise/ear, tangy/mild |
| Recipe | flour, inclusion, price | a named loaf |
| Dress & Score | stencil, topping, pattern match, cleanliness | looks, a pattern, a stencil, a topping |
| Oven | crust shade | blonde / golden / bold |
| Treat tray | quality | muffin / bun / bagel |

**Starters.** You never see the numbers. The microbiome is drawn literally inside the jar: pink round **Yeasties** and blue bean **Lactos**.
- **Pep** shows as the microbe count, the bubbles, the domed top and the overnight rise past the rubber band.
- **Tang** shows as the mix of Yeasties and Lactos.
- White flour gives milder starter, rye gives tangier starter plus extra pep, and whole wheat is balanced.
- An unfed night brings a hooch layer, a sleepy face and "z z z". Feeding revives the starter, and it can never die.
- Every feed adds one spoon of **discard** (up to 6). Two spoons make a treat tray, so pet care feeds the secondary economy.

**Doughs.** Strength is the starter's Pep at mix time, which is why feeding before mixing matters. Stretch-and-fold quality comes from the four swipe directions. Doughs left an extra night over-proof slightly.

**Scoring** (`proof_core::scoring`):
- Strokes are compared with pattern templates (Big Ear, Cross, Wheat Stalk, Leaf). Each stroke is resampled, paired greedily in either direction, and scored with a gaussian falloff. Extra strokes carry a penalty.
- Cleanliness rewards long, steady strokes.
- Freehand is always valid.
- In the oven, each cut's **bloom** comes from spring × cleanliness. The **ear** comes from long decisive cuts.

**Customers.** There are eight regulars, each with weighted likes:
- Mimi 🐰 — hearts, mild, sesame
- Bruno 🐻 — bold crust, big ears, rye
- Pip 🐤 — tangy
- Sir Whiskers 🐱 — a critic who always asks for two things and tips double
- Hazel 🦔 — seeds, whole wheat
- Momo 🐸 — "surprise me!" (anything new to them)
- Clover 🐑 — blonde and mild
- Otto 🦦 — treats

Satisfaction is 72% order fit and 28% quality. Everyone always pays at least 70% of the price, and delighted customers tip. Hearts at 3, 6 and 10 bring keepsake gifts.

**Economy.** Coins buy catalog unlocks, and levels come from friendship XP. The autoplay balance run (`proof_tools autoplay`) reaches the full catalog in about 18 in-game days. Launch content:
- 3 flours
- 6 recipes
- 2 shapes
- 4 scoring guides
- 4 stencils
- 3 toppings
- 3 treats
- 8 regulars
- up to 3 starter jars
- fridge sizes of 2, 4 and 6 bannetons
- a Dutch-oven lid

## 5. Art direction: riso-print kawaii

- **Inks.** Every pixel is printed with **four spot inks** (pink, yellow, blue and a key ink) on cream paper. Inks **multiply**, so pink over yellow gives coral and yellow over blue gives frog-green.
- **Halftone.** It's used for shading and blush, and the **bake level is halftone density** on the crust.
- **Misregistration and grain.** Each ink has a slight offset and there's paper grain. The key ink stays in register so faces stay crisp.
- **Editions.** Each time of day is a different *edition* with different inks loaded: Dawn (coral, sunflower, aqua, brown), Daylight (fluoro pink, yellow, riso blue, navy) and Dusk (berry, lamp-yellow, violet, plum). Palettes crossfade between phases.
- **Print-in.** Every screen change and big moment **prints in**: the ink layers slam out of register and spring back.
- **Printed props.** Order tickets have zig-zag tear-offs, results are rubber stamps, jars wear masking-tape names, and the night page is a zine called *The Daily Crumb*.
- **Kawaii rules.** Dot eyes with a paper sparkle, "w" mouths, halftone blush, rounded key lines, squash and stretch. The dough has no face while you cut it.
- **Procedural art.** All art is generated in Rust with no image assets; the only external asset is the Fredoka font (OFL). Sound is synthesised in Rust too.

## 6. Tech

- **`proof_core`** (pure Rust): rules, simulation, gestures, scoring, procedural art → `DrawList`s, and the synth.
- **`proof_raster`** (tiny-skia): `DrawList` → one RGBA texture where each channel is an ink plate.
  - Halftone screens are drawn per ink angle.
  - "Last draw wins" per plate, like a real master.
  - It handles knockouts, opaque paper backing and clipping.
- **`proof_gd`** (gdext): one `Game` node.
  - Screens are plain Rust structs that exchange `Action`s and `Event`s with the core.
  - A shared `riso_ink.gdshader` multiplies the plates with misregistration and grain.
- **Godot** provides the window, input, text, audio and export. There is no GDScript.
