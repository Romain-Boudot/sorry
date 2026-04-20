import {
  Room,
  RoomEvent,
  Track,
  TrackPublication,
  Participant,
  RemoteParticipant,
  ConnectionQuality,
  RemoteTrackPublication,
} from "livekit-client";
import { reactive } from "vue";

// ══════════════════════════════════════
//  Media state — reactive, source of truth for all track info
// ══════════════════════════════════════

export interface ParticipantMedia {
  identity: string;
  name: string;
  camera: boolean;
  screenShare: boolean;
}

export type QualityLevel = "excellent" | "good" | "poor" | "unknown";

/**
 * Reactive media state — components can read this directly.
 * Updated automatically by LiveKit events + initial scan.
 */
export const mediaState = reactive({
  /** Audio elements keyed by participant identity */
  audioElements: new Map<string, HTMLAudioElement>(),
  /** Per-participant media info (camera, screen share) */
  participants: new Map<string, ParticipantMedia>(),
  /** Per-participant remote-audio mute (local-only — server-side audio still arrives). */
  mutedRemotes: new Set<string>(),
  /** Whether local audio output is deafened (all remote audio muted) */
  deafened: false,
  /** Screen-share streams the local user has opted-in to view.
   *  Audio + camera are auto-subscribed; screen shares require explicit opt-in
   *  to spare bandwidth. Key = participant identity. */
  watchedScreens: new Set<string>(),
  /** Per-participant connection quality (LiveKit ConnectionQualityChanged). */
  quality: new Map<string, QualityLevel>(),
  /** Incremented on any track change — triggers computed re-evaluation */
  version: 0,
});

// ══════════════════════════════════════
//  Audio management — attach/detach/mute without touching subscriptions
// ══════════════════════════════════════

function attachAudio(identity: string, track: any) {
  // Detach from any previous element
  const existing = mediaState.audioElements.get(identity);
  if (existing) {
    existing.srcObject = null;
    existing.remove();
  }

  const el = track.attach() as HTMLAudioElement;
  el.muted = mediaState.deafened || mediaState.mutedRemotes.has(identity);
  el.dataset.identity = identity;
  document.body.appendChild(el);
  mediaState.audioElements.set(identity, el);
}

function detachAudio(identity: string) {
  const el = mediaState.audioElements.get(identity);
  if (el) {
    el.srcObject = null;
    el.remove();
    mediaState.audioElements.delete(identity);
  }
}

function setAllAudioMuted(muted: boolean) {
  mediaState.deafened = muted;
  for (const [identity, el] of mediaState.audioElements) {
    el.muted = muted || mediaState.mutedRemotes.has(identity);
  }
}

function cleanupAllMedia() {
  for (const [identity] of mediaState.audioElements) {
    detachAudio(identity);
  }
  mediaState.audioElements.clear();
  mediaState.participants.clear();
  mediaState.mutedRemotes.clear();
  mediaState.watchedScreens.clear();
  mediaState.quality.clear();
  mediaState.deafened = false;
  mediaState.version++;
}

// ══════════════════════════════════════
//  Auto-subscribe policy: audio + camera always; screen-share opt-in.
// ══════════════════════════════════════

/** Should this publication be auto-subscribed when published? */
function shouldAutoSubscribe(pub: TrackPublication): boolean {
  if (pub.kind === Track.Kind.Audio) return true;
  if (pub.kind === Track.Kind.Video && pub.source === Track.Source.Camera) return true;
  return false; // screen shares wait for explicit watchScreen()
}

/** Subscribe to all "always-on" tracks (audio + camera) currently published by remotes.
 *  Called once at connect time; new publications are handled by the TrackPublished event. */
function autoSubscribeExisting(room: Room) {
  for (const participant of room.remoteParticipants.values()) {
    for (const pub of participant.trackPublications.values()) {
      const rpub = pub as RemoteTrackPublication;
      if (shouldAutoSubscribe(rpub) && !rpub.isSubscribed) {
        rpub.setSubscribed(true);
      }
    }
  }
}

/** Public API: opt-in to a screen share. Returns true if the publication exists. */
export function watchScreen(identity: string): boolean {
  if (!currentRoom) return false;
  const participant = currentRoom.remoteParticipants.get(identity);
  if (!participant) return false;
  for (const pub of participant.trackPublications.values()) {
    if (pub.source === Track.Source.ScreenShare) {
      (pub as RemoteTrackPublication).setSubscribed(true);
      mediaState.watchedScreens.add(identity);
      mediaState.version++;
      return true;
    }
  }
  return false;
}

