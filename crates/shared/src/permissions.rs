/// Permissions système en bitmask.
/// Ne jamais supprimer ou réordonner — ajouter à la fin uniquement.

// Général
pub const ADMINISTRATOR: i64 = 1 << 0;
pub const MANAGE_CHANNELS: i64 = 1 << 1;
pub const MANAGE_ROLES: i64 = 1 << 2;
pub const MANAGE_SERVER: i64 = 1 << 3;
pub const KICK_MEMBERS: i64 = 1 << 4;
pub const BAN_MEMBERS: i64 = 1 << 5;
pub const CREATE_INVITE: i64 = 1 << 6;
pub const CHANGE_NICKNAME: i64 = 1 << 7;
pub const MANAGE_NICKNAMES: i64 = 1 << 8;

// Texte
pub const VIEW_CHANNELS: i64 = 1 << 9;
pub const SEND_MESSAGES: i64 = 1 << 10;
pub const MANAGE_MESSAGES: i64 = 1 << 11;
pub const READ_MESSAGE_HISTORY: i64 = 1 << 12;
pub const ATTACH_FILES: i64 = 1 << 13;
pub const MENTION_EVERYONE: i64 = 1 << 14;
pub const ADD_REACTIONS: i64 = 1 << 15;
pub const EMBED_LINKS: i64 = 1 << 16;

// Vocal
pub const CONNECT: i64 = 1 << 17;
pub const SPEAK: i64 = 1 << 18;
pub const STREAM: i64 = 1 << 19;
pub const MUTE_MEMBERS: i64 = 1 << 20;
pub const DEAFEN_MEMBERS: i64 = 1 << 21;
pub const MOVE_MEMBERS: i64 = 1 << 22;
pub const USE_VOICE_ACTIVITY: i64 = 1 << 23;
pub const PRIORITY_SPEAKER: i64 = 1 << 24;

/// Permissions par défaut pour un nouveau membre
pub const DEFAULT_MEMBER: i64 = VIEW_CHANNELS
    | SEND_MESSAGES
    | READ_MESSAGE_HISTORY
    | EMBED_LINKS
    | ATTACH_FILES
    | ADD_REACTIONS
    | CONNECT
    | SPEAK
    | STREAM
    | USE_VOICE_ACTIVITY
    | CHANGE_NICKNAME
    | CREATE_INVITE;

/// Toutes les permissions (admin)
pub const ALL: i64 = (1 << 25) - 1;

/// Vérifie si un bitmask contient une permission
pub fn has(permissions: i64, permission: i64) -> bool {
    permissions & ADMINISTRATOR != 0 || permissions & permission == permission
}

/// Calcule les permissions finales d'un user à partir de ses rôles
/// puis applique les overwrites du channel
pub fn compute(
    role_permissions: &[i64],
    channel_allow: i64,
    channel_deny: i64,
) -> i64 {
    let mut base: i64 = 0;
    for &p in role_permissions {
        base |= p;
    }

    // Admin bypass les overwrites
    if base & ADMINISTRATOR != 0 {
        return ALL;
    }

    // Appliquer les overwrites du channel
    base &= !channel_deny;
    base |= channel_allow;

    base
}
