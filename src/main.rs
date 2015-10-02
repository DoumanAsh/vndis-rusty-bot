extern crate irc;

use irc::client::prelude::*;

mod utils;

///Handler to direct msgs i.e. addresses to bot
fn direct_response(nickname: &String, usr_msg: &String) -> String {
    let usr_msg = usr_msg.to_lowercase();
    let parts: Vec<&str> = usr_msg.split_whitespace().collect();
    match parts[1] {
        "ping" => format!("{}: pong", nickname),
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
        _ => format!("{}: ...", nickname),
    }
}

///Handler to all messages in general
fn indirect_response(nickname: &String, usr_msg: &String) -> Option<String> {
    let usr_msg = usr_msg.to_lowercase();
    match &usr_msg[..] {
        "!ping" => Some(format!("{}: pong", nickname)),
        _ if usr_msg.contains("tadaima") || usr_msg.contains("тадайма") || usr_msg.contains("ただいま")=> Some(format!("{}: okaeri", nickname)),
        _ => None,
    }
}

fn main() {
    let server = IrcServer::new("config.json").unwrap();
    server.identify().unwrap();
    loop {
        for message in server.iter() {
            match message {
                Ok(message) => {
                    match &message.command[..] {
                        "PRIVMSG" => {
                            let nickname = message.prefix.unwrap_or("".to_string());
                            let nickname = nickname[..nickname.find('!').unwrap_or(0)].to_string();
                            let usr_msg = message.suffix.unwrap_or("".to_string());

                            if usr_msg.starts_with("KuuRusty:") {
                                server.send_privmsg(&message.args[0], &direct_response(&nickname, &usr_msg)[..]).unwrap();
                            }
                            else if let Some(msg) = indirect_response(&nickname, &usr_msg) {
                                server.send_privmsg(&message.args[0], &msg[..]).unwrap();

                            }
                            println!("{}: {}", nickname, usr_msg);
                        },
                        _ => (),
                    }
                },
                Err(err) => println!(">>>ERROR: {}", err),
            }
        }
    }
}
