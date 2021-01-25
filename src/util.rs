use actix_web::HttpResponse;

pub enum Abort {
    NotFound,
    NotAllowed,
    BincodeError(bincode::Error),
}

#[derive(Debug)]
pub enum Error {
    SledError(sled::Error),
    BcryptError(bcrypt::BcryptError),
    ActixError(actix_web::Error),
    BincodeError(bincode::Error),
    AuthenticationError,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &self {
            Self::SledError(e) => e.fmt(f),
            Self::BcryptError(e) => e.fmt(f),
            Self::ActixError(e) => e.fmt(f),
            Self::BincodeError(e) => e.fmt(f),
            Self::AuthenticationError => f.write_str("AuthenticationError"),
        }
    }
}

impl From<sled::Error> for Error {
    fn from(error: sled::Error) -> Self {
        Self::SledError(error)
    }
}

impl From<bcrypt::BcryptError> for Error {
    fn from(error: bcrypt::BcryptError) -> Self {
        Self::BcryptError(error)
    }
}

impl From<actix_web::Error> for Error {
    fn from(error: actix_web::Error) -> Self {
        Self::ActixError(error)
    }
}

impl From<bincode::Error> for Error {
    fn from(error: bincode::Error) -> Self {
        Self::BincodeError(error)
    }
}

impl actix_web::error::ResponseError for Error {
    fn error_response(&self) -> HttpResponse {
        match &self {
            Self::AuthenticationError => HttpResponse::Unauthorized().finish(),
            _ => HttpResponse::InternalServerError().finish(),
        }
    }
}
