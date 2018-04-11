use ncurses::*;
use ring::manager::Manager;
use ring::api::interaction::Interaction;
use std;
use std::sync::{Arc, Mutex};

static COLOR_BACKGROUND: i16 = 16;
static COLOR_KEYWORD: i16 = 18;
static COLOR_SELECTED: i16 = 2;

pub struct UserInterface {
    max_x: i32,
    max_y: i32,
    current_mode: String,
    current_account: String,
    current_contact: String,
}

impl UserInterface {

    pub fn new() -> UserInterface {
        UserInterface {
            max_x: 0,
            max_y: 0,
            current_mode: String::from("accounts"),
            current_account: String::new(),
            current_contact: String::new()
        }
    }

    pub fn draw(&mut self, manager: Arc<Mutex<Manager>>) {
        let locale_conf = LcCategory::all;
        setlocale(locale_conf, "");
        // Setup ncurses.
        initscr();
        raw();
        // Allow for extended keyboard (like F1)
        keypad(stdscr(), true);
        noecho();
        // Invisible cursor.
        curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);
        self.init_colors();

        let mut max_x = 0;
        let mut max_y = 0;
        getmaxyx(stdscr(), &mut max_y, &mut max_x);
        self.max_x = max_x;
        self.max_y = max_y;


        let mut exit = false;
        while !exit {
            refresh();

            if self.current_mode == "add_account" {
                self.draw_add_account_popup(manager.clone(), false);
            } else if self.current_mode == "import_account" {
                self.draw_add_account_popup(manager.clone(), true);
            } else if self.current_mode == "add_contact" {
                self.draw_contacts_popup(manager.clone(), true);
            } else if self.current_mode == "send_interaction" {
                self.draw_contacts_popup(manager.clone(), false);
            } else {
                self.draw_borders();
                let win = newwin(self.max_y, 1, 0, self.max_x/3);
                box_(win, 0, 0);
                wrefresh(win);
                if self.current_mode != "accounts" {
                    let win = newwin(self.max_y, 1, 0, 2*self.max_x/3);
                    box_(win, 0, 0);
                    wrefresh(win);
                }
                self.draw_accounts(manager.clone());
                self.draw_contacts(manager.clone());
                self.draw_interactions(manager.clone());
                self.draw_menu(manager.clone());

                timeout(1000);
                let key = getch(); // TODO make into thread and check when new value
                if self.current_mode == "accounts" {
                    if key == ' ' as i32 {
                        // enable account
                        let accounts = manager.lock().unwrap().accounts.clone();
                        for account in accounts {
                            if self.current_account == account.id {
                                manager.lock().unwrap().enable_account(&*self.current_account, !account.enabled);
                            }
                        }
                    } else if key == 258 /* BOTTOM KEY */ ||  key == 259 /* UP KEY */ {
                        // Select next account
                        let mut select = false;
                        let mut accounts = manager.lock().unwrap().accounts.clone();
                        if key == 259 {
                            accounts.reverse();
                        }
                        for account in accounts {
                            if select {
                                self.current_account = account.id;
                                break;
                            } else if self.current_account == account.id {
                                select = true;
                            }
                        }
                    } else if key == 10 /* ENTER */ {
                        self.current_mode = String::from("contacts");
                    } else if key == 27 /* ESC */ {
                        exit = true;
                    } else if key == 97 /* A */ {
                        self.current_mode = String::from("add_account");
                    } else if key == 105 /* I */ {
                        self.current_mode = String::from("import_account");
                    } else if key == 114 /* R */ {
                        // remove account
                        manager.lock().unwrap().rm_account(&*self.current_account);
                        self.current_account = String::new();
                    }
                } else if self.current_mode == "contacts" {
                    let requests = manager.lock().unwrap().get_requests(&*self.current_account);
                    if key == 27 /* ESC */ {
                        self.current_contact = String::new();
                        self.current_mode = String::from("accounts");
                    } else if key == 258 /* BOTTOM KEY */ ||  key == 259 /* UP KEY */ {
                        // Select next account
                        let mut select = false;
                        let mut contacts = manager.lock().unwrap().get_contacts(&*self.current_account);
                        let mut requests = manager.lock().unwrap().get_requests(&*self.current_account);
                        if key == 259 {
                            contacts.reverse();
                            requests.reverse();
                        }
                        let all_contacts = if key == 259 {
                            let mut all_contacts = Vec::new();
                            all_contacts.append(&mut contacts);
                            all_contacts.append(&mut requests);
                            all_contacts
                        } else {
                            let mut all_contacts = Vec::new();
                            all_contacts.append(&mut requests);
                            all_contacts.append(&mut contacts);
                            all_contacts
                        };
                        for contact in all_contacts {
                            if select {
                                self.current_contact = contact;
                                break;
                            } else if self.current_contact == contact {
                                select = true;
                            }
                        }
                    } else if key ==  114 /* R */ {
                        if let Some(_) = requests.iter().position(|r| &*r == &*self.current_contact) {
                            manager.lock().unwrap().accept_request(&*self.current_account, &*self.current_contact, false);
                        } else {
                            manager.lock().unwrap().rm_contact(&*self.current_account, &*self.current_contact, false);
                        }
                        self.current_contact = String::new();
                    } else if key ==  98 /* B */ {
                        manager.lock().unwrap().rm_contact(&*self.current_account, &*self.current_contact, true);
                        self.current_contact = String::new();
                    } else if key == 97 /* A */ {
                        if let Some(_) = requests.iter().position(|r| &*r == &*self.current_contact) {
                            manager.lock().unwrap().accept_request(&*self.current_account, &*self.current_contact, true);
                        } else {
                            self.current_mode = String::from("add_contact");
                        }
                    } else if key == 10 /* Enter */ {
                        self.current_mode = String::from("send_interaction");
                    } else if key == 99 /* C */ {
                        manager.lock().unwrap().place_call(&*self.current_account, &*self.current_contact);
                    }
                }
            }
        }

