#![cfg_attr(test, allow(dead_code))]
extern crate irc;
extern crate hyper;
extern crate url;
extern crate crossbeam;
extern crate time;

use irc::client::prelude::*;
use irc::client::conn::NetStream;
use std::io::{Write, Read, BufWriter, BufReader};
mod utils;
mod log;

const GITHUB_AUTH: &'static str = include_str!("github_token.txt");
const VNDIS: &'static str  = "#vndis";
const MASTER: &'static str = "Douman";
const USAGE: &'static str  = "Available commands:\n
ping      - to get pong in response. Available via !\n
grep <vn> - to get search link on vndb\n
google    - to get search link on google\n
log <cmd> - access to log facilities. See log help for more information. Available via !\n
help      - to get this message";

///Represents bot responses
#[derive(Debug)]
enum BotResponse {
    None,
    Private(String),
    Channel(String)
}

struct KuuBot {
    server: IrcServer<BufReader<NetStream>, BufWriter<NetStream>>,
    nick: String,
    joined: bool,
}

impl std::fmt::Display for KuuBot {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "KuuBot(nick={}, joined={})", self.nick, self.joined)
    }
}

impl KuuBot {
    #[inline(always)]
    ///Creates default bot.
    fn new() -> KuuBot {
        KuuBot {
            server: IrcServer::new("config.json").unwrap(),
            nick: "".to_string(),
            joined: false,
        }
    }

    ///Reconnects bot with delay.
    fn reconnect(&mut self, delay_ms: u32) {
        self.joined = false;
        std::thread::sleep_ms(delay_ms);
        self.server.reconnect().unwrap();
        self.server.identify().unwrap();
    }

    ///Handler to direct msgs i.e. to bot.
    fn direct_response(&self, nickname: &String, usr_msg: &String, log: &mut log::IrcLog) -> BotResponse {
        let usr_msg = usr_msg.to_lowercase();
        let parts: Vec<&str> = usr_msg.split_whitespace().collect();
        match parts[1] {
            "ping" | "пинг"       => BotResponse::Channel("pong".to_string()),
            "grep" | "find"       => KuuBot::command_grep(&parts),
            "google"              => KuuBot::command_google(&parts),
            "log"                 => self.command_log(nickname, &parts[2..], log),
            "about" | "status"    => self.command_about(nickname, &log),
            "help"                => KuuBot::command_help(),
            "rape"                => KuuBot::command_rape(&parts[2..]),
            "huiping" | "хуйпинг" => BotResponse::Channel("死になさいゴミムシ".to_string()),
            _                     => BotResponse::Channel("...".to_string()),
        }
    }

    #[inline]
    ///Handler to all messages in general.
    fn indirect_response(&self, nickname: &String, usr_msg: &String, log: &mut log::IrcLog) -> BotResponse {
        let usr_msg = usr_msg.to_lowercase();
        match &usr_msg[..] {
            "!ping" | "!пинг"                 => BotResponse::Channel("pong".to_string()),
            "!huiping" | "!хуйпинг"           => BotResponse::Channel("死になさいゴミムシ".to_string()),
            _ if usr_msg.starts_with("!log")  => self.command_log(nickname, &usr_msg.split_whitespace().skip(1).collect::<Vec<&str>>(), log),
            _ if usr_msg.contains("tadaima") ||
                 usr_msg.contains("тадайма") ||
                 usr_msg.contains("ただいま") => BotResponse::Channel("okaeri".to_string()),
            _                                 => BotResponse::None,
        }
    }

    #[inline(always)]
    ///Sends private message.
    fn send_msg(&self, to: &str, message: &str) {
        self.server.send_privmsg(to, message).unwrap();
    }

    #[inline(always)]
    ///Sends bot's response
    fn send_response(&self, response: BotResponse, nickname: &String) {
        match response {
            BotResponse::Channel(text) => { self.send_msg(VNDIS, &format!("{}: {}", nickname, &text)); },
            //for private response we allow to send several.
            BotResponse::Private(text) => { for line in text.lines() { self.send_msg(&nickname, line); } },
            BotResponse::None => (),
        }
    }

