#![feature(proc_macro_hygiene)]
#![feature(decl_macro)]

#[macro_use]
extern crate rocket;

#[macro_use]
extern crate lazy_static;

use std::thread;
use std::sync::mpsc;
use std::io::Cursor;
use rand::Rng;

pub use std::io::{Read};
pub use std::process::Command;
pub use std::io::prelude::*;
pub use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use rocket_contrib::json::Json;

use rocket::http::Status;
use rocket::request::Request;
use rocket::response;
use rocket::response::{Responder, Response};
use rocket::{Data, Outcome::*};
use rocket::data::{self, FromDataSimple};
use rocket::http::hyper::header::{AccessControlAllowOrigin};
use rocket::http::{Cookie, Cookies};

pub mod global;
pub mod game;

const	SENTE: u8 = 0;
const   GOTE: u8 = 1;
const   FURIGOMA: u8 = 2;

// Always use a limit to prevent DoS attacks.
const LIMIT: u64 = 256;

#[derive(Debug)]
struct ApiResponse {
    body: String,
}

impl<'r> Responder<'r> for ApiResponse {
    // It is necessary for this struct to implement the Responder trait .
    fn respond_to(self, _req: &Request) -> response::Result<'r> {
        let body: String = self.body.clone();
        Response::build()
            .status(Status::Ok)
            .header(AccessControlAllowOrigin::Any)
            .sized_body(Cursor::new(body))
            .ok()
    }
}

impl FromDataSimple for ApiResponse {
    type Error = String;

