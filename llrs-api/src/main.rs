use clap::{App, Arg, ArgMatches};
use libllrs::{Error as WaifusimsError, MangaService, Waifusims};
use log::*;
use nameof::name_of;
use std::net::SocketAddr;
use tiberius::{AuthMethod, Config};
use warp::Filter;

#[derive(Debug)]
struct ServerConfig {
    pub addr: SocketAddr,
    pub sql_config: SqlConfig,
}

#[derive(Debug)]
struct SqlConfig {
    pub sql_user: String,
    pub sql_pass: String,
    pub sql_domain: String,
    pub sql_database: String,
}

impl<'a> From<ArgMatches<'a>> for ServerConfig {
    fn from(arg_matches: ArgMatches<'a>) -> Self {
        let addr = arg_matches
            .value_of(name_of!(addr in ServerConfig))
            .expect("should have defaulted if not provided");
        info!("{:?}", addr);
        let addr: SocketAddr = addr
            .to_owned()
            .parse()
            .expect("must be a valid socket addr. eg: 127.0.0.1:8080");
        let sql_user = arg_matches
            .value_of(name_of!(sql_user in SqlConfig))
            .expect("required")
            .to_owned();
        let sql_pass = arg_matches
            .value_of(name_of!(sql_pass in SqlConfig))
            .expect("required")
            .to_owned();
        let sql_domain = arg_matches
            .value_of(name_of!(sql_domain in SqlConfig))
            .expect("required")
            .to_owned();
        let sql_database = arg_matches
            .value_of(name_of!(sql_database in SqlConfig))
            .expect("required")
            .to_owned();

        ServerConfig {
            addr,
            sql_config: SqlConfig {
                sql_user,
                sql_pass,
                sql_domain,
                sql_database,
            },
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let arg_matches = App::new("Waifusims API")
        .version("0.1.0")
        .author("James N. <james@niis.me>")
        .about("llrs api client using warp")
        .arg(
            Arg::with_name(name_of!(addr in ServerConfig))
                .short("a")
                .long("address")
                .value_name("IP_ADDRESS:PORT")
                .help("ip address to bind to")
                .takes_value(true)
                .default_value("127.0.0.1:42069"),
        )
        .arg(
            Arg::with_name(name_of!(sql_user in SqlConfig))
                .short("u")
                .long("username")
                .value_name("SQL_USERNAME")
                .help("username for sql password auth")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(name_of!(sql_pass in SqlConfig))
                .short("p")
                .long("password")
                .value_name("SQL_USER_PASSWORD")
                .help("password for sql password auth")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(name_of!(sql_domain in SqlConfig))
                .short("d")
                .long("domain")
                .value_name("SQL_SRV_ADDR")
                .help("address of sql server")
                .takes_value(true)
                .required(true),
        )
        .arg(
            Arg::with_name(name_of!(sql_database in SqlConfig))
                .short("n")
                .long("database-node")
                .value_name("SQL_SRV_DATABASE")
                .help("DATABASE DATABASE")
                .takes_value(true)
                .required(true),
        )
        .get_matches();
    let config = ServerConfig::from(arg_matches);

    let mut db_config = Config::new();
    // TODO: Get creds from arguments
    db_config.host(config.sql_config.sql_domain);
    let auth = AuthMethod::sql_server(config.sql_config.sql_user, config.sql_config.sql_pass);
    db_config.authentication(auth);
    db_config.trust_cert();
    db_config.database(config.sql_config.sql_database);

    // TODO: Connection pooling with deadpool? or just Arc<Waifuims>
    let config_copy = db_config.clone();
    let list_manga = warp::path::end().and_then(move || {
        let db_config = config_copy.clone();
        async move {
            let mut llrs = Waifusims::new(db_config.clone()).await.expect("ok");
            match llrs.get_all_manga_titles().await {
                Ok(mangas) => Ok::<warp::reply::Json, warp::Rejection>(warp::reply::json(&mangas)),
                Err(err) => Err(Error::from(err).into()),
            }
        }
    });

    // TODO: return message for id? < 0
    let config_copy = db_config.clone();
    let list_chapters = warp::path!("manga" / i32).and_then(move |manga_id| {
        let db_config = config_copy.clone();
        async move {
            let mut llrs = Waifusims::new(db_config.clone()).await.expect("ok");
            match llrs.get_manga_chapters(manga_id).await {
                Ok(mangas) => Ok::<warp::reply::Json, warp::Rejection>(warp::reply::json(&mangas)),
                Err(err) => Err(Error::from(err).into()),
            }
        }
    });

    // TODO: return message for id? < 0
    let config_copy = db_config.clone();
    let list_pages =
        warp::path!("manga" / i32 / String).and_then(move |manga_id, chapter_number: String| {
            let db_config = config_copy.clone();
            async move {
                let mut llrs = Waifusims::new(db_config.clone()).await.expect("ok");
                match llrs.get_pages(manga_id, &chapter_number).await {
                    Ok(mangas) => {
                        Ok::<warp::reply::Json, warp::Rejection>(warp::reply::json(&mangas))
                    }
                    Err(err) => Err(Error::from(err).into()),
                }
            }
        });

    let routes = list_manga
        .or(list_chapters)
        .or(list_pages)
        .with(warp::cors().allow_any_origin());

    warp::serve(routes).run(config.addr).await;
}

#[derive(Debug)]
struct Error {
    inner: WaifusimsError,
}

impl From<WaifusimsError> for Error {
    fn from(error: WaifusimsError) -> Error {
        Error { inner: error }
    }
}

impl warp::reject::Reject for Error {}
