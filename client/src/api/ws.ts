import type { Snapshot, SequencedEvent, WsConnectionState, WsConnection } from "./types";

/**
 * Cree une connexion WS avec :
 * - Backoff exponentiel (1s -> 2s -> 4s -> 8s -> max 30s)
 * - Tracking du seq number pour detecter les gaps
 * - Snapshot automatique a la connexion (envoye par le serveur)
 * - RequestSnapshot si gap detecte
 */
export function createWsConnection(
  baseUrl: string,
  getToken: () => string,
  callbacks: {
    onSnapshot: (snapshot: Snapshot) => void;
    onEvent: (event: SequencedEvent) => void;
    onStateChange: (state: WsConnectionState) => void;
  }
): WsConnection {
  let destroyed = false;
  let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

  const conn: WsConnection = {
    ws: null,
    state: "disconnected",
    lastSeq: 0,
    reconnectAttempt: 0,
    destroy: () => {
      destroyed = true;
      if (reconnectTimer) clearTimeout(reconnectTimer);
      conn.ws?.close();
      conn.ws = null;
      setState("disconnected");
    },
  };

  function setState(s: WsConnectionState) {
    conn.state = s;
    callbacks.onStateChange(s);
  }

  function connect() {
    if (destroyed) return;

    // Fermer proprement l'ancien WS avant d'en creer un nouveau
    if (conn.ws) {
      const old = conn.ws;
      old.onclose = null; // eviter que le onclose relance un reconnect
      old.onmessage = null;
      old.onerror = null;
      old.close();
      conn.ws = null;
    }

    const isReconnect = conn.reconnectAttempt > 0;
    setState(isReconnect ? "reconnecting" : "connecting");

    const url = new URL(baseUrl);
    const proto = url.protocol === "https:" ? "wss:" : "ws:";
    const ws = new WebSocket(`${proto}//${url.host}/ws?token=${getToken()}`);
    conn.ws = ws;

    ws.onopen = () => {
      conn.reconnectAttempt = 0;
      setState("connected");
    };

    ws.onmessage = (e) => {
      try {
        const msg = JSON.parse(e.data);

        // Snapshot (envoye automatiquement a la connexion ou sur RequestSnapshot)
        if (msg.type === "Snapshot") {
          const snapshot = msg.data as Snapshot;
          conn.lastSeq = snapshot.seq;
          callbacks.onSnapshot(snapshot);
          return;
        }

        // Event sequence normal
        const event = msg as SequencedEvent;
        if (event.seq !== undefined) {
          // Ignorer les events deja couverts par le snapshot
          if (event.seq <= conn.lastSeq) return;
          // Detecter un gap dans la sequence
          if (conn.lastSeq > 0 && event.seq > conn.lastSeq + 1) {
            console.warn(`[WS] Gap detecte: attendu ${conn.lastSeq + 1}, recu ${event.seq}. Demande de snapshot.`);
            ws.send(JSON.stringify({ type: "RequestSnapshot" }));
            return;
          }
          conn.lastSeq = event.seq;
        }

        callbacks.onEvent(event);
      } catch {
        // ignore malformed
      }
    };

    ws.onclose = () => {
      if (destroyed) return;
      conn.ws = null;
      scheduleReconnect();
    };

    ws.onerror = () => {
      // onclose sera appele apres
    };
  }

  function scheduleReconnect() {
    if (destroyed) return;
    setState("reconnecting");
    conn.reconnectAttempt++;
    // Backoff exponentiel: 1s, 2s, 4s, 8s, 16s, 30s max
    const delay = Math.min(1000 * Math.pow(2, conn.reconnectAttempt - 1), 30000);
    reconnectTimer = setTimeout(connect, delay);
  }

  // Connexion initiale
  connect();
  return conn;
}
