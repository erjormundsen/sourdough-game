//! Scoring (the blade kind): pattern templates and the stroke matcher.
//!
//! All coordinates are loaf-normalised: the loaf outline spans ±rx × ±ry from
//! [`Shape::radii`], y points down. A player's cuts are compared with templates by
//! resampling every stroke, pairing strokes greedily by mean distance (either direction),
//! and turning the distance into a 0..1 match. Freehand is always valid; matching a
//! pattern is a bonus and lets us *name* what the player drew.

use crate::content::{Pattern, Shape};
use crate::geom::{V2, dist_to_polyline, polyline_len, quad_bezier, resample, v2};
use serde::{Deserialize, Serialize};

const SAMPLES: usize = 16;

fn line(a: V2, b: V2) -> Vec<V2> {
    vec![a, b]
}

/// Template strokes for a pattern on a boule, in unit coordinates.
fn boule_template(p: Pattern) -> Vec<Vec<V2>> {
    match p {
        Pattern::Ear => vec![quad_bezier(v2(-0.6, 0.32), v2(-0.05, -0.28), v2(0.6, 0.12), 10)],
        Pattern::Cross => vec![line(v2(0.0, -0.62), v2(0.0, 0.62)), line(v2(-0.62, 0.0), v2(0.62, 0.0))],
        Pattern::Wheat => {
            let mut v = vec![line(v2(0.0, -0.66), v2(0.0, 0.7))];
            for i in 0..4 {
                let y = -0.34 + i as f32 * 0.28;
                v.push(line(v2(-0.1, y), v2(-0.4, y - 0.24)));
                v.push(line(v2(0.1, y), v2(0.4, y - 0.24)));
            }
            v
        }
        Pattern::Leaf => {
            let mut v = vec![
                quad_bezier(v2(0.0, 0.7), v2(0.08, 0.0), v2(0.0, -0.68), 10),
                quad_bezier(v2(0.0, -0.68), v2(-0.62, -0.1), v2(0.0, 0.7), 12),
                quad_bezier(v2(0.0, -0.68), v2(0.62, -0.1), v2(0.0, 0.7), 12),
            ];
            for (i, len) in [0.2f32, 0.28, 0.2].iter().enumerate() {
                let y = -0.3 + i as f32 * 0.3;
                v.push(line(v2(0.03, y + 0.1), v2(0.03 + len, y - 0.08)));
                v.push(line(v2(0.03, y + 0.1), v2(0.03 - len, y - 0.08)));
            }
            v
        }
    }
}

/// Template strokes for `p` on `shape`.
pub fn template(p: Pattern, shape: Shape) -> Vec<Vec<V2>> {
    match shape {
        Shape::Boule => boule_template(p),
        Shape::Batard => {
            if p == Pattern::Ear {
                return vec![quad_bezier(v2(-1.0, 0.1), v2(0.0, -0.12), v2(1.0, -0.08), 10)];
            }
            // Lay the boule design along the long axis.
            let (rx, ry) = shape.radii();
            boule_template(p)
                .into_iter()
                .map(|s| s.into_iter().map(|q| v2(-q.y * rx, q.x * ry)).collect())
                .collect()
        }
    }
}

/// Mean distance between two resampled strokes, trying both directions.
fn stroke_dist(a: &[V2], b: &[V2]) -> f32 {
    let fwd: f32 = a.iter().zip(b).map(|(p, q)| p.dist(*q)).sum::<f32>() / a.len() as f32;
    let rev: f32 = a.iter().zip(b.iter().rev()).map(|(p, q)| p.dist(*q)).sum::<f32>() / a.len() as f32;
    fwd.min(rev)
}

/// How well `cuts` reproduce `template`, 0..1.
pub fn match_score(cuts: &[Vec<V2>], template: &[Vec<V2>]) -> f32 {
    if cuts.is_empty() || template.is_empty() {
        return 0.0;
    }
    let tpl: Vec<Vec<V2>> = template.iter().map(|s| resample(s, SAMPLES)).collect();
    let mut left: Vec<Vec<V2>> = cuts.iter().filter(|c| c.len() >= 2).map(|s| resample(s, SAMPLES)).collect();
    let sigma = 0.16;
    let mut total = 0.0;
    for t in &tpl {
        let best =
            left.iter().enumerate().map(|(i, c)| (i, stroke_dist(t, c))).min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((i, d)) = best {
            total += (-(d / sigma).powi(2)).exp();
            left.remove(i);
        }
    }
    let base = total / tpl.len() as f32;
    // Extra cuts muddle the design.
    let extra = left.len() as f32;
    (base * (1.0 - 0.12 * extra).max(0.3)).clamp(0.0, 1.0)
}

/// How clean a single stroke is: long, decisive and not wobbly (0..1).
pub fn cleanliness(stroke: &[V2]) -> f32 {
    if stroke.len() < 2 {
        return 0.0;
    }
    let len = polyline_len(stroke);
    if len < 0.08 {
        return 0.2;
    }
    let chord = stroke[0].dist(*stroke.last().unwrap());
    // Wiggle: how far the path strays from a smooth resampled version of itself.
    let smooth = resample(stroke, 6);
    let wiggle: f32 = stroke.iter().map(|p| dist_to_polyline(*p, &smooth)).sum::<f32>() / stroke.len() as f32;
    let straightness = (chord / len).clamp(0.0, 1.0);
    let steady = (1.0 - wiggle * 12.0).clamp(0.0, 1.0);
    (0.35 + 0.35 * steady + 0.3 * straightness.powf(0.5)).clamp(0.0, 1.0)
}

