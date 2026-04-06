import {
  Room,
  RoomEvent,
  Track,
  TrackPublication,
  Participant,
  RemoteParticipant,
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

/**
 * Reactive media state — components can read this directly.
 * Updated automatically by LiveKit events + initial scan.
 */
export const mediaState = reactive({
  /** Audio elements keyed by participant identity */
  audioElements: new Map<string, HTMLAudioElement>(),
  /** Per-participant media info (camera, screen share) */
  participants: new Map<string, ParticipantMedia>(),
  /** Whether local audio output is deafened (all remote audio muted) */
  deafened: false,
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
  el.muted = mediaState.deafened;
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
  for (const el of mediaState.audioElements.values()) {
    el.muted = muted;
  }
}

function cleanupAllMedia() {
  for (const [identity] of mediaState.audioElements) {
    detachAudio(identity);
  }
  mediaState.audioElements.clear();
  mediaState.participants.clear();
  mediaState.deafened = false;
  mediaState.version++;
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
    mediaState.version++;
    callbacks.onParticipantLeft(participant.identity);
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
    await room.connect(url, token);

    // Apply saved audio devices
    const savedMic = localStorage.getItem("audioInputDevice");
    const savedSpeaker = localStorage.getItem("audioOutputDevice");
    if (savedMic) await room.switchActiveDevice("audioinput", savedMic);
    if (savedSpeaker) await room.switchActiveDevice("audiooutput", savedSpeaker);

    // Publish microphone
    await room.localParticipant.setMicrophoneEnabled(true);

    currentRoom = room;

    // Initial scan — catches tracks already published by participants who joined before us
    scanAllTracks(room);

    // Attach audio for already-subscribed remote tracks
    for (const participant of room.remoteParticipants.values()) {
      for (const pub of participant.audioTrackPublications.values()) {
        if (pub.track && pub.isSubscribed) {
          attachAudio(participant.identity, pub.track);
        }
      }
    }

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
