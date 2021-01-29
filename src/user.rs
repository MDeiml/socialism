use crate::{block::Block, session::Session, util::Error};
use actix_web::{web, HttpResponse};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::value::RawValue;
use sled::Transactional;
use std::convert::TryInto;

const USERS_PASSWORD_TREE: &[u8] = b"users_password";
pub const USERS_TREE: &[u8] = b"users";
const USERS_USERNAME_TREE: &[u8] = b"users_username";

#[derive(Deserialize)]
pub struct Login {
    username: String,
    #[serde(deserialize_with = "valid_password")]
    password: String,
}

fn valid_password<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let password: String = Deserialize::deserialize(deserializer)?;
    if password.len() >= 4 {
        Ok(password)
    } else {
        Err(serde::de::Error::invalid_length(4, &"4 or more characters"))
    }
}

#[derive(Serialize, Deserialize)]
pub struct User {
    pub username: String,
    pub blocks: Vec<Block>,
}

pub async fn register(
    db: web::Data<sled::Db>,
    login: web::Json<Login>,
) -> Result<HttpResponse, Error> {
    let users_tree = db.open_tree(USERS_TREE)?;
    let users_username_tree = db.open_tree(USERS_USERNAME_TREE)?;
    let users_password_tree = db.open_tree(USERS_PASSWORD_TREE)?;
    let login = login.into_inner();
    let serialized = serde_json::to_vec(&User {
        username: login.username.clone(),
        blocks: Vec::new(),
    })?;
    let password_hash = bcrypt::hash(&login.password, 4)?;
    let result = (&users_tree, &users_username_tree, &users_password_tree).transaction(
        |(users_tree, users_username_tree, users_password_tree)| {
            let user_id = users_tree.generate_id()?;
            if let Some(_) =
                users_username_tree.insert(login.username.as_bytes(), &user_id.to_be_bytes())?
            {
                sled::transaction::abort(())?;
            }
            users_tree.insert(&user_id.to_be_bytes(), serialized.as_slice())?;
            users_password_tree.insert(&user_id.to_be_bytes(), password_hash.as_bytes())?;
            Ok(())
        },
    );
    match result {
        Ok(()) => Ok(HttpResponse::Ok().finish()),
        Err(sled::transaction::TransactionError::Abort(_)) => Ok(HttpResponse::Conflict().finish()),
        Err(sled::transaction::TransactionError::Storage(err)) => Err(Error::SledError(err)),
    }
}

pub async fn login(
    db: web::Data<sled::Db>,
    login: web::Json<Login>,
) -> Result<HttpResponse, Error> {
    let db = db.into_inner();
    let Login { username, password } = login.into_inner();
    let users_password_tree = db.open_tree(USERS_PASSWORD_TREE)?;
    let users_username_tree = db.open_tree(USERS_USERNAME_TREE)?;
    match users_username_tree.get(username.as_bytes())? {
        None => Ok(HttpResponse::Unauthorized().finish()),
        Some(id) => {
            let password_hash = users_password_tree
                .get(&id)?
                .expect("Missing user_id")
                .as_ref()
                .into();
            let password_hash = String::from_utf8(password_hash).unwrap();
            let id = u64::from_be_bytes(id.as_ref().try_into().unwrap());
            if bcrypt::verify(&password, &password_hash)? {
                let session = Session::new(&db, id)?;
                Ok(HttpResponse::Ok().json(session.token))
            } else {
                Ok(HttpResponse::Unauthorized().finish())
            }
        }
    }
}

pub async fn logout(
    db: web::Data<sled::Db>,
    session: web::Query<Session>,
) -> Result<HttpResponse, Error> {
    session.delete(&db.into_inner())?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn get(
    db: web::Data<sled::Db>,
    session: web::Query<Session>,
) -> Result<HttpResponse, Error> {
    let user_id: u64 = session.get(&db)?;
    let users_tree = db.open_tree(USERS_TREE)?;
    let user = users_tree
        .get(user_id.to_be_bytes())?
        .expect("Missing user_id");
    let user: Box<RawValue> = serde_json::from_slice(&user)?;
    Ok(HttpResponse::Ok().json(user))
}

pub async fn add_block(
    db: web::Data<sled::Db>,
    session: web::Query<Session>,
    block: web::Json<Block>,
) -> Result<HttpResponse, Error> {
    let block = block.into_inner();
    let user_id: u64 = session.get(&db)?;
    let users_tree = db.open_tree(USERS_TREE)?;
    let result = users_tree.transaction(|users_tree| {
        let user = users_tree
            .get(user_id.to_be_bytes())?
            .expect("Missing user_id");
        let mut user: User = serde_json::from_slice(&user)
            .map_err(|err| sled::transaction::ConflictableTransactionError::Abort(Some(err)))?;
        if user.blocks.iter().any(|b| block.intersects(b)) {
            sled::transaction::abort(None)
        } else {
            user.blocks.push(block.clone());
            let user = serde_json::to_vec(&user)
                .map_err(|err| sled::transaction::ConflictableTransactionError::Abort(Some(err)))?;
            users_tree.insert(&user_id.to_be_bytes(), user)?;
            Ok(())
        }
    });
    match result {
        Ok(()) => Ok(HttpResponse::Ok().finish()),
        Err(sled::transaction::TransactionError::Storage(err)) => Err(Error::SledError(err)),
        Err(sled::transaction::TransactionError::Abort(abort)) => match abort {
            None => Ok(HttpResponse::Conflict().finish()),
            Some(err) => Err(Error::SerdeError(err)),
        },
    }
}

pub async fn remove_block(
    db: web::Data<sled::Db>,
    session: web::Query<Session>,
    block: web::Json<Block>,
) -> Result<HttpResponse, Error> {
    let block = block.into_inner();
    let user_id: u64 = session.get(&db)?;
    let users_tree = db.open_tree(USERS_TREE)?;
    let result = users_tree.transaction(|users_tree| {
        let user = users_tree
            .get(user_id.to_be_bytes())?
            .expect("Missing user_id");
        let mut user: User = serde_json::from_slice(&user)
            .map_err(|err| sled::transaction::ConflictableTransactionError::Abort(Some(err)))?;
        let index = user
            .blocks
            .iter()
            .position(|b| b == &block)
            .ok_or(sled::transaction::ConflictableTransactionError::Abort(None))?;
        user.blocks.remove(index);
        let user = serde_json::to_vec(&user)
            .map_err(|err| sled::transaction::ConflictableTransactionError::Abort(Some(err)))?;
        users_tree.insert(&user_id.to_be_bytes(), user)?;
        Ok(())
    });
    match result {
        Ok(()) => Ok(HttpResponse::Ok().finish()),
        Err(sled::transaction::TransactionError::Storage(err)) => Err(Error::SledError(err)),
        Err(sled::transaction::TransactionError::Abort(abort)) => match abort {
            None => Ok(HttpResponse::NotFound().finish()),
            Some(err) => Err(Error::SerdeError(err)),
        },
    }
}
