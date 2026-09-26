//! The regulars: eight animal customers drawn as shoulder-up portraits.

use super::{Expr, FaceParts, LINE, face, face_parts};
use crate::content::Species;
use crate::draw::{DrawList, Paint};
use crate::geom::{
    V2, arc, chaikin, circle, ellipse, quad_bezier, rect, rect_poly, rounded_rect, scallop, star, v2,
};
use crate::ink::Ink;
use std::f32::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CritterView {
    pub species: Species,
    pub expr: Expr,
    /// Gentle idle bob phase (seconds).
    pub t: f32,
}

/// Shoulders with rounded top corners and a flat bottom at y = 0.
fn shoulders(w: f32, h: f32) -> Vec<V2> {
    let r = (w * 0.36).min(h);
    let mut p = vec![v2(-w * 0.5, 0.0)];
    p.extend(arc(v2(-w * 0.5 + r, -h + r), r, PI, PI * 1.5));
    p.extend(arc(v2(w * 0.5 - r, -h + r), r, PI * 1.5, PI * 2.0));
    p.push(v2(w * 0.5, 0.0));
    p
}

/// Fur/skin fills as ink recipes (overprints make browns, greens and greys).
fn brown(d: &mut DrawList, poly: &[V2], dark: f32) {
    d.backing(poly);
    d.fill(Ink::Yellow, 0.85, poly);
    d.fill(Ink::Pink, 0.52, poly);
    d.ht(Ink::Key, 0.12 + dark, poly);
}

fn paper(d: &mut DrawList, poly: &[V2]) {
    d.backing(poly);
}

fn outlined(d: &mut DrawList, poly: &[V2]) {
    d.outline(Ink::Key, LINE, poly);
}

/// Draw a customer with the bottom centre of their shoulders at the origin.
pub fn critter(d: &mut DrawList, v: &CritterView) {
    let bob = (v.t * 2.2).sin() * 3.0;
    let head_c = v2(0.0, -168.0 + bob);
    super::shadow(d, v2(0.0, 2.0), 110.0, 12.0);
    match v.species {
        Species::Bunny => bunny(d, head_c, v.expr),
        Species::Bear => bear(d, head_c, v.expr),
        Species::Duck => duck(d, head_c, v.expr),
        Species::Cat => cat(d, head_c, v.expr),
        Species::Hedgehog => hedgehog(d, head_c, v.expr),
        Species::Frog => frog(d, head_c, v.expr),
        Species::Sheep => sheep(d, head_c, v.expr),
        Species::Otter => otter(d, head_c, v.expr),
    }
}

fn bunny(d: &mut DrawList, h: V2, e: Expr) {
    let body = shoulders(200.0, 96.0);
    d.backing(&body);
    d.fill(Ink::Pink, 0.5, &body);
    d.ht_add(Ink::Pink, 0.25, &body);
    outlined(d, &body);
    // Peter-pan collar.
    for sx in [-1.0, 1.0] {
        let c = ellipse(v2(sx * 26.0, -88.0), 30.0, 17.0, sx * 0.35);
        paper(d, &c);
        outlined(d, &c);
    }
    for y in [-58.0, -32.0] {
        d.fill(Ink::Key, 1.0, &circle(v2(0.0, y), 4.5));
    }
    for (sx, rot) in [(-1.0, -0.16), (1.0, 0.2)] {
        let ear = ellipse(h + v2(sx * 32.0, -92.0), 22.0, 60.0, rot);
        paper(d, &ear);
        d.fill(Ink::Pink, 0.7, &ellipse(h + v2(sx * 32.0, -86.0), 10.0, 42.0, rot));
        outlined(d, &ear);
    }
    let head = ellipse(h, 76.0, 70.0, 0.0);
    paper(d, &head);
    outlined(d, &head);
    face(d, h + v2(0.0, 8.0), 96.0, e, V2::ZERO);
    d.fill(Ink::Pink, 1.0, &ellipse(h + v2(0.0, 13.0), 7.0, 5.0, 0.0));
}

