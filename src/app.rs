use crate::app_state::AppState;
use crate::config::APP_CONFIG;
use crate::routes;
use crate::utils::extractor::BearerOrSmartIpKeyExtractor;
use crate::{api_docs::ApiDoc, core::middleware::http_logger::http_logger};
use axum::{Router, middleware};
use http::header;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tower::ServiceBuilder;
use tower_governor::GovernorLayer;
use tower_governor::governor::GovernorConfigBuilder;
use tower_http::{ServiceBuilderExt, cors::CorsLayer, propagate_header::PropagateHeaderLayer};
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

pub async fn create_app() -> eyre::Result<Router> {
    let app_state = AppState::init().await?;
    let (mut router, api) = OpenApiRouter::<AppState>::with_openapi(ApiDoc::openapi())
        .nest("/health", routes::health::route::create_route())
        .nest(
            "/api/v1",
            OpenApiRouter::new()
                .merge(routes::account::route::create_route())
                .nest("/notification", routes::notification::route::create_route()),
        )
        .with_state(app_state)
        .split_for_parts();

    if APP_CONFIG.swagger_enabled {
        let config = utoipa_swagger_ui::Config::new(["/"]).display_request_duration(true);
        let swagger_ui = SwaggerUi::new("/swagger-ui").url("/", api).config(config);
        router = router.merge(swagger_ui);
    }

    // TODO: error as json response
    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(1)
            .burst_size(APP_CONFIG.rate_limit_req_per_sec)
            .key_extractor(BearerOrSmartIpKeyExtractor)
            .finish()
            .unwrap(),
    );
    let governor_limiter = governor_conf.limiter().clone();
    let interval = Duration::from_secs(60);
    tokio::spawn(async move {
        loop {
            sleep(interval).await;
            tracing::debug!("rate limiting storage size: {}", governor_limiter.len());
            governor_limiter.retain_recent();
        }
    });

    let cors_layer = if let Some(whitelist) = &APP_CONFIG.cors_origin_whitelist {
        CorsLayer::new()
            .allow_headers(tower_http::cors::Any)
            .allow_methods(tower_http::cors::Any)
            .expose_headers(tower_http::cors::Any)
            .allow_origin(
                whitelist
                    .iter()
                    .map(|origin| origin.parse().unwrap())
                    .collect::<Vec<_>>(),
            )
    } else {
        CorsLayer::permissive()
    };

    let sensitive_headers: Arc<[_]> = vec![header::AUTHORIZATION, header::COOKIE].into();

    let middleware = ServiceBuilder::new()
        .layer(PropagateHeaderLayer::new(header::HeaderName::from_static(
            "x-request-id",
        )))
        .sensitive_request_headers(sensitive_headers.clone())
        .layer(middleware::from_fn(http_logger))
        .sensitive_response_headers(sensitive_headers)
        .compression()
        .layer(cors_layer)
        .layer(GovernorLayer {
            config: governor_conf,
        });

    let router = Router::new().merge(router).layer(middleware);
    Ok(router)
}
