//! Message-related WS event handlers — extracted from the main match.

use crate::state::AppState;
use shared::events::ServerEvent;
use shared::permissions;

type WsResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

pub async fn handle_send(
    state: &AppState,
    user_id: i64,
    channel_id: i64,
    content: String,
    reply_to_id: Option<i64>,
) -> WsResult {
    if !crate::perms::check_channel_permission(&state.db, user_id, channel_id, permissions::SEND_MESSAGES)
        .await
        .unwrap_or(false)
    {
        return Ok(());
    }
    let message = crate::db::messages::create(&state.db, channel_id, user_id, &content, reply_to_id).await?;
    state.broadcast(ServerEvent::MessageCreate(message));
    Ok(())
}

pub async fn handle_edit(
    state: &AppState,
    user_id: i64,
    message_id: i64,
    content: String,
) -> WsResult {
    let row = crate::db::messages::find_by_id(&state.db, message_id)
        .await?
        .ok_or("message not found")?;

    if row.author_id != user_id {
        return Ok(());
    }

    crate::db::messages::update_content(&state.db, message_id, &content).await?;
    let mut updated = crate::db::messages::to_model(
        &crate::db::messages::find_by_id(&state.db, message_id)
            .await?
            .ok_or("message not found")?,
    );
    let _ = crate::db::messages::enrich_with_attachments(&state.db, std::slice::from_mut(&mut updated)).await;
    let _ = crate::db::messages::enrich_with_reactions(&state.db, std::slice::from_mut(&mut updated)).await;
    state.broadcast(ServerEvent::MessageUpdate(updated));
    Ok(())
}

pub async fn handle_toggle_reaction(
    state: &AppState,
    user_id: i64,
    message_id: i64,
    emoji: String,
) -> WsResult {
    if emoji.is_empty() || emoji.chars().count() > 32 {
        return Ok(());
    }

    let row = crate::db::messages::find_by_id(&state.db, message_id)
        .await?
        .ok_or("message not found")?;

    let added = crate::db::messages::toggle_reaction(&state.db, message_id, user_id, &emoji).await?;

    if added {
        state.broadcast(ServerEvent::ReactionAdded {
            message_id,
            channel_id: row.channel_id,
            emoji,
            user_id,
        });
    } else {
        state.broadcast(ServerEvent::ReactionRemoved {
            message_id,
            channel_id: row.channel_id,
            emoji,
            user_id,
        });
    }
    Ok(())
}

pub async fn handle_delete(state: &AppState, user_id: i64, message_id: i64) -> WsResult {
    let row = crate::db::messages::find_by_id(&state.db, message_id)
        .await?
        .ok_or("message not found")?;

    let perms = crate::db::roles::get_user_permissions(&state.db, user_id).await?;
    if row.author_id != user_id && !permissions::has(perms, permissions::MANAGE_MESSAGES) {
        return Ok(());
    }

    let was_moderation = row.author_id != user_id;
    crate::db::messages::delete(&state.db, message_id).await?;

    if was_moderation {
        let _ = crate::db::audit::log(&state.db, user_id, "message.delete", Some(row.author_id), Some(row.channel_id), None, None).await;
    }

    let storage = state.storage.clone();
    tokio::spawn(async move {
        if let Err(e) = crate::storage::delete_prefix(&storage, &format!("{}/", message_id)).await {
            tracing::error!("Failed to clean up files for message {}: {}", message_id, e);
        }
    });

    state.broadcast(ServerEvent::MessageDelete { id: message_id });
    Ok(())
}