    #[inline(always)]
    ///Returns bot's response
    fn get_response(&self, nickname: &String, usr_msg: &String, log: &mut log::IrcLog) -> BotResponse {
        if usr_msg.starts_with(&self.nick) {
            self.direct_response(&nickname, &usr_msg, log)
        }
        else {
            self.indirect_response(&nickname, &usr_msg, log)
        }
    }

    #[inline]
    ///Handler to all VNDIS messages.
    fn vndis_msg(&self, message: irc::client::data::message::Message, log: &mut log::IrcLog) {
        if let (Some(nickname), Some(usr_msg)) = (utils::get_nick(&message.prefix), message.suffix) {
            let response = self.get_response(&nickname, &usr_msg, log);

            self.send_response(response, &nickname);

            log.add(log::IrcEntry::new(nickname, usr_msg));
            println!("{}", log.back().unwrap())
        }
        else {
            println!(">>>ERROR: bad message over vndis")
        }
    }

    #[inline]
    ///Handler to private queries.
    fn private_query(&self, message: irc::client::data::message::Message, log: &log::IrcLog) {
        if let Some(nickname) = utils::get_nick(&message.prefix) {
            if nickname.starts_with(MASTER) {
                let usr_msg = message.suffix.unwrap().to_lowercase();
                let mut parts = usr_msg.split_whitespace();

                let response = match parts.next() {
                    Some("status") | Some("about") => self.command_about(&nickname, log),
                    None                           => BotResponse::Private("Umm...? What? You said nothing. Master, is everything ok?".to_string()),
                    _                              => BotResponse::Private("Did you mispell command? Try again :)".to_string()),
                };

                self.send_response(response, &nickname);
            }
            else if !nickname.starts_with("py-ctcp") {
                self.send_msg(&nickname, "Please do not bother me");
                self.send_msg(VNDIS, &format!("{}: master, some weird {} is trying to abuse me :(", MASTER, &nickname));
            }
        }
        else {
            println!(">>>ERROR: Got private query from who knows whom :(");
        }
    }

    #[inline]
    ///Message dispatcher.
    fn handle_msg(&self, message: irc::client::data::message::Message, log: &mut log::IrcLog) {
        if !self.joined { return; }

        match &*message.args[0] {
            VNDIS => self.vndis_msg(message, log),
            //Most possibly private query.
            _ if message.args[0].starts_with(&self.nick) => self.private_query(message, log),
            _ => println!(">>>ERROR: Got unexpected message={:?}", message),
        }
    }

    #[inline(always)]
    ///Welcome joined persons on VNDIS.
    fn welcome_vndis(&self, nickname: &String) -> BotResponse {
        match nickname {
            _ if nickname.starts_with(MASTER) => BotResponse::Channel("Welcome, dear master!".to_string()),
            _ => BotResponse::None,
        }
    }

    #[inline(always)]
    ///Welcome joined persons.
    fn welcome(&self, message: irc::client::data::message::Message) {
        if let (Some(nickname), Some(usr_msg)) = (utils::get_nick(&message.prefix), message.suffix) {
            let response = match &usr_msg[..] {
                VNDIS => self.welcome_vndis(&nickname),
                _     => BotResponse::None,
            };

            self.send_response(response, &nickname);
        }
    }

