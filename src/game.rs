use std::time::Instant;
use std::sync::mpsc;

use super::global;

const	NONE: u8 = 0;
const   SOLDIER1: u8 = 1;
const	KING1: u8 = 2;
const   SOLDIER2: u8 = 3;
const	KING2: u8 = 4;
const   WALL: u8 = 5;
const	GOAL: u8 = 6;

const   TIMEOUT: u64 = 30;

#[derive(Debug)]
pub struct GameInfo {
    pub room_id: u8,  // 0~3
    pub turn: u8, // 0 or 1
    pub tx: mpsc::Sender<String>,
    pub rx: mpsc::Receiver<String>,
    pub board: [[u8; 7]; 7],
}

impl GameInfo {
    pub fn from_data(room_id: u8, turn: u8, tx: mpsc::Sender<String>, rx: mpsc::Receiver<String>) -> GameInfo {
        GameInfo {
            room_id: room_id,
            turn: turn,
            tx: tx,
            rx: rx,
            board: [[WALL,WALL,WALL,WALL,WALL,WALL,WALL],
                    [WALL,SOLDIER1,SOLDIER1,KING1,SOLDIER1,SOLDIER1,WALL],
                    [WALL,NONE,NONE,NONE,NONE,NONE,WALL],
                    [WALL,NONE,NONE,GOAL,NONE,NONE,WALL],
                    [WALL,NONE,NONE,NONE,NONE,NONE,WALL],
                    [WALL,SOLDIER2,SOLDIER2,KING2,SOLDIER2,SOLDIER2,WALL],
                    [WALL,WALL,WALL,WALL,WALL,WALL,WALL]],
        }
    }

    fn set(&mut self, parameters: Vec<usize>) -> Result<(), String> {
        if parameters[0] <= 0 || parameters[1] <= 0 || parameters[0] >= 6 || parameters[1] >= 6 {
            return Err(String::from("invalid index"));
        }
        if self.board[parameters[0]][parameters[1]] != 2*self.turn+1 && self.board[parameters[0]][parameters[1]] != 2*self.turn+2 {
            return Err(String::from("not your piece"));
        }
        let mut dst: [usize; 2] = [parameters[0], parameters[1]];
        match parameters[2] {
            0 => {
                if self.board[parameters[0]-1][parameters[1]] != NONE && self.board[parameters[0]-1][parameters[1]] != GOAL {
                    return Err(String::from("cannot move"));
                }
                while self.board[dst[0]-1][dst[1]] == NONE || self.board[dst[0]-1][dst[1]] == GOAL {
                    dst[0] -= 1;
                }
            },
            1 => {
                if self.board[parameters[0]][parameters[1]+1] != NONE && self.board[parameters[0]][parameters[1]+1] != GOAL {
                    return Err(String::from("cannot move"));
                }
                while self.board[dst[0]][dst[1]+1] == NONE || self.board[dst[0]][dst[1]+1] == GOAL {
                    dst[1] += 1;
                }
            },
            2 => {
                if self.board[parameters[0]+1][parameters[1]] != NONE && self.board[parameters[0]+1][parameters[1]] != GOAL {
                    return Err(String::from("cannot move"));
                }
                while self.board[dst[0]+1][dst[1]] == NONE || self.board[dst[0]+1][dst[1]] == GOAL {
                    dst[0] += 1;
                }
            },
            3 => {
                if self.board[parameters[0]][parameters[1]-1] != NONE && self.board[parameters[0]][parameters[1]-1] != GOAL {
                    return Err(String::from("cannot move"));
                }
                while self.board[dst[0]][dst[1]-1] == NONE || self.board[dst[0]][dst[1]-1] == GOAL {
                    dst[1] -= 1;
                }
            },
            _ => return Err(String::from("invalid direction")),
        }
        let piece = self.board[parameters[0]][parameters[1]];
        if parameters[0] == 3 && parameters[1] == 3 {
            self.board[parameters[0]][parameters[1]] = GOAL;
        } else {
            self.board[parameters[0]][parameters[1]] = NONE;
        }
        self.board[dst[0]][dst[1]] = piece;
        Ok(())
    }

