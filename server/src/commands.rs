use ::actix_web::{dev::ServiceFactory, web, App, HttpResponse};
use actix_web::dev::ServiceRequest;
use asteroid_colonies_logic::{building::BuildingType, construction::Construction, Pos};
use serde::Deserialize;

use crate::ServerData;

#[derive(Deserialize)]
struct MovePayload {
    from: Pos,
    to: Pos,
}

async fn move_(
    data: web::Data<ServerData>,
    payload: web::Json<MovePayload>,
) -> actix_web::Result<HttpResponse> {
    println!("move {:?} -> {:?}", payload.from, payload.to);
    let mut game = data.game.write().unwrap();
    game.move_building(
        payload.from[0],
        payload.from[1],
        payload.to[0],
        payload.to[1],
    )
    .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
    Ok(HttpResponse::Ok().content_type("text/plain").body("ok"))
}

#[derive(Deserialize)]
struct ExcavatePayload {
    x: i32,
    y: i32,
}

async fn excavate(
    data: web::Data<ServerData>,
    payload: web::Json<ExcavatePayload>,
) -> actix_web::Result<HttpResponse> {
    println!("excavate {} {}", payload.x, payload.y);
    let mut game = data.game.write().unwrap();
    game.excavate(payload.x, payload.y)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
    Ok(HttpResponse::Ok().content_type("text/plain").body("ok"))
}

#[derive(Deserialize)]
struct BuildPayload {
    pos: [i32; 2],
    #[serde(rename = "type")]
    ty: BuildingType,
}

async fn build(
    data: web::Data<ServerData>,
    payload: web::Json<BuildPayload>,
) -> actix_web::Result<HttpResponse> {
    println!("build {:?} {:?}", payload.pos, payload.ty);
    let mut game = data.game.write().unwrap();
    game.build(payload.pos[0], payload.pos[1], payload.ty)
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
    Ok(HttpResponse::Ok().content_type("text/plain").body("ok"))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BuildPlanPayload {
    build_plan: Vec<Construction>,
}

async fn build_plan(
    data: web::Data<ServerData>,
    payload: web::Json<BuildPlanPayload>,
) -> actix_web::Result<HttpResponse> {
    println!("build {:?}", payload.0.build_plan);
    let mut game = data.game.write().unwrap();
    game.build_plan(&payload.0.build_plan);
    Ok(HttpResponse::Ok().content_type("text/plain").body("ok"))
}

pub(crate) fn register_commands<T>(app: App<T>) -> App<T>
where
    T: ServiceFactory<ServiceRequest, Config = (), Error = actix_web::Error, InitError = ()>,
{
    app.route("/api/excavate", web::post().to(excavate))
        .route("/api/move", web::post().to(move_))
        .route("/api/build", web::post().to(build))
        .route("/api/build_plan", web::post().to(build_plan))
}
