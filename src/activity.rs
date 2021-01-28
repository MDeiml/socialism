use std::{collections::HashMap, convert::TryInto};

use crate::{
    group::Group,
    user::Session,
    util::{Abort, Error},
};
use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sled::Transactional;

const ACTIVITIES_TREE: &[u8] = b"activities";
const ACTIVITIES_USER_TREE: &[u8] = b"activities_user";

#[derive(Serialize, Deserialize, Clone)]
pub struct Activity {
    group_id: u64,
    start: u64,
    end: u64,
    description: String,
    min_participants: u32,
    max_participants: u32,
    #[serde(default)]
    accepted: u32,
    #[serde(default)]
    pending: u32,
}

#[derive(Serialize, Deserialize, PartialEq, Eq)]
pub enum Status {
    Pending,
    Accepted,
    Denied,
}

pub async fn create(
    session: web::Query<Session>,
    db: web::Data<sled::Db>,
    activity: web::Json<Activity>,
) -> Result<HttpResponse, Error> {
    let user_id: u64 = session.get(&db)?;
    let activity = activity.into_inner();
    let activities_tree = db.open_tree(ACTIVITIES_TREE)?;
    let activities_user_tree = db.open_tree(ACTIVITIES_USER_TREE)?;
    let groups_tree = db.open_tree(crate::group::GROUPS_TREE)?;
    let result = (&activities_tree, &activities_user_tree, &groups_tree).transaction(
        |(activities_tree, activities_user_tree, groups_tree)| {
            let group = groups_tree.get(activity.group_id.to_be_bytes())?.ok_or(
                sled::transaction::ConflictableTransactionError::Abort(Abort::NotFound),
            )?;
            let group: Group = serde_json::from_slice(&group).map_err(|err| {
                sled::transaction::ConflictableTransactionError::Abort(Abort::SerdeError(err))
            })?;
            if !group.users.contains_key(&user_id) {
                sled::transaction::abort(Abort::NotFound)?;
            }
            let activity_id = activities_tree.generate_id()?;

            let mut activity = activity.clone();
            activity.pending = group.users.len() as u32;
            let mut key = Vec::with_capacity(16);
            for (user_id, _) in group.users {
                key.clear();
                key.extend_from_slice(&user_id.to_be_bytes());
                key.extend_from_slice(&activity_id.to_be_bytes());
                // TODO
                activities_user_tree.insert(
                    key.as_slice(),
                    serde_json::to_vec(&Status::Pending).map_err(|err| {
                        sled::transaction::ConflictableTransactionError::Abort(Abort::SerdeError(
                            err,
                        ))
                    })?,
                )?;
            }

            activities_tree.insert(
                &activity_id.to_be_bytes(),
                serde_json::to_vec(&activity).map_err(|err| {
                    sled::transaction::ConflictableTransactionError::Abort(Abort::SerdeError(err))
                })?,
            )?;
            Ok(activity_id)
        },
    );
    match result {
        Ok(activity_id) => Ok(HttpResponse::Ok().json(activity_id)),
        Err(sled::transaction::TransactionError::Storage(err)) => Err(Error::SledError(err)),
        Err(sled::transaction::TransactionError::Abort(abort)) => match abort {
            Abort::NotFound => Ok(HttpResponse::NotFound().finish()),
            Abort::NotAllowed => Ok(HttpResponse::Forbidden().finish()),
            Abort::SerdeError(err) => Err(Error::SerdeError(err)),
        },
    }
}

#[derive(Serialize)]
pub struct ActivityStats {
    activity: Box<RawValue>,
    status: Status,
}

pub async fn list(
    session: web::Query<Session>,
    db: web::Data<sled::Db>,
) -> Result<HttpResponse, Error> {
    let user_id: u64 = session.get(&db)?;
    let activities_tree = db.open_tree(ACTIVITIES_TREE)?;
    let activities_user_tree = db.open_tree(ACTIVITIES_USER_TREE)?;
    let activities = activities_user_tree
        .scan_prefix(user_id.to_be_bytes())
        .map(|res| -> Result<_, Error> {
            let (k, v) = res?;
            let activity_id = u64::from_be_bytes(k[8..16].try_into().unwrap());
            let status: Status = serde_json::from_slice(&v)?;
            let activity = activities_tree
                .get(activity_id.to_be_bytes())?
                .expect("Missing activity_id");
            let activity = serde_json::from_slice(&activity)?;
            Ok((activity_id, ActivityStats { activity, status }))
        })
        .collect::<Result<HashMap<_, _>, _>>()?;
    Ok(HttpResponse::Ok().json(activities))
}

#[derive(Deserialize)]
pub struct StatusChange {
    activity_id: u64,
    status: Status,
}

pub async fn change_status(
    session: web::Query<Session>,
    db: web::Data<sled::Db>,
    params: web::Json<StatusChange>,
) -> Result<HttpResponse, Error> {
    let user_id: u64 = session.get(&db)?;
    let activities_user_tree = db.open_tree(ACTIVITIES_USER_TREE)?;
    let activities_tree = db.open_tree(ACTIVITIES_TREE)?;
    let mut key = Vec::with_capacity(16);
    let status = serde_json::to_vec(&params.status)?;
    key.extend_from_slice(&user_id.to_be_bytes());
    key.extend_from_slice(&params.activity_id.to_be_bytes());
    let result = (&activities_tree, &activities_user_tree).transaction(
        |(activities_tree, activities_user_tree)| {
            if let Some(old_status) =
                activities_user_tree.insert(key.as_slice(), status.as_slice())?
            {
                let old_status: Status = serde_json::from_slice(&old_status).map_err(|err| {
                    sled::transaction::ConflictableTransactionError::Abort(Abort::SerdeError(err))
                })?;
                if old_status != params.status {
                    let activity = activities_tree
                        .get(params.activity_id.to_be_bytes())?
                        .expect("Missing activity_id");
                    let mut activity: Activity =
                        serde_json::from_slice(&activity).map_err(|err| {
                            sled::transaction::ConflictableTransactionError::Abort(
                                Abort::SerdeError(err),
                            )
                        })?;
                    match old_status {
                        Status::Pending => activity.pending -= 1,
                        Status::Accepted => activity.accepted -= 1,
                        _ => (),
                    }
                    match params.status {
                        Status::Pending => activity.pending += 1,
                        Status::Accepted => activity.accepted += 1,
                        _ => (),
                    }
                    let activity = serde_json::to_vec(&activity).map_err(|err| {
                        sled::transaction::ConflictableTransactionError::Abort(Abort::SerdeError(
                            err,
                        ))
                    })?;
                    activities_tree.insert(&params.activity_id.to_be_bytes(), activity)?;
                }
                Ok(())
            } else {
                sled::transaction::abort(Abort::NotAllowed)
            }
        },
    );
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
