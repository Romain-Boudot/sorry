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

/// Generate a short-lived admin token for LiveKit API calls
fn admin_token(api_key: &str, api_secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;

    #[derive(Serialize)]
    struct AdminClaims {
        iss: String,
        exp: usize,
        nbf: usize,
        video: AdminGrant,
    }

    #[derive(Serialize)]
    struct AdminGrant {
        #[serde(rename = "roomAdmin")]
        room_admin: bool,
        room: String,
    }

    let claims = AdminClaims {
        iss: api_key.to_string(),
        exp: now + 60,
        nbf: 0,
        video: AdminGrant {
            room_admin: true,
            room: String::new(), // empty = all rooms
        },
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(api_secret.as_bytes()),
    )
}

/// Mute/unmute a participant's audio track via LiveKit API
pub async fn set_participant_muted(
    livekit_url: &str,
    api_key: &str,
    api_secret: &str,
    room_name: &str,
    identity: &str,
    muted: bool,
) -> Result<(), String> {
    if livekit_url.is_empty() { return Ok(()); }

    let token = admin_token(api_key, api_secret).map_err(|e| e.to_string())?;

    // First list participant's tracks
    let client = reqwest::Client::new();
    let base = livekit_url.trim_end_matches('/');

    #[derive(Serialize)]
    struct GetParticipantReq { room: String, identity: String }

    #[derive(serde::Deserialize)]
    struct Track { sid: String, r#type: String, muted: bool }

    #[derive(serde::Deserialize)]
    struct Participant { tracks: Option<Vec<Track>> }

    let participant: Participant = client
        .post(format!("{}/twirp/livekit.RoomService/GetParticipant", base))
        .bearer_auth(&token)
        .json(&GetParticipantReq { room: room_name.to_string(), identity: identity.to_string() })
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;

    // Mute all audio tracks
    if let Some(tracks) = participant.tracks {
        for track in tracks {
            if track.r#type == "AUDIO" {
                #[derive(Serialize)]
                struct MuteReq { room: String, identity: String, track_sid: String, muted: bool }

                let _ = client
                    .post(format!("{}/twirp/livekit.RoomService/MutePublishedTrack", base))
                    .bearer_auth(&token)
                    .json(&MuteReq {
                        room: room_name.to_string(),
                        identity: identity.to_string(),
                        track_sid: track.sid,
                        muted,
                    })
                    .send()
                    .await;
            }
        }
    }

    Ok(())
}

/// Kick a participant from a LiveKit room
pub async fn remove_participant(
    livekit_url: &str,
    api_key: &str,
    api_secret: &str,
    room_name: &str,
    identity: &str,
) -> Result<(), String> {
    if livekit_url.is_empty() { return Ok(()); }

    let token = admin_token(api_key, api_secret).map_err(|e| e.to_string())?;
    let client = reqwest::Client::new();
    let base = livekit_url.trim_end_matches('/');

    #[derive(Serialize)]
    struct RemoveReq { room: String, identity: String }

    client
        .post(format!("{}/twirp/livekit.RoomService/RemoveParticipant", base))
        .bearer_auth(&token)
        .json(&RemoveReq { room: room_name.to_string(), identity: identity.to_string() })
        .send()
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
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
