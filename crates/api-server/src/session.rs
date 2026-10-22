//! Simulation sessions and the background runner.
//!
//! The runner wakes every [`TICK`] of wall-clock time and advances the simulation by
//! `speed * TICK` of simulated time, i.e. a whole number of fixed physics steps (the
//! fractional remainder is carried to the next tick). Changing the replay speed changes
//! only how many steps run per tick - never the physical time step - so live runs are
//! identical to batch runs of the same configuration. The stepping itself runs on the
//! blocking thread pool so that request handling is never blocked by computation.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use simulation_engine::{FinishReason, Simulation, SimulationConfig};
use uuid::Uuid;

/// Wall-clock period of the runner.
pub const TICK: Duration = Duration::from_millis(50);
/// Steps per tick when running "as fast as possible" (speed <= 0).
pub const MAX_STEPS_PER_TICK: u64 = 4000;
/// Maximum number of stored sessions; the oldest inactive sessions are evicted first.
pub const MAX_SESSIONS: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    Created,
    Running,
    Paused,
    Finished,
}

pub struct Session {
    pub id: Uuid,
    pub created_at_ms: u64,
    pub config: SimulationConfig,
    pub sim: Simulation,
    pub state: RunState,
    /// Simulated seconds per wall-clock second; `<= 0` = as fast as possible.
    pub speed: f64,
    pub runner_active: bool,
    step_remainder: f64,
}

impl Session {
    pub fn new(
        config: SimulationConfig,
        speed: f64,
    ) -> Result<Self, simulation_engine::SimulationError> {
        let sim = Simulation::new(config.clone())?;
        Ok(Self {
            id: Uuid::new_v4(),
            created_at_ms: now_ms(),
            config,
            sim,
            state: RunState::Created,
            speed,
            runner_active: false,
            step_remainder: 0.0,
        })
    }

    /// Restore the initial conditions (same configuration and seeds).
    pub fn reset(&mut self) -> Result<(), simulation_engine::SimulationError> {
        self.sim = Simulation::new(self.config.clone())?;
        self.state = RunState::Created;
        self.step_remainder = 0.0;
        Ok(())
    }

    /// Number of physics steps to execute in one runner tick.
    pub fn steps_for_tick(&mut self) -> u64 {
        if self.speed <= 0.0 {
            return MAX_STEPS_PER_TICK;
        }
        let exact =
            self.speed * TICK.as_secs_f64() / self.config.simulation.dt_s + self.step_remainder;
        let n = exact.floor();
        self.step_remainder = exact - n;
        (n as u64).min(MAX_STEPS_PER_TICK)
    }

    /// Advance one tick. Returns `true` while the runner should continue.
    pub fn tick(&mut self) -> bool {
        match self.state {
            RunState::Running => {
                let n = self.steps_for_tick();
                self.sim.advance(n);
                if self.sim.finished().is_some() {
                    self.state = RunState::Finished;
                    false
                } else {
                    true
                }
            }
            RunState::Paused => true,
            RunState::Created | RunState::Finished => false,
        }
    }

    pub fn finish_reason(&self) -> Option<FinishReason> {
        self.sim.finished()
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub type SessionRef = Arc<Mutex<Session>>;

#[derive(Default)]
pub struct SessionStore {
    sessions: RwLock<HashMap<Uuid, SessionRef>>,
}

impl SessionStore {
    pub fn get(&self, id: &Uuid) -> Option<SessionRef> {
        self.sessions.read().ok()?.get(id).cloned()
    }

    pub fn insert(&self, s: Session) -> Result<SessionRef, String> {
        let mut map = self
            .sessions
            .write()
            .map_err(|_| "session store poisoned".to_string())?;
        if map.len() >= MAX_SESSIONS {
            // Evict the oldest session that is not running.
            let victim = map
                .iter()
                .filter_map(|(id, s)| {
                    s.lock()
                        .ok()
                        .filter(|s| s.state != RunState::Running)
                        .map(|s| (*id, s.created_at_ms))
                })
                .min_by_key(|(_, t)| *t)
                .map(|(id, _)| id);
            match victim {
                Some(id) => {
                    map.remove(&id);
                }
                None => return Err(format!("all {MAX_SESSIONS} sessions are running")),
            }
        }
        let id = s.id;
        let r = Arc::new(Mutex::new(s));
        map.insert(id, r.clone());
        Ok(r)
    }

    pub fn remove(&self, id: &Uuid) -> bool {
        self.sessions
            .write()
            .map(|mut m| m.remove(id).is_some())
            .unwrap_or(false)
    }

    pub fn all(&self) -> Vec<SessionRef> {
        self.sessions
            .read()
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default()
    }

    pub fn len(&self) -> usize {
        self.sessions.read().map(|m| m.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Background task that drives a running session until it finishes, is paused
/// indefinitely and deleted, or is stopped.
pub async fn run_session(store: Arc<SessionStore>, id: Uuid) {
    let mut interval = tokio::time::interval(TICK);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        interval.tick().await;
        let Some(session) = store.get(&id) else { break };
        let session_for_task = session.clone();
        let keep_going = tokio::task::spawn_blocking(move || match session_for_task.lock() {
            Ok(mut s) => s.tick(),
            Err(_) => false,
        })
        .await
        .unwrap_or(false);
        if !keep_going {
            if let Ok(mut s) = session.lock() {
                s.runner_active = false;
                if s.sim.finished().is_some() {
                    s.state = RunState::Finished;
                }
            }
            tracing::info!(%id, "runner finished");
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_per_tick_preserve_fixed_dt() {
        let mut s = Session::new(SimulationConfig::default(), 1.0).unwrap();
        // 1x speed, 50 ms tick, dt 10 ms: 5 steps per tick.
        assert_eq!(s.steps_for_tick(), 5);
        s.speed = 0.3; // 1.5 steps per tick on average: remainder carried over
        let total: u64 = (0..10).map(|_| s.steps_for_tick()).sum();
        assert!(total.abs_diff(15) <= 1, "total {total}");
        s.speed = 0.0;
        assert_eq!(s.steps_for_tick(), MAX_STEPS_PER_TICK);
    }
}
