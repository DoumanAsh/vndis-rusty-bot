extern crate irc;

use irc::client::prelude::*;
use irc::client::conn::NetStream;
use std::io::{BufWriter, BufReader};
mod utils;
mod log;

const VNDIS: &'static str = "#vndis";

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

    ///Starts bot and handle messages.
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
                                println!("Joined {}", VNDIS);
                            },
                            _ => (),
                        }
                    },
                    Err(err) => println!(">>>ERROR: {}", err),
                }
            }
        }
    }

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

        if usr_msg.starts_with("KuuRusty") {
            self.server.send_privmsg(&message.args[0], &self.direct_response(&nickname, &usr_msg)[..]).unwrap();
        }
        else if let Some(msg) = self.indirect_response(&nickname, &usr_msg) {
            self.server.send_privmsg(&message.args[0], &msg[..]).unwrap();

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
    fn direct_response(&self, nickname: &String, usr_msg: &String) -> String {
        let usr_msg = usr_msg.to_lowercase();
        let parts: Vec<&str> = usr_msg.split_whitespace().collect();
        match parts[1] {
            "ping" | "пинг" => format!("{}: pong", nickname),
            "grep" | "find" => {
                    if parts.len() < 4 {
                        return format!("{}: ... bully...", nickname);
                    }

                    match parts[2] {
                        "vn" => format!("{}: vndb: https://vndb.org/v/all?q={};fil=tagspoil-0;o=d;s=rel", nickname, parts[3..].join("+")),
                        _ => format!("{}: ... bully...", nickname),
                    }
            },
            "google" => {
                if parts.len() < 3 {
                    return format!("{}: ... bully...", nickname);
                }

                format!("{}: http://lmgtfy.com/?q={}", nickname, parts[2..].join("+"))
            },
            "huiping" | "хуйпинг" => format!("{}: disappear, ゴミムシ", nickname),
            _ => format!("{}: ...", nickname),
        }
    }

    #[inline]
    ///Handler to all messages in general
    fn indirect_response(&self, nickname: &String, usr_msg: &String) -> Option<String> {
        let usr_msg = usr_msg.to_lowercase();
        match &usr_msg[..] {
            "!ping" | "!пинг" => Some(format!("{}: pong", nickname)),
            "!huiping" | "!хуйпинг" => Some(format!("{}: disappear, ゴミムシ", nickname)),
            _ if usr_msg.contains("tadaima") || usr_msg.contains("тадайма") || usr_msg.contains("ただいま")=> Some(format!("{}: okaeri", nickname)),
            _ => None,
        }
    }
}

fn main() {
    let mut bot = KuuBot::new();
    bot.run();
}