    fn from_data(_req: &rocket::Request, data: Data) -> data::Outcome<Self, String> {
        // Ensure the content type is correct before opening the data.
        // let person_ct = ContentType::new("application", "x-person");
        // if req.content_type() != Some(&person_ct) {
        //     return Outcome::Forward(data);
        // }

        // Read the data into a String.
        let mut string = String::new();
        if let Err(e) = data.open().take(LIMIT).read_to_string(&mut string) {
            return Failure((Status::InternalServerError, format!("{:?}", e)));
        }

        // Return successfully.
        Success(ApiResponse { body: string })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoomInfo {
    pub id: u8,
    pub password: String,
    pub rule: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoomCheck {
    pub id: u8,
    pub password: String,
}

#[get("/room_list")]
fn room_list_get(mut cookies: Cookies) -> ApiResponse {
    cookies.remove_private(Cookie::named("id"));
    let res: String = global::ROOMLIST_HTML.clone();
    ApiResponse {
        body: res,
    }
}

#[post("/room_list")]
fn room_list_post() -> ApiResponse {
    let mut room_list: [u8; 4] = [0; 4];
    let rules = global::RULES.lock().unwrap();
    for i in 0..4 {
        if let Some(r) = rules[i] {
            room_list[i] = r;
        } else {
            room_list[i] = 3;
        }
    }
    ApiResponse {
        body: format!("{}/{}/{}/{}", room_list[0], room_list[1], room_list[2], room_list[3]),
    }
}

#[post("/make_room", data = "<room_info>")]
fn make_room(mut cookies: Cookies, room_info: Json<RoomInfo>) -> ApiResponse {
    let mut rules = global::RULES.lock().unwrap();
    if let None = rules[room_info.0.id as usize] {
        global::PASSWORDS.lock().unwrap()[room_info.0.id as usize] = Some(room_info.0.password);
        rules[room_info.0.id as usize] = Some(room_info.0.rule);
        global::RULES_RESULT.lock().unwrap()[room_info.0.id as usize] = Some(room_info.0.rule);
        let cookie = Cookie::build("id", format!("{}0", room_info.0.id))
            .path("/")
            // .secure(true)
            .finish();
        cookies.add_private(cookie);
        ApiResponse {
            body: String::from("Ok"),
        }
    } else {
        ApiResponse {
            body: String::from("Err"),
        }
    }
}

#[post("/enter_room", data = "<room_check>")]
fn enter_room(mut cookies: Cookies, room_check: Json<RoomCheck>) -> ApiResponse {
    if let Some(p) = &global::PASSWORDS.lock().unwrap()[room_check.0.id as usize] {
        let chk = global::STARTED.lock().unwrap();
        if room_check.0.password == *p && !chk[room_check.0.id as usize] {
            let cookie = Cookie::build("id", format!("{}1", room_check.0.id))
                .path("/")
                // .secure(true)
                .finish();
            cookies.add_private(cookie);
            ApiResponse {
                body: String::from("Ok"),
            }
        } else {
            ApiResponse {
                body: String::from("Err"),
            }
        }
    } else {
        ApiResponse {
            body: String::from("Err"),
        }
    }
}

#[get("/playing")]
fn playing(mut cookies: Cookies) -> ApiResponse {
    if let Some(cookie) = cookies.get_private("id") {
        let room_id: usize = cookie.value().chars().nth(0).unwrap().to_string().parse().unwrap();
        if cookie.value().chars().nth(1).unwrap() == '0' {
            let mut rules = global::RULES_RESULT.lock().unwrap();
            if let Some(FURIGOMA) = rules[cookie.value().chars().nth(0).unwrap().to_string().parse::<usize>().unwrap()] {
                let mut rng = rand::thread_rng();
                let furigoma: u8 = rng.gen::<u8>() % 2;
                rules[cookie.value().chars().nth(0).unwrap().to_string().parse::<usize>().unwrap()] = Some(furigoma);
                if furigoma == SENTE {
                    let res: String = global::PLAYING_SENTE_HTML.clone();
                    return ApiResponse {
                        body: res,
                    };
                } else {
                    let res: String = global::PLAYING_GOTE_HTML.clone();
                    return ApiResponse {
                        body: res,
                    };
                }
            } else if let Some(SENTE) = rules[cookie.value().chars().nth(0).unwrap().to_string().parse::<usize>().unwrap()] {
                let res: String = global::PLAYING_SENTE_HTML.clone();
                return ApiResponse {
                    body: res,
                };
            } else if let Some(GOTE) = rules[cookie.value().chars().nth(0).unwrap().to_string().parse::<usize>().unwrap()] {
                let res: String = global::PLAYING_GOTE_HTML.clone();
                return ApiResponse {
                    body: res,
                };
            } else {
                return ApiResponse {
                    body: String::from("reject"),
                };
            }
        } else {
            let mut rules = global::RULES_RESULT.lock().unwrap();
            if let Some(FURIGOMA) = rules[room_id] {
                let mut rng = rand::thread_rng();
                let furigoma: u8 = rng.gen::<u8>() % 2;
                rules[room_id] = Some(furigoma);
                
                thread::spawn(move || {
                    let (tx1, rx1) = mpsc::channel();
                    let (tx2, rx2) = mpsc::channel();
                    global::SENDERS.lock().unwrap()[room_id] = Some(tx1);
                    global::RECEIVERS.lock().unwrap()[room_id] = Some(rx2);
                    let mut game = game::GameInfo::from_data(room_id as u8, furigoma, tx2, rx1);
                    game.play_game();
                });
                global::STARTED.lock().unwrap()[room_id] = true;

                if furigoma == SENTE {
                    let res: String = global::PLAYING_GOTE_HTML.clone();
                    return ApiResponse {
                        body: res,
                    };
                } else {
                    let res: String = global::PLAYING_SENTE_HTML.clone();
                    return ApiResponse {
                        body: res,
                    };
                }
            } else if let Some(SENTE) = rules[room_id] {

                thread::spawn(move || {
                    let (tx1, rx1) = mpsc::channel();
                    let (tx2, rx2) = mpsc::channel();
                    global::SENDERS.lock().unwrap()[room_id] = Some(tx1);
                    global::RECEIVERS.lock().unwrap()[room_id] = Some(rx2);
                    let mut game = game::GameInfo::from_data(room_id as u8, SENTE, tx2, rx1);
                    game.play_game();
                });

                let res: String = global::PLAYING_GOTE_HTML.clone();
                return ApiResponse {
                    body: res,
                };
            } else if let Some(GOTE) = rules[room_id] {

                thread::spawn(move || {
                    let (tx1, rx1) = mpsc::channel();
                    let (tx2, rx2) = mpsc::channel();
                    global::SENDERS.lock().unwrap()[room_id] = Some(tx1);
                    global::RECEIVERS.lock().unwrap()[room_id] = Some(rx2);
                    let mut game = game::GameInfo::from_data(room_id as u8, GOTE, tx2, rx1);
                    game.play_game();
                });

                let res: String = global::PLAYING_SENTE_HTML.clone();
                return ApiResponse {
                    body: res,
                };
            } else {
                return ApiResponse {
                    body: String::from("reject"),
                };
            }
        }
    } else {
        ApiResponse {
            body: String::from("reject"),
        }
    }
}

#[post("/playing/board")]
fn playing_board(mut cookies: Cookies) -> ApiResponse {
    if let Some(cookie) = cookies.get_private("id") {
        let val: String = format!("{}board", cookie.value().chars().nth(1).unwrap());
        if let None = global::SENDERS.lock().unwrap()[cookie.value().chars().nth(0).unwrap().to_string().parse::<usize>().unwrap()].as_ref() {
            return ApiResponse {
                body: String::from("Err"),
            };
        }
        if let Err(_) = global::SENDERS.lock().unwrap()[cookie.value().chars().nth(0).unwrap().to_string().parse::<usize>().unwrap()].as_ref().unwrap().send(val) {
            return ApiResponse {
                body: String::from("Err"),
            };
        }
        loop {
            if let Ok(response) = global::RECEIVERS.lock().unwrap()[cookie.value().chars().nth(0).unwrap().to_string().parse::<usize>().unwrap()].as_ref().unwrap().recv_timeout(Duration::from_millis(100)) {
                if let Some(_) = response.find("winner") {
                    cookies.remove_private(Cookie::named("id"));
                }
                return ApiResponse {
                    body: response,
                };
            } else {
                return ApiResponse {
                    body: String::from("Err"),
                };
            }
        }
    } else {
        return ApiResponse {
            body: String::from("reject"),
        };
    }
}

#[post("/playing/set", data = "<set>")]
fn playing_set(mut cookies: Cookies, set: ApiResponse) -> ApiResponse {
    if let Some(cookie) = cookies.get_private("id") {
        let set: String = set.body[4..7].to_string();
        let mut val: String = format!("{}set{:?}", cookie.value().chars().nth(1).unwrap(), set);
        val.retain(|c| c != '"');
        println!("{:?}", val);
        global::SENDERS.lock().unwrap()[cookie.value().chars().nth(0).unwrap().to_string().parse::<usize>().unwrap()].as_ref().unwrap().send(val).unwrap();
        loop {
            if let Ok(response) = global::RECEIVERS.lock().unwrap()[cookie.value().chars().nth(0).unwrap().to_string().parse::<usize>().unwrap()].as_ref().unwrap().recv_timeout(Duration::from_millis(100)) {
                println!("{:?}", response);
                return ApiResponse {
                    body: response,
                };
            } else {
                return ApiResponse {
                    body: String::from("Err"),
                };
            }
        }
    } else {
        return ApiResponse {
            body: String::from("reject"),
        };
    }
}

#[get("/room_list/manage/clear/room_0")]
fn clear_room_0() -> ApiResponse {
    if !global::STARTED.lock().unwrap()[0] {
        global::RULES.lock().unwrap()[0] = None;
        global::RULES_RESULT.lock().unwrap()[0] = None;
        global::PASSWORDS.lock().unwrap()[0] = None;
    }
    ApiResponse {
        body: String::from("Ok"),
    }
}

#[get("/room_list/manage/clear_rooms")]
fn clear_rooms() -> ApiResponse {
    for i in 0..4 {
        if !global::STARTED.lock().unwrap()[i] {
            global::RULES.lock().unwrap()[i] = None;
            global::RULES_RESULT.lock().unwrap()[i] = None;
            global::PASSWORDS.lock().unwrap()[i] = None;
        }
    }
    ApiResponse {
        body: String::from("Ok"),
    }
}

fn main() {
    rocket::ignite()
        .mount("/", routes![room_list_get, room_list_post, make_room, enter_room, playing, playing_board, playing_set, clear_rooms])
        .launch();
}