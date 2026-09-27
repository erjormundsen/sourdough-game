//! Proof's house style: the shared vocabulary every art module draws with.
//!
//! # Rules
//! 1. **Silhouette first.** Every object reads as one confident shape with an `OUTER` key
//!    contour. Internal structure (seams, rims, fur tufts, lid bands) uses `INNER`; tiny
//!    details (whiskers, stitches, crumb holes, seeds) use `DETAIL`. Never outline small
//!    details with `OUTER`.
//! 2. **Overlap order is physical.** Draw back to front: cast shadow → far parts (back ear,
//!    tail) → body → near parts (front paw, collar, bow) → face → highlights. Parts that join
//!    (ear into head, arm into body) should *tuck* — the near part's `backing()` covers the
//!    far part's contour so no line crosses where the two meet.
//! 3. **Everything sits on something.** Objects resting on a surface get [`contact_shadow`]
//!    at their base; floating objects (steam, sparkles) get none.
//! 4. **Kawaii face grammar.** Eyes sit just below the horizontal centre of the head, blush
//!    sits under and outside the eyes, the mouth is small. Faces are drawn mostly in the key
//!    ink, which the press keeps in register so expressions stay crisp.
//! 5. **Four inks only** (see [`crate::ink::Ink`]). Mix colours by overprinting: yellow +
//!    pink = coral/orange crust, yellow + blue = leaf green, blue + pink = plum. Tints and
//!    shading use halftone (`ht`) or the press's grain; big flat solids should be rare.
//! 6. **Opaque objects use `backing()`** so the multiply-printed layers below don't show
//!    through; see [`crate::draw::DrawList::backing`].
//!
//! # Anchors
//! Asset origins are a contract with the screen layouts: jars, critters and Toasty are
//! drawn with their **bottom centre** at the origin; loaves, dough, treats and icons are
//! **centred**. Footprint constants live next to each asset (`jar::JAR_W`, `critters::BUST_H`,
//! `oven::OVEN_W`, ...). Changing an origin or a footprint is an API change.

use crate::draw::DrawList;
use crate::geom::{V2, ellipse};
use crate::ink::Ink;

/// Silhouette contour weight at 1× art scale.
pub const OUTER: f32 = 5.0;
/// Internal structure lines.
pub const INNER: f32 = 3.2;
/// Fine details (whiskers, stitches, seeds, crumb holes).
pub const DETAIL: f32 = 2.0;

/// A grounding shadow under an object resting on a surface: a tight, denser core where the
/// object touches plus a soft halftone falloff. `c` is the contact point.
pub fn contact_shadow(d: &mut DrawList, c: V2, rx: f32, ry: f32) {
    d.ht(Ink::Blue, 0.34, &ellipse(c, rx, ry, 0.0));
    d.ht_add(Ink::Blue, 0.3, &ellipse(c, rx * 0.72, ry * 0.62, 0.0));
}