/// Mirror symmetry of a set of cuts about the vertical axis (0..1).
pub fn symmetry(cuts: &[Vec<V2>]) -> f32 {
    if cuts.is_empty() {
        return 0.0;
    }
    let mirrored: Vec<Vec<V2>> = cuts.iter().map(|s| s.iter().map(|p| v2(-p.x, p.y)).collect()).collect();
    match_score(&mirrored, cuts)
}

/// Result of scoring one loaf.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScoreReport {
    /// Best recognised pattern and how well it matched.
    pub pattern: Option<Pattern>,
    pub pattern_match: f32,
    /// Average stroke cleanliness.
    pub clean: f32,
    /// Overall looks contribution 0..1.
    pub looks: f32,
    pub cut_count: u32,
}

/// Evaluate a scored loaf. `guide` is the pattern the player chose to trace (if any).
pub fn evaluate(cuts: &[Vec<V2>], shape: Shape, guide: Option<Pattern>) -> ScoreReport {
    let cuts: Vec<Vec<V2>> =
        cuts.iter().filter(|c| c.len() >= 2 && polyline_len(c) > 0.05).cloned().collect();
    let mut best: Option<(Pattern, f32)> = None;
    for p in Pattern::ALL {
        let m = match_score(&cuts, &template(p, shape));
        if best.is_none_or(|(_, bm)| m > bm) {
            best = Some((p, m));
        }
    }
    let clean = if cuts.is_empty() {
        0.0
    } else {
        cuts.iter().map(|c| cleanliness(c)).sum::<f32>() / cuts.len() as f32
    };
    let (pattern, pattern_match) = match (guide, best) {
        (Some(g), _) => (Some(g), match_score(&cuts, &template(g, shape))),
        (None, Some((p, m))) if m >= 0.55 => (Some(p), m),
        (None, Some((_, m))) => (None, m),
        _ => (None, 0.0),
    };
    let named = if pattern.is_some() { pattern_match } else { 0.0 };
    let n = cuts.len() as f32;
    // Freehand is always fine: 1–6 clean cuts look good; a recognised pattern looks great.
    let variety = if n == 0.0 { 0.0 } else { (1.0 - ((n - 3.0).abs() / 6.0)).clamp(0.4, 1.0) };
    let looks = (0.45 * clean + 0.2 * variety + 0.35 * named.max(symmetry(&cuts) * 0.6)).clamp(0.0, 1.0);
    ScoreReport { pattern, pattern_match, clean, looks, cut_count: cuts.len() as u32 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Rng;

    fn jitter(strokes: &[Vec<V2>], amt: f32, seed: u64) -> Vec<Vec<V2>> {
        let mut r = Rng::new(seed);
        strokes
            .iter()
            .map(|s| {
                resample(s, 20).into_iter().map(|p| p + v2(r.range(-amt, amt), r.range(-amt, amt))).collect()
            })
            .collect()
    }

    #[test]
    fn perfect_template_matches_itself() {
        for shape in [Shape::Boule, Shape::Batard] {
            for p in Pattern::ALL {
                let t = template(p, shape);
                let m = match_score(&t, &t);
                assert!(m > 0.95, "{p:?} {shape:?} {m}");
            }
        }
    }

    #[test]
    fn jittered_template_still_matches() {
        for p in Pattern::ALL {
            let t = template(p, Shape::Boule);
            let j = jitter(&t, 0.04, 3);
            let m = match_score(&j, &t);
            assert!(m > 0.8, "{p:?} {m}");
        }
    }

    #[test]
    fn random_scribbles_do_not_match() {
        let mut r = Rng::new(9);
        for p in Pattern::ALL {
            let t = template(p, Shape::Boule);
            let scribble: Vec<Vec<V2>> = (0..t.len())
                .map(|_| (0..6).map(|_| v2(r.range(-0.8, 0.8), r.range(-0.8, 0.8))).collect())
                .collect();
            let m = match_score(&scribble, &t);
            assert!(m < 0.3, "{p:?} {m}");
        }
    }

    #[test]
    fn freehand_cross_is_recognised_and_reversed_strokes_count() {
        let mut cuts = template(Pattern::Cross, Shape::Boule);
        cuts[0].reverse();
        let r = evaluate(&cuts, Shape::Boule, None);
        assert_eq!(r.pattern, Some(Pattern::Cross));
        assert!(r.looks > 0.7, "{r:?}");
    }

    #[test]
    fn decisive_strokes_are_cleaner_than_wobbly_ones() {
        let straight: Vec<V2> = (0..20).map(|i| v2(-0.6 + i as f32 * 0.06, 0.0)).collect();
        let wobbly: Vec<V2> =
            (0..20).map(|i| v2(-0.6 + i as f32 * 0.06, if i % 2 == 0 { 0.06 } else { -0.06 })).collect();
        assert!(cleanliness(&straight) > cleanliness(&wobbly) + 0.1);
    }

    #[test]
    fn no_cuts_scores_zero_looks() {
        let r = evaluate(&[], Shape::Boule, None);
        assert_eq!(r.cut_count, 0);
        assert_eq!(r.looks, 0.0);
    }
}
