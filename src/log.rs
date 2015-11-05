//! Logging module

extern crate time;
extern crate libc;
use self::libc::{c_void};

use std::collections::vec_deque::VecDeque;
use std::fmt;
use std::io::{Write, Read, Seek, BufRead};
use std;

use utils;

const TIME_FORMAT: &'static str = "%x %X";

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
            FilterLog::Last(from) => {
                //@TODO: this is a hacky date comparing via seconds conversion.
                //       But for now live with it.
                //       The issue: you get bad Tm from file with strptime()
                //       NOTE: That strptime() sets year as 15, while Tm would have 115.
                let from_seconds: i64 = if from.tm_year > 100 { from.tm_year * 3153600 } else { from.tm_year * 31536000 } as i64 +
                                        (from.tm_mon * 2592000) as i64 +
                                        (from.tm_mday * 86400) as i64 +
                                        (from.tm_hour * 3600) as i64 +
                                        (from.tm_min * 60) as i64 +
                                        from.tm_sec as i64;
                let time_seconds: i64 = if time.tm_year > 100 { time.tm_year * 3153600 } else { time.tm_year * 31536000 } as i64 +
                                        (time.tm_mon * 2592000) as i64 +
                                        (time.tm_mday * 86400) as i64 +
                                        (time.tm_hour * 3600) as i64 +
                                        (time.tm_min * 60) as i64 +
                                        time.tm_sec as i64;

                from_seconds < time_seconds
            },
        }
    }
}

impl fmt::Display for FilterLog {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            FilterLog::None => write!(f, "None"),
            FilterLog::Last(from) => write!(f, "Last({})", from.strftime(TIME_FORMAT).unwrap()),
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
    fn buff_to_file(&mut self) {
        let len = self.inner.len();

        if len <= 20 {return;}

        self.fs_buf.seek(std::io::SeekFrom::End(0)).unwrap();
        //range is exclusive at the end
        for _ in 0..len-19 {
            self.fs_buf.write_fmt(format_args!("{}\n", self.inner.pop_front().unwrap())).unwrap();
        }
        self.fs_buf.flush().unwrap()
    }

    ///Dumps all logs except for last 20
    fn dump_to_file(&mut self) {
        let len = self.inner.len();

        if len == 0 {return;}

        self.fs_buf.seek(std::io::SeekFrom::End(0)).unwrap();
        //range is exclusive at the end
        for _ in 0..len {
            self.fs_buf.write_fmt(format_args!("{}\n", self.inner.pop_front().unwrap())).unwrap();
        }
        self.fs_buf.flush().unwrap()
    }

    #[inline(always)]
    /// Adds entry to log.
    pub fn add(&mut self, entry: IrcEntry) {
        if self.len() >= self.capacity() {
            self.buff_to_file();
        }
        self.inner.push_back(entry);
    }