    ///Starts bot which continuously handles messages.
    fn run(&mut self) {
        let mut log = log::IrcLog::new();
        self.server.identify().unwrap();
        loop {
            for message in self.server.iter() {
                match message {
                    Ok(message) => {
                        match &message.command[..] {
                            "PRIVMSG" => self.handle_msg(message, &mut log),
                            "JOIN"    => if !self.joined {
                                self.joined = message.suffix.unwrap_or("".to_string()) == VNDIS;
                                if self.joined {
                                    self.nick = utils::get_nick(&message.prefix).unwrap_or_else(|| panic!("Unable to confirm own nick!?"));
                                    println!(">>>Joined {}", VNDIS);
                                }
                            }
                            else {
                                self.welcome(message);
                            },
                            "KICK"   => if message.suffix.unwrap_or("".to_string()) == VNDIS {
                                println!(">>>KICKED OUT OF {}", &*message.args[0]);
                                self.joined = false;
                                match &*message.args[0] {
                                    VNDIS => self.server.send_join(VNDIS).unwrap(),
                                    _     => (),
                                }
                            },
                            _        => (),
                        }
                    },
                    Err(err) => println!(">>>ERROR: {}", err),
                }
            }
            println!(">>>ERROR: Connection loss");
            self.reconnect(10);
            std::io::stdout().flush().unwrap();
        }
    }

    ///Upload log dump to pastebin.
    fn upload(&self, log: &mut log::IrcLog, nickname: &String, filter: &log::FilterLog) {
        crossbeam::scope(|scope| {
            let paste = log.get_all(filter);

            scope.spawn(|| {
                if paste.is_empty() {
                    self.send_msg(VNDIS, &format!("{}: I'm sorry there are no logs for your request :(", nickname));
                    return;
                }

                let log_size = paste.matches("\\n").count();
                let mut headers = hyper::header::Headers::new();
                headers.set(hyper::header::Authorization(hyper::header::Basic{username: "DoumanAsh".to_owned(),
                                                                              password: Some(GITHUB_AUTH.trim().to_owned()) }));
                headers.set(hyper::header::UserAgent("vndis_rusty_bot/1.0".to_owned()));
                let client = hyper::Client::new();

                let body = format!("{{\"description\": \"#vndis logs\", \"files\": {{ \"vndis_log\": {{ \"content\": \"{}\"}} }} }}", &paste);

                let mut res = client.request(hyper::method::Method::Patch, "https://api.github.com/gists/9f58fe727c0cea299c46")
                                    .headers(headers)
                                    .body(&body)
                                    .send()
                                    .unwrap();

                let mut link = String::new();
                res.read_to_string(&mut link).unwrap();
                if let Some(pos) = link.find("raw_url") {
                    let pos = pos + 10;
                    let link = &link[pos..];
                    let end = link.find("\"").unwrap();
                    self.send_msg(VNDIS, &format!("{}: log dump: {} | len={} | Filter={}", nickname, &link[..end], log_size, filter));
                }
                else {
                    self.send_msg(VNDIS, &format!("{}: i failed to upload logs :( Check up reason with master.", nickname));
                    println!(">>>ERROR: bad github gist result:{}", &link);
                }
            });
        });
    }

    #[inline]
    ///Parse num for log command. Allowed range [-20:20].
    fn log_parse_num(num_str: &str) -> Result<isize, BotResponse> {
        let parse_res = num_str.parse::<isize>();
        if parse_res.is_err(){
            return Err(BotResponse::Channel(format!(">{}< is not normal integer... bully", num_str)));
        }

        let num: isize = parse_res.unwrap();
        if num == 0 {
            return Err(BotResponse::Channel(format!("umm... {}? Are you stupid?", num)));
        }
        else if num > 20 || num < -20 {
            return Err(BotResponse::Channel(format!(">{}< is too much... I do not wanna flood you.", num)));
        }
        Ok(num)
    }

    #[inline(always)]
    ///Handler for command help.
    fn command_help() -> BotResponse {
        BotResponse::Private(USAGE.to_string())
    }

    #[inline]
    ///Handler for command about.
    ///
    ///Response only to master.
    fn command_about(&self, nickname: &String, log: &log::IrcLog) -> BotResponse {
        if nickname.starts_with(MASTER) {
            BotResponse::Private(format!("{} {}", &self, log))
        }
        else {
            BotResponse::Channel("This is only for my master!".to_string())
        }
    }

