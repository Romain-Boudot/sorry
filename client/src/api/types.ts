export interface User {
  id: number;
  display_name: string;
  avatar_url: string | null;
  username?: string;
  created_at?: string;
  guest?: boolean;
  /** Public key (base64 X25519) — present once user has provisioned a DM keypair. */
  public_key?: string | null;
  key_fingerprint?: string | null;
}

export interface DmMessage {
  id: number;
  sender_id: number;
  recipient_id: number;
  ciphertext: string;
  nonce: string;
  sender_key_fingerprint: string;
  created_at: string;
  reply_to_id?: number | null;
  edited?: boolean;
  reactions?: Reaction[];
  // Client-only — populated after decryption.
  plaintext?: string;
  /** True if decryption failed (e.g. recipient's key has rotated since). */
  undecryptable?: boolean;
  // Optimistic send state.
  pending?: boolean;
  failed?: boolean;
}

export interface DmConversation {
  user_id: number;
  last_message_id: number;
  last_message_at: string;
}

export interface BannedUser {
  id: number;
  display_name: string;
  username: string;
  avatar_url: string | null;
  banned_at: number;
}

export interface ChannelGroup {
  id: number;
  name: string;
  position: number;
}

export interface Channel {
  id: number;
  name: string;
  kind: "text" | "voice";
  position: number;
  group_id: number | null;
  description: string | null;
  user_limit: number | null;
}

export interface Role {
  id: number;
  name: string;
  permissions: number;
  color: string | null;
  position: number;
}

export interface Attachment {
  id: number;
  filename: string;
  content_type: string;
  size: number;
  url: string;
}

export interface ChannelAttachment {
  id: number;
  filename: string;
  content_type: string;
  size: number;
  url: string;
  author_id: number;
  created_at: string;
}

export interface ServerStats {
  version: string;
  uptime_secs: number;
  users_total: number;
  users_online: number;
  users_guests: number;
  channels_text: number;
  channels_voice: number;
  messages_total: number;
  messages_today: number;
  files_total: number;
  files_size_bytes: number;
  bans_active: number;
  invites_active: number;
  db_size_bytes: number;
  disk_free_bytes: number;
  disk_total_bytes: number;
}

export interface ReplyPreview {
  id: number;
  author_id: number;
  content: string;
}

export interface Mention {
  kind: "user" | "role";
  id: number;
}

export interface Reaction {
  emoji: string;
  count: number;
  user_ids: number[];
}

export interface Message {
  id: number;
  channel_id: number;
  author_id: number;
  content: string;
  created_at: string;
  attachments: Attachment[];
  reply_to?: ReplyPreview;
  mentions: Mention[];
  reactions: Reaction[];
  pinned: boolean;
  webhook_id?: number | null;
  webhook_username?: string | null;
  webhook_avatar_url?: string | null;
  // Client-only: optimistic send state (never sent from server)
  nonce?: string;
  pending?: boolean;
  failed?: boolean;
  pendingFiles?: File[];
  pendingReplyToId?: number;
}

/** Public webhook descriptor surfaced via the snapshot — used to render messages. */
export interface WebhookInfo {
  id: number;
  channel_id: number;
  name: string;
  avatar_url: string | null;
}

/** Full webhook record (with token) — only returned via admin endpoints. */
export interface Webhook extends WebhookInfo {
  token: string;
  created_by: number;
  created_at: string;
}

export interface NotificationPref {
  scope: "channel" | "server";
  target_id: number;
  level: "all" | "mentions" | "nothing";
  mute_until: string | null;
}

export interface VoiceUserState {
  muted: boolean;
  deafened: boolean;
  force_muted: boolean;
  force_deafened: boolean;
  screen_sharing: boolean;
  camera_on: boolean;
}

export interface Invite {
  code: string;
  created_by: number;
  max_uses: number | null;
  uses: number;
  expires_at: number | null;
  created_at: number;
  role_id: number | null;
  guest: boolean;
}

export interface ChannelOverwrite {
  channel_id: number;
  role_id: number;
  allow: number;
  deny: number;
}

export interface AuditLog {
  id: number;
  actor_id: number;
  action: string;
  target_user_id: number | null;
  target_channel_id: number | null;
  target_role_id: number | null;
  details: string | null;
  created_at: string;
}

export interface ServerLogEntry {
  timestamp: string;
  level: string;
  target: string;
  message: string;
}

export interface MeResponse {
  user: User;
  permissions: number;
  users: User[];
  online_users: number[];
  voice_state: Record<number, Record<number, VoiceUserState>>;
  roles: Role[];
  user_roles: Record<number, number[]>;
  max_file_size: number;
}

export interface ServerEvent {
  type: string;
  data: unknown;
}

/** Event sequence recu du serveur (contient seq + type + data) */
export interface SequencedEvent extends ServerEvent {
  seq: number;
}

/** Snapshot complet recu a la connexion/reconnexion */
export interface Snapshot {
  seq: number;
  user: User;
  permissions: number;
  users: User[];
  online_users: number[];
  channels: Channel[];
  groups: ChannelGroup[];
  roles: Role[];
  user_roles: Record<number, number[]>;
  voice_state: Record<number, Record<number, VoiceUserState>>;
  channel_overwrites: ChannelOverwrite[];
  webhooks?: WebhookInfo[];
  server_name: string;
  server_description: string | null;
  server_icon_url: string | null;
  max_file_size: number;
}

/** Etats FSM de la connexion WebSocket */
export type WsConnectionState =
  | "disconnected"
  | "connecting"
  | "connected"
  | "reconnecting";

export interface WsConnection {
  ws: WebSocket | null;
  state: WsConnectionState;
  lastSeq: number;
  reconnectAttempt: number;
  /** Annule la reconnexion en cours et ferme le WS */
  destroy: () => void;
}
