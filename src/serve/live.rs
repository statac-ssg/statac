//! What the browser is told after a save, and the held request that tells it.
//!
//! A page carries the generation it was served at. It asks "anything after
//! N?" and the request is held until there is. A page that missed a round
//! (asleep, reloading, newly the tab that asks) so still gets the right
//! answer the next time it asks, and a page served after the last round is
//! never told to reload again.

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use super::panel::Panel;
use crate::json;
use crate::problem::Told;

/// How the site stands after a round, for the page to show.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct Status {
    /// Whether the last round stopped, leaving the pages of an earlier one.
    pub stopped: bool,
    /// One line for the reader.
    pub line: String,
    /// Every problem of the last round, drawn as `check` draws them.
    pub problems: String,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Kind {
    /// Nothing has happened since the page's generation.
    None,
    /// Rounds have run, and the page as loaded is still right.
    Said,
    Styles(Vec<(String, String)>),
    Reload,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Reply {
    pub generation: u64,
    pub kind: Kind,
    /// The site's files that changed since the asker's generation.
    pub changed: Vec<String>,
    /// Whether a round is building now.
    pub building: bool,
    pub status: Status,
}

/// How many rounds are remembered. A page further behind reloads.
const KEPT: usize = 64;

pub struct Hub {
    state: Mutex<State>,
    moved: Condvar,
}

struct State {
    generation: u64,
    /// The generation before the oldest round remembered.
    oldest: u64,
    history: VecDeque<(u64, Told, Vec<String>)>,
    status: Status,
    panel: Arc<Panel>,
    waiting: usize,
    building: bool,
}

impl Default for Hub {
    fn default() -> Self {
        Self::new()
    }
}

impl Hub {
    pub fn new() -> Self {
        // Generations start from the clock, so a page left open from an
        // earlier run of the server is always behind, and reloads.
        let start = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(1, |since| since.as_millis() as u64);
        Hub {
            state: Mutex::new(State {
                generation: start,
                oldest: start,
                history: VecDeque::new(),
                status: Status::default(),
                panel: Arc::default(),
                waiting: 0,
                building: false,
            }),
            moved: Condvar::new(),
        }
    }

    /// A thread that stopped while holding the state left it whole: every
    /// change to it is finished before anything else is called.
    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub fn generation(&self) -> u64 {
        self.state().generation
    }

    /// How many requests are being held open right now.
    pub fn waiting(&self) -> usize {
        self.state().waiting
    }

    /// How the site stands before any round has been told.
    pub fn set_status(&self, status: Status) {
        self.state().status = status;
    }

    /// What the panel on a page shows from now on. Set before the round
    /// it is of is published, so a page told of the round finds it.
    pub fn set_panel(&self, panel: Panel) {
        self.state().panel = Arc::new(panel);
    }

    pub fn panel(&self) -> Arc<Panel> {
        Arc::clone(&self.state().panel)
    }

    /// Whether a round is building, which answers every held request
    /// that asked to be told.
    pub fn set_building(&self, building: bool) {
        let mut state = self.state();
        if state.building != building {
            state.building = building;
            self.moved.notify_all();
        }
    }

    /// Records a round, answers every held request, and gives the round's
    /// generation.
    pub fn publish(&self, told: Told, changed: Vec<String>, status: Status) -> u64 {
        let mut state = self.state();
        state.generation += 1;
        let generation = state.generation;
        state.history.push_back((generation, told, changed));
        while state.history.len() > KEPT {
            if let Some((dropped, ..)) = state.history.pop_front() {
                state.oldest = dropped;
            }
        }
        state.status = status;
        state.building = false;
        self.moved.notify_all();
        generation
    }

    /// Answers at once when a round has run since `since`, and otherwise
    /// holds until one does or `longest` passes. Given whether the asker
    /// knows a round to be building, it answers too once that isn't so.
    pub fn wait(&self, since: u64, building: Option<bool>, longest: Duration) -> Reply {
        let deadline = Instant::now() + longest;
        let mut state = self.state();
        state.waiting += 1;
        while state.generation == since && building.is_none_or(|known| known == state.building) {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            state = self
                .moved
                .wait_timeout(state, left)
                .unwrap_or_else(PoisonError::into_inner)
                .0;
        }
        state.waiting -= 1;
        reply(&state, since)
    }
}

fn reply(state: &State, since: u64) -> Reply {
    let generation = state.generation;
    let answer = |kind, changed| Reply {
        generation,
        kind,
        changed,
        building: state.building,
        status: state.status.clone(),
    };
    if since == generation {
        return answer(Kind::None, Vec::new());
    }
    // Ahead of the server, or behind everything remembered: only a reload
    // is sure to be right.
    if since > generation || since < state.oldest {
        return answer(Kind::Reload, Vec::new());
    }
    let mut changed = Vec::new();
    let mut styles: Vec<(String, String)> = Vec::new();
    let mut reload = false;
    for (_, told, files) in state.history.iter().filter(|(g, ..)| *g > since) {
        changed.extend(files.iter().cloned());
        match told {
            Told::Nothing => {}
            Told::Reload => reload = true,
            Told::Styles(moved) => {
                for (old, new) in moved {
                    // A stylesheet that moved twice is found on the page
                    // at its first address.
                    match styles.iter_mut().find(|(_, now)| now == old) {
                        Some(earlier) => earlier.1 = new.clone(),
                        None => styles.push((old.clone(), new.clone())),
                    }
                }
            }
        }
    }
    changed.sort();
    changed.dedup();
    let kind = match (reload, styles.is_empty()) {
        (true, _) => Kind::Reload,
        (false, false) => Kind::Styles(styles),
        (false, true) => Kind::Said,
    };
    answer(kind, changed)
}

impl Reply {
    pub fn to_json(&self) -> String {
        let (kind, styles) = match &self.kind {
            Kind::None => ("none", &[][..]),
            Kind::Said => ("said", &[][..]),
            Kind::Reload => ("reload", &[][..]),
            Kind::Styles(styles) => ("styles", &styles[..]),
        };
        let styles: Vec<String> = styles
            .iter()
            .map(|(old, new)| format!("[{},{}]", json::string(old), json::string(new)))
            .collect();
        let changed: Vec<String> = self.changed.iter().map(|path| json::string(path)).collect();
        format!(
            "{{\"generation\":{},\"kind\":\"{kind}\",\"styles\":[{}],\"changed\":[{}],\
             \"building\":{},\"status\":{{\"stopped\":{},\"line\":{},\"problems\":{}}}}}",
            self.generation,
            styles.join(","),
            changed.join(","),
            self.building,
            self.status.stopped,
            json::string(&self.status.line),
            json::string(&self.status.problems),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn moved(old: &str, new: &str) -> Told {
        Told::Styles(vec![(old.into(), new.into())])
    }

    fn asked(hub: &Hub, since: u64) -> Reply {
        hub.wait(since, None, Duration::ZERO)
    }

    #[test]
    fn a_page_from_the_current_generation_is_told_nothing() {
        let hub = Hub::new();
        assert_eq!(asked(&hub, hub.generation()).kind, Kind::None);
    }

    #[test]
    fn styles_alone_swap_and_anything_else_reloads() {
        let hub = Hub::new();
        let start = hub.generation();
        hub.publish(
            moved("/site.1.css", "/site.2.css"),
            vec!["assets/site.css".into()],
            Status::default(),
        );
        assert_eq!(
            asked(&hub, start).kind,
            Kind::Styles(vec![("/site.1.css".into(), "/site.2.css".into())])
        );
        let swapped = hub.generation();
        hub.publish(
            Told::Reload,
            vec!["content/post-1.md".into()],
            Status::default(),
        );
        let both = asked(&hub, start);
        assert_eq!(both.kind, Kind::Reload);
        assert_eq!(both.changed, ["assets/site.css", "content/post-1.md"]);
        assert_eq!(asked(&hub, swapped).changed, ["content/post-1.md"]);
    }

    #[test]
    fn a_stylesheet_that_moved_twice_is_known_by_its_first_address() {
        let hub = Hub::new();
        let start = hub.generation();
        hub.publish(moved("/a.1.css", "/a.2.css"), vec![], Status::default());
        hub.publish(moved("/b.1.css", "/b.2.css"), vec![], Status::default());
        hub.publish(moved("/a.2.css", "/a.3.css"), vec![], Status::default());
        assert_eq!(
            asked(&hub, start).kind,
            Kind::Styles(vec![
                ("/a.1.css".into(), "/a.3.css".into()),
                ("/b.1.css".into(), "/b.2.css".into())
            ])
        );
    }

    #[test]
    fn a_round_that_changes_no_built_file_only_says_how_the_site_stands() {
        let hub = Hub::new();
        let start = hub.generation();
        let status = Status {
            stopped: true,
            line: "1 error stopped the build.".into(),
            problems: "error  Unclear date".into(),
        };
        hub.publish(Told::Nothing, vec![], status.clone());
        let reply = asked(&hub, start);
        assert_eq!(reply.kind, Kind::Said);
        assert_eq!(reply.status, status);
        // A page loaded afterwards is told the same, with nothing to do.
        let later = asked(&hub, hub.generation());
        assert_eq!((later.kind, later.status), (Kind::None, status));
    }

    #[test]
    fn a_page_from_an_earlier_run_reloads() {
        let hub = Hub::new();
        assert_eq!(asked(&hub, 1).kind, Kind::Reload);
        assert_eq!(asked(&hub, u64::MAX).kind, Kind::Reload);
    }

    #[test]
    fn a_page_behind_everything_remembered_reloads() {
        let hub = Hub::new();
        let start = hub.generation();
        for _ in 0..KEPT {
            hub.publish(Told::Nothing, vec![], Status::default());
        }
        assert_eq!(asked(&hub, start).kind, Kind::Said);
        hub.publish(Told::Nothing, vec![], Status::default());
        assert_eq!(asked(&hub, start).kind, Kind::Reload);
        assert_eq!(asked(&hub, start + 1).kind, Kind::Said);
    }

    #[test]
    fn a_held_ask_is_answered_when_a_round_lands() {
        let hub = std::sync::Arc::new(Hub::new());
        let since = hub.generation();
        let asker = {
            let hub = hub.clone();
            std::thread::spawn(move || hub.wait(since, None, Duration::from_secs(10)))
        };
        while hub.waiting() == 0 {
            std::thread::yield_now();
        }
        // One that didn't ask to be told of building isn't.
        hub.set_building(true);
        hub.publish(Told::Reload, vec![], Status::default());
        let reply = asker.join().unwrap();
        assert_eq!((reply.kind, reply.building), (Kind::Reload, false));
    }

    #[test]
    fn a_held_ask_is_answered_when_a_round_starts_building_and_ends() {
        let hub = std::sync::Arc::new(Hub::new());
        let since = hub.generation();
        let ask = |building| {
            let hub = hub.clone();
            std::thread::spawn(move || hub.wait(since, Some(building), Duration::from_secs(10)))
        };
        let asker = ask(false);
        while hub.waiting() == 0 {
            std::thread::yield_now();
        }
        hub.set_building(true);
        let reply = asker.join().unwrap();
        assert_eq!((reply.kind, reply.building), (Kind::None, true));
        let asker = ask(true);
        while hub.waiting() == 0 {
            std::thread::yield_now();
        }
        hub.set_building(false);
        let reply = asker.join().unwrap();
        assert_eq!((reply.kind, reply.building), (Kind::None, false));
    }

    #[test]
    fn the_answer_is_json_whatever_the_text_holds() {
        let reply = Reply {
            generation: 7,
            kind: Kind::Styles(vec![("/a \"1\".css".into(), "/a.css".into())]),
            changed: vec!["content/a\\b.md".into()],
            building: true,
            status: Status {
                stopped: false,
                line: "one\ntwo".into(),
                problems: "</script>".into(),
            },
        };
        assert_eq!(
            reply.to_json(),
            "{\"generation\":7,\"kind\":\"styles\",\
             \"styles\":[[\"/a \\\"1\\\".css\",\"/a.css\"]],\
             \"changed\":[\"content/a\\\\b.md\"],\"building\":true,\
             \"status\":{\"stopped\":false,\"line\":\"one\\ntwo\",\
             \"problems\":\"\\u003c/script\\u003e\"}}"
        );
    }
}