/** Public API: stop receiving a screen share (frees bandwidth). */
export function unwatchScreen(identity: string) {
  if (!currentRoom) {
    mediaState.watchedScreens.delete(identity);
    return;
  }
  const participant = currentRoom.remoteParticipants.get(identity);
  if (participant) {
    for (const pub of participant.trackPublications.values()) {
      if (pub.source === Track.Source.ScreenShare) {
        (pub as RemoteTrackPublication).setSubscribed(false);
      }
    }
  }
  mediaState.watchedScreens.delete(identity);
  mediaState.version++;
}

/** Toggle local-only mute for one remote participant's audio. */
export function toggleRemoteMute(identity: string): boolean {
  const isMuted = mediaState.mutedRemotes.has(identity);
  const next = !isMuted;
  if (next) mediaState.mutedRemotes.add(identity);
  else mediaState.mutedRemotes.delete(identity);

  const el = mediaState.audioElements.get(identity);
  if (el) el.muted = next || mediaState.deafened;
  mediaState.version++;
  return next;
}

// ══════════════════════════════════════
//  Track scanning — build media state from current room
// ══════════════════════════════════════

function scanParticipantMedia(identity: string, name: string, pubs: Iterable<TrackPublication>) {
  let camera = false;
  let screenShare = false;

  for (const pub of pubs) {
    if (pub.kind !== Track.Kind.Video) continue;
    if (pub.isMuted) continue;
    if (pub.source === Track.Source.ScreenShare) {
      screenShare = true;
    } else if (pub.source === Track.Source.Camera) {
      camera = true;
    }
  }

  mediaState.participants.set(identity, { identity, name, camera, screenShare });
}

/** Full scan of all participants — called after connect and on any track change. */
function scanAllTracks(room: Room) {
  mediaState.participants.clear();

  // Local
  const local = room.localParticipant;
  scanParticipantMedia(local.identity, local.name || local.identity, local.trackPublications.values());

  // Remote
  for (const p of room.remoteParticipants.values()) {
    scanParticipantMedia(p.identity, p.name || p.identity, p.trackPublications.values());
  }

  mediaState.version++;
}

// ══════════════════════════════════════
//  Room lifecycle
// ══════════════════════════════════════

let currentRoom: Room | null = null;

export interface VoiceCallbacks {
  onConnected: (room: Room) => void;
  onDisconnected: () => void;
  onParticipantJoined: (identity: string, name: string) => void;
  onParticipantLeft: (identity: string) => void;
  onActiveSpeakersChanged: (identities: string[]) => void;
  onTrackChanged?: () => void;
  onError: (error: string) => void;
}

/** Try to switch to the device id saved in localStorage; on failure, drop the entry. */
async function applySavedDevice(
  room: Room,
  kind: MediaDeviceKind,
  storageKey: string,
) {
  const saved = localStorage.getItem(storageKey);
  if (!saved) return;
  try {
    await room.switchActiveDevice(kind, saved);
  } catch (e) {
    console.warn(`[voice] Saved ${kind} device "${saved}" unavailable, clearing.`, e);
    localStorage.removeItem(storageKey);
  }
}

