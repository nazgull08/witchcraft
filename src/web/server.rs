use std::path::Path;
use std::sync::Arc;
use std::net::Ipv4Addr;

use crate::storage::candles::CandleStore;
use crate::storage::order_book::OrderBook;
use crate::web::routes::{get_docs, get_routes};
use async_graphql::Schema;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::fs::{FileServer, NamedFile};
use rocket::http::Header;
use rocket::{get, routes, Build, Config, Rocket};
use rocket::{Request, Response};
use rocket_okapi::swagger_ui::make_swagger_ui;

use super::graphql::Query;
use super::routes::get_graphql_routes;

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info{
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _: &'r Request<'_>, res: &mut Response<'r>) {
        res.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        res.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "GET, POST, PUT, DELETE, OPTIONS",
        ));
        res.set_header(Header::new(
            "Access-Control-Allow-Headers",
            "Content-Type, Authorization",
        ));
    }
}

#[get("/")]
async fn index() -> Option<NamedFile> {
    NamedFile::open(Path::new("static/index.html")).await.ok()
}

pub fn rocket(port: u16, order_book: Arc<OrderBook>, candle_store: Arc<CandleStore>) -> Rocket<Build> {
    let config = Config {
        address: Ipv4Addr::new(0, 0, 0, 0).into(),
        port,
        ..Config::default()
    };

    let schema = Schema::build(
        Query,
        async_graphql::EmptyMutation,
        async_graphql::EmptySubscription,
    )
    .data(Arc::clone(&candle_store))
    .finish();

    rocket::custom(config)
        .manage(candle_store)
        .manage(schema)
        .mount("/", routes![index]) // Добавляем маршрут для index.html
        .mount("/static", FileServer::from("static")) // Раздаём файлы из папки static
        .mount("/", get_routes())
        .mount("/api", get_graphql_routes())
        .mount("/swagger", make_swagger_ui(&get_docs()))
        .attach(CORS)
}
