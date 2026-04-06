pub mod events;
pub mod mentions;
pub mod models;
pub mod permissions;

/// System role IDs — never delete or reassign these.
pub const OWNER_ROLE_ID: i64 = 1;
pub const EVERYONE_ROLE_ID: i64 = 2;

/// The owner user always has ID 1 (created at bootstrap).
pub const OWNER_USER_ID: i64 = 1;

/// Convenience: the first custom role ID (anything > EVERYONE_ROLE_ID).
pub const MIN_CUSTOM_ROLE_ID: i64 = 3;

/// Returns true if `role_id` is a system role that cannot be deleted/assigned manually.
pub fn is_system_role(role_id: i64) -> bool {
    role_id <= EVERYONE_ROLE_ID
}