    fn finalize(&self) {
        global::RULES.lock().unwrap()[self.room_id as usize] = None;
        global::RULES_RESULT.lock().unwrap()[self.room_id as usize] = None;
        global::PASSWORDS.lock().unwrap()[self.room_id as usize] = None;
        global::STARTED.lock().unwrap()[self.room_id as usize] = false;
    }

    pub fn play_game(&mut self) {
        let mut winner: Option<u8> = None;
        let mut check_box: u8;
        loop { // main loop
            if self.board[3][3] == KING1 {
                winner = Some(0);
            } else if self.board[3][3] == KING2 {
                winner = Some(1);
            }
            if let Some(p) = winner { // if game has been finished
                check_box = 0b00;
                let timer = Instant::now();
                while check_box != 0b11 {
                    if timer.elapsed().as_secs() >= TIMEOUT {
                        self.finalize();
                        panic!("timeout");
                    }
                    match self.rx.recv() {
                        Ok(request) => {
                            if let Some(_) = request.find("board") {
                                if request.starts_with("0") {
                                    check_box |= 2_u8.pow(0 as u32);
                                    let response: String = format!("winner{}", p);
                                    self.tx.send(response).unwrap();
                                } else if request.starts_with("1") {
                                    check_box |= 2_u8.pow(1 as u32);
                                    let response: String = format!("winner{}", p);
                                    self.tx.send(response).unwrap();
                                } else {
                                    panic!("unexpected request");
                                }
                            } else {
                                let response: String = String::from("reject");
                                self.tx.send(response).unwrap();
                            }
                        },
                        Err(_) => (),
                    }
                }
                self.finalize();
                println!("The winner is Player{}.", p);
                return;
            }

            check_box = 0b00;
            let timer = Instant::now();
            while check_box != 0b11 { // board
                if timer.elapsed().as_secs() >= TIMEOUT {
                    self.finalize();
                    panic!("timeout");
                }
                match self.rx.recv() {
                    Ok(request) => {
                        if let Some(_) = request.find("board") {
                            let mut response: String = String::new();
                            response.push((self.turn + 48) as char);
                            for x in 1..6 {
                                for y in 1..6 {
                                    response.push((&self.board[x as usize][y as usize] + 48) as char);
                                }
                            }
                            if request.starts_with("0") {
                                check_box |= 2_u8.pow(0 as u32);
                                self.tx.send(response).unwrap();
                            } else if request.starts_with("1") {
                                check_box |= 2_u8.pow(1 as u32);
                                self.tx.send(response).unwrap();
                            } else {
                                panic!("unexpected request");
                            }
                        } else {
                            let response: String = String::from("now board");
                            self.tx.send(response).unwrap();
                        }
                    },
                    Err(_) => (),
                }
            }

            let timer = Instant::now();
            loop { // set
                if timer.elapsed().as_secs() >= TIMEOUT {
                    self.finalize();
                    panic!("timeout");
                }
                match self.rx.recv() {
                    Ok(request) => {
                        if let Some(_) = request.find("set") {
                            if request.starts_with(&self.turn.to_string()) { // if the request is from the correct player
                                let parameters: Vec<usize> = request[4..7].chars().map(|x| x.to_string().parse::<usize>().unwrap()).collect();
                                if let Err(s) = self.set(parameters) {
                                    self.tx.send(s).unwrap();
                                } else {
                                    let response: String = String::from("Ok");
                                    self.tx.send(response).unwrap();
                                    break;
                                }
                            } else {
                                let response: String = String::from("not your turn");
                                self.tx.send(response).unwrap();
                            }
                        } else {
                            let response: String = String::from("now set");
                            self.tx.send(response).unwrap();
                        }
                    },
                    Err(_) => (),
                }
            }

            if self.turn == 0 {
                self.turn = 1;
            } else {
                self.turn = 0;
            }
        }
    }
}