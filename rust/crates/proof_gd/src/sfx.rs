//! Plays the Rust-synthesised sound effects through a small pool of players.

use godot::classes::audio_stream_wav::Format;
use godot::classes::{AudioStreamPlayer, AudioStreamWav, Input, Node};
use godot::prelude::*;
pub use proof_core::sfx::Sound as Sfx;
use proof_core::sfx::{RATE, render, to_pcm16};
use std::collections::HashMap;

pub struct Mixer {
    streams: HashMap<Sfx, Gd<AudioStreamWav>>,
    players: Vec<Gd<AudioStreamPlayer>>,
    next: usize,
    pub muted: bool,
    pub haptics: bool,
}

impl Mixer {
    pub fn new(parent: &mut Gd<Node>) -> Mixer {
        let mut streams = HashMap::new();
        for s in Sfx::ALL {
            let mut w = AudioStreamWav::new_gd();
            w.set_format(Format::FORMAT_16_BITS);
            w.set_mix_rate(RATE as i32);
            w.set_stereo(false);
            w.set_data(&PackedByteArray::from(to_pcm16(&render(s))));
            streams.insert(s, w);
        }
        let players = (0..8)
            .map(|_| {
                let p = AudioStreamPlayer::new_alloc();
                parent.add_child(&p);
                p
            })
            .collect();
        Mixer { streams, players, next: 0, muted: false, haptics: true }
    }

    pub fn play(&mut self, s: Sfx, pitch: f32) {
        if self.muted {
            return;
        }
        let Some(stream) = self.streams.get(&s) else { return };
        let n = self.players.len();
        let p = &mut self.players[self.next];
        self.next = (self.next + 1) % n;
        p.set_stream(stream);
        p.set_pitch_scale(pitch.clamp(0.5, 2.0));
        p.set_volume_db(-4.0);
        p.play();
    }

    pub fn stop_all(&mut self) {
        for p in &mut self.players {
            p.stop();
            p.set_stream(Gd::<godot::classes::AudioStream>::null_arg());
        }
    }

    pub fn buzz(&self, ms: i32) {
        if self.haptics {
            Input::singleton().vibrate_handheld_ex().duration_ms(ms).done();
        }
    }
}
