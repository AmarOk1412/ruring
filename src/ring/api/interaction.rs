use std::fmt;
use time::Tm;

#[derive(Clone)]
pub struct Interaction
{
    pub author_ring_id: String,
    pub body: String,
    pub time: Tm
}
// Used for println!
impl fmt::Display for Interaction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.author_ring_id, self.body)
    }
}