fn bear(d: &mut DrawList, h: V2, e: Expr) {
    let body = shoulders(220.0, 100.0);
    paper(d, &body);
    d.backing(&body);
    d.fill(Ink::Yellow, 0.25, &body);
    outlined(d, &body);
    let bib = rounded_rect(rect(-52.0, -80.0, 104.0, 90.0), 14.0);
    d.backing(&bib);
    d.fill(Ink::Blue, 0.7, &bib);
    outlined(d, &bib);
    for sx in [-1.0, 1.0] {
        d.line(Ink::Key, 9.0, &[v2(sx * 44.0, -76.0), v2(sx * 70.0, -100.0)]);
        d.fill(Ink::Yellow, 1.0, &circle(v2(sx * 38.0, -64.0), 7.0));
        d.outline(Ink::Key, 2.5, &circle(v2(sx * 38.0, -64.0), 7.0));
    }
    for sx in [-1.0, 1.0] {
        let ear = circle(h + v2(sx * 58.0, -52.0), 26.0);
        brown(d, &ear, 0.0);
        d.fill(Ink::Pink, 0.9, &circle(h + v2(sx * 58.0, -50.0), 13.0));
        outlined(d, &ear);
    }
    let head = ellipse(h, 80.0, 72.0, 0.0);
    brown(d, &head, 0.0);
    outlined(d, &head);
    let muzzle = ellipse(h + v2(0.0, 26.0), 34.0, 25.0, 0.0);
    paper(d, &muzzle);
    d.backing(&muzzle);
    d.fill(Ink::Yellow, 0.35, &muzzle);
    outlined(d, &muzzle);
    face_parts(d, h + v2(0.0, 2.0), 108.0, e, V2::ZERO, FaceParts { eyes: true, mouth: false, blush: true });
    d.fill(Ink::Key, 1.0, &ellipse(h + v2(0.0, 16.0), 11.0, 7.5, 0.0));
    let r = 6.0;
    let m = h + v2(0.0, 30.0);
    let mut w = arc(m + v2(-r, 0.0), r, PI * 0.05, PI * 0.95);
    w.reverse();
    w.extend(arc(m + v2(r, 0.0), r, PI * 0.05, PI * 0.95).into_iter().rev());
    d.line(Ink::Key, 3.5, &w);
}

fn duck(d: &mut DrawList, h: V2, e: Expr) {
    let body = shoulders(196.0, 94.0);
    d.backing(&body);
    d.fill(Ink::Blue, 0.45, &body);
    outlined(d, &body);
    for y in [-62.0, -34.0] {
        d.fill(Ink::Yellow, 1.0, &circle(v2(14.0, y), 6.0));
        d.outline(Ink::Key, 2.5, &circle(v2(14.0, y), 6.0));
    }
    d.line(Ink::Key, 3.0, &[v2(0.0, -92.0), v2(0.0, 0.0)]);
    let head = ellipse(h, 72.0, 70.0, 0.0);
    d.backing(&head);
    d.fill(Ink::Yellow, 1.0, &head);
    outlined(d, &head);
    // Bucket hat: crown behind, band, brim in front.
    let crown = rounded_rect(rect(h.x - 50.0, h.y - 104.0, 100.0, 60.0), 26.0);
    d.knock(&crown);
    d.backing(&crown);
    d.fill(Ink::Blue, 0.62, &crown);
    outlined(d, &crown);
    let band = rect_poly(rect(h.x - 50.0, h.y - 66.0, 100.0, 13.0));
    d.knock(&band);
    d.backing(&band);
    d.fill(Ink::Pink, 0.85, &band);
    d.outline(Ink::Key, 3.0, &band);
    let brim = ellipse(h + v2(0.0, -50.0), 86.0, 17.0, -0.05);
    d.knock(&brim);
    d.backing(&brim);
    d.fill(Ink::Blue, 0.62, &brim);
    d.ht_add(Ink::Key, 0.12, &brim);
    outlined(d, &brim);
    face_parts(d, h + v2(0.0, 4.0), 100.0, e, V2::ZERO, FaceParts { eyes: true, mouth: false, blush: true });
    let beak = chaikin(
        &[
            h + v2(-30.0, 18.0),
            h + v2(0.0, 10.0),
            h + v2(30.0, 18.0),
            h + v2(20.0, 34.0),
            h + v2(-20.0, 34.0),
        ],
        2,
        true,
    );
    d.fill(Ink::Yellow, 1.0, &beak);
    d.fill(Ink::Pink, 0.8, &beak);
    outlined(d, &beak);
    d.line(Ink::Key, 2.5, &[h + v2(-24.0, 23.0), h + v2(24.0, 23.0)]);
}

