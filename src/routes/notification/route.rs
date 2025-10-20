use crate::app_state::AppState;
use utoipa_axum::router::OpenApiRouter;

pub fn create_route() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
    // Add notification routes here when implemented
}
