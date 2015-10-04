//! Logging module

extern crate time;
use std::collections::vec_deque::VecDeque;
use std::fmt;
use std;

pub struct IrcLog(VecDeque<IrcEntry>);

impl IrcLog {
    /// Creates log with default capacity 500.
    pub fn new() -> IrcLog {
        IrcLog(VecDeque::with_capacity(500))
    }

    /// Adds entry to log.
    pub fn add(&mut self, entry: IrcEntry) {
        if self.0.len() >= self.0.capacity() {
            self.0.pop_front();
        }
        self.0.push_back(entry);
    }

    /// Returns iterator over underlying buffer.
    pub fn iter(&self) -> std::collections::vec_deque::Iter<IrcEntry> {
        self.0.iter()
    }

}

#[derive(Clone, Debug)]
pub struct IrcEntry {
    time: time::Tm,
    nickname: String,
    message: String
}

impl IrcEntry {
    /// Creates new log entry from message and nick
    pub fn new(nick: String, msg: String) -> IrcEntry {
        IrcEntry {
            time: time::now(),
            nickname: nick,
            message: msg,
        }
    }
}

impl PartialEq for IrcEntry {
    fn eq(&self, right: &IrcEntry) -> bool {
        self.time == right.time
    }

    fn ne(&self, right: &IrcEntry) -> bool {
        self.time != right.time
    }
}

impl fmt::Display for IrcEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "[{}] <{}> {}", self.time.strftime("%x %X.%f").unwrap(), self.nickname, self.message)
    }
}
