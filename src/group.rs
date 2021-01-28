use crate::{
    user::Session,
    util::{Abort, Error},
};
use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sled::Transactional;
use std::{collections::HashMap, convert::TryInto};

pub const GROUPS_TREE: &[u8] = b"groups";
const GROUPS_USER_TREE: &[u8] = b"groups_user";

#[derive(Serialize, Deserialize)]
pub struct Group {
    pub name: String,
    pub users: HashMap<u64, bool>, // TODO
}

pub async fn create(
    session: web::Query<Session>,
    db: web::Data<sled::Db>,
    name: web::Json<String>,
) -> Result<HttpResponse, Error> {
    let user_id: u64 = session.get(&db)?;
    let groups_tree = db.open_tree(GROUPS_TREE)?;
    let groups_user_tree = db.open_tree(GROUPS_USER_TREE)?;
    let group_id = db.generate_id()?;
    let group = Group {
        name: name.into_inner(),
        users: std::iter::once((user_id, true)).collect(),
    };
    let group = serde_json::to_vec(&group)?;
    (&groups_tree, &groups_user_tree)
        .transaction(|(groups_tree, groups_user_tree)| {
            groups_tree.insert(&group_id.to_be_bytes(), group.as_slice())?;
            let mut key = Vec::with_capacity(16);
            key.extend_from_slice(&user_id.to_be_bytes());
            key.extend_from_slice(&group_id.to_be_bytes());
            groups_user_tree.insert(key, &[])?;
            Ok(())
        })
        .map_err(|err: sled::transaction::TransactionError<()>| match err {
            sled::transaction::TransactionError::Storage(err) => err,
            _ => unreachable!(),
        })?;
    Ok(HttpResponse::Ok().json(group_id))
}

pub async fn list(
    session: web::Query<Session>,
    db: web::Data<sled::Db>,
) -> Result<HttpResponse, Error> {
    let user_id: u64 = session.get(&db)?;
    let groups_tree = db.open_tree(GROUPS_TREE)?;
    let groups_user_tree = db.open_tree(GROUPS_USER_TREE)?;
    let groups = groups_user_tree
        .scan_prefix(user_id.to_be_bytes())
        .map(|res| -> Result<(u64, Box<RawValue>), Error> {
            let (k, _) = res?;
            let group = groups_tree.get(&k[8..16])?.expect("Missing group_id");
            let group_id = u64::from_be_bytes(k[8..16].try_into().unwrap());
            Ok((group_id, serde_json::from_slice(&group)?))
        })
        .collect::<Result<HashMap<_, _>, _>>()?;
    Ok(HttpResponse::Ok().json(groups))
}

#[derive(Deserialize)]
pub struct GroupUserParams {
    group_id: u64,
    user_id: u64,
}

pub async fn add_user(
    session: web::Query<Session>,
    db: web::Data<sled::Db>,
    params: web::Json<GroupUserParams>,
) -> Result<HttpResponse, Error> {
    let user_id: u64 = session.get(&db)?;
    let groups_tree = db.open_tree(GROUPS_TREE)?;
    let groups_user_tree = db.open_tree(GROUPS_USER_TREE)?;

    let result =
        (&groups_tree, &groups_user_tree).transaction(|(groups_tree, groups_user_tree)| {
            match groups_tree.get(&params.group_id.to_be_bytes())? {
                Some(group) => {
                    let mut group: Group = serde_json::from_slice(&group).map_err(|err| {
                        sled::transaction::ConflictableTransactionError::Abort(Abort::SerdeError(
                            err,
                        ))
                    })?;
                    match group.users.get(&user_id) {
                        Some(true) => {
                            group.users.insert(params.user_id, false);
                            let group = serde_json::to_vec(&group).map_err(|err| {
                                sled::transaction::ConflictableTransactionError::Abort(
                                    Abort::SerdeError(err),
                                )
                            })?;
                            groups_tree.insert(&params.group_id.to_be_bytes(), group)?;
                            let mut key = Vec::with_capacity(16);
                            key.extend_from_slice(&params.user_id.to_be_bytes());
                            key.extend_from_slice(&params.group_id.to_be_bytes());
                            groups_user_tree.insert(key, &[])?;
                            Ok(())
                        }
                        Some(false) => sled::transaction::abort(Abort::NotAllowed),
                        None => sled::transaction::abort(Abort::NotFound),
                    }
                }
                None => sled::transaction::abort(Abort::NotFound),
            }
        });
    match result {
        Ok(()) => Ok(HttpResponse::Ok().finish()),
        Err(sled::transaction::TransactionError::Storage(err)) => Err(Error::SledError(err)),
        Err(sled::transaction::TransactionError::Abort(abort)) => match abort {
            Abort::NotFound => Ok(HttpResponse::NotFound().finish()),
            Abort::NotAllowed => Ok(HttpResponse::Forbidden().finish()),
            Abort::SerdeError(err) => Err(Error::SerdeError(err)),
        },
    }
}

