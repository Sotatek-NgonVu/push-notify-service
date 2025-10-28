use utoipa::Modify;
use utoipa::OpenApi;
use utoipa::openapi::security::HttpAuthScheme;
use utoipa::openapi::security::HttpBuilder;
use utoipa::openapi::security::SecurityScheme;

use crate::routes::account::dtos::requests::{
    CreateUserFcmTokenRequestDto, DeactivateUserFcmTokenRequestDto, UserFcmTokenStatus,
};
use crate::routes::notification::dto::{
    EditNotifPreferenceRequestDto, MarkNotificationAsReadResponseDto, NotifPreferenceResponseDto,
    NotificationDto,
};
use crate::utils::pagination::PaginationResponseDto;
use crate::utils::structs::NotificationPreferences;

#[derive(OpenApi)]
#[openapi(
    modifiers(&SecurityModifier),
    servers(
        (url = "/"),
        (url = "https://api.raidenx.io"),
    ),
    components(
        schemas(
            // Account DTOs
            CreateUserFcmTokenRequestDto,
            DeactivateUserFcmTokenRequestDto,
            UserFcmTokenStatus,
            
            // Notification DTOs
            NotificationDto,
            MarkNotificationAsReadResponseDto,
            NotifPreferenceResponseDto,
            EditNotifPreferenceRequestDto,
            NotificationPreferences,
            
            // Pagination
            PaginationResponseDto<NotificationDto>,
        )
    ),
    tags(
        (name = "Account APIs", description = "Account and FCM token management endpoints"),
        (name = "Notification APIs", description = "Notification management endpoints"),
        (name = "Health", description = "Health check endpoints"),
    )
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
