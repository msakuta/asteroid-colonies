mod server;
mod session;
mod websocket;

use crate::{
    // api::set_timescale::set_timescale,
    server::ChatServer,
    // websocket::{websocket_index, NotifyBodyState, SetRocketStateWs},
};
use ::actix::prelude::*;
use ::actix_cors::Cors;
use ::actix_files::NamedFile;
use ::actix_web::{middleware, web, App, HttpRequest, HttpServer};
use ::asteroid_colonies_logic::AsteroidColoniesGame;
use ::clap::Parser;
use actix_web::HttpResponse;
use session::SessionId;
use std::{
    collections::HashSet,
    fs,
    io::BufReader,
    path::{Path, PathBuf},
    sync::{Mutex, RwLock},
    time::Instant,
};
use websocket::{websocket_index, NotifyState, NotifyStateEnum, SetStateBinWs, SetStateWs};

type Game = AsteroidColoniesGame;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    #[clap(default_value = ".")]
    path: String,
    #[clap(
        short,
        long,
        default_value = "3883",
        help = "The port number to listen to."
    )]
    port: u16,
    #[clap(
        short = 'H',
        long,
        default_value = "127.0.0.1",
        help = "The host address to listen to. By default, only the localhost can access."
    )]
    host: String,
    #[clap(short, long, default_value = "../dist")]
    asset_path: PathBuf,
    #[clap(long, default_value = "save.json")]
    autosave_file: PathBuf,
    #[clap(long, default_value = "5")]
    autosave_period_s: f64,
    #[clap(long)]
    autosave_pretty: bool,
    #[clap(long, default_value = "10")]
    push_period_s: f64,
    #[clap(long, default_value = "0.1", help = "Tick frequency in Hz")]
    tick_freq: f64,
    #[cfg(not(debug_assertions))]
    #[clap(
        long,
        default_value = "/usr/share/asteroid-colonies/js",
        help = "JavaScript and Wasm path"
    )]
    js_path: PathBuf,
    #[cfg(debug_assertions)]
    #[clap(long, default_value = ".", help = "JavaScript and Wasm path")]
    js_path: PathBuf,
}

struct ServerData {
    game: RwLock<AsteroidColoniesGame>,
    // asset_path: PathBuf,
    js_path: PathBuf,
    last_saved: Mutex<Instant>,
    last_pushed: Mutex<Instant>,
    autosave_file: PathBuf,
    srv: Addr<ChatServer>,
    sessions: RwLock<HashSet<SessionId>>,
}

impl ServerData {
    fn new_session(&self) -> SessionId {
        let session = SessionId::new();
        let mut sessions = self.sessions.write().unwrap();
        sessions.insert(session);
        session
    }
}

async fn new_session(data: web::Data<ServerData>) -> actix_web::Result<HttpResponse> {
    // let mut game = data.sessions;//.write().unwrap();

    let new_session = data.new_session();

    // if let Some(body) = game.get(id) {
    //     data.srv.do_send(NotifyNewBody {
    //         session_id: new_session,
    //         body: serde_json::to_value(&body)
    //             .map_err(|e| error::ErrorInternalServerError(e.to_string()))?,
    //         body_parent: body
    //             .parent
    //             .and_then(|parent| universe.get(parent))
    //             .map(|parent| parent.name.clone())
    //             .unwrap_or_else(|| "".to_string()),
    //     });
    // }

    println!("New session id: {:?}", new_session);

    Ok(HttpResponse::Ok().body(new_session.to_string()))
}

async fn get_state(data: web::Data<ServerData>) -> actix_web::Result<HttpResponse> {
    let start = Instant::now();

    let game = data.game.read().unwrap();

    let serialized = serialize_state(&game, false)?;

    println!(
        "Serialized game at tick {} in {:.3}ms",
        game.get_global_time(),
        start.elapsed().as_micros() as f64 * 1e-3
    );

    Ok(HttpResponse::Ok()
        .content_type("application/json")
        .body(serialized))
}

#[cfg(not(debug_assertions))]
async fn get_main_js() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/javascript")
        .body(include_str!("../../dist/js/main.js"))
}

#[cfg(not(debug_assertions))]
async fn get_index() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(include_str!("../../dist/index.html"))
}

async fn get_js_file(
    data: web::Data<ServerData>,
    req: HttpRequest,
) -> actix_web::Result<NamedFile> {
    let js_path = &data.js_path;
    let filename: PathBuf = req.match_info().query("filename").parse().unwrap();
    let path: PathBuf = js_path.join(&filename);
    println!("Requesting {path:?} -> {:?}", path.canonicalize());
    Ok(NamedFile::open(path)?)
}

// async fn get_asset_file(data: web::Data<ServerData>, req: HttpRequest) -> actix_web::Result<NamedFile> {
//     let asset_path = &data.asset_path;
//     let filename: PathBuf = req.match_info().query("filename").parse().unwrap();
//     let path: PathBuf = asset_path.join(&filename);
//     Ok(NamedFile::open(path)?)
// }