    #[inline]
    /// Reads all/filtered entries from underlying file buffer.
    pub fn fs_read(&mut self, filter: &FilterLog) -> String {
        if self.fs_buf.metadata().unwrap().len() == 0 {
            return "".to_string();
        }

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
            const DATA_END: usize = 18;
            let line = line.unwrap();
            let time_stamp = time::strptime(&line[DATA_START..DATA_END], TIME_FORMAT).unwrap();

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
    /// Returns formatted string with log inner entries.
    pub fn read_to_string(&self, filter: &FilterLog) -> String {
        let log_size = self.len();

        if log_size == 0 {
            "".to_string()
        }
        else {
            self.iter()
                .filter(|elem| filter.check(&elem.time()))
                .fold(String::with_capacity(log_size*50), |acc, item| acc + &format!("{}\n", item))
        }
    }

    #[inline(always)]
    ///Returns content of file and heap buffers.
    pub fn get_all(&mut self, filter: &FilterLog) -> String {
        format!("{}{}", self.fs_read(filter), self.read_to_string(filter))
    }

    #[inline(always)]
    pub fn back(&self) -> Option<&IrcEntry> {
        self.inner.back()
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    ///Returns size of log in bytes.
    #[inline(always)]
    pub fn heap_size(&self) -> usize {
        self.iter().fold(0, |n, elem| n + elem.heap_size()) +
        std::mem::size_of::<VecDeque<IrcEntry>>() +
        std::mem::size_of::<std::fs::File>()
    }
}

impl fmt::Display for IrcLog {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let heap_size = self.heap_size() as f32;
        write!(f, "Log(len={}, size={:.3}kb)", self.len(), heap_size/1024.0)
    }
}

impl Drop for IrcLog {
    fn drop(&mut self) {
        self.dump_to_file();
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

    #[inline(always)]
    pub fn nickname(&self) -> &String {
        &self.nickname
    }

    #[inline(always)]
    pub fn message(&self) -> &String {
        &self.message
    }


    #[inline(always)]
    pub fn heap_size(&self) -> usize {
        //there are 11 fields of i32 in Tm.
        std::mem::size_of::<i32>() * 11 +
        utils::heap_size_of(self.nickname().as_ptr() as *const c_void) +
        utils::heap_size_of(self.message().as_ptr() as *const c_void)
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
        write!(f, "[{}] <{}> {}", self.time.strftime(TIME_FORMAT).unwrap(), self.nickname, self.message)
    }
}

#[cfg(test)]
mod tests {
    extern crate time;
    use super::*;
    use std;

    #[test]
    fn test_irc_entry() {
        let entry = IrcEntry::new("Kuu".to_string(), "nya nya!".to_string());

        assert!(entry.nickname == "Kuu");
        assert!(entry.message == "nya nya!");
    }

    #[test]
    fn test_filter_log() {
        let time_now    = time::now();
        let time_before = time_now - time::Duration::minutes(10);
        let time_before = time_before.to_local();
        let time_after  = time_now + time::Duration::minutes(10);
        let time_after  = time_after.to_local();
        let no_filter   = FilterLog::None;
        let filter_now  = FilterLog::Last(time_now);

        assert!(no_filter.check(&time_before));
        assert!(format!("{}", no_filter) == "None");

        assert!(!filter_now.check(&time_before));
        assert!(filter_now.check(&time_after));
        assert!(format!("{}", filter_now) == format!("Last({})", time_now.strftime(super::TIME_FORMAT).unwrap()));
    }

    macro_rules! is_file {
        ($path:expr) => { std::fs::metadata($path).ok().map_or(false, |data| data.is_file()); };
    }

    #[test]
    fn test_irc_log() {
        let filter = FilterLog::None;
        std::env::set_current_dir(std::env::current_exe().unwrap().parent().unwrap())
                  .unwrap_or_else(|err| panic!("cannot enter my own directory :(. Err={}", err));

        let mut log = IrcLog::new();
        let old_capacity = log.capacity();

        assert!(is_file!("vndis.log"));
        assert!(log.len() == 0);
        assert!(log.read_to_string(&filter).is_empty());
        assert!(log.fs_read(&filter).is_empty());

        let flood_entry = IrcEntry::new("Kuu".to_string(), "nya nya!".to_string());
        let rare_entry = IrcEntry::new("Kuu".to_string(), "...".to_string());

        log.add(flood_entry.clone());
        log.add(rare_entry.clone());

        let mut expect_str = format!("{}\n{}\n", flood_entry, rare_entry);
        assert!(log.len() == 2);
        assert!(log.back() == Some(&rare_entry));
        assert!(log.read_to_string(&filter) == expect_str);
        assert!(log.fs_read(&filter).is_empty());

        for i in 0..old_capacity+1 {
            let entry = IrcEntry::new(format!("Kuu{}", i), format!("i={}", i));
            expect_str = expect_str + &format!("{}\n", &entry);
            log.add(entry);
        }

        assert!(log.capacity() == old_capacity);
        assert!(log.back().unwrap().nickname == format!("Kuu{}", old_capacity));
        assert!(log.back().unwrap().message == format!("i={}", old_capacity));
        assert!(log.get_all(&filter) == expect_str);

        drop(log);
        std::fs::remove_file("vndis.log").unwrap();
    }
}