fn cat(d: &mut DrawList, h: V2, e: Expr) {
    let body = shoulders(214.0, 98.0);
    d.backing(&body);
    d.fill(Ink::Key, 0.85, &body);
    outlined(d, &body);
    let shirt = vec![v2(-34.0, -98.0), v2(34.0, -98.0), v2(0.0, -20.0)];
    paper(d, &shirt);
    outlined(d, &shirt);
    let bow = [
        vec![v2(0.0, -80.0), v2(-28.0, -94.0), v2(-28.0, -66.0)],
        vec![v2(0.0, -80.0), v2(28.0, -94.0), v2(28.0, -66.0)],
    ];
    for b in &bow {
        let b = chaikin(b, 1, true);
        d.fill(Ink::Pink, 1.0, &b);
        outlined(d, &b);
    }
    d.fill(Ink::Pink, 1.0, &circle(v2(0.0, -80.0), 8.0));
    d.outline(Ink::Key, 3.0, &circle(v2(0.0, -80.0), 8.0));
    for sx in [-1.0f32, 1.0] {
        let ear =
            chaikin(&[h + v2(sx * 20.0, -50.0), h + v2(sx * 64.0, -86.0), h + v2(sx * 70.0, -26.0)], 1, true);
        d.backing(&ear);
        d.backing(&ear);
        d.fill(Ink::Blue, 0.38, &ear);
        d.ht(Ink::Key, 0.14, &ear);
        let inner =
            chaikin(&[h + v2(sx * 32.0, -44.0), h + v2(sx * 60.0, -70.0), h + v2(sx * 62.0, -34.0)], 1, true);
        d.fill(Ink::Pink, 0.75, &inner);
        outlined(d, &ear);
    }
    let head = ellipse(h, 84.0, 68.0, 0.0);
    d.backing(&head);
    d.fill(Ink::Blue, 0.38, &head);
    d.ht(Ink::Key, 0.14, &head);
    outlined(d, &head);
    face(d, h + v2(0.0, 6.0), 104.0, e, V2::ZERO);
    // Whiskers.
    for sx in [-1.0, 1.0] {
        for (i, dy) in [-6.0, 4.0, 14.0].iter().enumerate() {
            let a = h + v2(sx * 52.0, 20.0 + dy);
            let b = h + v2(sx * (92.0 - i as f32 * 4.0), 14.0 + dy * 1.6);
            d.line(Ink::Key, 2.2, &[a, b]);
        }
    }
    // Monocle.
    let mc = h + v2(26.0, 5.0);
    d.knock_p(0.5, crate::draw::Screen::Solid, 0b0111, &circle(mc, 20.0));
    d.outline(Ink::Key, 4.0, &circle(mc, 20.0));
    d.line(Ink::Key, 2.0, &quad_bezier(mc + v2(18.0, 10.0), mc + v2(40.0, 60.0), v2(40.0, -100.0), 12));
}

fn hedgehog(d: &mut DrawList, h: V2, e: Expr) {
    let body = shoulders(206.0, 96.0);
    d.backing(&body);
    d.fill(Ink::Pink, 0.5, &body);
    outlined(d, &body);
    let apron = rounded_rect(rect(-50.0, -84.0, 100.0, 100.0), 18.0);
    paper(d, &apron);
    outlined(d, &apron);
    let pocket = rounded_rect(rect(-22.0, -46.0, 44.0, 30.0), 8.0);
    d.backing(&pocket);
    d.fill(Ink::Pink, 0.35, &pocket);
    outlined(d, &pocket);
    let spikes = chaikin(&star(h + v2(0.0, -6.0), 104.0, 80.0, 16, 0.1), 1, true);
    d.backing(&spikes);
    d.fill(Ink::Yellow, 0.6, &spikes);
    d.backing(&spikes);
    d.fill(Ink::Pink, 0.4, &spikes);
    d.ht(Ink::Key, 0.4, &spikes);
    outlined(d, &spikes);
    let face_poly = ellipse(h + v2(0.0, 10.0), 66.0, 58.0, 0.0);
    paper(d, &face_poly);
    d.backing(&face_poly);
    d.fill(Ink::Yellow, 0.35, &face_poly);
    outlined(d, &face_poly);
    face(d, h + v2(0.0, 10.0), 92.0, e, V2::ZERO);
    d.fill(Ink::Key, 1.0, &ellipse(h + v2(0.0, 18.0), 7.0, 5.0, 0.0));
}

fn frog(d: &mut DrawList, h: V2, e: Expr) {
    let body = shoulders(216.0, 94.0);
    d.backing(&body);
    d.fill(Ink::Pink, 0.55, &body);
    outlined(d, &body);
    d.clipped(&body, |d| {
        for i in 0..14 {
            let x = -100.0 + (i % 5) as f32 * 50.0 + if (i / 5) % 2 == 1 { 25.0 } else { 0.0 };
            let y = -80.0 + (i / 5) as f32 * 32.0;
            d.fill(Ink::Yellow, 1.0, &circle(v2(x, y), 8.0));
        }
    });
    outlined(d, &body);
    let green = |d: &mut DrawList, p: &[V2]| {
        d.backing(p);
        d.fill(Ink::Yellow, 0.9, p);
        d.fill(Ink::Blue, 0.55, p);
    };
    for sx in [-1.0, 1.0] {
        let bump = circle(h + v2(sx * 42.0, -46.0), 30.0);
        green(d, &bump);
        outlined(d, &bump);
    }
    let head = ellipse(h + v2(0.0, 10.0), 88.0, 60.0, 0.0);
    green(d, &head);
    outlined(d, &head);
    // Hide the bump outlines inside the head, then draw the goggly eyes.
    for sx in [-1.0, 1.0] {
        let c = h + v2(sx * 42.0, -46.0);
        let white = circle(c, 19.0);
        paper(d, &white);
        outlined(d, &white);
        let pupil = match e {
            Expr::Happy | Expr::Proud | Expr::Sleepy => None,
            _ => Some(circle(c + v2(0.0, 3.0), 9.0)),
        };
        if let Some(p) = pupil {
            d.fill(Ink::Key, 1.0, &p);
            d.knock(&circle(c + v2(3.0, -1.0), 3.2));
        } else {
            d.line(Ink::Key, 4.0, &arc(c + v2(0.0, 6.0), 9.0, PI * 1.15, PI * 1.85));
        }
    }
    face_parts(
        d,
        h + v2(0.0, 22.0),
        112.0,
        e,
        V2::ZERO,
        FaceParts { eyes: false, mouth: false, blush: true },
    );
    let smile = arc(h + v2(0.0, 6.0), 40.0, PI * 0.2, PI * 0.8);
    d.line(Ink::Key, 4.0, &smile);
}

