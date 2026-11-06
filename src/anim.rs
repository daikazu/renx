//! Animation timing and small pure helpers. All state lives in `App`; this
//! module only knows about ticks, effects, and how to describe a change.

/// Milliseconds between animation frames (25 fps).
pub const TICK_MS: u64 = 40;
/// Length of the startup draw-in.
pub const INTRO_TICKS: u64 = 12;
/// How long a revealed rename row stays bright before settling.
pub const GLOW_TICKS: u64 = 12;
/// Target length of the whole rename cascade, regardless of row count.
pub const CASCADE_TICKS: u64 = 30;
/// Flash on a newly added or edited operation.
pub const OP_FLASH_TICKS: u64 = 8;
/// Pop on a toggled file.
pub const FILE_POP_TICKS: u64 = 5;
/// Period of the preview arrow pulse.
pub const PULSE_PERIOD: u64 = 40;

/// A short-lived visual effect tied to one row.
#[derive(Clone, Debug, PartialEq)]
pub enum EffectKind {
    /// Flash the operation at this index in the stack.
    OpFlash(usize),
    /// Pop the selection marker of the file at this index in `App::files`.
    FilePop(usize),
}

#[derive(Clone, Debug)]
pub struct Effect {
    pub kind: EffectKind,
    pub started: u64,
    pub duration: u64,
}

impl Effect {
    /// Progress in `0.0..1.0`, or `None` once the effect has run its course.
    pub fn progress(&self, tick: u64) -> Option<f32> {
        let elapsed = tick.saturating_sub(self.started);
        if elapsed >= self.duration {
            None
        } else {
            Some(elapsed as f32 / self.duration as f32)
        }
    }
}

/// One file in the post-rename results screen.
#[derive(Clone, Debug)]
pub struct DoneRow {
    pub old: String,
    pub new: String,
    pub ok: bool,
}

/// State of the results screen shown after renaming.
#[derive(Clone, Debug)]
pub struct DoneState {
    pub rows: Vec<DoneRow>,
    pub started: u64,
    /// Tick at which the user skipped the cascade, if they did.
    pub skipped_at: Option<u64>,
    pub success: usize,
    pub failed: usize,
}

impl DoneState {
    pub fn new(rows: Vec<DoneRow>, started: u64) -> Self {
        let success = rows.iter().filter(|r| r.ok).count();
        let failed = rows.len() - success;
        DoneState {
            rows,
            started,
            skipped_at: None,
            success,
            failed,
        }
    }

    /// Ticks between row reveals, and rows revealed per step, chosen so the
    /// whole cascade takes roughly `CASCADE_TICKS` however many rows there are.
    fn pacing(&self) -> (u64, u64) {
        let n = self.rows.len().max(1) as u64;
        let delay = (CASCADE_TICKS / n).clamp(1, 4);
        let per_step = (n / CASCADE_TICKS).max(1);
        (delay, per_step)
    }

    /// How many rows are showing their new name at `tick`.
    pub fn revealed(&self, tick: u64, animate: bool) -> usize {
        if !animate || self.skipped_at.is_some() {
            return self.rows.len();
        }
        let (delay, per_step) = self.pacing();
        let elapsed = tick.saturating_sub(self.started);
        ((elapsed / delay) * per_step).min(self.rows.len() as u64) as usize
    }

    /// The tick at which row `i` was (or will be) revealed.
    fn reveal_tick(&self, i: usize) -> u64 {
        let (delay, per_step) = self.pacing();
        let natural = self.started + ((i as u64) / per_step + 1) * delay;
        match self.skipped_at {
            Some(at) => natural.min(at),
            None => natural,
        }
    }

    /// Brightness of row `i`: `1.0` the moment it appears, fading to `0.0`.
    /// `None` while the row is still waiting to be revealed.
    pub fn row_glow(&self, i: usize, tick: u64, animate: bool) -> Option<f32> {
        if i >= self.revealed(tick, animate) {
            return None;
        }
        if !animate {
            return Some(0.0);
        }
        let since = tick.saturating_sub(self.reveal_tick(i)) as f32;
        Some((1.0 - since / GLOW_TICKS as f32).clamp(0.0, 1.0))
    }

    /// Whether every row is revealed and finished glowing.
    pub fn complete(&self, tick: u64, animate: bool) -> bool {
        let n = self.rows.len();
        self.revealed(tick, animate) == n
            && (0..n).all(|i| self.row_glow(i, tick, animate) == Some(0.0))
    }
}

/// Split `new` into segments marked `true` where it differs from `old`,
/// using the common prefix and suffix. A common suffix shorter than two
/// characters is treated as part of the change so `jpg -> png` reads as one
/// edit rather than `pn` plus an unchanged `g`.
pub fn diff_segments(old: &str, new: &str) -> Vec<(String, bool)> {
    let o: Vec<char> = old.chars().collect();
    let n: Vec<char> = new.chars().collect();

    let prefix = o.iter().zip(&n).take_while(|(a, b)| a == b).count();
    let max_suffix = o.len().min(n.len()) - prefix;
    let mut suffix = o
        .iter()
        .rev()
        .zip(n.iter().rev())
        .take(max_suffix)
        .take_while(|(a, b)| a == b)
        .count();
    if suffix < 2 {
        suffix = 0;
    }

    let mut out = Vec::new();
    let push = |out: &mut Vec<(String, bool)>, chars: &[char], changed: bool| {
        if !chars.is_empty() {
            out.push((chars.iter().collect(), changed));
        }
    };
    push(&mut out, &n[..prefix], false);
    push(&mut out, &n[prefix..n.len() - suffix], true);
    push(&mut out, &n[n.len() - suffix..], false);
    out
}

/// A smooth 0..1 wave with the given period in ticks.
pub fn pulse(tick: u64, period: u64) -> f32 {
    let phase = (tick % period) as f32 / period as f32;
    0.5 + 0.5 * (phase * std::f32::consts::TAU).sin()
}

/// Linear interpolation.
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}