    #[inline]
    ///Handler for command google.
    fn command_google(parts: &[&str]) -> BotResponse {
        if parts.len() < 3 {
            return BotResponse::Channel("... bully...".to_string());
        }

        BotResponse::Channel(format!("http://lmgtfy.com/?q={}", parts[2..].join("+")))
    }

    #[inline]
    ///Handler for command grep/find.
    fn command_grep(parts: &[&str]) -> BotResponse {
        if parts.len() < 4 {
            return BotResponse::Channel("... bully...".to_string());
        }

        match parts[2] {
            "vn" => BotResponse::Channel(format!("vndb: https://vndb.org/v/all?q={};fil=tagspoil-0;o=d;s=rel", parts[3..].join("+"))),
            _ => BotResponse::Channel("... bully...".to_string()),
        }
    }

    #[inline(always)]
    fn command_rape(parts: &[&str]) -> BotResponse {
        match parts.iter().next() {
            Some(&MASTER) => BotResponse::Channel("umm... no... :(".to_string()),
            None | _      => BotResponse::Channel("へんたい！".to_string()),
        }
    }

    ///Parses string "num<d/m/h" to time.
    fn parse_filter_time(filter_str: &str) -> Result<time::Tm, BotResponse> {
        const TYPES: &'static[char] = &['m', 'h', 'd'];
        let mut filter_chars = filter_str.chars();
        let filter_type = filter_chars.next_back().unwrap();
        let filter_val = filter_chars.collect::<String>().parse::<i64>();

        if filter_val.is_err() || !TYPES.contains(&filter_type) {
            return Err(BotResponse::Channel(format!(">{}< is not normal filter, dummy. It should be num<m/h/d>", filter_str)));
        }

        //@TODO: handle 0 value as bad one?
        let filter_val = filter_val.unwrap();
        if filter_val < 0 {
            return Err(BotResponse::Channel("filter cannot be negative... dummy.".to_string()));
        }

        let time_before = time::now() - match filter_type {
            'm' => time::Duration::minutes(filter_val),
            'h' => time::Duration::hours(filter_val),
            _ => time::Duration::days(filter_val),
        };
        //See implementation of Sub<Duration>.
        //It seems that result will be in UTC isntead of original timezone.
        //For now just manually convert it.
        Ok(time_before.to_local())
    }

    ///Handler for command log.
    fn command_log(&self, nickname: &String, parts: &[&str], log: &mut log::IrcLog) -> BotResponse {
        let mut parts = parts.iter();
        match parts.next() {
            Some(&"last") => {
                let num: isize;
                if let Some(val) = parts.next() {
                    match KuuBot::log_parse_num(val) {
                        Ok(parse_result) => { num = parse_result },
                        Err(parse_err) => return parse_err,
                    }
                }
                else {
                    num = 20;
                }

                if num > 0 {
                    let num = num as usize;
                    let first = format!("Last {} messages\n", num);
                    BotResponse::Private(log.iter().rev().take(num).collect::<Vec<_>>().into_iter().rev().fold(first, |acc, item| acc + &format!("{}\n", item)))
                }
                else {
                    let num = num.abs() as usize;
                    let first = format!("First {} messages\n", num);
                    BotResponse::Private(log.iter().take(num).fold(first, |acc, item| acc + &format!("{}\n", item)))
                }

            },
            Some(&"first") => {
                let num: isize;
                if let Some(val) = parts.next() {
                    match KuuBot::log_parse_num(val) {
                        Ok(parse_result) => { num = parse_result },
                        Err(parse_err) => return parse_err,
                    }
                }
                else {
                    num = 20;
                }

                if num < 0 {
                    let num = num.abs() as usize;
                    let first = format!("Last {} messages\n", num);
                    BotResponse::Private(log.iter().rev().take(num).collect::<Vec<_>>().into_iter().rev().fold(first, |acc, item| acc + &format!("{}\n", item)))
                }
                else {
                    let num = num as usize;
                    let first = format!("First {} messages\n", num);
                    BotResponse::Private(log.iter().take(num).fold(first, |acc, item| acc + &format!("{}\n", item)))
                }

            },
            Some(&"dump") => {
                let filter: log::FilterLog;
                match parts.next() {
                    Some(&"last") => {
                        if let Some(filter_str) = parts.next() {
                            match KuuBot::parse_filter_time(filter_str) {
                                Ok(time)      => filter = log::FilterLog::Last(time),
                                Err(response) => return response,
                            }
                        }
                        else {
                            return BotResponse::Channel("you forgot to tell me filter value :(".to_string());
                        }
                    },
                    None => filter = log::FilterLog::None,
                    filter @ _ => {
                        return BotResponse::Channel(format!("there is no such filter >{}<, dummy!", filter.unwrap()));
                    },
                }

                self.upload(log, nickname, &filter);
                BotResponse::None
            },
            Some(&"len") => BotResponse::Private(format!("Log size is {}", log.len())),
            Some(&"help") => BotResponse::Private("log <first/last> [num] | <len> | <dump> [last num<m/h/d>]".to_string()),
            None => BotResponse::Channel("Um... what do you want? Do you need help?".to_string()),
            _ => BotResponse::Channel("I don't know such command...".to_string()),
        }
    }
}

