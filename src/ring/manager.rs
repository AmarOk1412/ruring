use ring::api::account::Account;
use ring::api::interaction::Interaction;

use dbus::{Connection, ConnectionItem, BusType, Message};
use dbus::arg::{Array, Dict};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use time;

/**
 * This class is used to interface Ring daemon's to any client using this library.
 * Should be one unique instance of this and is used to access accounts informations
 */
pub struct Manager {
    pub accounts: Vec<Account>,
    pub messages: Vec<(String, Interaction)>,

    ring_dbus: &'static str,
    configuration_path: &'static str,
    configuration_iface: &'static str,
    call_path: &'static str,
    call_iface: &'static str,
}

// TODO connect to account signals to update this manager
impl Manager {
    pub fn new() -> Result<Manager, &'static str> {
        let mut manager = Manager {
            accounts: Vec::new(),
            messages: Vec::new(),

            ring_dbus: "cx.ring.Ring",
            configuration_path: "/cx/ring/Ring/ConfigurationManager",
            configuration_iface: "cx.ring.Ring.ConfigurationManager",
            call_path: "/cx/ring/Ring/CallManager",
            call_iface: "cx.ring.Ring.CallManager",
        };

        manager.get_account_list();

        Ok(manager)
    }

    /**
     * Listen from interresting signals from dbus and call handlers
     * @param self
     */
    pub fn handle_signals(manager: Arc<Mutex<Manager>>) {
        // Use another dbus connection to listen signals.
        let dbus_listener = Connection::get_private(BusType::Session).unwrap();
        dbus_listener.add_match("interface=cx.ring.Ring.ConfigurationManager,member=incomingAccountMessage").unwrap();
        dbus_listener.add_match("interface=cx.ring.Ring.ConfigurationManager,member=incomingTrustRequest").unwrap();
        dbus_listener.add_match("interface=cx.ring.Ring.ConfigurationManager,member=accountsChanged").unwrap();
        dbus_listener.add_match("interface=cx.ring.Ring.ConfigurationManager,member=registrationStateChanged").unwrap();
        // For each signals, call handlers.
        for i in dbus_listener.iter(1) {
            let mut m = manager.lock().unwrap();
            m.handle_accounts_signals(&i);
            m.handle_registration_changed(&i);
            if let Some((account_id, interaction)) = m.handle_interactions(&i) {
                println!("New interaction for {}: {}", account_id, interaction);
                m.messages.push((account_id, interaction));
            };
            if let Some((account_id, from)) = m.handle_requests(&i) {
                println!("New request for {}: {}", account_id, from);
            };
        }
    }

    /**
     * Enable or not a Ring account
     * @param self
     * @param accountId
     * @param enable if need to enable the account
     */
    pub fn enable_account(&self, account_id: &str, enable: bool) {
        let dbus_msg = Message::new_method_call(self.ring_dbus, self.configuration_path, self.configuration_iface,
                                                "sendRegister");
        if !dbus_msg.is_ok() {
            error!("sendRegister call fails. Please verify daemon's API.");
            return;
        }
        let conn = Connection::get_private(BusType::Session);
        if !conn.is_ok() {
            return;
        }
        let dbus = conn.unwrap();
        let _ = dbus.send_with_reply_and_block(
            dbus_msg.unwrap().append2(account_id, enable), 2000);
    }

