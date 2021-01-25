use crate::{user::Session, util::Error};
use actix_web::{web, HttpResponse};
use serde::Deserialize;
use std::convert::TryInto;

// user_id, start -> end
const BLOCKS_TREE: &[u8] = b"blocks";

#[derive(Deserialize)]
pub struct Block {
    start: u64,
    end: u64,
}

pub async fn add(
    session: web::Query<Session>,
    db: web::Data<sled::Db>,
    block: web::Json<Block>,
) -> Result<HttpResponse, Error> {
    let user_id: u64 = session.get(&db)?;
    let blocks_tree = db.open_tree(BLOCKS_TREE)?;
    let mut key = Vec::with_capacity(16);
    key.extend_from_slice(&user_id.to_be_bytes());
    key.extend_from_slice(&block.start.to_be_bytes());
    match blocks_tree.get_lt(&key)? {
        Some((_, before))
            if u64::from_be_bytes(before.as_ref().try_into().unwrap()) > block.start =>
        {
            Ok(HttpResponse::Conflict().finish())
        }
        _ => {
            blocks_tree.insert(&key, &block.end.to_be_bytes())?;
            Ok(HttpResponse::Ok().finish())
        }
    }
}

pub async fn remove(
    session: web::Query<Session>,
    db: web::Data<sled::Db>,
    start: web::Json<u64>,
) -> Result<HttpResponse, Error> {
    let user_id: u64 = session.get(&db)?;
    let blocks_tree = db.open_tree(BLOCKS_TREE)?;
    let mut key = Vec::with_capacity(16);
    key.extend_from_slice(&user_id.to_be_bytes());
    key.extend_from_slice(&start.to_be_bytes());
    match blocks_tree.remove(&key)? {
        Some(_) => Ok(HttpResponse::Ok().finish()),
        None => Ok(HttpResponse::NotFound().finish()),
    }
}

pub async fn list(
    session: web::Query<Session>,
    db: web::Data<sled::Db>,
) -> Result<HttpResponse, Error> {
    let user_id: u64 = session.get(&db)?;
    let blocks_tree = db.open_tree(BLOCKS_TREE)?;
    let blocks = blocks_tree
        .scan_prefix(user_id.to_be_bytes())
        .map(|res| -> sled::Result<(u64, u64)> {
            let (k, v) = res?;
            let start = u64::from_be_bytes(k[8..16].try_into().unwrap());
            let end = u64::from_be_bytes(v.as_ref().try_into().unwrap());
            Ok((start, end))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(HttpResponse::Ok().json(blocks))
}
