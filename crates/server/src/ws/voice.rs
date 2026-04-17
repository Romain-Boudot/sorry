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
    let room = state.room_name(channel_id);
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

    // Check user limit (ADMINISTRATOR and MOVE_MEMBERS bypass)
    let user_perms = crate::db::roles::get_user_permissions(&state.db, user_id).await.unwrap_or(0);
    let bypass = permissions::has(user_perms, permissions::ADMINISTRATOR)
        || permissions::has(user_perms, permissions::MOVE_MEMBERS);
    if !bypass {
        if let Some(row) = crate::db::channels::find_by_id(&state.db, channel_id).await? {
            if let Some(limit) = row.user_limit {
                let voice = state.voice_state.read().unwrap();
                let count = voice.get(&channel_id).map(|u| u.len()).unwrap_or(0) as i64;
                if count >= limit {
                    return Ok(());
                }
            }
        }
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
    screen_sharing: bool,
    camera_on: bool,
) -> WsResult {
    let result = mutate_voice_state(state, user_id, |vs| {
        vs.muted = muted || vs.force_muted;
        vs.deafened = deafened || vs.force_deafened;
        vs.screen_sharing = screen_sharing;
        vs.camera_on = camera_on;
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
        let action = if muted { "voice.force_mute" } else { "voice.force_unmute" };
        let _ = crate::db::audit::log(&state.db, actor_id, action, Some(target_id), Some(cid), None, None).await;
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
        let action = if deafened { "voice.force_deafen" } else { "voice.force_undeafen" };
        let _ = crate::db::audit::log(&state.db, actor_id, action, Some(target_id), Some(cid), None, None).await;
    }
    Ok(())
}

pub async fn handle_move(
    state: &AppState,
    actor_id: i64,
    target_id: i64,
    to_channel_id: i64,
) -> WsResult {
    let perms = crate::db::roles::get_user_permissions(&state.db, actor_id).await?;
    if !permissions::has(perms, permissions::MOVE_MEMBERS) {
        return Ok(());
    }

    // Check target is in a voice channel
    let from_channel = {
        let voice = state.voice_state.read().unwrap();
        let mut found = None;
        for (cid, users) in voice.iter() {
            if users.contains_key(&target_id) {
                found = Some(*cid);
                break;
            }
        }
        found
    };

    let from_channel = match from_channel {
        Some(cid) => cid,
        None => return Ok(()),
    };

    if from_channel == to_channel_id {
        return Ok(());
    }

    // Check destination is a voice channel
    let dest = crate::db::channels::find_by_id(&state.db, to_channel_id)
        .await?
        .ok_or("channel not found")?;
    if dest.kind != "voice" {
        return Ok(());
    }


    // Remove from old, add to new
    {
        let mut voice = state.voice_state.write().unwrap();
        if let Some(users) = voice.get_mut(&from_channel) {
            users.remove(&target_id);
        }
        voice.entry(to_channel_id).or_default().insert(target_id, VoiceUserState::default());
    }

    state.broadcast(ServerEvent::UserLeftVoice {
        user_id: target_id,
        channel_id: from_channel,
    });

    let user = crate::db::users::find_by_id(&state.db, target_id)
        .await?
        .ok_or("user not found")?;

    state.broadcast(ServerEvent::UserJoinedVoice {
        user: user.clone(),
        channel_id: to_channel_id,
        voice_state: VoiceUserState::default(),
    });

    // Generate a LiveKit token for the moved user (bypass CONNECT check, grant SPEAK + STREAM)
    // No need to remove from old room — the client disconnects itself via rejoinWithToken
    if !state.livekit_url.is_empty() {
        let room_name = state.room_name(to_channel_id);
        let identity = format!("user-{}", target_id);

        let token = crate::livekit::generate_token(
            &state.livekit_api_key,
            &state.livekit_api_secret,
            &room_name,
            &identity,
            &user.display_name,
            true,  // can_speak
            true,  // can_stream
        )
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })?;

        state.broadcast(ServerEvent::VoiceMoved {
            user_id: target_id,
            channel_id: to_channel_id,
            token,
            url: state.livekit_url.clone(),
        });
    }

    let _ = crate::db::audit::log(&state.db, actor_id, "voice.move", Some(target_id), Some(to_channel_id), None, None).await;

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

        let room = state.room_name(cid);
        let identity = format!("user-{}", target_id);
        let lk_url = state.livekit_internal_url.clone();
        let lk_key = state.livekit_api_key.clone();
        let lk_secret = state.livekit_api_secret.clone();
        tokio::spawn(async move {
            if let Err(e) = crate::livekit::remove_participant(&lk_url, &lk_key, &lk_secret, &room, &identity).await {
                tracing::error!("LiveKit kick failed: {}", e);
            }
        });

        let _ = crate::db::audit::log(&state.db, actor_id, "voice.kick", Some(target_id), Some(cid), None, None).await;
    }
    Ok(())
}
