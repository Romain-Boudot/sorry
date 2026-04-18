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
    nonce: Option<String>,
) -> WsResult {
    if !crate::perms::check_channel_permission(&state.db, user_id, channel_id, permissions::SEND_MESSAGES)
        .await
        .unwrap_or(false)
    {
        return Ok(());
    }
    let message = crate::db::messages::create(&state.db, channel_id, user_id, &content, reply_to_id).await?;
    state.broadcast(ServerEvent::MessageCreate { message, nonce });
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

/// Save an end-to-end encrypted DM and deliver it only to sender + recipient.
/// Server sees ciphertext+nonce only — never plaintext.
pub async fn handle_send_dm(
    state: &AppState,
    sender_id: i64,
    recipient_id: i64,
    ciphertext: String,
    nonce: String,
    sender_key_fingerprint: String,
    reply_to_id: Option<i64>,
) -> WsResult {
    if recipient_id == sender_id {
        return Ok(());
    }
    if ciphertext.is_empty() || ciphertext.len() > 16_384 {
        return Ok(());
    }
    if nonce.is_empty() || nonce.len() > 64 {
        return Ok(());
    }
    if crate::db::users::find_by_id(&state.db, recipient_id).await?.is_none() {
        return Ok(());
    }

    // reply_to_id doit pointer sur un DM de la même conversation (sinon ignoré).
    let validated_reply = match reply_to_id {
        Some(rid) => match crate::db::dms::find_by_id(&state.db, rid).await? {
            Some(orig) => {
                let same_pair = (orig.sender_id == sender_id && orig.recipient_id == recipient_id)
                    || (orig.sender_id == recipient_id && orig.recipient_id == sender_id);
                if same_pair { Some(rid) } else { None }
            }
            None => None,
        },
        None => None,
    };

    let dm = crate::db::dms::create(
        &state.db,
        sender_id,
        recipient_id,
        &ciphertext,
        &nonce,
        &sender_key_fingerprint,
        validated_reply,
    )
    .await?;

    state.deliver_direct(&[sender_id, recipient_id], ServerEvent::DmCreate(dm));
    Ok(())
}

pub async fn handle_edit_dm(
    state: &AppState,
    user_id: i64,
    message_id: i64,
    ciphertext: String,
    nonce: String,
) -> WsResult {
    if ciphertext.is_empty() || ciphertext.len() > 16_384 { return Ok(()); }
    if nonce.is_empty() || nonce.len() > 64 { return Ok(()); }

    let Some(dm) = crate::db::dms::find_by_id(&state.db, message_id).await? else {
        return Ok(());
    };
    if dm.sender_id != user_id {
        return Ok(()); // seul l'auteur peut éditer
    }

    crate::db::dms::update_ciphertext(&state.db, message_id, &ciphertext, &nonce).await?;

    let updated = crate::db::dms::find_by_id(&state.db, message_id).await?.ok_or("dm gone")?;
    state.deliver_direct(&[dm.sender_id, dm.recipient_id], ServerEvent::DmUpdate(updated));
    Ok(())
}

pub async fn handle_delete_dm(state: &AppState, user_id: i64, message_id: i64) -> WsResult {
    let Some(dm) = crate::db::dms::find_by_id(&state.db, message_id).await? else {
        return Ok(());
    };
    if dm.sender_id != user_id {
        return Ok(()); // seul l'auteur peut supprimer
    }

    crate::db::dms::delete(&state.db, message_id).await?;
    state.deliver_direct(
        &[dm.sender_id, dm.recipient_id],
        ServerEvent::DmDelete {
            id: message_id,
            sender_id: dm.sender_id,
            recipient_id: dm.recipient_id,
        },
    );
    Ok(())
}

pub async fn handle_toggle_dm_reaction(
    state: &AppState,
    user_id: i64,
    message_id: i64,
    emoji: String,
) -> WsResult {
    if emoji.is_empty() || emoji.chars().count() > 32 { return Ok(()); }

    let Some(dm) = crate::db::dms::find_by_id(&state.db, message_id).await? else {
        return Ok(());
    };
    // Seuls les participants à la conversation peuvent réagir.
    if user_id != dm.sender_id && user_id != dm.recipient_id {
        return Ok(());
    }

    let added = crate::db::dms::toggle_reaction(&state.db, message_id, user_id, &emoji).await?;
    let evt = if added {
        ServerEvent::DmReactionAdded {
            dm_id: message_id,
            peer_a: dm.sender_id,
            peer_b: dm.recipient_id,
            user_id,
            emoji,
        }
    } else {
        ServerEvent::DmReactionRemoved {
            dm_id: message_id,
            peer_a: dm.sender_id,
            peer_b: dm.recipient_id,
            user_id,
            emoji,
        }
    };
    state.deliver_direct(&[dm.sender_id, dm.recipient_id], evt);
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