fn serialize_state(game: &Game, autosave_pretty: bool) -> serde_json::Result<String> {
    // if autosave_pretty {
    //     serde_json::to_string_pretty(game)
    // } else {
    // serde_json::to_string(game.serialize())
    // }
    game.serialize(autosave_pretty)
}

fn save_file(autosave_file: &Path, serialized: &str) {
    println!(
        "[{:?}] Writing {}",
        std::thread::current().id(),
        serialized.len()
    );
    let start = Instant::now();
    fs::write(autosave_file, serialized.as_bytes()).expect("Write to save file should succeed");
    println!(
        "Wrote in {:.3}ms",
        start.elapsed().as_micros() as f64 * 1e-3
    );
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let mut game =
        Game::new(None).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let start = Instant::now();
    if let Ok(data) = fs::File::open(&args.autosave_file).map(BufReader::new) {
        if let Err(e) = game.deserialize(data) {
            eprintln!("Error on loading serialized data: {}", e);
        } else {
            eprintln!(
                "Deserialized data {} object in {}ms",
                game.iter_cell().count(),
                start.elapsed().as_micros() as f64 * 1e-3
            );
        }
    }

    let data = web::Data::new(ServerData {
        game: RwLock::new(game),
        // asset_path: args.asset_path,
        js_path: args.js_path,
        last_saved: Mutex::new(Instant::now()),
        last_pushed: Mutex::new(Instant::now()),
        autosave_file: args.autosave_file,
        srv: ChatServer::new().start(),
        sessions: RwLock::new(HashSet::new()),
    });
    let data_copy = data.clone();
    let data_copy2 = data.clone();

    let autosave_period_s = args.autosave_period_s;
    let autosave_pretty = args.autosave_pretty;
    let push_period_s = args.push_period_s;

    actix_web::rt::spawn(async move {
        let mut interval =
            actix_web::rt::time::interval(std::time::Duration::from_secs_f64(args.tick_freq));
        loop {
            interval.tick().await;

            let start = Instant::now();

            let mut game = data_copy.game.write().unwrap();
            if let Err(e) = game.tick() {
                println!("Tick error: {e}");
            }

            let mut last_saved = data_copy.last_saved.lock().unwrap();
            if autosave_period_s < last_saved.elapsed().as_secs_f64() {
                if let Ok(serialized) = serialize_state(&game, autosave_pretty) {
                    let autosave_file = data_copy.autosave_file.clone();
                    actix_web::rt::spawn(async move {
                        save_file(&autosave_file, &serialized);
                    });
                }
                *last_saved = Instant::now();
            }

            let mut last_pushed = data_copy.last_pushed.lock().unwrap();
            if push_period_s < last_pushed.elapsed().as_secs_f64() {
                // if let Ok(serialized) = serialize_state(&game, false) {
                //     data_copy.srv.do_send(NotifyState {
                //         session_id: None,
                //         set_state: NotifyStateEnum::SetState(SetStateWs(serialized)),
                //     });
                // }
                if let Ok(serialized) = game.serialize_bin() {
                    data_copy.srv.do_send(NotifyState {
                        session_id: None,
                        set_state: NotifyStateEnum::SetStateBin(SetStateBinWs(serialized)),
                    });
                }
                *last_pushed = Instant::now();
            }

            if game.get_global_time() % 100 == 0 {
                println!(
                    "[{:?}] Tick {}, calc: {:.3}ms",
                    std::thread::current().id(),
                    game.get_global_time(),
                    start.elapsed().as_micros() as f64 * 1e-3,
                );
            }
        }
    });

    let _result = HttpServer::new(move || {
        let cors = Cors::permissive()
            // .allowed_methods(vec!["GET", "POST"])
            // .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
            // .allowed_header(http::header::CONTENT_TYPE)
            .max_age(3600);

        let app = App::new()
            .wrap(middleware::Compress::default())
            .wrap(cors)
            .app_data(data.clone())
            .service(websocket_index)
            .route("/api/session", web::post().to(new_session))
            .route("/api/load", web::get().to(get_state));
        // .route("/api/time_scale", web::post().to(set_timescale));
        #[cfg(not(debug_assertions))]
        {
            app.route("/", web::get().to(get_index))
                .route("/js/main.js", web::get().to(get_main_js))
                .route("/js/{filename:.*}", web::get().to(get_js_file))
        }
        #[cfg(debug_assertions)]
        app.route("/js/{filename:.*}", web::get().to(get_js_file))
    })
    .bind((args.host.as_str(), args.port))?
    .run()
    .await;

    if let Ok(serialized) = serialize_state(&data_copy2.game.read().unwrap(), autosave_pretty) {
        save_file(&data_copy2.autosave_file, &serialized);
    }
    Ok(())
}
