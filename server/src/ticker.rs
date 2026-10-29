use std::{thread, time::{Duration, Instant}};
use thread_priority::{ThreadBuilderExt, ThreadPriority};
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
        let mut tick_speed_rx = self.tick_speed_rx;

        thread::Builder::new()
            .name("cobalt-serverticker".to_string())
            .spawn_with_priority(ThreadPriority::Max, move |r| {
                if r.is_err() {
                    eprintln!("[ticker] failed to set max thread priority..");
                }

                let mut current_speed = *tick_speed_rx.borrow();
                let mut target_dt = Duration::from_secs(1) / current_speed;
                let mut interval = spin_sleep_util::interval(target_dt);
                
                let mut last_successful_tick = Instant::now();
                let mut dropped_ticks_counter = 0u64;

                loop {
                    if tick_speed_rx.has_changed().unwrap_or(false) {
                        let new_speed = *tick_speed_rx.borrow_and_update();
                        if new_speed != current_speed && new_speed > 0 {
                            current_speed = new_speed;
                            target_dt = Duration::from_secs(1) / current_speed;
                            interval = spin_sleep_util::interval(target_dt);
                        }
                    }

                    let scheduled_tick_time = interval.tick(); 

                    let now = Instant::now();
                    let thread_delay = now.duration_since(scheduled_tick_time);

                    if thread_delay > target_dt {
                        eprintln!(
                            "[ticker] we are lagging behind! a tick took {:.2}ms. missed approx {:.4} tick(s)",
                            thread_delay.as_secs_f32() * 1000.0,
                            thread_delay.as_secs_f32() / target_dt.as_secs_f32()
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

