use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::fs::File;
pub use std::io::{Read};

lazy_static! {
    pub static ref RECEIVERS: Arc<Mutex<[Option<mpsc::Receiver<String>>; 4]>> = Arc::new(Mutex::new([None, None, None, None]));
    pub static ref SENDERS: Arc<Mutex<[Option<mpsc::Sender<String>>; 4]>> = Arc::new(Mutex::new([None, None, None, None]));
    // pub static ref COOKIES: Arc<Mutex<[[Option<String>; 2]; 4]>> = Arc::new(Mutex::new([[None; 2]; 4]));
    pub static ref PASSWORDS: Arc<Mutex<[Option<String>; 4]>> = Arc::new(Mutex::new([None, None, None, None]));
    pub static ref RULES: Arc<Mutex<[Option<u8>; 4]>> = Arc::new(Mutex::new([None, None, None, None]));
    pub static ref ROOMLIST_HTML: String = {
        let mut file = File::open("room_list.html").unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        contents
    };
    pub static ref PLAYING_SENTE_HTML: String = {
        let mut file = File::open("playing_sente.html").unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        contents
    };
    pub static ref PLAYING_GOTE_HTML: String = {
        let mut file = File::open("playing_gote.html").unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        contents
    };
}