pub async fn remove_user(
    session: web::Query<Session>,
    db: web::Data<sled::Db>,
    params: web::Json<GroupUserParams>,
) -> Result<HttpResponse, Error> {
    let user_id: u64 = session.get(&db)?;
    let groups_tree = db.open_tree(GROUPS_TREE)?;
    let groups_user_tree = db.open_tree(GROUPS_USER_TREE)?;

    let result =
        (&groups_tree, &groups_user_tree).transaction(|(groups_tree, groups_user_tree)| {
            match groups_tree.get(&params.group_id.to_be_bytes())? {
                Some(group) => {
                    let mut group: Group = serde_json::from_slice(&group).map_err(|err| {
                        sled::transaction::ConflictableTransactionError::Abort(Abort::SerdeError(
                            err,
                        ))
                    })?;
                    match group.users.get(&user_id) {
                        Some(admin) if *admin || user_id == params.user_id => {
                            match group.users.remove(&params.user_id) {
                                None => sled::transaction::abort(Abort::NotFound),
                                Some(other_admin) if other_admin && user_id != params.user_id => {
                                    sled::transaction::abort(Abort::NotAllowed)
                                }
                                Some(_) => {
                                    let group = serde_json::to_vec(&group).map_err(|err| {
                                        sled::transaction::ConflictableTransactionError::Abort(
                                            Abort::SerdeError(err),
                                        )
                                    })?;
                                    groups_tree.insert(&params.group_id.to_be_bytes(), group)?;
                                    let mut key = Vec::with_capacity(16);
                                    key.extend_from_slice(&params.user_id.to_be_bytes());
                                    key.extend_from_slice(&params.group_id.to_be_bytes());
                                    groups_user_tree.remove(key)?;
                                    Ok(())
                                }
                            }
                        }
                        Some(_) => sled::transaction::abort(Abort::NotAllowed),
                        None => sled::transaction::abort(Abort::NotFound),
                    }
                }
                None => sled::transaction::abort(Abort::NotFound),
            }
        });
    match result {
        Ok(()) => Ok(HttpResponse::Ok().finish()),
        Err(sled::transaction::TransactionError::Storage(err)) => Err(Error::SledError(err)),
        Err(sled::transaction::TransactionError::Abort(abort)) => match abort {
            Abort::NotFound => Ok(HttpResponse::NotFound().finish()),
            Abort::NotAllowed => Ok(HttpResponse::Forbidden().finish()),
            Abort::SerdeError(err) => Err(Error::SerdeError(err)),
        },
    }
}

pub async fn make_admin(
    session: web::Query<Session>,
    db: web::Data<sled::Db>,
    params: web::Json<GroupUserParams>,
) -> Result<HttpResponse, Error> {
    let user_id: u64 = session.get(&db)?;
    let groups_tree = db.open_tree(GROUPS_TREE)?;

    let result = groups_tree.transaction(|groups_tree| {
        match groups_tree.get(&params.group_id.to_be_bytes())? {
            Some(group) => {
                let mut group: Group = serde_json::from_slice(&group).map_err(|err| {
                    sled::transaction::ConflictableTransactionError::Abort(Abort::SerdeError(err))
                })?;
                match group.users.get(&user_id) {
                    Some(true) => match group.users.insert(params.user_id, true) {
                        None => sled::transaction::abort(Abort::NotFound),
                        Some(_) => {
                            let group = serde_json::to_vec(&group).map_err(|err| {
                                sled::transaction::ConflictableTransactionError::Abort(
                                    Abort::SerdeError(err),
                                )
                            })?;
                            groups_tree.insert(&params.group_id.to_be_bytes(), group)?;
                            Ok(())
                        }
                    },
                    Some(false) => sled::transaction::abort(Abort::NotAllowed),
                    None => sled::transaction::abort(Abort::NotFound),
                }
            }
            None => sled::transaction::abort(Abort::NotFound),
        }
    });
    match result {
        Ok(()) => Ok(HttpResponse::Ok().finish()),
        Err(sled::transaction::TransactionError::Storage(err)) => Err(Error::SledError(err)),
        Err(sled::transaction::TransactionError::Abort(abort)) => match abort {
            Abort::NotFound => Ok(HttpResponse::NotFound().finish()),
            Abort::NotAllowed => Ok(HttpResponse::Forbidden().finish()),
            Abort::SerdeError(err) => Err(Error::SerdeError(err)),
        },
    }
}
