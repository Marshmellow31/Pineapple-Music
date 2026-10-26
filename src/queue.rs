use crate::model::Track;
use rand::seq::SliceRandom;

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Repeat {
    #[default]
    Off,
    All,
    One,
}

impl Repeat {
    pub fn cycle(self) -> Self {
        match self {
            Self::Off => Self::All,
            Self::All => Self::One,
            Self::One => Self::Off,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Repeat off",
            Self::All => "Repeat all",
            Self::One => "Repeat one",
        }
    }
}

#[derive(Default)]
pub struct Queue {
    pub items: Vec<Track>,
    pub current: Option<usize>,
    pub repeat: Repeat,
    pub shuffle: bool,
    pub generation: u64,
    history: Vec<usize>,
    remaining: Vec<usize>,
}

impl Queue {
    pub fn active(&self) -> Option<&Track> {
        self.current.and_then(|i| self.items.get(i))
    }

    pub fn replace(&mut self, tracks: Vec<Track>, index: usize) -> Option<Track> {
        self.items = tracks;
        self.history.clear();
        self.current = None;
        self.reshuffle();
        self.select(index)
    }

    pub fn select(&mut self, index: usize) -> Option<Track> {
        let track = self.items.get(index)?.clone();
        if let Some(old) = self.current {
            self.history.push(old);
            if self.history.len() > 1000 {
                self.history.remove(0);
            }
        }
        self.current = Some(index);
        self.remaining.retain(|i| *i != index);
        self.generation = self.generation.wrapping_add(1);
        Some(track)
    }

    pub fn set_shuffle(&mut self, enabled: bool) {
        self.shuffle = enabled;
        self.reshuffle();
    }

    fn reshuffle(&mut self) {
        self.remaining = (0..self.items.len())
            .filter(|i| Some(*i) != self.current)
            .collect();
        self.remaining.shuffle(&mut rand::thread_rng());
    }

    pub fn enqueue(&mut self, track: Track) {
        self.remaining.push(self.items.len());
        self.items.push(track);
    }

    pub fn advance(&mut self, generation: u64, manual: bool) -> Option<Track> {
        if generation != self.generation {
            return None;
        }
        let current = self.current?;
        let next = if !manual && self.repeat == Repeat::One {
            current
        } else if self.shuffle {
            if self.remaining.is_empty() && self.repeat == Repeat::All {
                self.reshuffle();
            }
            if self.items.len() == 1 && self.repeat == Repeat::All {
                current
            } else {
                self.remaining.pop()?
            }
        } else if current + 1 < self.items.len() {
            current + 1
        } else if self.repeat == Repeat::All {
            0
        } else {
            return None;
        };
        self.select(next)
    }

    pub fn previous(&mut self) -> Option<Track> {
        let index = self
            .history
            .pop()
            .or_else(|| self.current.and_then(|i| i.checked_sub(1)))?;
        let track = self.items.get(index)?.clone();
        self.current = Some(index);
        self.generation = self.generation.wrapping_add(1);
        Some(track)
    }

    pub fn remove(&mut self, index: usize) -> bool {
        if Some(index) == self.current || index >= self.items.len() {
            return false;
        }
        self.items.remove(index);
        if let Some(current) = &mut self.current
            && *current > index
        {
            *current -= 1;
        }
        self.history = self
            .history
            .iter()
            .filter_map(|&i| {
                if i == index {
                    None
                } else {
                    Some(if i > index { i - 1 } else { i })
                }
            })
            .collect();
        self.remaining = self
            .remaining
            .iter()
            .filter_map(|&i| {
                if i == index {
                    None
                } else {
                    Some(if i > index { i - 1 } else { i })
                }
            })
            .collect();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::MediaSource;
    fn tracks() -> Vec<Track> {
        (0..4)
            .map(|i| Track {
                id: i.to_string(),
                title: i.to_string(),
                artist: "a".into(),
                album: "b".into(),
                source: MediaSource::LocalFile(format!("{i}.wav").into()),
                artwork: None,
                duration: None,
                album_id: None,
            })
            .collect()
    }
    #[test]
    fn stale_completion_cannot_advance_replaced_queue() {
        let mut q = Queue::default();
        q.replace(tracks(), 0);
        let old = q.generation;
        q.replace(tracks(), 2);
        assert!(q.advance(old, false).is_none());
        assert_eq!(q.current, Some(2));
    }
    #[test]
    fn repeat_one_manual_skip_and_previous() {
        let mut q = Queue::default();
        q.replace(tracks(), 1);
        q.repeat = Repeat::One;
        assert_eq!(q.advance(q.generation, false).unwrap().id, "1");
        assert_eq!(q.advance(q.generation, true).unwrap().id, "2");
        assert_eq!(q.previous().unwrap().id, "1");
    }
    #[test]
    fn shuffle_visits_each_remaining_track_once() {
        let mut q = Queue::default();
        q.replace(tracks(), 0);
        q.set_shuffle(true);
        let mut ids = vec!["0".to_owned()];
        while let Some(t) = q.advance(q.generation, false) {
            ids.push(t.id);
        }
        ids.sort();
        assert_eq!(ids, vec!["0", "1", "2", "3"]);
    }
    #[test]
    fn shuffle_repeat_all_repeats_single_item_and_off_stops() {
        let mut q = Queue::default();
        q.replace(vec![tracks()[0].clone()], 0);
        q.set_shuffle(true);
        q.repeat = Repeat::All;
        assert_eq!(q.advance(q.generation, false).unwrap().id, "0");
        q.repeat = Repeat::Off;
        assert!(q.advance(q.generation, false).is_none());
    }
    #[test]
    fn removal_preserves_current_item_and_shuffle_indices() {
        let mut q = Queue::default();
        q.replace(tracks(), 2);
        q.set_shuffle(true);
        assert!(!q.remove(2));
        assert!(q.remove(0));
        assert_eq!(q.active().unwrap().id, "2");
        while q.advance(q.generation, false).is_some() {}
    }
}
