extern crate dbus;
#[macro_use] extern crate log;
extern crate env_logger;
extern crate ncurses;
extern crate time;

mod ring;
mod userinterface;

use ring::manager::Manager;
use userinterface::UserInterface;
use std::sync::{Arc, Mutex};
use std::thread;


fn main() {
    env_logger::init();

    let shared_manager : Arc<Mutex<Manager>> = Arc::new(Mutex::new(Manager::new().ok().expect("Can't initialize ConfigurationManager")));
    let shared_manager_cloned = shared_manager.clone();
    let test = thread::spawn(move || {
        let mut ui = UserInterface::new();
        ui.draw(shared_manager_cloned);
    });
    Manager::handle_signals(shared_manager);
    let _ = test.join();
    // TODO proper quit
}

// TODO NAME SERVER
// TODO LINK TO RORI
