use std::sync::{Arc, Mutex};
use std::sync::mpsc;
use std::fs::File;
pub use std::io::Read;

lazy_static! {
    pub static ref RECEIVERS: Arc<Mutex<[Option<mpsc::Receiver<String>>; 4]>> = Arc::new(Mutex::new([None, None, None, None]));
    pub static ref SENDERS: Arc<Mutex<[Option<mpsc::Sender<String>>; 4]>> = Arc::new(Mutex::new([None, None, None, None]));
    pub static ref PASSWORDS: Arc<Mutex<[Option<String>; 4]>> = Arc::new(Mutex::new([None, None, None, None]));
    pub static ref RULES: Arc<Mutex<[Option<u8>; 4]>> = Arc::new(Mutex::new([None, None, None, None]));
    pub static ref RULES_RESULT: Arc<Mutex<[Option<u8>; 4]>> = Arc::new(Mutex::new([None, None, None, None]));
    pub static ref STARTED:  Arc<Mutex<[bool; 4]>> = Arc::new(Mutex::new([false; 4]));
    pub static ref ROOMLIST_HTML: String = {
        let mut file = File::open("room_list.html").unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        contents
    };
    pub static ref PLAYING_0_HTML: String = {
        let mut file = File::open("playing_0.html").unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        contents
    };
    pub static ref PLAYING_1_HTML: String = {
        let mut file = File::open("playing_1.html").unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        contents
    };
}