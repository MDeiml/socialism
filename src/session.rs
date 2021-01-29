use crate::util::Error;
use rand::RngCore;
use serde::Deserialize;
use std::convert::TryInto;
use std::fmt::Write;

const SESSIONS_TREE: &[u8] = b"sessions";

#[derive(Deserialize)]
pub struct Session {
    pub token: String,
}

impl Session {
    pub fn new(db: &sled::Db, user_id: u64) -> Result<Self, Error> {
        let session_tree = db.open_tree(SESSIONS_TREE)?;
        let mut bytes = [0u8; 16];
        let mut rng = rand::thread_rng();
        rng.fill_bytes(bytes.as_mut());
        let mut token = String::with_capacity(bytes.len() * 2);
        for b in bytes.iter() {
            write!(token, "{:02x}", b).unwrap();
        }
        session_tree.insert(token.as_bytes(), &user_id.to_be_bytes())?;
        Ok(Session { token })
    }

    pub fn get(&self, db: &sled::Db) -> Result<u64, Error> {
        let session_tree = db.open_tree(SESSIONS_TREE)?;
        match session_tree.get(self.token.as_bytes())? {
            Some(user_id) => Ok(u64::from_be_bytes(user_id.as_ref().try_into().unwrap())),
            None => Err(Error::AuthenticationError),
        }
    }

    pub fn delete(&self, db: &sled::Db) -> Result<(), Error> {
        let session_tree = db.open_tree(SESSIONS_TREE)?;
        session_tree.remove(self.token.as_bytes())?;
        Ok(())
    }
}