fn main() {
    //Enter directory of bot's executable just in case
    std::env::set_current_dir(std::env::current_exe().unwrap().parent().unwrap())
              .unwrap_or_else(|err| panic!("cannot enter my own directory :(. Err={}", err));

    let mut bot = KuuBot::new();
    bot.run();
}

#[cfg(test)]
mod tests {
    const CONFIG: &'static str = include_str!("../config.json");
    use std;
    use std::io::Write;

    fn pre_condition() {
        std::env::set_current_dir(std::env::current_exe().unwrap().parent().unwrap())
                  .unwrap_or_else(|err| panic!("cannot enter my own directory :(. Err={}", err));

        let mut file = std::fs::File::create("config.json").unwrap();
        file.write_all(CONFIG.as_bytes()).unwrap_or(())
    }

    fn post_condition() {
        std::fs::remove_file("vndis.log").unwrap_or(());
        std::fs::remove_file("config.json").unwrap_or(())
    }

    #[test]
    fn test_cmd_about() {
        pre_condition();

        let bot = super::KuuBot::new();
        let log = super::log::IrcLog::new();

        let response = bot.command_about(&"DoumanAsh".to_string(), &log);
        assert!(match response {
            super::BotResponse::Private(text) => text == format!("{} {}", &bot, &log),
            _ => false
        });

        let response = bot.command_about(&"!DoumanAsh".to_string(), &log);
        assert!(match response {
            super::BotResponse::Channel(text) => text == "This is only for my master!",
            _ => false
        });

        post_condition();
    }

    #[test]
    fn test_cmd_google() {
        let parts = vec!["Kuu:", "google"];
        let response = super::KuuBot::command_google(&parts);
        assert!(match response {
            super::BotResponse::Channel(text) => text == "... bully...",
            _ => false
        });

        let parts = vec!["Kuu:", "google", "vn", "sengoku", "hime"];
        let response = super::KuuBot::command_google(&parts);
        assert!(match response {
            super::BotResponse::Channel(text) => text == format!("http://lmgtfy.com/?q={}", parts[2..].join("+")),
            _ => false
        });
    }

    #[test]
    fn test_cmd_grep() {
        let parts = vec!["Kuu:", "grep"];
        let response = super::KuuBot::command_grep(&parts);
        assert!(match response {
            super::BotResponse::Channel(text) => text == "... bully...",
            _ => false
        });

        let parts = vec!["Kuu:", "grep", "vn", "Sengoku", "Hime"];
        let response = super::KuuBot::command_grep(&parts);
        assert!(match response {
            super::BotResponse::Channel(text) => text == format!("vndb: https://vndb.org/v/all?q={};fil=tagspoil-0;o=d;s=rel", parts[3..].join("+")),
            _ => false
        });
    }

