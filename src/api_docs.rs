use utoipa::Modify;
use utoipa::OpenApi;
use utoipa::openapi::security::HttpAuthScheme;
use utoipa::openapi::security::HttpBuilder;
use utoipa::openapi::security::SecurityScheme;

#[derive(OpenApi)]
#[openapi(
    modifiers(&SecurityModifier),
    servers(
    (url = "/"),
    (url = "https://api.raidenx.io"),
    ),
)]
pub struct ApiDoc;

struct SecurityModifier;
impl Modify for SecurityModifier {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap();
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
        components.add_security_scheme(
            "basic_auth",
            SecurityScheme::Http(HttpBuilder::new().scheme(HttpAuthScheme::Basic).build()),
        );
    }
}
