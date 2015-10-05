//! Logging module

extern crate time;
use std::collections::vec_deque::VecDeque;
use std::fmt;
use std::io::{Write, Read, Seek, BufRead};
use std;

pub enum FilterLog {
    None,
    Last(time::Tm)
}

impl FilterLog {
    ///Checks if given element is within allowed time range
    #[inline]
    pub fn check(&self, time: &time::Tm) -> bool {
        match *self {
            FilterLog::None => true,
            FilterLog::Last(from) => from > *time,
        }
    }
}

impl fmt::Display for FilterLog {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            FilterLog::None => write!(f, "None"),
            FilterLog::Last(from) => write!(f, "Last({})", from.strftime("%x %X.%f").unwrap()),
        }
    }
}

pub struct IrcLog {
    inner: VecDeque<IrcEntry>,
    fs_buf: std::fs::File
}

impl IrcLog {
    /// Creates log with default capacity 500.
    #[inline(always)]
    pub fn new() -> IrcLog {
        IrcLog {
            inner: VecDeque::with_capacity(500),
            //Open file log once for both write/read.
            //To read/write correctly be sure to .seek() at needed position
            fs_buf: std::fs::OpenOptions::new().read(true)
                                               .write(true)
                                               .create(true)
                                               .open("vndis.log")
                                               .unwrap()
        }
    }

    ///Dumps all logs except for last 20
    pub fn dump_to_fs(&mut self) {
        let len = self.inner.len();

        if len <= 20 {return;}

        self.fs_buf.seek(std::io::SeekFrom::End(0)).unwrap();
        //range is exclusive at the end
        for _ in 0..len-19 {
            self.fs_buf.write_fmt(format_args!("{}\n", self.inner.pop_front().unwrap())).unwrap();
        }
        self.fs_buf.flush().unwrap()
    }

    #[inline(always)]
    /// Adds entry to log.
    pub fn add(&mut self, entry: IrcEntry) {
        if self.inner.len() >= self.inner.capacity() {
            self.dump_to_fs();
        }
        self.inner.push_back(entry);
    }

    #[inline]
    /// Reads all/filtered entries from underlying file buffer.
    pub fn fs_read(&mut self, filter: &FilterLog) -> String {
        self.fs_buf.seek(std::io::SeekFrom::Start(0)).unwrap();

        let reader = std::io::BufReader::new(&mut self.fs_buf);
        let lines = reader.lines();

        //Let's try to peek the number of lines for effective allocation
        let acc_str = if let (_, Some(len)) = lines.size_hint() {
            String::with_capacity(len*50)
        }
        else {
            String::new()
        };

        lines.fold(acc_str, |acc, line| {
            const DATA_START: usize = 1;
            const DATA_END: usize = 28;
            let line = line.unwrap();
            let time_stamp = time::strptime(&line[DATA_START..DATA_END], "%x %X.%f").unwrap();

            if filter.check(&time_stamp) {
                acc + &format!("{}\n", line)
            }
            else {
                acc
            }
        })
    }

    #[inline(always)]
    /// Returns iterator over underlying buffer.
    pub fn iter(&self) -> std::collections::vec_deque::Iter<IrcEntry> {
        self.inner.iter()
    }

    #[inline(always)]
    pub fn back(&self) -> Option<&IrcEntry> {
        self.inner.back()
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

#[derive(Clone, Debug)]
pub struct IrcEntry {
    time: time::Tm,
    nickname: String,
    message: String
}

impl IrcEntry {
    #[inline(always)]
    /// Creates new log entry from message and nick
    pub fn new(nick: String, msg: String) -> IrcEntry {
        IrcEntry {
            time: time::now(),
            nickname: nick,
            message: msg,
        }
    }

    #[inline(always)]
    pub fn time(&self) -> time::Tm {
        self.time
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
