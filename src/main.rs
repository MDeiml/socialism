mod activity;
mod block;
mod group;
mod session;
mod user;
mod util;

use actix_web::{middleware, web, App, HttpServer};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    let db = sled::open("./database").unwrap();
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .service(
                web::scope("/")
                    .route("/user", web::post().to(user::register))
                    .route("/user", web::get().to(user::get))
                    .route("/session", web::post().to(user::login))
                    .route("/session", web::delete().to(user::logout))
                    .route("/block", web::post().to(user::add_block))
                    .route("/block", web::delete().to(user::remove_block))
                    .route("/group", web::post().to(group::create))
                    .route("/group", web::get().to(group::list))
                    .route("/group/user", web::post().to(group::add_user))
                    .route("/group/user", web::delete().to(group::remove_user))
                    .route("/group/admin", web::post().to(group::make_admin))
                    .route("/activity", web::post().to(activity::create))
                    .route("/activity", web::get().to(activity::list))
                    .route("/activity/status", web::post().to(activity::change_status)),
            )
            .data(db.clone())
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
