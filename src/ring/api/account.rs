use std::fmt;

#[derive(Debug, Clone)]
pub struct Account {
    pub id: String,
    pub ring_id: String,
    pub alias: String,
    pub enabled: bool,
}
// Used for println!
impl fmt::Display for Account {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{}]: {} ({}) - Active: {}", self.id, self.ring_id, self.alias, self.enabled)
    }
}


impl Account {
    pub fn null() -> Account {
        Account {
            id: String::new(),
            ring_id: String::new(),
            alias: String::new(),
            enabled: false,
        }
    }
}
