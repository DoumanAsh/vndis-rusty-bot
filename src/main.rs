extern crate irc;

use irc::client::prelude::*;
use irc::client::conn::NetStream;
use std::io::{BufWriter, BufReader};
mod utils;
mod log;

const VNDIS: &'static str = "#vndis";

enum BotResponse {
    None,
    Private(String),
    Channel(String)
}

struct KuuBot {
    server: IrcServer<BufReader<NetStream>, BufWriter<NetStream>>,
}

impl KuuBot {
    #[inline(always)]
    ///Creates default bot.
    fn new() -> KuuBot {
        KuuBot {
            server: IrcServer::new("config.json").unwrap(),
        }
    }

    ///Starts bot which continuously handles messages.
    fn run(&mut self) {
        let mut joined = false;
        let mut log = log::IrcLog::new();
        self.server.identify().unwrap();
        loop {
            for message in self.server.iter() {
                match message {
                    Ok(message) => {
                        match &message.command[..] {
                            "PRIVMSG" =>  if joined { self.handle_msg(message, &mut log); },
                            "JOIN" =>  if !joined && message.suffix.unwrap_or("".to_string()) == VNDIS {
                                joined = true;
                                println!(">>>Joined {}", VNDIS);
                            },
                            _ => (),
                        }
                    },
                    Err(err) => println!(">>>ERROR: {}", err),
                }
            }
            println!(">>>Connection is lost. Reconnect after 1s");
            std::thread::sleep_ms(1000);
            println!(">>>Reconnect");
            self.server.reconnect().unwrap();
            self.server.identify().unwrap();
        }
    }

    #[inline]
    fn handle_msg(&self, message: irc::client::data::message::Message, log: &mut log::IrcLog) {
        match &*message.args[0] {
            VNDIS => self.vndis_msg(message, log),
            //Most possibly private query.
            _ if message.args[0].starts_with("KuuRusty") => self.private_query(message),
            _ => println!(">>>ERROR: Got unexpected message={:?}", message),
        }
    }

    #[inline]
    ///Handler to all VNDIS messages
    fn vndis_msg(&self, message: irc::client::data::message::Message, log: &mut log::IrcLog) {
        let nickname = message.prefix.unwrap_or("".to_string());
        let nickname = nickname[..nickname.find('!').unwrap_or(0)].to_string();
        let usr_msg = message.suffix.unwrap_or("".to_string());

        let response: BotResponse;
        if usr_msg.starts_with("KuuRusty") {
            response = self.direct_response(&nickname, &usr_msg, log)
        }
        else {
            response = self.indirect_response(&nickname, &usr_msg)
        }

        match response {
            BotResponse::Channel(text) => {self.server.send_privmsg(VNDIS, &text).unwrap();},
            BotResponse::Private(text) => {
                //for private response we allow to send several.
                for line in text.lines() {
                    self.server.send_privmsg(&nickname, &line).unwrap();
                }
            },
            BotResponse::None => (),
        }

        log.add(log::IrcEntry::new(nickname, usr_msg));
        println!("{}", log.back().unwrap());
    }

    #[inline]
    ///Handler to private queries.
    fn private_query(&self, message: irc::client::data::message::Message) {
        if let Some(nickname) = message.prefix {
            let nickname = nickname[..nickname.find('!').unwrap_or(0)].to_string();
            if !nickname.starts_with("Douman") {
                self.server.send_privmsg(&nickname, "Please do not bother me").unwrap();
                self.server.send_privmsg(VNDIS, &format!("Douman: master, some weird {} bullies me :(", nickname)).unwrap();
            }
        }
        else {
            println!(">>>ERROR: Got private query from who knows whom :(");
        }
    }

    ///Handler to direct msgs i.e. addresses to bot
    fn direct_response(&self, nickname: &String, usr_msg: &String, log: &mut log::IrcLog) -> BotResponse {
        let usr_msg = usr_msg.to_lowercase();
        let parts: Vec<&str> = usr_msg.split_whitespace().collect();
        match parts[1] {
            "ping" | "пинг" => BotResponse::Channel(format!("{}: pong", nickname)),
            "grep" | "find" => {
                    if parts.len() < 4 {
                        return BotResponse::Channel(format!("{}: ... bully...", nickname));
                    }

                    match parts[2] {
                        "vn" => BotResponse::Channel(format!("{}: vndb: https://vndb.org/v/all?q={};fil=tagspoil-0;o=d;s=rel", nickname, parts[3..].join("+"))),
                        _ => BotResponse::Channel(format!("{}: ... bully...", nickname)),
                    }
            },
            "google" => {
                if parts.len() < 3 {
                    return BotResponse::Channel(format!("{}: ... bully...", nickname));
                }

                BotResponse::Channel(format!("{}: http://lmgtfy.com/?q={}", nickname, parts[2..].join("+")))
            },
            "log" => self.command_log(nickname, &parts, log),
            "huiping" | "хуйпинг" => BotResponse::Channel(format!("{}: 死になさい、ゴミムシ", nickname)),
            _ => BotResponse::Channel(format!("{}: ...", nickname)),
        }
    }

    #[inline]
    ///Handler to all messages in general
    fn indirect_response(&self, nickname: &String, usr_msg: &String) -> BotResponse {
        let usr_msg = usr_msg.to_lowercase();
        match &usr_msg[..] {
            "!ping" | "!пинг" => BotResponse::Channel(format!("{}: pong", nickname)),
            "!huiping" | "!хуйпинг" => BotResponse::Channel(format!("{}: 死になさい、ゴミムシ", nickname)),
            _ if usr_msg.contains("tadaima") || usr_msg.contains("тадайма") || usr_msg.contains("ただいま")=> BotResponse::Channel(format!("{}: okaeri", nickname)),
            _ => BotResponse::None,
        }
    }

    #[inline]
    ///Parse num for log command. Allowed range [-20:20]
    fn log_parse_num(&self, nickname: &String, num_str: &str) -> Result<isize, BotResponse> {
        let parse_res = num_str.parse::<isize>();
        if parse_res.is_err(){
            return Err(BotResponse::Channel(format!("{}: >{}< is not normal number... bully", nickname, num_str)));
        }

        let num: isize = parse_res.unwrap();
        if num == 0 {
            return Err(BotResponse::Channel(format!("{}: umm... {}? Are you stupid?", nickname, num)));
        }
        else if num > 20 || num < -20 {
            return Err(BotResponse::Channel(format!("{}: >{}< is too much... I do not wanna flood you.", nickname, num)));
        }
        Ok(num)
    }

    ///Handler for command log
    fn command_log(&self, nickname: &String, parts: &Vec<&str>, log: &mut log::IrcLog) -> BotResponse {
        match parts[2] {
            "last" => {
                let num: isize;
                if parts.len() > 3 {
                    match self.log_parse_num(nickname, parts[3]) {
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
            "first" => {
                let num: isize;
                if parts.len() > 3 {
                    match self.log_parse_num(nickname, parts[3]) {
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
            "len" => BotResponse::Private(format!("Log size is {}", log.len())),
            "help" => BotResponse::Private("log <first/last> [num] | <len>".to_string()),
            _ => BotResponse::Channel(format!("{}: I don't know such command...", nickname)),
        }
    }
}

fn main() {
    let mut bot = KuuBot::new();
    bot.run();
}