    #[test]
    fn test_log_parse_num() {
        let test_str = "x-25";
        let response = super::KuuBot::log_parse_num(test_str);
        assert!(response.is_err());
        let response = response.err().unwrap();
        assert!(match response {
            super::BotResponse::Channel(text) => text == format!(">{}< is not normal integer... bully", test_str),
            _ => false
        });

        let test_str = "0";
        let response = super::KuuBot::log_parse_num(test_str);
        assert!(response.is_err());
        let response = response.err().unwrap();
        assert!(match response {
            super::BotResponse::Channel(text) => text == "umm... 0? Are you stupid?",
            _ => false
        });

        let test_str = "21";
        let response = super::KuuBot::log_parse_num(test_str);
        assert!(response.is_err());
        let response = response.err().unwrap();
        assert!(match response {
            super::BotResponse::Channel(text) => text == format!(">{}< is too much... I do not wanna flood you.", 21),
            _ => false
        });

        let test_str = "-21";
        let response = super::KuuBot::log_parse_num(test_str);
        assert!(response.is_err());
        let response = response.err().unwrap();
        assert!(match response {
            super::BotResponse::Channel(text) => text == format!(">{}< is too much... I do not wanna flood you.", -21),
            _ => false
        });

        let test_str = "20";
        let response = super::KuuBot::log_parse_num(test_str);
        assert!(response.is_ok());
        assert!(response.unwrap() == 20);

        let test_str = "-20";
        let response = super::KuuBot::log_parse_num(test_str);
        assert!(response.is_ok());
        assert!(response.unwrap() == -20);
    }

    #[test]
    fn test_parse_filter_time() {
        assert!(super::KuuBot::parse_filter_time("20m").is_ok());
        assert!(super::KuuBot::parse_filter_time("20666m").is_ok());
        assert!(super::KuuBot::parse_filter_time("20d").is_ok());
        assert!(super::KuuBot::parse_filter_time("20h").is_ok());
        assert!(super::KuuBot::parse_filter_time("0h").is_ok());

        assert!(super::KuuBot::parse_filter_time("-20h").is_err());
        assert!(super::KuuBot::parse_filter_time("0Gsdasdsa").is_err());
        assert!(super::KuuBot::parse_filter_time("G1").is_err());
        assert!(super::KuuBot::parse_filter_time("5").is_err());
    }

    #[test]
    fn test_simple_responses() {
        pre_condition();

        let bot = super::KuuBot::new();
        let mut log = super::log::IrcLog::new();

        let default_nick = "Someone".to_string();

        let usr_msg = "Kuu: ping".to_string();
        let response = bot.direct_response(&default_nick, &usr_msg, &mut log);
        assert!(match response {
            super::BotResponse::Channel(text) => text == "pong",
            _ => false
        });

        let usr_msg = "Kuu: asdasdasfdsgfdsgdfgdfg".to_string();
        let response = bot.direct_response(&default_nick, &usr_msg, &mut log);
        assert!(match response {
            super::BotResponse::Channel(text) => text == "...",
            _ => false
        });

        let usr_msg = "Kuu: huiping".to_string();
        let response = bot.direct_response(&default_nick, &usr_msg, &mut log);
        assert!(match response {
            super::BotResponse::Channel(text) => text == "死になさいゴミムシ",
            _ => false
        });

        let usr_msg = "!ping".to_string();
        let response = bot.indirect_response(&default_nick, &usr_msg, &mut log);
        assert!(match response {
            super::BotResponse::Channel(text) => text == "pong",
            _ => false
        });

        let usr_msg = "!huiping".to_string();
        let response = bot.indirect_response(&default_nick, &usr_msg, &mut log);
        assert!(match response {
            super::BotResponse::Channel(text) => text == "死になさいゴミムシ",
            _ => false
        });

        let usr_msg = "Kuu: tadaima".to_string();
        let response = bot.indirect_response(&default_nick, &usr_msg, &mut log);
        assert!(match response {
            super::BotResponse::Channel(text) => text == "okaeri",
            _ => false
        });

        post_condition();
    }
}
