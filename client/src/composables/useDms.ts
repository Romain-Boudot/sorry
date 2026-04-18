/**
 * DMs E2EE — keypair lifecycle, send/decrypt, conversation loading.
 *
 * Design :
 * - Une keypair par serveur (localStorage `dmKey:{serverId}`).
 * - Au connect, on appelle `ensureKeypair()` : génère + upload si absent, sinon vérifie que le
 *   fingerprint serveur correspond à notre clé locale (sinon on republie — cas appareil neuf).
 * - À chaque DM reçu, on déchiffre et on store le plaintext (jamais persisté).
 * - Si on n'arrive pas à déchiffrer (clé rotée), on flag `undecryptable` pour l'UI.
 */
import { api, type DmMessage, type User } from "../api";
import { store, type SavedServer, type ServerState } from "../store";
import {
  decrypt,
  encrypt,
  generateKeypair,
  loadKeypair,
  saveKeypair,
  saveFingerprints,
  type Keypair,
} from "../crypto";
import { showToast } from "./useToast";
import { fireDmNotification } from "./useNotifications";

/**
 * Charge ou génère la keypair DM, puis publie la clé publique si le serveur ne la connaît pas.
 * L'UI (DmView) détecte l'échec de publication via `state.user.key_fingerprint !== state.dmKeypair.fingerprint`.
 */
export async function ensureKeypair(server: SavedServer, state: ServerState): Promise<Keypair> {
  let kp = loadKeypair(server.id);
  if (!kp) {
    kp = await generateKeypair();
    saveKeypair(server.id, kp);
  }
  state.dmKeypair = kp;

  if (state.user?.key_fingerprint !== kp.fingerprint) {
    await api.uploadPublicKey(server.url, server.token, kp.publicKey);
    if (state.user) {
      state.user.public_key = kp.publicKey;
      state.user.key_fingerprint = kp.fingerprint;
    }
  }
  return kp;
}

/** Récupère et déchiffre l'historique DM avec un peer ; popule `state.dms`. */
export async function loadConversation(
  server: SavedServer,
  state: ServerState,
  peerId: number,
): Promise<void> {
  if (!state.dmKeypair) return;

  const peer = state.users.get(peerId);
  if (!peer?.public_key) return;

  const messages = await api.listDmMessages(server.url, server.token, peerId, 50);
  const myKp = state.dmKeypair;
  const peerPub = peer.public_key;

  // Grâce à la symétrie DH, on déchiffre avec peerPub + myPriv dans les deux sens
  // (messages entrants ET messages qu'on a envoyés soi-même).
  const decrypted = await Promise.all(
    messages.map(async (m) => {
      const plain = await decrypt(m.ciphertext, m.nonce, peerPub, myKp.secretKey);
      return plain
        ? ({ ...m, plaintext: plain } as DmMessage)
        : ({ ...m, undecryptable: true } as DmMessage);
    }),
  );

  state.dms.set(peerId, decrypted.reverse());
  rememberFingerprint(state, server.id, peerId, peer.key_fingerprint ?? null);
}

/** Charge la liste des conversations actives (peers avec qui on a déjà échangé). */
export async function loadConversations(server: SavedServer, state: ServerState): Promise<void> {
  const convs = await api.listDmConversations(server.url, server.token);
  state.dmConversations = convs.map((c) => c.user_id);
}