export async function joinVoice(
  url: string,
  token: string,
  callbacks: VoiceCallbacks
): Promise<Room> {
  await leaveVoice();

  const room = new Room({
    adaptiveStream: true,
    dynacast: true,
  });

  // We manage subscriptions ourselves: audio + camera auto, screen share opt-in.
  // (Set on the connect call below since `autoSubscribe` is a connect option.)

  room.on(RoomEvent.Disconnected, () => {
    currentRoom = null;
    cleanupAllMedia();
    callbacks.onDisconnected();
  });

  room.on(RoomEvent.ParticipantConnected, (participant: RemoteParticipant) => {
    callbacks.onParticipantJoined(participant.identity, participant.name || participant.identity);
  });

  room.on(RoomEvent.ParticipantDisconnected, (participant: RemoteParticipant) => {
    detachAudio(participant.identity);
    mediaState.participants.delete(participant.identity);
    mediaState.watchedScreens.delete(participant.identity);
    mediaState.mutedRemotes.delete(participant.identity);
    mediaState.quality.delete(participant.identity);
    mediaState.version++;
    callbacks.onParticipantLeft(participant.identity);
  });

  // ── Track published (autoSubscribe:false → we choose what to subscribe) ──
  room.on(RoomEvent.TrackPublished, (publication, participant) => {
    if (shouldAutoSubscribe(publication)) {
      publication.setSubscribed(true);
    }
    // Refresh the discovery list so the UI shows the screen-share badge
    // even before the user opts-in to view it.
    scanParticipantMedia(participant.identity, participant.name || participant.identity, participant.trackPublications.values());
    mediaState.version++;
    callbacks.onTrackChanged?.();
  });

  room.on(RoomEvent.TrackUnpublished, (publication, participant) => {
    // If a watched screen-share went away, drop it from the watch set.
    if (publication.source === Track.Source.ScreenShare) {
      mediaState.watchedScreens.delete(participant.identity);
    }
    scanParticipantMedia(participant.identity, participant.name || participant.identity, participant.trackPublications.values());
    mediaState.version++;
    callbacks.onTrackChanged?.();
  });

  // ── Connection quality (per participant) ──
  room.on(RoomEvent.ConnectionQualityChanged, (quality, participant) => {
    let level: QualityLevel = "unknown";
    if (quality === ConnectionQuality.Excellent) level = "excellent";
    else if (quality === ConnectionQuality.Good) level = "good";
    else if (quality === ConnectionQuality.Poor) level = "poor";
    mediaState.quality.set(participant.identity, level);
    mediaState.version++;
  });

  room.on(RoomEvent.ActiveSpeakersChanged, (speakers: Participant[]) => {
    callbacks.onActiveSpeakersChanged(speakers.map((s) => s.identity));
  });

  // ── Track subscribed: attach audio, update media state ──
  room.on(RoomEvent.TrackSubscribed, (track, _publication, participant) => {
    if (track.kind === Track.Kind.Audio) {
      attachAudio(participant.identity, track);
    }
    // Rescan this participant's media
    scanParticipantMedia(participant.identity, participant.name || participant.identity, participant.trackPublications.values());
    mediaState.version++;
    callbacks.onTrackChanged?.();
  });

  room.on(RoomEvent.TrackUnsubscribed, (track, _publication, participant) => {
    if (track.kind === Track.Kind.Audio) {
      detachAudio(participant.identity);
    }
    track.detach().forEach((el) => el.remove());
    if (participant) {
      scanParticipantMedia(participant.identity, participant.name || participant.identity, participant.trackPublications.values());
    }
    mediaState.version++;
    callbacks.onTrackChanged?.();
  });

  // ── Track mute/unmute: update media state (screen share stop = muted) ──
  room.on(RoomEvent.TrackMuted, (_publication, participant) => {
    if (participant) {
      scanParticipantMedia(participant.identity, participant.name || participant.identity, participant.trackPublications.values());
      mediaState.version++;
      callbacks.onTrackChanged?.();
    }
  });

  room.on(RoomEvent.TrackUnmuted, (_publication, participant) => {
    if (participant) {
      scanParticipantMedia(participant.identity, participant.name || participant.identity, participant.trackPublications.values());
      mediaState.version++;
      callbacks.onTrackChanged?.();
    }
  });

  // ── Local track publish/unpublish ──
  room.on(RoomEvent.LocalTrackPublished, () => {
    scanAllTracks(room);
    callbacks.onTrackChanged?.();
  });
  room.on(RoomEvent.LocalTrackUnpublished, () => {
    scanAllTracks(room);
    callbacks.onTrackChanged?.();
  });

  try {
    await room.connect(url, token, { autoSubscribe: false });

    // Apply saved audio devices.
    // A device ID stored from another browser/profile (or after the user unplugged
    // the mic) won't match anything here — fall back to the system default and
    // clear the stale entry so we don't fail the whole connect flow.
    await applySavedDevice(room, "audioinput", "audioInputDevice");
    await applySavedDevice(room, "audiooutput", "audioOutputDevice");

    // Publish microphone
    await room.localParticipant.setMicrophoneEnabled(true);

    currentRoom = room;

    // Initial scan — catches tracks already published by participants who joined before us.
    scanAllTracks(room);

    // Subscribe audio + camera for already-published remote tracks (TrackPublished
    // doesn't fire for pre-existing pubs at connect time).
    autoSubscribeExisting(room);

    callbacks.onConnected(room);
    return room;
  } catch (e: any) {
    callbacks.onError(e.message || "Erreur de connexion vocale");
    throw e;
  }
}