        endwin();
    }

    fn init_colors(&mut self) {
        start_color();
        init_color(COLOR_BLACK, 0, 0, 0);
        init_color(COLOR_WHITE, 255 * 4, 255 * 4, 255 * 4);
        init_pair(COLOR_SELECTED, COLOR_BLACK, COLOR_WHITE);
    }

    fn draw_borders(&mut self) -> WINDOW {
        let win = newwin(self.max_y, self.max_x, 0, 0);
        box_(win, 0, 0);
        wrefresh(win);
        mvprintw(LINES() - 2, 1, "ruring v1.0.0");
        win
    }

    fn draw_menu(&mut self, manager: Arc<Mutex<Manager>>) {
        let attr = COLOR_PAIR(COLOR_SELECTED);
        let mut menu_str = String::new();
        if self.current_mode == "accounts" {
            menu_str = String::from("ESC: quit | A: Add | R: Remove | SPACE: Enable | I: Import | Enter: Select");
        } else if self.current_mode == "contacts" {
            let requests = manager.lock().unwrap().get_requests(&*self.current_account);
            if let Some(_) = requests.iter().position(|r| &*r == &*self.current_contact) {
                menu_str = String::from("ESC: return | A: Accept | R: Discard");
            } else {
                menu_str = String::from("ESC: return | A: Add | R: Remove | W: Send message");
            }
        }
        while menu_str.len() < self.max_x as usize {
            menu_str += " ";
        }
        attron(attr);
        mvprintw(0, 0, &*menu_str);
        attroff(attr);
    }

    fn draw_accounts(&mut self, manager: Arc<Mutex<Manager>>) {
        let mut row = 3;
        attron(A_BOLD());
        mvprintw(row, 2, "RORI Accounts:");
        attroff(A_BOLD());
        row += 2;
        for account in manager.lock().unwrap().accounts.clone() {
            let mut account_str = String::new();
            if account.enabled {
                account_str += "[x] ";
            } else {
                account_str += "[ ] ";
            }
            let account_identity = format!("{} ({})", account.alias, account.ring_id);
            account_str += &*account_identity;
            let mut set_focus = false;
            if self.current_mode == "accounts" {
                if self.current_account.len() == 0 {
                    self.current_account = account.id;
                    set_focus = true;
                } else if self.current_account == account.id {
                    set_focus = true;
                }
            }
            let attr = COLOR_PAIR(COLOR_SELECTED);
            if set_focus {
                attron(attr);
            }
            mvprintw(row, 2, &*account_str);
            if set_focus {
                attroff(attr);
            }
            row += 1;
        }
    }

    fn draw_contacts(&mut self, manager: Arc<Mutex<Manager>>) {
        if self.current_mode == "contacts" {
            let mut row = 3;
            // Current requests
            let requests = manager.lock().unwrap().get_requests(&*self.current_account);
            if requests.len() != 0 {
                attron(A_BOLD());
                mvprintw(row, self.max_x/3 + 4, "Requests:");
                attroff(A_BOLD());
                row += 2;
                for contact in requests {
                    let mut set_focus = false;
                    if self.current_mode == "contacts" {
                        if self.current_contact.len() == 0 {
                            self.current_contact = contact.clone();
                            set_focus = true;
                        } else if self.current_contact == contact {
                            set_focus = true;
                        }
                    }
                    let attr = COLOR_PAIR(COLOR_SELECTED);
                    if set_focus {
                        attron(attr);
                    }
                    mvprintw(row, self.max_x/3 + 4, &*contact);
                    if set_focus {
                        attroff(attr);
                    }
                    row += 1;
                }
                row += 2;
            }
            // Current contacts
            attron(A_BOLD());
            mvprintw(row, self.max_x/3 + 4, "Contacts:");
            attroff(A_BOLD());
            row += 2;
            for contact in manager.lock().unwrap().get_contacts(&*self.current_account) {
                let mut set_focus = false;
                if self.current_mode == "contacts" {
                    if self.current_contact.len() == 0 {
                        self.current_contact = contact.clone();
                        set_focus = true;
                    } else if self.current_contact == contact {
                        set_focus = true;
                    }
                }
                let attr = COLOR_PAIR(COLOR_SELECTED);
                if set_focus {
                    attron(attr);
                }
                mvprintw(row, self.max_x/3 + 4, &*contact);
                if set_focus {
                    attroff(attr);
                }
                row += 1;
            }
        }
    }

    fn draw_add_account_popup(&mut self, manager: Arc<Mutex<Manager>>, import: bool) {
        let (start_x, start_y) = (self.max_x/4, self.max_y/2 - 8);

        let mut username = String::new();
        let mut password = String::new();
        let mut exit = false;
        let mut focus = "username";

        while !exit {
            let win = newwin(16, self.max_x/2, start_y, start_x);
            box_(win, 0, 0);

            let title = "Add new RING account";
            mvprintw(start_y + 2, self.max_x/2 - title.len() as i32/2, title);

            let first_info = if import {"Path:"} else {"Username:"};
            let second_info = "Password:";
            let start_label = start_x + 2;
            let label_size = std::cmp::max(first_info.len(), second_info.len()) as i32;
            let start_edit_view = start_x + label_size + 6;

            mvprintw(start_y + 4, start_label,first_info);
            let width = self.max_x/2 - label_size - 12;
            let attr = COLOR_PAIR(COLOR_SELECTED);
            attron(attr);
            let mut username_entry = username.clone();
            for _ in 0..(width - username.len() as i32) {
                username_entry += " ";
            }
            mvprintw(start_y + 4, start_edit_view, &*username_entry);
            attroff(attr);


            mvprintw(start_y + 8, start_label, "Password:");
            attron(attr);
            let mut password_entry = String::new();
            for _ in 0..(password.len() as i32) {
                password_entry += "*";
            }
            for _ in 0..(width - password.len() as i32) {
                password_entry += " ";
            }
            mvprintw(start_y + 8, start_edit_view, &*password_entry);
            attroff(attr);
            wrefresh(win);

            if focus == "ok_btn" {
                attron(attr);
            }
            mvprintw(start_y + 12, self.max_x/2 - 6 - "< OK >".len() as i32, "< OK >");
            if focus == "ok_btn" {
                attroff(attr);
            }
            if focus == "cancel_btn" {
                attron(attr);
            }
            mvprintw(start_y + 12, self.max_x/2 + 6, "< Cancel >");
            if focus == "cancel_btn" {
                attroff(attr);
            }

            let key = getch();
            if key == -1 /* ERR */ {}
            else if key == 27 /* ESC */ {
                self.current_mode = String::from("accounts");
                exit = true;
            } else if key == 9 /* TAB */ {
                focus = match focus {
                    "username" => "password",
                    "password" => "ok_btn",
                    "ok_btn" => "cancel_btn",
                    "cancel_btn" => "username",
                    _ => {
                        exit = true;
                        ""
                    }
                }
            } else if key == 10 /* ENTER */ {
                match focus {
                    "ok_btn" => {
                        manager.lock().unwrap().add_account(&*username, &*password, import);
                        self.current_mode = String::from("accounts");
                        exit = true;
                    },
                    "cancel_btn" => {
                        self.current_mode = String::from("accounts");
                        exit = true;
                    },
                    _ => { }
                }
            } else if key == 263 /* BACKSPACE */ {
                match focus {
                    "username" => {
                        username.pop();
                    },
                    "password" => {
                        password.pop();
                    },
                    _ => { }
                }
            } else {
                match focus {
                    "username" => {
                        username += &*std::char::from_u32(key as u32).unwrap_or(' ').to_string();
                    },
                    "password" => {
                        password += &*std::char::from_u32(key as u32).unwrap_or(' ').to_string();
                    },
                    "ok_btn" => {},
                    "cancel_btn" => {},
                    _ => {
                        exit = true;
                    }
                }
            }
        }
    }

    fn draw_contacts_popup(&mut self, manager: Arc<Mutex<Manager>>, add: bool) {
        let (start_x, start_y) = (self.max_x/4, self.max_y/2 - 5);

        let mut entry = String::new();
        let mut exit = false;
        let mut focus = "entry";

        while !exit {
            let win = newwin(10, self.max_x/2, start_y, start_x);
            box_(win, 0, 0);

            let title = if add {"Add new contact"} else {"Send message"};
            mvprintw(start_y + 2, self.max_x/2 - title.len() as i32/2, title);

            let first_info = if add {"Id:"} else {"Message:"};
            let start_label = start_x + 2;
            let label_size = first_info.len() as i32;
            let start_edit_view = start_x + label_size + 6;

            mvprintw(start_y + 4, start_label,first_info);
            let width = self.max_x/2 - label_size - 12;
            let attr = COLOR_PAIR(COLOR_SELECTED);
            attron(attr);
            let mut info_entry = entry.clone();
            for _ in 0..(width - entry.len() as i32) {
                info_entry += " ";
            }
            mvprintw(start_y + 4, start_edit_view, &*info_entry);
            attroff(attr);

            wrefresh(win);

            if focus == "ok_btn" {
                attron(attr);
            }
            mvprintw(start_y + 7, self.max_x/2 - 6 - "< OK >".len() as i32, "< OK >");
            if focus == "ok_btn" {
                attroff(attr);
            }
            if focus == "cancel_btn" {
                attron(attr);
            }
            mvprintw(start_y + 7, self.max_x/2 + 6, "< Cancel >");
            if focus == "cancel_btn" {
                attroff(attr);
            }


            let key = getch();
            if key == -1 /* ERR */ {}
            else if key == 27 /* ESC */ {
                self.current_mode = String::from("contacts");
                exit = true;
            } else if key == 9 /* TAB */ {
                focus = match focus {
                    "entry" => "ok_btn",
                    "ok_btn" => "cancel_btn",
                    "cancel_btn" => "entry",
                    _ => {
                        exit = true;
                        ""
                    }
                }
            } else if key == 10 /* ENTER */ {
                match focus {
                    "ok_btn" => {
                        if add {
                            manager.lock().unwrap().add_contact(&*self.current_account, &*entry);
                        } else {
                            manager.lock().unwrap().send_interaction(&*self.current_account, &*self.current_contact, &*entry);
                        }
                        self.current_mode = String::from("contacts");
                        exit = true;
                    },
                    "cancel_btn" => {
                        self.current_mode = String::from("contacts");
                        exit = true;
                    },
                    _ => { }
                }
            } else if key == 263 /* BACKSPACE */ {
                match focus {
                    "entry" => {
                        entry.pop();
                    },
                    _ => { }
                }
            } else {
                match focus {
                    "entry" => {
                        entry += &*std::char::from_u32(key as u32).unwrap_or(' ').to_string();
                    },
                    "ok_btn" => {},
                    "cancel_btn" => {},
                    _ => {
                        exit = true;
                    }
                }
            }
        }
    }

    fn draw_interactions(&mut self, manager: Arc<Mutex<Manager>>) {
        if self.current_mode == "contacts" {
            let mut row = 3;
            // Linked interactions
            let interactions = manager.lock().unwrap().messages.clone();
            let mut interactions: Vec<(String, Interaction)> = interactions.into_iter()
               .filter(|tup| tup.0 == self.current_account && tup.1.author_ring_id == self.current_contact)
               .collect();
            interactions.reverse();

            if interactions.len() != 0 {
                for (_, interaction) in interactions {
                    if row == self.max_y {
                        return;
                    }
                    let interaction_str = format!("{}: {}", interaction.time.rfc3339(), interaction.body);
                    mvprintw(row, 2*self.max_x/3 + 4, &*interaction_str);
                    row += 1;
                }
            }
        }
    }
}
