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
        Rgb(
            ((h >> 16) & 0xff) as f32 / 255.0,
            ((h >> 8) & 0xff) as f32 / 255.0,
            (h & 0xff) as f32 / 255.0,
        )
    }
    pub fn lerp(self, o: Rgb, t: f32) -> Rgb {
        Rgb(
            self.0 + (o.0 - self.0) * t,
            self.1 + (o.1 - self.1) * t,
            self.2 + (o.2 - self.2) * t,
        )
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
    pub fn palette(self) -> Palette {
        match self {
            // Morning bake: coral-pink, sunflower, aqua, warm brown key.
            Edition::Dawn => Palette {
                paper: Rgb::hex(0xFBF3E6),
                inks: [
                    Rgb::hex(0xFF6F91),
                    Rgb::hex(0xFFD23F),
                    Rgb::hex(0x62C6E0),
                    Rgb::hex(0x6B3F35),
                ],
            },
            // Shop day: fluorescent pink, yellow, riso blue, navy key.
            Edition::Daylight => Palette {
                paper: Rgb::hex(0xF8F2E6),
                inks: [
                    Rgb::hex(0xFF5DAE),
                    Rgb::hex(0xFFE24A),
                    Rgb::hex(0x3E8FD0),
                    Rgb::hex(0x2E3A6B),
                ],
            },
            // Evening prep: berry pink, lamp-light sunflower, violet-blue, plum key.
            Edition::Dusk => Palette {
                paper: Rgb::hex(0xF1E6D6),
                inks: [
                    Rgb::hex(0xF0609E),
                    Rgb::hex(0xF9BE4B),
                    Rgb::hex(0x6A6CC4),
                    Rgb::hex(0x33264F),
                ],
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
