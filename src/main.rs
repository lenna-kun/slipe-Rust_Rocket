#![feature(proc_macro_hygiene)]
#![feature(decl_macro)]

#[macro_use]
extern crate rocket;

extern crate crossbeam_channel;

use std::thread;
use std::sync::atomic::{AtomicU8, AtomicBool, Ordering};
use std::io::{self, Cursor};
use rand::Rng;

use std::io::Read;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use rocket_contrib::json::Json;

use rocket::request::Request;
use rocket::response::{self, Responder, Response, NamedFile};
use rocket::{Data, Outcome::*};
use rocket::data::{self, FromDataSimple};
use rocket::http::hyper::header::{AccessControlAllowOrigin};
use rocket::http::{Cookie, Cookies, Status};
use rocket::State;

pub mod game;

const	SENTE: u8 = 0;
const   GOTE: u8 = 1;
const   FURIGOMA: u8 = 2;

// Always use a limit to prevent DoS attacks.
const LIMIT: u64 = 256;

struct ManagedData {
    send_to_gamethread_txrx: [(crossbeam_channel::Sender<String>, crossbeam_channel::Receiver<String>); 4],
    receive_from_gamethread_txrx: [(crossbeam_channel::Sender<String>, crossbeam_channel::Receiver<String>); 4],
    passwords: [[AtomicU8; 10]; 4],
    rules: [AtomicU8; 4],
    rules_result: [AtomicU8; 4],
    playing: [AtomicBool; 4],
}

impl ManagedData {
    fn new() -> ManagedData {
        ManagedData {
            send_to_gamethread_txrx: [crossbeam_channel::bounded(0), crossbeam_channel::bounded(0), crossbeam_channel::bounded(0), crossbeam_channel::bounded(0)],
            receive_from_gamethread_txrx: [crossbeam_channel::bounded(0), crossbeam_channel::bounded(0), crossbeam_channel::bounded(0), crossbeam_channel::bounded(0)],
            passwords: [[AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0)], 
                        [AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0)],
                        [AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0)],
                        [AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0)]],
            rules: [AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0)],
            rules_result: [AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0), AtomicU8::new(0)],
            playing: [AtomicBool::new(false), AtomicBool::new(false), AtomicBool::new(false), AtomicBool::new(false)],
        }
    }
    fn confirm_gamethread_timeout(&self, id: usize, timeout: Duration) -> bool {
        let val = String::from("alive?");
        if let Ok(_) = self.send_to_gamethread_txrx[id].0.send_timeout(val, timeout) {
            true
        } else {
            false
        }
    }
}

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
struct RoomInfo {
    id: u8,
    password: String,
    rule: u8,
}

#[derive(Debug, Serialize, Deserialize)]
struct RoomCheck {
    id: u8,
    password: String,
}

#[get("/room_list")]
fn room_list_get(mut cookies: Cookies) -> io::Result<NamedFile> {
    cookies.remove_private(Cookie::named("id"));
    NamedFile::open("static/room_list.html")
}

#[post("/room_list")]
fn room_list_post(state: State<ManagedData>) -> ApiResponse {
    let mut room_list: [u8; 4] = [0; 4];
    for i in 0..4 {
        if state.rules[i].load(Ordering::Relaxed) == 3 {
            room_list[i] = 3;
        } else if !state.confirm_gamethread_timeout(i, Duration::from_millis(100)) {
            room_list[i] = 3;
            state.rules[i].swap(3, Ordering::Relaxed);
            state.rules_result[i].swap(3, Ordering::Relaxed);
            state.playing[i].swap(false, Ordering::Relaxed);
        } else {
            room_list[i] = state.rules[i].load(Ordering::Relaxed);
        }
    }
    ApiResponse {
        body: format!("{}/{}/{}/{}", room_list[0], room_list[1], room_list[2], room_list[3]),
    }
}

#[post("/make_room", data = "<room_info>")]
fn make_room(mut cookies: Cookies, state: State<ManagedData>, room_info: Json<RoomInfo>) -> ApiResponse {
    let id: usize = room_info.0.id as usize;
    if state.confirm_gamethread_timeout(id, Duration::from_millis(100)) { // if the game thread existed
        return ApiResponse {
            body: String::from("Err"),
        };
    }
    let password_bytes = room_info.0.password.as_bytes();
    for i in 0..state.passwords[id].len() { // set password
        if i < room_info.0.password.len() {
            state.passwords[id][i].swap(password_bytes[i], Ordering::Relaxed);
        } else {
            state.passwords[id][i].swap(0, Ordering::Relaxed);
        }
    }
    state.rules[id].swap(room_info.0.rule, Ordering::Relaxed); // set rule
    state.rules_result[id].swap(room_info.0.rule, Ordering::Relaxed); // set rule
    let cookie = Cookie::build("id", format!("{}0", id)) // create cookie
        .path("/")
        // .secure(true)
        .finish();
    cookies.add_private(cookie);
    let (tx, rx) = (state.receive_from_gamethread_txrx[id].0.clone(), state.send_to_gamethread_txrx[id].1.clone());
    match room_info.0.rule {
        FURIGOMA => {
            let mut rng = rand::thread_rng();
            let furigoma: u8 = rng.gen::<u8>() % 2;
            state.rules_result[id].swap(furigoma, Ordering::Relaxed);
            thread::spawn(move || {
                let mut game = game::GameInfo::from_data(id as u8, furigoma, tx, rx);
                game.play_game();
            });
            return ApiResponse {
                body: String::from("Ok"),
            };
        },
        SENTE => {
            thread::spawn(move || {
                let mut game = game::GameInfo::from_data(id as u8, SENTE, tx, rx);
                game.play_game();
            });
            return ApiResponse {
                body: String::from("Ok"),
            };
        },
        GOTE => {
            thread::spawn(move || {
                let mut game = game::GameInfo::from_data(id as u8, GOTE, tx, rx);
                game.play_game();
            });
            return ApiResponse {
                body: String::from("Ok"),
            };
        },
        _ => {
            return ApiResponse {
                body: String::from("reject"),
            };
        },
    }
}