export async function sendDm(
  server: SavedServer,
  state: ServerState,
  recipientId: number,
  plaintext: string,
  replyToId?: number | null,
): Promise<void> {
  if (!plaintext.trim()) return;
  if (!state.dmKeypair) {
    showToast("Cle DM pas encore initialisee", "error");
    return;
  }
  const recipient = state.users.get(recipientId);
  if (!recipient?.public_key) {
    showToast("Ce destinataire n'a pas encore de cle de chiffrement", "error");
    return;
  }
  if (!state.wsConnection?.ws) return;

  const { ciphertext, nonce } = await encrypt(
    plaintext,
    recipient.public_key,
    state.dmKeypair.secretKey,
  );

  state.wsConnection.ws.send(
    JSON.stringify({
      type: "SendDm",
      data: {
        recipient_id: recipientId,
        ciphertext,
        nonce,
        sender_key_fingerprint: state.dmKeypair.fingerprint,
        reply_to_id: replyToId ?? null,
      },
    }),
  );

  const localMsg: DmMessage = {
    id: -Date.now(),
    sender_id: state.user!.id,
    recipient_id: recipientId,
    ciphertext,
    nonce,
    sender_key_fingerprint: state.dmKeypair.fingerprint,
    created_at: new Date().toISOString(),
    reply_to_id: replyToId ?? null,
    reactions: [],
    plaintext,
    pending: true,
  };
  const list = state.dms.get(recipientId) ?? [];
  list.push(localMsg);
  state.dms.set(recipientId, list);
  if (!state.dmConversations.includes(recipientId)) {
    state.dmConversations = [recipientId, ...state.dmConversations];
  }
}

/** Re-encrypte le nouveau plaintext avec la clé du destinataire et envoie EditDm. */
export async function editDm(
  state: ServerState,
  messageId: number,
  newPlaintext: string,
): Promise<void> {
  if (!newPlaintext.trim()) return;
  if (!state.dmKeypair || !state.wsConnection?.ws) return;

  // Retrouve le peer à partir du message.
  let peerId: number | null = null;
  let msgRef: DmMessage | undefined;
  for (const [pid, list] of state.dms) {
    const m = list.find((x) => x.id === messageId);
    if (m) { peerId = pid; msgRef = m; break; }
  }
  if (!peerId || !msgRef) return;
  if (msgRef.sender_id !== state.user?.id) return;

  const peer = state.users.get(peerId);
  if (!peer?.public_key) return;

  const { ciphertext, nonce } = await encrypt(newPlaintext, peer.public_key, state.dmKeypair.secretKey);
  state.wsConnection.ws.send(JSON.stringify({
    type: "EditDm",
    data: { message_id: messageId, ciphertext, nonce },
  }));

  // Optimistic : update local immédiatement.
  msgRef.ciphertext = ciphertext;
  msgRef.nonce = nonce;
  msgRef.plaintext = newPlaintext;
  msgRef.edited = true;
}

export function deleteDm(state: ServerState, messageId: number) {
  if (!state.wsConnection?.ws) return;
  state.wsConnection.ws.send(JSON.stringify({
    type: "DeleteDm",
    data: { message_id: messageId },
  }));
}

export function toggleDmReaction(state: ServerState, messageId: number, emoji: string) {
  if (!state.wsConnection?.ws) return;
  state.wsConnection.ws.send(JSON.stringify({
    type: "ToggleDmReaction",
    data: { message_id: messageId, emoji },
  }));
}

/** Charge des messages plus anciens et les prépend à la liste existante (scroll infini). */
export async function loadOlderDms(
  server: SavedServer,
  state: ServerState,
  peerId: number,
): Promise<number> {
  const list = state.dms.get(peerId) ?? [];
  const oldest = list.find((m) => m.id > 0);
  if (!oldest) return 0;
  if (!state.dmKeypair) return 0;
  const peer = state.users.get(peerId);
  if (!peer?.public_key) return 0;

  const messages = await api.listDmMessages(server.url, server.token, peerId, 50, oldest.id);
  if (!messages.length) return 0;

  const myKp = state.dmKeypair;
  const peerPub = peer.public_key;
  const decrypted = await Promise.all(messages.map(async (m) => {
    const plain = await decrypt(m.ciphertext, m.nonce, peerPub, myKp.secretKey);
    return plain ? { ...m, plaintext: plain } as DmMessage : { ...m, undecryptable: true } as DmMessage;
  }));

  // Backend renvoie newest first → on reverse pour prépender dans l'ordre chrono.
  state.dms.set(peerId, [...decrypted.reverse(), ...list]);
  return decrypted.length;
}

