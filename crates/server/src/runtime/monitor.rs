use std::time::{Duration, Instant};

use crate::log::warn_log;

const WINDOW: Duration = Duration::from_secs(1);
const LOW_TPS: f64 = 10.0;
const CRITICAL_TPS: f64 = 5.0;
const LABELS: [&str; 2] = ["mspt spike", "slow traffic"];

#[derive(Debug, Clone, Copy)]
pub(super) enum Work {
    Tick,
    Traffic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Health {
    Normal,
    Low,
    Critical,
}

impl Health {
    fn of(tps: f64, rate: u32) -> Self {
        let rate = f64::from(rate);
        if rate > CRITICAL_TPS && tps <= CRITICAL_TPS {
            Self::Critical
        } else if rate > LOW_TPS && tps <= LOW_TPS {
            Self::Low
        } else {
            Self::Normal
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct Stall {
    count: u32,
    worst: Duration,
}

#[derive(Debug)]
pub(super) struct Monitor {
    window: Instant,
    ticks: u32,
    stalls: [Stall; 2],
    health: Health,
}

impl Monitor {
    pub(super) fn new() -> Self {
        Self {
            window: Instant::now(),
            ticks: 0,
            stalls: [Stall::default(); 2],
            health: Health::Normal,
        }
    }

    pub(super) fn measure<T>(
        &mut self,
        work: Work,
        budget: Duration,
        run: impl FnOnce() -> T,
    ) -> T {
        let started = Instant::now();
        let result = run();
        let took = started.elapsed();
        if took > budget {
            let stall = &mut self.stalls[work as usize];
            stall.count += 1;
            stall.worst = stall.worst.max(took);
        }
        result
    }

    pub(super) fn tick(&mut self, rate: u32) {
        self.ticks += 1;
        let elapsed = self.window.elapsed();
        if elapsed < WINDOW {
            return;
        }
        self.report(elapsed, rate);
        self.window = Instant::now();
        self.ticks = 0;
        self.stalls = [Stall::default(); 2];
    }

    fn report(&mut self, elapsed: Duration, rate: u32) {
        let seconds = elapsed.as_secs_f64();
        let tps = f64::from(self.ticks) / seconds;
        let health = Health::of(tps, rate);
        if health != self.health {
            match health {
                Health::Normal => warn_log!("tps recovered to {tps:.1}"),
                Health::Low => warn_log!("tps dropped to {tps:.1} (target {rate})"),
                Health::Critical => warn_log!("tps critically low at {tps:.1} (target {rate})"),
            }
            self.health = health;
        }

        let budget = 1000.0 / f64::from(rate);
        for (label, stall) in LABELS.iter().zip(self.stalls) {
            if stall.count > 0 {
                warn_log!(
                    "{label}: {} over {budget:.1} ms in the last {seconds:.1}s, worst {:.1} ms",
                    stall.count,
                    stall.worst.as_secs_f64() * 1000.0,
                );
            }
        }
    }
}