#[post("/enter_room", data = "<room_check>")]
fn enter_room(mut cookies: Cookies, state: State<ManagedData>, room_check: Json<RoomCheck>) -> ApiResponse {
    let id: usize = room_check.0.id as usize;
    if !state.confirm_gamethread_timeout(id, Duration::from_millis(100)) { // if game thread has not been created
        return ApiResponse {
            body: String::from("Err"),
        };
    }
    if state.playing[id].load(Ordering::Relaxed) { // if other "player2" has already entered the room
        return ApiResponse {
            body: String::from("Err"),
        };
    }
    let password_bytes = room_check.0.password.as_bytes();
    for i in 0..state.passwords[id].len() {
        if i < room_check.0.password.len() {
            if state.passwords[id][i].load(Ordering::Relaxed) != password_bytes[i] { // wrong password
                return ApiResponse {
                    body: String::from("Err"),
                };
            }
        } else {
            if state.passwords[id][i].load(Ordering::Relaxed) != 0 { // wrong password
                return ApiResponse {
                    body: String::from("Err"),
                };
            }
        }
    }
    let cookie = Cookie::build("id", format!("{}1", room_check.0.id))
        .path("/")
        // .secure(true)
        .finish();
    cookies.add_private(cookie);
    ApiResponse {
        body: String::from("Ok"),
    }
}

#[get("/playing")]
fn playing(mut cookies: Cookies) -> io::Result<NamedFile> {
    if let Some(cookie) = cookies.get_private("id") {
        if cookie.value().chars().nth(1).unwrap() == '0' {
            NamedFile::open("static/playing_0.html")
        } else {
            NamedFile::open("static/playing_1.html")
        }
    } else {
        NamedFile::open("static/reject.html")
    }
}

#[post("/playing/board")]
fn playing_board(mut cookies: Cookies, state: State<ManagedData>) -> ApiResponse {
    if let Some(cookie) = cookies.get_private("id") {
        let id: usize = cookie.value().chars().nth(0).unwrap().to_string().parse().unwrap();
        let val: String = format!("{}board", cookie.value().chars().nth(1).unwrap());
        match state.send_to_gamethread_txrx[id].0.send_timeout(val, Duration::from_millis(100)) {
            Ok(_) => {
                if let Ok(response) = state.receive_from_gamethread_txrx[id].1.recv_timeout(Duration::from_millis(100)) {
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
            },
            Err(_) => {
                return ApiResponse {
                    body: String::from("Err"),
                };
            },
        }
    } else {
        return ApiResponse {
            body: String::from("reject"),
        };
    }
}

#[post("/playing/set", data = "<set>")]
fn playing_set(mut cookies: Cookies, state: State<ManagedData>, set: ApiResponse) -> ApiResponse {
    if let Some(cookie) = cookies.get_private("id") {
        let id: usize = cookie.value().chars().nth(0).unwrap().to_string().parse().unwrap();
        let set: String = set.body[4..7].to_string();
        let mut val: String = format!("{}set{:?}", cookie.value().chars().nth(1).unwrap(), set);
        val.retain(|c| c != '"');
        // println!("{:?}", val);
        match state.send_to_gamethread_txrx[id].0.send_timeout(val, Duration::from_millis(100)) {
            Ok(_) => {
                if let Ok(response) = state.receive_from_gamethread_txrx[id].1.recv_timeout(Duration::from_millis(100)) {
                    // println!("{:?}", response);
                    return ApiResponse {
                        body: response,
                    };
                } else {
                    return ApiResponse {
                        body: String::from("Err"),
                    };
                }
            },
            Err(_) => {
                return ApiResponse {
                    body: String::from("Err"),
                };
            },
        }
    } else {
        return ApiResponse {
            body: String::from("reject"),
        };
    }
}

#[post("/playing/unload")]
fn unload(mut cookies: Cookies, state: State<ManagedData>) -> ApiResponse {
    if let Some(cookie) = cookies.get_private("id") {
        let id: usize = cookie.value().chars().nth(0).unwrap().to_string().parse().unwrap();
        let val = String::from("unload");
        match state.send_to_gamethread_txrx[id].0.send_timeout(val, Duration::from_millis(100)) {
            Ok(_) => {
                cookies.remove_private(Cookie::named("id"));
                return ApiResponse {
                    body: String::from("Ok"),
                };
            },
            Err(_) => {
                return ApiResponse {
                    body: String::from("Err"),
                };
            },
        }
    } else {
        return ApiResponse {
            body: String::from("reject"),
        };
    }
}

#[get("/static/win.mp3")]
fn win_mp3() -> io::Result<NamedFile> {
    NamedFile::open("static/win.mp3")
}

#[get("/static/lose.mp3")]
fn lose_mp3() -> io::Result<NamedFile> {
    NamedFile::open("static/lose.mp3")
}

#[get("/static/bgm.mp3")]
fn bgm_mp3() -> io::Result<NamedFile> {
    NamedFile::open("static/bgm.mp3")
}

fn main() {
    rocket::ignite()
        .manage(ManagedData::new())
        .mount("/", routes![room_list_get, room_list_post, make_room, enter_room, playing, playing_board, playing_set, unload, win_mp3, lose_mp3, bgm_mp3])
        .launch();
}