// Private methos

    /**
     * Get current ring accounts
     * @param self: the manager to modify
     *
     */
    pub fn get_account_list(&mut self) {
        let mut account_list: Vec<Account> = Vec::new();
        let dbus_msg = Message::new_method_call(self.ring_dbus, self.configuration_path,
                                                self.configuration_iface,
                                                "getAccountList");
        if !dbus_msg.is_ok() {
            error!("getAccountList fails. Please verify daemon's API.");
            return;
        }
        let conn = Connection::get_private(BusType::Session);
        if !conn.is_ok() {
            return;
        }
        let dbus = conn.unwrap();
        let response = dbus.send_with_reply_and_block(dbus_msg.unwrap(), 2000).unwrap();
        // getAccountList returns one argument, which is an array of strings.
        let accounts: Array<&str, _>  = match response.get1() {
            Some(array) => array,
            None => return
        };
        for account in accounts {
            account_list.push(self.build_account(account));
        }
        self.accounts = account_list;
    }

    /**
     * Handle new interactions signals
     * @param self
     * @param ci
     * @return (accountId, interaction)
     */
    fn handle_interactions(&self, ci: &ConnectionItem) -> Option<(String, Interaction)> {
        // Check signal
        let msg = if let &ConnectionItem::Signal(ref signal) = ci { signal } else { return None };
        if &*msg.interface().unwrap() != "cx.ring.Ring.ConfigurationManager" { return None };
        if &*msg.member().unwrap() != "incomingAccountMessage" { return None };
        // incomingAccountMessage return three arguments
        let (account_id, author_ring_id, payloads) = msg.get3::<&str, &str, Dict<&str, &str, _>>();
        let author_ring_id = author_ring_id.unwrap().to_string();
        let mut body = String::new();
        for detail in payloads.unwrap() {
            match detail {
                (key, value) => {
                    if key == "text/plain" {
                        body = value.to_string();
                    }
                }
            }
        };
        let interaction = Interaction {
            author_ring_id: author_ring_id,
            body: body,
            time: time::now()
        };
        Some((account_id.unwrap().to_string(), interaction))
    }

    fn handle_registration_changed(&mut self, ci: &ConnectionItem) {
        // Check signal
        let msg = if let &ConnectionItem::Signal(ref signal) = ci { signal } else { return };
        if &*msg.interface().unwrap() != "cx.ring.Ring.ConfigurationManager" { return };
        if &*msg.member().unwrap() != "registrationStateChanged" { return };
        let (account_id, registration_state, _, _) = msg.get4::<&str, &str, u64, &str>();
        for account in self.accounts.iter_mut() {
            if account.id == account_id.unwrap_or("") {
                account.enabled = registration_state.unwrap_or("") == "REGISTERED";
            }
        }
    }

    fn handle_accounts_signals(&mut self, ci: &ConnectionItem) {
        // Check signal
        let msg = if let &ConnectionItem::Signal(ref signal) = ci { signal } else { return };
        if &*msg.interface().unwrap() != "cx.ring.Ring.ConfigurationManager" { return };
        if &*msg.member().unwrap() != "accountsChanged" { return };
        self.get_account_list()
    }

    /**
     * Handle new pending requests signals
     * @param self
     * @param ci
     * @return (accountId, from)
     */
    fn handle_requests(&self, ci: &ConnectionItem) -> Option<(String, String)> {
        // Check signal
        let msg = if let &ConnectionItem::Signal(ref signal) = ci { signal } else { return None };
        if &*msg.interface().unwrap() != "cx.ring.Ring.ConfigurationManager" { return None };
        if &*msg.member().unwrap() != "incomingTrustRequest" { return None };
        // incomingTrustRequest return three arguments
        let (account_id, from, _, _) = msg.get4::<&str, &str, Dict<&str, &str, _>, u64>();
        Some((account_id.unwrap().to_string(), from.unwrap().to_string()))
    }

    /**
     * Build a new account with an id from the daemon
     * @param self
     * @param id the account id to build
     * @return the account retrieven
     */
    fn build_account(&self, id: &str) -> Account {
        let dbus_msg = Message::new_method_call(self.ring_dbus, self.configuration_path,
                                                self.configuration_iface,
                                                "getAccountDetails");
        if !dbus_msg.is_ok() {
            error!("getAccountDetails fails. Please verify daemon's API.");
            return Account::null();
        }
        let conn = Connection::get_private(BusType::Session);
        if !conn.is_ok() {
            return Account::null();
        }
        let dbus = conn.unwrap();
        let response = dbus.send_with_reply_and_block(
                                           dbus_msg.unwrap().append1(id), 2000
                                       ).unwrap();
        let details: Dict<&str, &str, _> = match response.get1() {
            Some(details) => details,
            None => {
                return Account::null();
            }
        };
        let mut alias = "";
        let mut ring_id = "";
        let mut enabled = true;
        for detail in details {
            match detail {
                (key, value) => {
                    if key == "Account.enable" {
                        enabled = value == "true";
                    }
                    if key == "Account.alias" {
                        alias = value;
                    }
                    if key == "Account.username" {
                        ring_id = value;
                    }
                }
            }
        }
        Account {
            id: id.to_owned(),
            ring_id: ring_id.to_string(),
            alias: alias.to_string(),
            enabled: enabled,
        }
    }





    pub fn add_account(&self, main_info: &str, password: &str, from_archive: bool) -> Account {
        let mut details: HashMap<&str, &str> = HashMap::new();
        if from_archive {
            details.insert("Account.archivePath", main_info);
        } else {
            details.insert("Account.alias", main_info);
        }
        details.insert("Account.type", "RING");
        details.insert("Account.archivePassword", password);
        let details = Dict::new(details.iter());
        let dbus_msg = Message::new_method_call(self.ring_dbus, self.configuration_path, self.configuration_iface,
                                                "addAccount");
        if !dbus_msg.is_ok() {
            error!("addAccount fails. Please verify daemon's API.");
            return Account::null();
        }
        let conn = Connection::get_private(BusType::Session);
        if !conn.is_ok() {
            return Account::null();
        }
        let dbus = conn.unwrap();
        let response = dbus.send_with_reply_and_block(dbus_msg.unwrap()
                                                                .append1(details), 2000).unwrap();
        // addAccount returns one argument, which is a string.
        let account_added: &str  = match response.get1() {
            Some(account) => account,
            None => ""
        };
        info!("New account: {:?}", account_added);
        self.build_account(account_added)
    }

    pub fn rm_account(&self, id: &str) {
        let dbus_msg = Message::new_method_call(self.ring_dbus, self.configuration_path, self.configuration_iface,
                                                "removeAccount");
        if !dbus_msg.is_ok() {
            error!("removeAccount fails. Please verify daemon's API.");
            return;
        }
        let conn = Connection::get_private(BusType::Session);
        if !conn.is_ok() {
            return;
        }
        let dbus = conn.unwrap();
        let _ = dbus.send_with_reply_and_block(dbus_msg.unwrap().append1(id), 2000);
        info!("Remove account: {:?}", id);
    }

    pub fn send_interaction(&self, from: &str, destination: &str, body: &str) -> u64 {
        let mut payloads: HashMap<&str, &str> = HashMap::new();
        payloads.insert("text/plain", body);
        let payloads = Dict::new(payloads.iter());

        let dbus_msg = Message::new_method_call(self.ring_dbus, self.configuration_path, self.configuration_iface,
                                                "sendTextMessage");
        if !dbus_msg.is_ok() {
            error!("sendTextMessage fails. Please verify daemon's API.");
            return 0;
        }
        let conn = Connection::get_private(BusType::Session);
        if !conn.is_ok() {
            return 0;
        }
        let dbus = conn.unwrap();
        let response = dbus.send_with_reply_and_block(dbus_msg.unwrap().append3(from, destination, payloads), 2000).unwrap();
        // sendTextMessage returns one argument, which is a u64.
        let interaction_id: u64  = match response.get1() {
            Some(interaction_id) => interaction_id,
            None => 0
        };
        interaction_id
    }


    pub fn send_trust_request(&self, from: &str, destination: &str) {
        // TODO image
        let buf = &[0x00u8];
        let payloads = Array::new(buf.iter());

        let dbus_msg = Message::new_method_call(self.ring_dbus, self.configuration_path, self.configuration_iface,
                                                "sendTrustMessage");
        if !dbus_msg.is_ok() {
            error!("sendTrustMessage fails. Please verify daemon's API.");
            return;
        }
        let conn = Connection::get_private(BusType::Session);
        if !conn.is_ok() {
            return;
        }
        let dbus = conn.unwrap();
        let _ = dbus.send_with_reply_and_block(dbus_msg.unwrap().append3(from, destination, payloads), 2000);
    }

    pub fn add_contact(&self, account_id: &str, contact: &str) {
        let dbus_msg = Message::new_method_call(self.ring_dbus, self.configuration_path, self.configuration_iface,
                                                "addContact");
        if !dbus_msg.is_ok() {
            error!("addContact fails. Please verify daemon's API.");
            return;
        }
        let conn = Connection::get_private(BusType::Session);
        if !conn.is_ok() {
            return;
        }
        let dbus = conn.unwrap();
        let _ = dbus.send_with_reply_and_block(dbus_msg.unwrap().append2(account_id, contact), 2000);
    }

    pub fn rm_contact(&self, account_id: &str, contact: &str, banned: bool) {
        let dbus_msg = Message::new_method_call(self.ring_dbus, self.configuration_path, self.configuration_iface,
                                                "removeContact");
        if !dbus_msg.is_ok() {
            error!("removeContact fails. Please verify daemon's API.");
            return;
        }
        let conn = Connection::get_private(BusType::Session);
        if !conn.is_ok() {
            return;
        }
        let dbus = conn.unwrap();
        let _ = dbus.send_with_reply_and_block(dbus_msg.unwrap().append3(account_id, contact, banned), 2000);
    }

    pub fn get_contacts(&self, account_id: &str) -> Vec<String> {
        let mut contacts: Vec<String> = Vec::new();
        let dbus_msg = Message::new_method_call(self.ring_dbus, self.configuration_path, self.configuration_iface,
                                                "getContacts");
        if !dbus_msg.is_ok() {
            error!("getContacts fails. Please verify daemon's API.");
            return contacts;
        }
        let conn = Connection::get_private(BusType::Session);
        if !conn.is_ok() {
            return Vec::new();
        }
        let dbus = conn.unwrap();
        let response = dbus.send_with_reply_and_block(dbus_msg.unwrap().append1(account_id), 2000).unwrap();
        let contacts_vec: Array<Dict<&str, &str, _>, _> = match response.get1() {
            Some(details) => details,
            None => {
                return contacts;
            }
        };
        for details in contacts_vec {
            for detail in details {
                match detail {
                    (key, value) => {
                        if key == "id" {
                            contacts.push(value.to_string());
                        }
                    }
                }
            }
        }
        contacts
    }

    pub fn get_requests(&self, account_id: &str) -> Vec<String> {
        let mut requests: Vec<String> = Vec::new();
        let dbus_msg = Message::new_method_call(self.ring_dbus, self.configuration_path, self.configuration_iface,
                                                "getTrustRequests");
        if !dbus_msg.is_ok() {
            error!("getTrustRequests fails. Please verify daemon's API.");
            return requests;
        }
        let conn = Connection::get_private(BusType::Session);
        if !conn.is_ok() {
            return Vec::new();
        }
        let dbus = conn.unwrap();
        let response = dbus.send_with_reply_and_block(dbus_msg.unwrap().append1(account_id), 2000).unwrap();
        let requests_vec: Array<Dict<&str, &str, _>, _> = match response.get1() {
            Some(details) => details,
            None => {
                return requests;
            }
        };
        for details in requests_vec {
            for detail in details {
                match detail {
                    (key, value) => {
                        if key == "from" {
                            requests.push(value.to_string());
                        }
                    }
                }
            }
        }
        requests
    }

    pub fn accept_request(&self, account_id: &str, from: &str, accept: bool) -> bool {
        let method = if accept {"acceptTrustRequest"} else {"discardTrustRequest"};
        let dbus_msg = Message::new_method_call(self.ring_dbus, self.configuration_path, self.configuration_iface,
                                                method);
        if !dbus_msg.is_ok() {
            error!("method call fails. Please verify daemon's API.");
            return false;
        }
        let conn = Connection::get_private(BusType::Session);
        if !conn.is_ok() {
            return false;
        }
        let dbus = conn.unwrap();
        let response = dbus.send_with_reply_and_block(
            dbus_msg.unwrap().append3(account_id, from, accept), 2000).unwrap();
        match response.get1() {
            Some(result) => {
                return result;
            },
            None => {
                return false;
            }
        };
    }

    pub fn place_call(&self, account_id: &str, destination: &str) -> String {
        let dbus_msg = Message::new_method_call(self.ring_dbus, self.call_path, self.call_iface,
                                                "placeCall");
        if !dbus_msg.is_ok() {
            error!("placeCall call fails. Please verify daemon's API.");
            return String::new();
        }
        let conn = Connection::get_private(BusType::Session);
        if !conn.is_ok() {
            return String::new();
        }
        let dbus = conn.unwrap();
        let response = dbus.send_with_reply_and_block(
            dbus_msg.unwrap().append2(account_id, format!("ring:{}", destination)), 2000).unwrap();
        match response.get1() {
            Some(result) => {
                return result;
            },
            None => {
                return String::new();
            }
        };
    }

}