/**
 * Traite un DmCreate reçu par WS : déchiffre + push dans le state.
 * Met à jour `dmConversations` et `dmUnread` si nécessaire.
 */
export async function handleIncomingDm(
  state: ServerState,
  serverId: string,
  msg: DmMessage,
): Promise<void> {
  const me = state.user?.id;
  if (!me) return;
  if (msg.sender_id !== me && msg.recipient_id !== me) return; // pas pour nous

  const peerId = msg.sender_id === me ? msg.recipient_id : msg.sender_id;

  // Notre propre echo (depuis un autre device ou retour du serveur sur send) :
  // remplace le pending optimiste si présent, sinon déchiffre via peerPub + myPriv (DH symmetry).
  if (msg.sender_id === me) {
    const list = state.dms.get(peerId) ?? [];
    const idx = list.findIndex(
      (m) => m.pending && m.ciphertext === msg.ciphertext && m.nonce === msg.nonce,
    );
    if (idx >= 0) {
      list[idx] = { ...msg, plaintext: list[idx].plaintext };
    } else {
      const peer = state.users.get(peerId);
      if (peer?.public_key && state.dmKeypair) {
        const plain = await decrypt(msg.ciphertext, msg.nonce, peer.public_key, state.dmKeypair.secretKey);
        list.push(plain ? { ...msg, plaintext: plain } : { ...msg, undecryptable: true });
      } else {
        list.push({ ...msg, undecryptable: true });
      }
    }
    state.dms.set(peerId, list);
    if (!state.dmConversations.includes(peerId)) {
      state.dmConversations = [peerId, ...state.dmConversations];
    }
    return;
  }

  // DM entrant : on déchiffre + notifie.
  const peer = state.users.get(peerId);
  const senderPub = peer?.public_key;

  if (!state.dmKeypair || !senderPub) {
    pushDm(state, peerId, { ...msg, undecryptable: true });
    bumpUnread(state, serverId, peerId);
    fireDmNotification(store, state, serverId, peerId, "Nouveau message prive");
    return;
  }

  // Vérifie le fingerprint contre celui qu'on a vu en dernier (TOFU).
  const known = state.knownFingerprints.get(peerId);
  if (known && known !== msg.sender_key_fingerprint) {
    showToast(
      `La cle DM de ${peer?.display_name ?? `User #${peerId}`} a change. Verifiez son empreinte.`,
      "warning",
      6000,
    );
  }
  rememberFingerprint(state, serverId, peerId, msg.sender_key_fingerprint);

  const plain = await decrypt(msg.ciphertext, msg.nonce, senderPub, state.dmKeypair.secretKey);
  pushDm(state, peerId, plain ? { ...msg, plaintext: plain } : { ...msg, undecryptable: true });

  const isViewingNow =
    store.activeServerId === serverId && state.activeDmUserId === peerId;
  if (!isViewingNow) {
    bumpUnread(state, serverId, peerId);
    const name = peer?.display_name ?? "Quelqu'un";
    showToast(`${name} : ${plain ? truncate(plain, 60) : "message chiffre"}`, "info", 4000);
  }
  fireDmNotification(store, state, serverId, peerId, plain ?? "Nouveau message prive");
}

function truncate(s: string, n: number): string {
  return s.length <= n ? s : s.slice(0, n) + "...";
}

function pushDm(state: ServerState, peerId: number, msg: DmMessage) {
  const list = state.dms.get(peerId) ?? [];
  list.push(msg);
  state.dms.set(peerId, list);
  if (!state.dmConversations.includes(peerId)) {
    state.dmConversations = [peerId, ...state.dmConversations];
  }
}

function bumpUnread(state: ServerState, serverId: string, peerId: number) {
  const isViewing =
    store.activeServerId === serverId && state.activeDmUserId === peerId;
  if (isViewing) return;
  state.dmUnread.set(peerId, (state.dmUnread.get(peerId) ?? 0) + 1);
}