export async function leaveVoice() {
  if (currentRoom) {
    await currentRoom.disconnect();
    currentRoom = null;
  }
  cleanupAllMedia();
}

// ══════════════════════════════════════
//  Microphone controls
// ══════════════════════════════════════

export function toggleMute(): boolean {
  if (!currentRoom) return false;
  const mic = currentRoom.localParticipant.isMicrophoneEnabled;
  currentRoom.localParticipant.setMicrophoneEnabled(!mic);
  return !mic;
}

export function setMuted(muted: boolean) {
  if (!currentRoom) return;
  currentRoom.localParticipant.setMicrophoneEnabled(!muted);
}

// ══════════════════════════════════════
//  Deafen — mute audio elements, keep subscriptions alive
// ══════════════════════════════════════

export function toggleDeafen(): boolean {
  const next = !mediaState.deafened;
  setAllAudioMuted(next);
  return next;
}

export function setDeafened(deafened: boolean) {
  setAllAudioMuted(deafened);
}

// ══════════════════════════════════════
//  Screen share
// ══════════════════════════════════════

export async function startScreenShare(): Promise<boolean> {
  if (!currentRoom) return false;
  try {
    await currentRoom.localParticipant.setScreenShareEnabled(true, {
      resolution: { width: 1920, height: 1080, frameRate: 30 },
      contentHint: "detail",
    });
    return true;
  } catch {
    return false;
  }
}

export async function stopScreenShare(): Promise<void> {
  if (!currentRoom) return;
  await currentRoom.localParticipant.setScreenShareEnabled(false);
}

export function isScreenSharing(): boolean {
  if (!currentRoom) return false;
  return currentRoom.localParticipant.isScreenShareEnabled;
}

// ══════════════════════════════════════
//  Webcam
// ══════════════════════════════════════

export async function setCameraEnabled(enabled: boolean): Promise<boolean> {
  if (!currentRoom) return false;
  try {
    await currentRoom.localParticipant.setCameraEnabled(enabled);
    return true;
  } catch {
    return false;
  }
}

export function isCameraEnabled(): boolean {
  if (!currentRoom) return false;
  return currentRoom.localParticipant.isCameraEnabled;
}

export async function getVideoDevices(): Promise<MediaDeviceInfo[]> {
  try {
    return await Room.getLocalDevices("videoinput");
  } catch {
    return [];
  }
}

export async function switchCamera(deviceId: string) {
  if (!currentRoom) return;
  await currentRoom.switchActiveDevice("videoinput", deviceId);
}

// ══════════════════════════════════════
//  Audio devices
// ══════════════════════════════════════

export function isConnected(): boolean {
  return currentRoom !== null && currentRoom.state === "connected";
}

export function getCurrentRoom(): Room | null {
  return currentRoom;
}

export async function getAudioDevices(): Promise<{ inputs: MediaDeviceInfo[]; outputs: MediaDeviceInfo[] }> {
  try {
    const devices = await Room.getLocalDevices("audioinput");
    const outputs = await Room.getLocalDevices("audiooutput");
    return { inputs: devices, outputs };
  } catch {
    return { inputs: [], outputs: [] };
  }
}

export async function switchMicrophone(deviceId: string) {
  if (!currentRoom) return;
  await currentRoom.switchActiveDevice("audioinput", deviceId);
}

export async function switchSpeaker(deviceId: string) {
  if (!currentRoom) return;
  await currentRoom.switchActiveDevice("audiooutput", deviceId);
}

export function setMicEnabled(enabled: boolean) {
  if (!currentRoom) return;
  currentRoom.localParticipant.setMicrophoneEnabled(enabled);
}

// ══════════════════════════════════════
//  Per-track stats (FPS / resolution / bitrate)
// ══════════════════════════════════════

export interface TrackStats {
  width: number;
  height: number;
  fps: number;
  bitrateKbps: number;
}

const lastBytes = new Map<string, { bytes: number; ts: number }>();

/** Compute live stats for a local or remote video track (camera / screen share).
 *  Local tracks report outbound-rtp (what we publish), remote tracks report
 *  inbound-rtp (what we receive). */
