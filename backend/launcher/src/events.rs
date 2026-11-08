//! Messages from background work to the window.

use std::sync::mpsc::Sender;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub enum Event {
    Log(String),
    Phase(String),
    Fraction(f32),
    Done(Result<(), String>),
    GameLog(String),
    GameExited(i32),
}

/// A `Sender` behind a mutex, which is what makes the callbacks `Sync`: download workers call
/// them from several threads.
pub struct Emitter {
    tx: Mutex<Sender<Event>>,
}

impl Emitter {
    pub fn new(tx: Sender<Event>) -> Emitter {
        Emitter { tx: Mutex::new(tx) }
    }

    pub fn send(&self, event: Event) {
        let _ = self.tx.lock().unwrap().send(event);
    }

    pub fn log(&self, text: impl Into<String>) {
        self.send(Event::Log(text.into()));
    }

    pub fn phase(&self, text: impl Into<String>) {
        self.send(Event::Phase(text.into()));
    }

    pub fn fraction(&self, value: f32) {
        self.send(Event::Fraction(value));
    }
}