/** Traite un DmUpdate — replace le message dans state.dms et re-déchiffre. */
export async function handleDmUpdate(state: ServerState, msg: DmMessage): Promise<void> {
  const me = state.user?.id;
  if (!me) return;
  const peerId = msg.sender_id === me ? msg.recipient_id : msg.sender_id;
  const peer = state.users.get(peerId);
  if (!peer?.public_key || !state.dmKeypair) return;

  const plain = await decrypt(msg.ciphertext, msg.nonce, peer.public_key, state.dmKeypair.secretKey);
  const list = state.dms.get(peerId) ?? [];
  const idx = list.findIndex((m) => m.id === msg.id);
  if (idx >= 0) {
    list[idx] = plain
      ? { ...msg, plaintext: plain }
      : { ...msg, undecryptable: true };
    state.dms.set(peerId, list);
  }
}

/** Traite un DmDelete — retire le message de state.dms. */
export function handleDmDelete(state: ServerState, msg: { id: number; sender_id: number; recipient_id: number }) {
  const me = state.user?.id;
  if (!me) return;
  const peerId = msg.sender_id === me ? msg.recipient_id : msg.sender_id;
  const list = state.dms.get(peerId);
  if (!list) return;
  const idx = list.findIndex((m) => m.id === msg.id);
  if (idx >= 0) {
    list.splice(idx, 1);
    state.dms.set(peerId, list);
  }
}

/** Ajoute/retire une réaction sur un DM. peer_a / peer_b = les deux bouts de la conv. */
export function handleDmReaction(
  state: ServerState,
  data: { dm_id: number; peer_a: number; peer_b: number; user_id: number; emoji: string },
  added: boolean,
) {
  const me = state.user?.id;
  if (!me) return;
  const peerId = data.peer_a === me ? data.peer_b : data.peer_a;
  const list = state.dms.get(peerId);
  if (!list) return;
  const msg = list.find((m) => m.id === data.dm_id);
  if (!msg) return;
  if (!msg.reactions) msg.reactions = [];

  const existing = msg.reactions.find((r) => r.emoji === data.emoji);
  if (added) {
    if (existing) {
      if (!existing.user_ids.includes(data.user_id)) {
        existing.user_ids.push(data.user_id);
        existing.count++;
      }
    } else {
      msg.reactions.push({ emoji: data.emoji, count: 1, user_ids: [data.user_id] });
    }
  } else if (existing) {
    existing.user_ids = existing.user_ids.filter((id) => id !== data.user_id);
    existing.count--;
    if (existing.count <= 0) {
      msg.reactions = msg.reactions.filter((r) => r.emoji !== data.emoji);
    }
  }
}

/** Réagit à un UserKeyUpdate (rotation de clé d'un peer). */
export function handleUserKeyUpdate(state: ServerState, serverId: string, user: User) {
  const known = state.knownFingerprints.get(user.id);
  if (known && user.key_fingerprint && known !== user.key_fingerprint) {
    const name = state.users.get(user.id)?.display_name ?? `User #${user.id}`;
    showToast(`Nouvelle cle DM pour ${name}. Les anciens messages restent illisibles.`, "warning", 6000);
  }
  if (user.key_fingerprint) {
    state.knownFingerprints.set(user.id, user.key_fingerprint);
    saveFingerprints(serverId, state.knownFingerprints);
  }
}

function rememberFingerprint(state: ServerState, serverId: string, peerId: number, fp: string | null) {
  if (!fp) return;
  if (state.knownFingerprints.get(peerId) === fp) return;
  state.knownFingerprints.set(peerId, fp);
  saveFingerprints(serverId, state.knownFingerprints);
}

export function closeDm() {
  const state = store.serverStates.get(store.activeServerId ?? "");
  if (!state) return;
  state.activeDmUserId = null;
}
