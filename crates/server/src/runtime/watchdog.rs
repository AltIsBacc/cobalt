use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering::Relaxed};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};

use crate::log::warn_log;

const THREAD: &str = "cobalt-watchdog";
const POLL: Duration = Duration::from_millis(250);
const WINDOW: Duration = Duration::from_secs(1);
const HANG: Duration = Duration::from_secs(1);
const IDLE: u64 = 0;
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

impl Work {
    fn activity(self) -> &'static str {
        match self {
            Self::Tick => "in a tick",
            Self::Traffic => "handling traffic",
        }
    }
}

#[derive(Default)]
struct Stall {
    count: AtomicU32,
    worst: AtomicU64,
}

struct Pulse {
    origin: Instant,
    ticks: AtomicU64,
    rate: AtomicU32,
    busy: AtomicU64,
    stalls: [Stall; 2],
}

#[derive(Clone)]
pub(super) struct Heartbeat {
    pulse: Arc<Pulse>,
    _alive: Sender<()>,
}

impl Heartbeat {
    pub(super) fn start(rate: u32) -> Self {
        let pulse = Arc::new(Pulse {
            origin: Instant::now(),
            ticks: AtomicU64::new(0),
            rate: AtomicU32::new(rate),
            busy: AtomicU64::new(IDLE),
            stalls: Default::default(),
        });
        let (alive, stop) = mpsc::channel();
        let watched = Arc::clone(&pulse);

        let spawned =
            thread::Builder::new().name(THREAD.into()).spawn(move || watch(&watched, &stop));
        if let Err(error) = spawned {
            warn_log!("could not start the watchdog: {error}");
        }
        Self { pulse, _alive: alive }
    }

    pub(super) fn tick(&self, rate: u32) {
        self.pulse.rate.store(rate, Relaxed);
        self.pulse.ticks.fetch_add(1, Relaxed);
    }

    pub(super) fn measure<T>(&self, work: Work, budget: Duration, run: impl FnOnce() -> T) -> T {
        let started = Instant::now();
        let since = started.duration_since(self.pulse.origin);
        self.pulse.busy.store(pack(since, work), Relaxed);
        let result = run();
        self.pulse.busy.store(IDLE, Relaxed);
        let took = started.elapsed();
        if took > budget {
            let stall = &self.pulse.stalls[work as usize];
            stall.count.fetch_add(1, Relaxed);
            stall.worst.fetch_max(nanos(took), Relaxed);
        }
        result
    }
}

struct Watch {
    window: Instant,
    ticks: u64,
    health: Health,
    reported: u64,
}

fn watch(pulse: &Pulse, stop: &Receiver<()>) {
    let mut watch =
        Watch { window: Instant::now(), ticks: 0, health: Health::Normal, reported: IDLE };
    while matches!(stop.recv_timeout(POLL), Err(RecvTimeoutError::Timeout)) {
        watch.poll(pulse);
    }
}

impl Watch {
    fn poll(&mut self, pulse: &Pulse) {
        self.check_hang(pulse);
        if self.window.elapsed() >= WINDOW {
            self.report(pulse);
        }
    }

    fn check_hang(&mut self, pulse: &Pulse) {
        let busy = pulse.busy.load(Relaxed);
        if busy == IDLE || busy == self.reported {
            return;
        }
        let (since, work) = unpack(busy);
        let stuck = pulse.origin.elapsed().saturating_sub(since);
        if stuck >= HANG {
            warn_log!("engine stuck {} for {:.1}s", work.activity(), stuck.as_secs_f64());
            self.reported = busy;
        }
    }

    fn report(&mut self, pulse: &Pulse) {
        let seconds = self.window.elapsed().as_secs_f64();
        self.window = Instant::now();

        let ticks = pulse.ticks.load(Relaxed);
        if ticks == 0 {
            return;
        }
        let rate = pulse.rate.load(Relaxed);
        let tps = f64::from(u32::try_from(ticks - self.ticks).unwrap_or(u32::MAX)) / seconds;
        self.ticks = ticks;

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
        for (label, stall) in LABELS.iter().zip(&pulse.stalls) {
            let count = stall.count.swap(0, Relaxed);
            if count > 0 {
                let worst = Duration::from_nanos(stall.worst.swap(0, Relaxed));
                warn_log!(
                    "{label}: {count} over {budget:.1} ms in the last {seconds:.1}s, worst {:.1} ms",
                    worst.as_secs_f64() * 1000.0,
                );
            }
        }
    }
}

fn pack(since: Duration, work: Work) -> u64 {
    ((nanos(since) + 1) << 1) | work as u64
}

fn unpack(busy: u64) -> (Duration, Work) {
    let work = if busy & 1 == 0 { Work::Tick } else { Work::Traffic };
    (Duration::from_nanos((busy >> 1) - 1), work)
}

fn nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}
