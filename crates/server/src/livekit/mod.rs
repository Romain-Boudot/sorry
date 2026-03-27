use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Serialize;

#[derive(Serialize)]
struct VideoGrant {
    room: String,
    #[serde(rename = "roomJoin")]
    room_join: bool,
    #[serde(rename = "canPublish")]
    can_publish: bool,
    #[serde(rename = "canSubscribe")]
    can_subscribe: bool,
    #[serde(rename = "canPublishData")]
    can_publish_data: bool,
}

#[derive(Serialize)]
struct LiveKitClaims {
    iss: String,
    sub: String,
    name: String,
    exp: usize,
    nbf: usize,
    video: VideoGrant,
}

pub fn generate_token(
    api_key: &str,
    api_secret: &str,
    room_name: &str,
    identity: &str,
    display_name: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;

    let claims = LiveKitClaims {
        iss: api_key.to_string(),
        sub: identity.to_string(),
        name: display_name.to_string(),
        exp: now + 6 * 3600, // 6 heures
        nbf: 0,
        video: VideoGrant {
            room: room_name.to_string(),
            room_join: true,
            can_publish: true,
            can_subscribe: true,
            can_publish_data: true,
        },
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(api_secret.as_bytes()),
    )
}