export async function getTrackStats(identity: string, source: "camera" | "screen_share"): Promise<TrackStats | null> {
  if (!currentRoom) return null;
  const wantedSource = source === "screen_share" ? Track.Source.ScreenShare : Track.Source.Camera;

  const isLocal = identity === currentRoom.localParticipant.identity;
  let webrtcEndpoint: RTCRtpReceiver | RTCRtpSender | undefined;
  let inbound = false;

  if (isLocal) {
    for (const p of currentRoom.localParticipant.videoTrackPublications.values()) {
      if (p.source === wantedSource && p.track) {
        webrtcEndpoint = (p.track as any).sender as RTCRtpSender | undefined;
        break;
      }
    }
  } else {
    const participant = currentRoom.remoteParticipants.get(identity);
    if (!participant) return null;
    for (const p of participant.trackPublications.values()) {
      if (p.kind === Track.Kind.Video && p.source === wantedSource) {
        webrtcEndpoint = ((p as RemoteTrackPublication).track as any)?.receiver as RTCRtpReceiver | undefined;
        inbound = true;
        break;
      }
    }
  }

  if (!webrtcEndpoint) return null;

  try {
    const report = await webrtcEndpoint.getStats();
    let width = 0, height = 0, fps = 0, bytes = 0;
    report.forEach((r: any) => {
      const targetType = inbound ? "inbound-rtp" : "outbound-rtp";
      if (r.type === targetType && r.kind === "video") {
        // Outbound simulcast: multiple reports exist (one per layer); keep the
        // highest resolution as "the" published stream.
        if ((r.frameWidth ?? 0) >= width) {
          width = r.frameWidth ?? width;
          height = r.frameHeight ?? height;
          fps = Math.round(r.framesPerSecond ?? fps);
        }
        bytes += inbound ? (r.bytesReceived ?? 0) : (r.bytesSent ?? 0);
      }
    });

    const key = `${identity}:${source}`;
    const now = performance.now();
    const prev = lastBytes.get(key);
    let bitrateKbps = 0;
    if (prev && now > prev.ts) {
      const deltaBytes = bytes - prev.bytes;
      const deltaSecs = (now - prev.ts) / 1000;
      if (deltaSecs > 0) bitrateKbps = Math.round((deltaBytes * 8) / deltaSecs / 1000);
    }
    lastBytes.set(key, { bytes, ts: now });

    return { width, height, fps, bitrateKbps };
  } catch {
    return null;
  }
}

// ══════════════════════════════════════
//  WebRTC connection stats
// ══════════════════════════════════════

export interface ConnectionStats {
  transport: "udp" | "tcp" | "turn-udp" | "turn-tcp" | "unknown";
  localAddress: string;
  remoteAddress: string;
  rtt: number;
}

export async function getConnectionStats(): Promise<ConnectionStats | null> {
  if (!currentRoom) return null;

  try {
    const engine = (currentRoom as any).engine;
    const pc: RTCPeerConnection | undefined =
      engine?.pcManager?.publisher?.pc ??
      engine?.publisher?.pc ??
      engine?.pcManager?.subscriber?.pc;

    if (!pc) return null;

    const stats = await pc.getStats();
    let selectedPair: any = null;
    const candidates = new Map<string, any>();

    stats.forEach((report: any) => {
      if (report.type === "local-candidate" || report.type === "remote-candidate") {
        candidates.set(report.id, report);
      }
      if (report.type === "candidate-pair" && (report.nominated || report.state === "succeeded")) {
        if (!selectedPair || report.nominated) selectedPair = report;
      }
    });

    if (!selectedPair) return null;

    const local = candidates.get(selectedPair.localCandidateId);
    const remote = candidates.get(selectedPair.remoteCandidateId);

    const protocol = (local?.protocol || "unknown").toLowerCase();
    const isRelay = local?.candidateType === "relay";

    let transport: ConnectionStats["transport"] = "unknown";
    if (isRelay && protocol === "udp") transport = "turn-udp";
    else if (isRelay && protocol === "tcp") transport = "turn-tcp";
    else if (isRelay) transport = "turn-tcp";
    else if (protocol === "udp") transport = "udp";
    else if (protocol === "tcp") transport = "tcp";

    return {
      transport,
      localAddress: local ? `${local.address}:${local.port}` : "",
      remoteAddress: remote ? `${remote.address}:${remote.port}` : "",
      rtt: selectedPair.currentRoundTripTime ? Math.round(selectedPair.currentRoundTripTime * 1000) : 0,
    };
  } catch {
    return null;
  }
}
