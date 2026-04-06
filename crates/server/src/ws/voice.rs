//! Voice-related WS event handlers — extracted from the main match.

use crate::state::AppState;
use shared::events::ServerEvent;
use shared::models::VoiceUserState;
use shared::permissions;

type WsResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

/// Remove a user from all voice channels and broadcast the leave event.
/// Returns the channel IDs they were removed from.
pub fn remove_from_all_channels(state: &AppState, user_id: i64) {
    let mut voice = state.voice_state.write().unwrap();
    for (channel_id, users) in voice.iter_mut() {
        if users.remove(&user_id).is_some() {
            state.broadcast(ServerEvent::UserLeftVoice {
                user_id,
                channel_id: *channel_id,
            });
        }
    }
}

/// Find the channel + voice state for a given user.
/// Applies a mutation to their VoiceUserState before returning.
fn mutate_voice_state(
    state: &AppState,
    target_id: i64,
    mutator: impl FnOnce(&mut VoiceUserState),
) -> Option<(i64, VoiceUserState)> {
    let mut voice = state.voice_state.write().unwrap();
    for (cid, users) in voice.iter_mut() {
        if let Some(vs) = users.get_mut(&target_id) {
            mutator(vs);
            return Some((*cid, vs.clone()));
        }
    }
    None
}

/// Spawn a LiveKit mute call in the background.
fn spawn_livekit_mute(state: &AppState, channel_id: i64, target_id: i64, muted: bool) {
    let room = format!("voice-{}", channel_id);
    let identity = format!("user-{}", target_id);
    let lk_url = state.livekit_internal_url.clone();
    let lk_key = state.livekit_api_key.clone();
    let lk_secret = state.livekit_api_secret.clone();
    tokio::spawn(async move {
        if let Err(e) = crate::livekit::set_participant_muted(&lk_url, &lk_key, &lk_secret, &room, &identity, muted).await {
            tracing::error!("LiveKit mute/deafen failed: {}", e);
        }
    });
}

pub async fn handle_join(state: &AppState, user_id: i64, channel_id: i64) -> WsResult {
    if !crate::perms::check_channel_permission(&state.db, user_id, channel_id, permissions::CONNECT)
        .await
        .unwrap_or(false)
    {
        return Ok(());
    }

    // Leave current voice channel if in one
    remove_from_all_channels(state, user_id);

    // Join new channel
    {
        let mut voice = state.voice_state.write().unwrap();
        voice.entry(channel_id).or_default().insert(user_id, VoiceUserState::default());
    }

    let user = crate::db::users::find_by_id(&state.db, user_id)
        .await?
        .ok_or("user not found")?;
    state.broadcast(ServerEvent::UserJoinedVoice {
        user,
        channel_id,
        voice_state: VoiceUserState::default(),
    });
    Ok(())
}

pub async fn handle_leave(state: &AppState, user_id: i64, channel_id: i64) -> WsResult {
    {
        let mut voice = state.voice_state.write().unwrap();
        if let Some(users) = voice.get_mut(&channel_id) {
            users.remove(&user_id);
        }
    }
    state.broadcast(ServerEvent::UserLeftVoice { user_id, channel_id });
    Ok(())
}

pub async fn handle_update_state(
    state: &AppState,
    user_id: i64,
    muted: bool,
    deafened: bool,
) -> WsResult {
    let result = mutate_voice_state(state, user_id, |vs| {
        vs.muted = muted || vs.force_muted;
        vs.deafened = deafened || vs.force_deafened;
    });
    if let Some((cid, vs)) = result {
        state.broadcast(ServerEvent::VoiceStateUpdate {
            user_id,
            channel_id: cid,
            voice_state: vs,
        });
    }
    Ok(())
}

pub async fn handle_force_mute(
    state: &AppState,
    actor_id: i64,
    target_id: i64,
    muted: bool,
) -> WsResult {
    let perms = crate::db::roles::get_user_permissions(&state.db, actor_id).await?;
    if !permissions::has(perms, permissions::MUTE_MEMBERS) {
        return Ok(());
    }

    let result = mutate_voice_state(state, target_id, |vs| {
        vs.force_muted = muted;
    });
    if let Some((cid, vs)) = result {
        spawn_livekit_mute(state, cid, target_id, muted);
        state.broadcast(ServerEvent::VoiceStateUpdate {
            user_id: target_id,
            channel_id: cid,
            voice_state: vs,
        });
    }
    Ok(())
}

pub async fn handle_force_deafen(
    state: &AppState,
    actor_id: i64,
    target_id: i64,
    deafened: bool,
) -> WsResult {
    let perms = crate::db::roles::get_user_permissions(&state.db, actor_id).await?;
    if !permissions::has(perms, permissions::DEAFEN_MEMBERS) {
        return Ok(());
    }

    let result = mutate_voice_state(state, target_id, |vs| {
        vs.force_deafened = deafened;
        vs.force_muted = deafened; // deafen implies mute
    });
    if let Some((cid, vs)) = result {
        spawn_livekit_mute(state, cid, target_id, deafened);
        state.broadcast(ServerEvent::VoiceStateUpdate {
            user_id: target_id,
            channel_id: cid,
            voice_state: vs,
        });
    }
    Ok(())
}

pub async fn handle_kick(state: &AppState, actor_id: i64, target_id: i64) -> WsResult {
    let perms = crate::db::roles::get_user_permissions(&state.db, actor_id).await?;
    if !permissions::has(perms, permissions::MOVE_MEMBERS) {
        return Ok(());
    }

    let kicked_channel = {
        let mut voice = state.voice_state.write().unwrap();
        let mut found = None;
        for (cid, users) in voice.iter_mut() {
            if users.remove(&target_id).is_some() {
                found = Some(*cid);
                break;
            }
        }
        found
    };

    if let Some(cid) = kicked_channel {
        state.broadcast(ServerEvent::UserLeftVoice {
            user_id: target_id,
            channel_id: cid,
        });

        let room = format!("voice-{}", cid);
        let identity = format!("user-{}", target_id);
        let lk_url = state.livekit_internal_url.clone();
        let lk_key = state.livekit_api_key.clone();
        let lk_secret = state.livekit_api_secret.clone();
        tokio::spawn(async move {
            if let Err(e) = crate::livekit::remove_participant(&lk_url, &lk_key, &lk_secret, &room, &identity).await {
                tracing::error!("LiveKit kick failed: {}", e);
            }
        });
    }
    Ok(())
}