fn sheep(d: &mut DrawList, h: V2, e: Expr) {
    let body = scallop(v2(0.0, 0.0), 104.0, 16, 0.08);
    let bottom = rect_poly(rect(-140.0, -140.0, 280.0, 140.0));
    d.clipped(&bottom, |d| {
        d.knock(&body);
        outlined(d, &body);
    });
    d.line(Ink::Key, LINE, &[v2(-104.0, 0.0), v2(104.0, 0.0)]);
    let scarf = rounded_rect(rect(-72.0, -98.0, 144.0, 26.0), 13.0);
    d.knock(&scarf);
    d.backing(&scarf);
    d.fill(Ink::Blue, 0.6, &scarf);
    outlined(d, &scarf);
    let tail = rounded_rect(rect(26.0, -84.0, 28.0, 62.0), 12.0);
    d.knock(&tail);
    d.backing(&tail);
    d.fill(Ink::Blue, 0.6, &tail);
    outlined(d, &tail);
    let wool = scallop(h + v2(0.0, -6.0), 90.0, 14, 0.1);
    paper(d, &wool);
    outlined(d, &wool);
    for sx in [-1.0, 1.0] {
        let ear = ellipse(h + v2(sx * 66.0, 4.0), 26.0, 12.0, sx * 0.4);
        d.backing(&ear);
        d.fill(Ink::Yellow, 0.4, &ear);
        d.ht(Ink::Pink, 0.2, &ear);
        outlined(d, &ear);
    }
    let fc = ellipse(h + v2(0.0, 12.0), 50.0, 56.0, 0.0);
    d.backing(&fc);
    d.fill(Ink::Yellow, 0.4, &fc);
    d.ht(Ink::Pink, 0.14, &fc);
    outlined(d, &fc);
    let tuft = scallop(h + v2(0.0, -44.0), 30.0, 8, 0.2);
    paper(d, &tuft);
    outlined(d, &tuft);
    face(d, h + v2(0.0, 18.0), 80.0, e, V2::ZERO);
}

fn otter(d: &mut DrawList, h: V2, e: Expr) {
    let body = shoulders(206.0, 96.0);
    paper(d, &body);
    d.clipped(&body, |d| {
        for i in 0..6 {
            d.fill(Ink::Blue, 0.75, &rect_poly(rect(-120.0, -100.0 + i as f32 * 20.0, 240.0, 9.0)));
        }
    });
    outlined(d, &body);
    for sx in [-1.0, 1.0] {
        let ear = circle(h + v2(sx * 60.0, -46.0), 16.0);
        brown(d, &ear, 0.12);
        outlined(d, &ear);
    }
    let head = ellipse(h, 76.0, 68.0, 0.0);
    brown(d, &head, 0.12);
    outlined(d, &head);
    let cheeks =
        [ellipse(h + v2(-18.0, 26.0), 24.0, 18.0, 0.0), ellipse(h + v2(18.0, 26.0), 24.0, 18.0, 0.0)];
    for c in &cheeks {
        paper(d, c);
    }
    for c in &cheeks {
        d.stroke_p(Paint::solid(Ink::Key, 1.0), 3.0, c, true);
    }
    d.knock(&ellipse(h + v2(0.0, 26.0), 30.0, 12.0, 0.0));
    face_parts(d, h + v2(0.0, 0.0), 100.0, e, V2::ZERO, FaceParts { eyes: true, mouth: false, blush: true });
    d.fill(Ink::Key, 1.0, &ellipse(h + v2(0.0, 16.0), 10.0, 7.0, 0.0));
    for sx in [-1.0, 1.0] {
        for dy in [0.0, 9.0] {
            d.line(Ink::Key, 2.0, &[h + v2(sx * 30.0, 24.0 + dy), h + v2(sx * 64.0, 20.0 + dy * 1.5)]);
        }
    }
}
