//! Preserve the former Windows recovery of an audible short utterance when
//! Silero missed it. Phrase segmentation belongs to the native VAD.
pub(crate) const SHORT_MAX: usize = 16_000 * 6;

#[derive(Default)]
pub struct ShortUtterance {
    beginning: Vec<f32>,
    captured: usize,
    detected: bool,
}
impl ShortUtterance {
    pub fn accept(&mut self, pcm: &[f32]) {
        self.captured += pcm.len();
        if !self.detected && self.beginning.len() < SHORT_MAX {
            let count = pcm.len().min(SHORT_MAX - self.beginning.len());
            self.beginning.extend_from_slice(&pcm[..count]);
        }
    }
    pub fn detected(&mut self, speech: bool) {
        if speech {
            self.detected = true;
            self.beginning.clear();
        }
    }
    pub fn finish(self) -> Vec<Vec<f32>> {
        if !self.detected && short_utterance(&self.beginning, self.captured) {
            vec![self.beginning]
        } else {
            Vec::new()
        }
    }
}

fn short_utterance(samples: &[f32], captured: usize) -> bool {
    if !(2_400..=SHORT_MAX).contains(&captured) || samples.len() < 2_400 {
        return false;
    }
    let mut squares = 0.0_f64;
    let mut peak = 0.0_f32;
    for &sample in samples {
        squares += f64::from(sample) * f64::from(sample);
        peak = peak.max(sample.abs());
    }
    (squares / samples.len() as f64).sqrt() >= 0.0025 && peak >= 0.012
}
