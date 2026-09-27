//! Riso inks. The game prints with four spot-ink "slots"; each time-of-day *edition*
//! loads different real-world-ish riso ink colours into those slots.

use serde::{Deserialize, Serialize};

/// The four ink drums. Art is authored against slots, never raw colours.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Ink {
    /// Warm accent: blush, hearts, yeasties, browning overprint.
    Pink = 0,
    /// Warm fill: dough, crumb, butter, light.
    Yellow = 1,
    /// Cool: glass, lactos, shadows, water, overprints to green/violet.
    Blue = 2,
    /// Key line ink: outlines, eyes, text. Always kept in register.
    Key = 3,
}

impl Ink {
    pub const ALL: [Ink; 4] = [Ink::Pink, Ink::Yellow, Ink::Blue, Ink::Key];
    pub fn idx(self) -> usize {
        self as usize
    }
}

/// Linear-ish sRGB colour, 0..1.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rgb(pub f32, pub f32, pub f32);

impl Rgb {
    pub const fn hex(h: u32) -> Rgb {
        Rgb(((h >> 16) & 0xff) as f32 / 255.0, ((h >> 8) & 0xff) as f32 / 255.0, (h & 0xff) as f32 / 255.0)
    }
    pub fn lerp(self, o: Rgb, t: f32) -> Rgb {
        Rgb(self.0 + (o.0 - self.0) * t, self.1 + (o.1 - self.1) * t, self.2 + (o.2 - self.2) * t)
    }
    pub fn times(self, o: Rgb) -> Rgb {
        Rgb(self.0 * o.0, self.1 * o.1, self.2 * o.2)
    }
    pub fn to_u8(self) -> [u8; 3] {
        [
            (self.0.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.1.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.2.clamp(0.0, 1.0) * 255.0).round() as u8,
        ]
    }
}

/// A print "edition": paper + the colours loaded into each ink slot.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Palette {
    pub paper: Rgb,
    pub inks: [Rgb; 4],
}

/// Time-of-day editions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Edition {
    Dawn,
    Daylight,
    Dusk,
}

impl Edition {
    /// Each edition is a set of real Riso drum inks on a natural uncoated stock (hex values
    /// are the printed swatches of the named RISO inks; dark keys are printed dense, as riso
    /// studios double-hit their darkest drum).
    pub fn palette(self) -> Palette {
        match self {
            // Morning bake: Fluorescent Orange (coral), Sunflower, Aqua, Brown key.
            Edition::Dawn => Palette {
                paper: Rgb::hex(0xFBF3E4),
                inks: [Rgb::hex(0xFF7477), Rgb::hex(0xFFB511), Rgb::hex(0x5EC8E5), Rgb::hex(0x6A3D33)],
            },
            // Shop day: Fluorescent Pink, Yellow, Cornflower, Federal Blue key. (Cornflower
            // rather than Riso Blue keeps key-ink labels legible on blue buttons.)
            Edition::Daylight => Palette {
                paper: Rgb::hex(0xF8F2E5),
                inks: [Rgb::hex(0xFF48B0), Rgb::hex(0xFFE800), Rgb::hex(0x62A8E5), Rgb::hex(0x34457A)],
            },
            // Evening prep: berry pink, Melon lamp-light, violet, Plum key.
            Edition::Dusk => Palette {
                paper: Rgb::hex(0xF2E7D5),
                inks: [Rgb::hex(0xEC5A95), Rgb::hex(0xFFAE3B), Rgb::hex(0x6C66C0), Rgb::hex(0x35264F)],
            },
        }
    }
}

impl Palette {
    pub fn lerp(&self, o: &Palette, t: f32) -> Palette {
        Palette {
            paper: self.paper.lerp(o.paper, t),
            inks: [0, 1, 2, 3].map(|i| self.inks[i].lerp(o.inks[i], t)),
        }
    }

    /// Multiply-composite four ink coverages (0..1 each) over a base colour.
    pub fn composite(&self, base: Rgb, cov: [f32; 4]) -> Rgb {
        let mut c = base;
        for (i, k) in cov.iter().enumerate() {
            let k = k.clamp(0.0, 1.0);
            c = c.times(Rgb(1.0, 1.0, 1.0).lerp(self.inks[i], k));
        }
        c
    }
}
