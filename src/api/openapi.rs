use actix_web::{HttpResponse, Responder};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api::nfag_controller::create,
        crate::api::nfag_controller::find_all,
        crate::api::nfag_controller::find_by_id,
        crate::api::nfag_controller::update,
        crate::api::nfag_controller::delete,

        crate::api::cons_sit_nfag_controller::create,
        crate::api::cons_sit_nfag_controller::find_all,
        crate::api::cons_sit_nfag_controller::find_by_id,
        crate::api::cons_sit_nfag_controller::update,
        crate::api::cons_sit_nfag_controller::delete,

        crate::api::cons_stat_serv_nfag_controller::create,
        crate::api::cons_stat_serv_nfag_controller::find_all,
        crate::api::cons_stat_serv_nfag_controller::find_by_id,
        crate::api::cons_stat_serv_nfag_controller::update,
        crate::api::cons_stat_serv_nfag_controller::delete,

        crate::api::evento_nfag_controller::create_evento_nfag,
        crate::api::evento_nfag_controller::find_all_eventos_nfag,
        crate::api::evento_nfag_controller::find_evento_nfag_by_id,
        crate::api::evento_nfag_controller::update_evento_nfag,
        crate::api::evento_nfag_controller::delete_evento_nfag,

        crate::api::ret_nfag_controller::create_ret_nfag,
        crate::api::ret_nfag_controller::find_all_ret_nfag,
        crate::api::ret_nfag_controller::find_ret_nfag_by_id,
        crate::api::ret_nfag_controller::update_ret_nfag,
        crate::api::ret_nfag_controller::delete_ret_nfag
    ),
    components(
        schemas(
            crate::models::nfag::Nfag,
            crate::models::nfag::CreateNfagRequest,
            crate::models::nfag::UpdateNfagRequest,

            crate::models::cons_sit_nfag::ConsSitNfag,
            crate::models::cons_sit_nfag::CreateConsSitNfagRequest,
            crate::models::cons_sit_nfag::UpdateConsSitNfagRequest,

            crate::models::cons_stat_serv_nfag::ConsStatServNfag,
            crate::models::cons_stat_serv_nfag::Tpamb,
            crate::models::cons_stat_serv_nfag::CreateConsStatServNfagRequest,
            crate::models::cons_stat_serv_nfag::UpdateConsStatServNfagRequest,

            crate::models::evento_nfag::EventoNfag,
            crate::models::evento_nfag::CreateEventoNfagRequest,
            crate::models::evento_nfag::UpdateEventoNfagRequest,

            crate::models::ret_nfag::RetNfag,
            crate::models::ret_nfag::CreateRetNfagRequest,
            crate::models::ret_nfag::UpdateRetNfagRequest
        )
    ),
    tags(
        (name = "nfag", description = "NFAg CRUD endpoints"),
        (name = "cons-sit-nfag", description = "Consulta Situação NFAg endpoints"),
        (name = "cons-stat-serv-nfag", description = "Consulta Status Serviço NFAg endpoints"),
        (name = "evento-nfag", description = "Evento NFAg endpoints"),
        (name = "ret-nfag", description = "Retorno NFAg endpoints")
    )
)]
pub struct ApiDoc;

pub async fn openapi_json() -> impl Responder {
    HttpResponse::Ok().json(ApiDoc::openapi())
}

pub async fn api_doc_redirect() -> impl Responder {
    HttpResponse::Found()
        .append_header((actix_web::http::header::LOCATION, "/swagger-ui/"))
        .finish()
}
