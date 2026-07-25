use std::{thread, time::{Duration, Instant}};

use tokio::sync::{mpsc, watch};

pub struct ServerTicker {
    tick_speed_tx: watch::Sender<u32>,
    tick_speed_rx: watch::Receiver<u32>,
}

impl ServerTicker {
    pub fn new(tick_speed: u32) -> Self {
        let (
            tick_speed_tx,
            tick_speed_rx
        ) = watch::channel(tick_speed);

        Self { tick_speed_tx, tick_speed_rx }
    }

    pub fn rate_handle(&self) -> watch::Sender<u32> {
        self.tick_speed_tx.clone()
    }

    pub fn spawn(self, tick_callback: mpsc::Sender<f32>) -> thread::JoinHandle<()> {
        let tick_speed_rx = self.tick_speed_rx;

        thread::Builder::new()
            .name("cobalt-serverticker".to_string())
            .spawn(move || {
                let target_dt = Duration::from_secs(1) / *tick_speed_rx.borrow();
                let mut interval = spin_sleep_util::interval(target_dt);
                
                let mut last_successful_tick = Instant::now();
                let mut dropped_ticks_counter = 0u64;

                loop {
                    let scheduled_tick_time = interval.tick(); 

                    let now = Instant::now();
                    let thread_delay = now.duration_since(scheduled_tick_time);

                    if thread_delay > target_dt {
                        let missed_ticks = (thread_delay.as_secs_f64() / target_dt.as_secs_f64()).floor() as u64;
                        eprintln!(
                            "[ticker] we are lagging behind! a tick took {:.2}ms. missed approx {} tick(s)",
                            thread_delay.as_secs_f32() * 1000.0,
                            missed_ticks
                        );
                    }

                    let dt = now.duration_since(last_successful_tick).as_secs_f32();
                    match tick_callback.try_send(dt) {
                        Ok(_) => {  
                            if dropped_ticks_counter > 0 {
                                eprintln!(
                                    "[ticker] the server caught up! was {} ticks behind. current dt: {:.2}ms",
                                    dropped_ticks_counter,
                                    dt * 1000.0
                                );
                                dropped_ticks_counter = 0;
                            }
                            last_successful_tick = now;
                        }
                        Err(mpsc::error::TrySendError::Full(_)) => {
                            dropped_ticks_counter += 1;

                            if dropped_ticks_counter == 1 || (dropped_ticks_counter - 1) % 20 == 0 {
                                eprintln!("[ticker] the server is lagging behind! dropped {} tick(s) so far", dropped_ticks_counter);
                            }
                        }
                        Err(mpsc::error::TrySendError::Closed(_)) => {
                            break;
                        }                
                    }
                }
            }).expect("failed to start server ticker thread.")
    }
}

