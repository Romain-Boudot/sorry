/**
 * E2EE pour les DMs : NaCl `box` (X25519 + XSalsa20-Poly1305) via libsodium-wrappers.
 *
 * Modèle :
 * - Une keypair par (serveur, user) — stockée en localStorage sous `dmKey:{serverId}`.
 * - Le serveur ne voit jamais le plaintext : il route ciphertext + nonce.
 * - TOFU : à la première vue d'une clé, on l'accepte. Si elle change, on prévient l'utilisateur
 *   (les anciens DMs restent illisibles — on accepte ce tradeoff plutôt qu'un backup serveur).
 */
import sodium from "libsodium-wrappers";

let readyPromise: Promise<void> | null = null;

export function ensureReady(): Promise<void> {
  if (!readyPromise) {
    readyPromise = sodium.ready;
  }
  return readyPromise;
}

export interface Keypair {
  publicKey: string;  // base64
  secretKey: string;  // base64
  fingerprint: string;
}

/**
 * Empreinte courte de la clé publique : BLAKE2b (via `crypto_generichash`),
 * tronquée à 8 octets, formatée `ab12:cd34:ef56:7890` pour vérification TOFU à l'oeil nu.
 *
 * Note : on utilise BLAKE2b plutôt que SHA-256 parce que SHA-256 n'est exposé que dans
 * `libsodium-wrappers-sumo` (plus lourd). BLAKE2b est tout aussi adapté pour un fingerprint.
 * L'appelant doit avoir await ensureReady() au moins une fois avant d'appeler ça.
 */
export function computeFingerprint(publicKeyB64: string): string {
  const bytes = sodium.from_base64(publicKeyB64, sodium.base64_variants.ORIGINAL);
  const hash = sodium.crypto_generichash(32, bytes, null);
  const hex = sodium.to_hex(hash).slice(0, 16);
  return hex.match(/.{1,4}/g)!.join(":");
}

export async function generateKeypair(): Promise<Keypair> {
  await ensureReady();
  const kp = sodium.crypto_box_keypair();
  const publicKey = sodium.to_base64(kp.publicKey, sodium.base64_variants.ORIGINAL);
  const secretKey = sodium.to_base64(kp.privateKey, sodium.base64_variants.ORIGINAL);
  return { publicKey, secretKey, fingerprint: computeFingerprint(publicKey) };
}

/** Renvoie la keypair persistée pour ce serveur, ou null si absente. */
export function loadKeypair(serverId: string): Keypair | null {
  try {
    const raw = localStorage.getItem(`dmKey:${serverId}`);
    if (!raw) return null;
    const obj = JSON.parse(raw);
    if (!obj.publicKey || !obj.secretKey) return null;
    return obj as Keypair;
  } catch {
    return null;
  }
}

export function saveKeypair(serverId: string, kp: Keypair) {
  localStorage.setItem(`dmKey:${serverId}`, JSON.stringify(kp));
}

export function deleteKeypair(serverId: string) {
  localStorage.removeItem(`dmKey:${serverId}`);
}

/**
 * Empreintes persistantes des pairs pour TOFU. Sans persistance, une rotation silencieuse
 * par un admin malveillant (ou un MITM) ne déclencherait aucun warning après un simple reload.
 */
const FP_KEY = (serverId: string) => `dmFingerprints:${serverId}`;

export function loadFingerprints(serverId: string): Map<number, string> {
  try {
    const raw = localStorage.getItem(FP_KEY(serverId));
    if (!raw) return new Map();
    const obj = JSON.parse(raw) as Record<string, string>;
    const map = new Map<number, string>();
    for (const [k, v] of Object.entries(obj)) map.set(Number(k), v);
    return map;
  } catch {
    return new Map();
  }
}

export function saveFingerprints(serverId: string, map: Map<number, string>) {
  const obj: Record<string, string> = {};
  for (const [k, v] of map) obj[String(k)] = v;
  localStorage.setItem(FP_KEY(serverId), JSON.stringify(obj));
}

export function deleteFingerprints(serverId: string) {
  localStorage.removeItem(FP_KEY(serverId));
}

export interface EncryptedPayload {
  ciphertext: string; // base64
  nonce: string;      // base64
}

export async function encrypt(
  plaintext: string,
  recipientPublicKeyB64: string,
  mySecretKeyB64: string,
): Promise<EncryptedPayload> {
  await ensureReady();
  const nonce = sodium.randombytes_buf(sodium.crypto_box_NONCEBYTES);
  const recipientPub = sodium.from_base64(recipientPublicKeyB64, sodium.base64_variants.ORIGINAL);
  const mySec = sodium.from_base64(mySecretKeyB64, sodium.base64_variants.ORIGINAL);
  const cipher = sodium.crypto_box_easy(
    sodium.from_string(plaintext),
    nonce,
    recipientPub,
    mySec,
  );
  return {
    ciphertext: sodium.to_base64(cipher, sodium.base64_variants.ORIGINAL),
    nonce: sodium.to_base64(nonce, sodium.base64_variants.ORIGINAL),
  };
}

/**
 * Tente de déchiffrer. Renvoie null en cas d'échec (clé changée, message corrompu, etc.).
 * On NE jette PAS d'exception : un DM non déchiffrable reste affichable comme tel dans l'UI.
 *
 * `peerPublicKeyB64` = la clé publique de l'AUTRE bout de la conversation :
 * - Pour un message entrant : la pub du sender.
 * - Pour un message sortant qu'on veut relire (après reload p.ex.) : la pub du destinataire.
 *   Grâce à la symétrie de Diffie-Hellman, `DH(peerPriv, myPub) == DH(myPriv, peerPub)` —
 *   donc on peut déchiffrer ses propres messages avec sa propre clé privée + la pub du destinataire.
 */
export async function decrypt(
  ciphertextB64: string,
  nonceB64: string,
  peerPublicKeyB64: string,
  mySecretKeyB64: string,
): Promise<string | null> {
  await ensureReady();
  try {
    const cipher = sodium.from_base64(ciphertextB64, sodium.base64_variants.ORIGINAL);
    const nonce = sodium.from_base64(nonceB64, sodium.base64_variants.ORIGINAL);
    const peerPub = sodium.from_base64(peerPublicKeyB64, sodium.base64_variants.ORIGINAL);
    const mySec = sodium.from_base64(mySecretKeyB64, sodium.base64_variants.ORIGINAL);
    const plain = sodium.crypto_box_open_easy(cipher, nonce, peerPub, mySec);
    return sodium.to_string(plain);
  } catch {
    return null;
  }
}
