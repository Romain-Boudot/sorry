import {
  Room,
  RoomEvent,
  Track,
  TrackPublication,
  Participant,
  RemoteParticipant,
  ConnectionQuality,
  RemoteTrackPublication,
  VideoQuality,
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

export type ViewQuality = "auto" | "high" | "medium" | "low";

/** Preferred viewer quality per (identity, source). "auto" = adaptive. */
export const watchQuality = new Map<string, ViewQuality>();

/** Set the quality preference for a remote video track we're subscribed to.
 *  - "auto" resets to HIGH (adaptive SDK can still pick lower for small tiles)
 *  - "high" / "medium" / "low" caps what the SFU sends us */
export function setViewQuality(identity: string, source: "camera" | "screen_share", q: ViewQuality) {
  if (!currentRoom) return;
  const wanted = source === "screen_share" ? Track.Source.ScreenShare : Track.Source.Camera;
  const participant = currentRoom.remoteParticipants.get(identity);
  if (!participant) return;
  for (const pub of participant.trackPublications.values()) {
    if (pub.kind !== Track.Kind.Video || pub.source !== wanted) continue;
    const rpub = pub as RemoteTrackPublication;
    const mapped = q === "low" ? VideoQuality.LOW : q === "medium" ? VideoQuality.MEDIUM : VideoQuality.HIGH;
    try { rpub.setVideoQuality(mapped); } catch {}
    break;
  }
  watchQuality.set(`${identity}:${source}`, q);
  mediaState.version++;
}

export function getViewQuality(identity: string, source: "camera" | "screen_share"): ViewQuality {
  return watchQuality.get(`${identity}:${source}`) ?? "auto";
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
    } else if (
      publication.source === Track.Source.ScreenShare &&
      mediaState.watchedScreens.has(participant.identity)
    ) {
      // Publisher re-published their screen (quality change, etc.) — we still
      // want to watch, so re-subscribe without requiring a user click.
      publication.setSubscribed(true);
    }
    scanParticipantMedia(participant.identity, participant.name || participant.identity, participant.trackPublications.values());
    mediaState.version++;
    callbacks.onTrackChanged?.();
  });

  room.on(RoomEvent.TrackUnpublished, (_publication, participant) => {
    // We intentionally keep `watchedScreens` populated even when a screen share
    // is unpublished: if the publisher republishes (quality change), we want to
    // auto-resubscribe. The entry is cleared on participant disconnect.
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

import { streamSettings, RESOLUTION_DIMS, type StreamPreset } from "./streamSettings";

// We always request the MAXIMUM capture resolution + framerate at start so
// `applyConstraints` can freely downscale later. Upscaling above the initial
// capture isn't possible — the encoder never sees frames larger/faster than
// what the source delivered. Browsers/cameras use `ideal` constraints here,
// so they silently fall back to what the source can actually produce.
const CAPTURE_MAX_SCREEN = { width: 3840, height: 2160, frameRate: 60 };
const CAPTURE_MAX_CAMERA = { width: 1920, height: 1080, frameRate: 60 };

function screenOpts(preset: StreamPreset) {
  return {
    capture: { resolution: CAPTURE_MAX_SCREEN, contentHint: preset.contentHint },
    publish: {
      videoEncoding: { maxBitrate: preset.bitrateKbps * 1000, maxFramerate: preset.fps },
      simulcast: preset.simulcast,
    },
  };
}

function cameraOpts(preset: StreamPreset) {
  return {
    capture: { resolution: CAPTURE_MAX_CAMERA },
    publish: {
      videoEncoding: { maxBitrate: preset.bitrateKbps * 1000, maxFramerate: preset.fps },
      simulcast: preset.simulcast,
    },
  };
}

export async function startScreenShare(): Promise<boolean> {
  if (!currentRoom) return false;
  try {
    const opts = screenOpts(streamSettings.screen.preset);
    await currentRoom.localParticipant.setScreenShareEnabled(true, opts.capture, opts.publish);
    // Apply our custom encoder params (degradationPreference, per-encoding
    // maxFramerate, simulcast layer toggle). LiveKit's publish options don't
    // expose degradationPreference, so without this call the initial publish
    // runs with conservative defaults and FPS gets choked.
    await applyLive(Track.Source.ScreenShare, streamSettings.screen.preset);
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
    if (enabled) {
      const opts = cameraOpts(streamSettings.camera.preset);
      await currentRoom.localParticipant.setCameraEnabled(true, opts.capture, opts.publish);
      // Apply degradationPreference / per-encoding maxFramerate / simulcast toggle
      // — none of these are exposed in LiveKit's publishOptions, so we layer
      // them on after the publish settles.
      await applyLive(Track.Source.Camera, streamSettings.camera.preset);
    } else {
      await currentRoom.localParticipant.setCameraEnabled(false);
    }
    return true;
  } catch {
    return false;
  }
}

export function isCameraEnabled(): boolean {
  if (!currentRoom) return false;
  return currentRoom.localParticipant.isCameraEnabled;
}

// ══════════════════════════════════════
//  Live stream settings apply
// ══════════════════════════════════════
//
// Changes are applied on the already-published track via:
//  - `MediaStreamTrack.applyConstraints(...)` for resolution + FPS
//  - `track.contentHint` for the encoder hint
//  - `RTCRtpSender.setParameters({ encodings })` for the max bitrate
//
// This avoids re-publishing the track, which for screen shares would re-open
// the browser's native source picker (bad UX) and cause a hard interruption
// on the viewer side. The downside: we can't upscale ABOVE the original
// capture resolution — if the user started in 1080p and wants 4K, they need
// to stop and restart the share manually. Downscaling always works.

export async function applyCameraSettings(): Promise<void> {
  if (!currentRoom || !isCameraEnabled()) return;
  await applyLive(Track.Source.Camera, streamSettings.camera.preset);
}

export async function applyScreenSettings(): Promise<void> {
  if (!currentRoom || !isScreenSharing()) return;
  await applyLive(Track.Source.ScreenShare, streamSettings.screen.preset);
}

async function applyLive(source: Track.Source, preset: StreamPreset) {
  if (!currentRoom) return;
  const { width, height } = RESOLUTION_DIMS[preset.resolution];

  for (const pub of currentRoom.localParticipant.videoTrackPublications.values()) {
    if (pub.source !== source || !pub.track) continue;

    const mediaTrack = (pub.track as any).mediaStreamTrack as MediaStreamTrack | undefined;
    if (mediaTrack) {
      try {
        await mediaTrack.applyConstraints({
          width: { ideal: width },
          height: { ideal: height },
          frameRate: { ideal: preset.fps },
        });
      } catch (e) {
        console.warn("[voice] applyConstraints failed", e);
      }
      if ("contentHint" in mediaTrack) {
        try { (mediaTrack as any).contentHint = preset.contentHint; } catch {}
      }
    }

    const sender = (pub.track as any).sender as RTCRtpSender | undefined;
    if (sender) {
      try {
        const params = sender.getParameters();
        if (params.encodings && params.encodings.length) {
          // Find the highest-resolution layer (smallest scaleResolutionDownBy or first one).
          const highIdx = params.encodings.reduce((best, enc, i, arr) => {
            const a = enc.scaleResolutionDownBy ?? 1;
            const b = arr[best].scaleResolutionDownBy ?? 1;
            return a < b ? i : best;
          }, 0);
          for (let i = 0; i < params.encodings.length; i++) {
            const enc = params.encodings[i];
            enc.maxBitrate = preset.bitrateKbps * 1000;
            enc.maxFramerate = preset.fps;
            // Simulcast OFF = only the highest layer stays active, others are
            // paused at the sender level (no bytes sent, CPU freed up).
            enc.active = preset.simulcast ? true : i === highIdx;
          }
          // degradationPreference tells the encoder what to sacrifice when
          // bandwidth/CPU is tight. Map from contentHint:
          //   - motion → maintain-framerate (drop resolution to keep smooth video)
          //   - detail / text → maintain-resolution (drop fps to keep text sharp)
          (params as any).degradationPreference =
            preset.contentHint === "motion" ? "maintain-framerate" : "maintain-resolution";
          await sender.setParameters(params);
        }
      } catch (e) {
        console.warn("[voice] setParameters failed", e);
      }
    }
  }
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
//  Per-track stats (FPS / resolution / bitrate / encoder health)
// ══════════════════════════════════════

export type LimitationReason = "none" | "cpu" | "bandwidth" | "other";

export interface LayerStats {
  /** Simulcast layer id (rid) — "q"/"h"/"f" or numeric. Undefined = single-layer. */
  rid?: string;
  width: number;
  height: number;
  fps: number;
  bitrateKbps: number;
  /** False if dynacast paused this layer or encoding.active = false. */
  active: boolean;
}

export interface TrackStats {
  /** Aggregate view (highest layer's resolution/fps, total bitrate across layers). */
  width: number;
  height: number;
  fps: number;
  bitrateKbps: number;
  /** Per-simulcast-layer breakdown. 1 entry for single-layer, 2-3 for simulcast. */
  layers: LayerStats[];

  // ── Publisher-only (sender) diagnostics ──
  /** Why the encoder is underperforming, or "none" when healthy. */
  limitationReason?: LimitationReason;
  /** Encoder implementation name, e.g. "libvpx", "ExternalEncoder" (hw), "OpenH264". */
  encoderImplementation?: string;
  /** Mean time spent encoding one frame (ms) — indicator of CPU load. */
  encodeMsPerFrame?: number;
  /** PLI/NACK/FIR counters received FROM viewers — signals of packet loss. */
  pliCount?: number;
  nackCount?: number;
  firCount?: number;
  /** Target bitrate LiveKit/BWE is currently trying to hit (kbps, sum of layers). */
  targetBitrateKbps?: number;
  /** What the OS/browser actually captures from the source (publisher only).
   *  If `captureFps` < requested, the browser/source is the bottleneck — not
   *  the encoder, not the network. Common case: Chrome on Windows caps screen
   *  capture to 30 FPS regardless of constraints. */
  captureWidth?: number;
  captureHeight?: number;
  captureFps?: number;
}

const lastBytes = new Map<string, { bytes: number; ts: number }>();
/** Per-layer bytes for accurate per-layer bitrate deltas. */
const lastLayerBytes = new Map<string, { bytes: number; ts: number }>();
/** Per-sender totalEncodeTime + framesEncoded for computing encode-time-per-frame deltas. */
const lastEncode = new Map<string, { time: number; frames: number }>();

function deltaBitrate(key: string, bytes: number, now: number): number {
  const prev = lastBytes.get(key);
  lastBytes.set(key, { bytes, ts: now });
  if (!prev || now <= prev.ts) return 0;
  const deltaBytes = bytes - prev.bytes;
  const deltaSecs = (now - prev.ts) / 1000;
  return deltaSecs > 0 ? Math.round((deltaBytes * 8) / deltaSecs / 1000) : 0;
}

/** Compute live stats for a local or remote video track (camera / screen share).
 *  Local tracks report outbound-rtp (publish), remote tracks report inbound-rtp. */
export async function getTrackStats(identity: string, source: "camera" | "screen_share"): Promise<TrackStats | null> {
  if (!currentRoom) return null;
  const wantedSource = source === "screen_share" ? Track.Source.ScreenShare : Track.Source.Camera;

  const isLocal = identity === currentRoom.localParticipant.identity;
  let webrtcEndpoint: RTCRtpReceiver | RTCRtpSender | undefined;
  let sender: RTCRtpSender | undefined;
  let inbound = false;

  let captureSettings: MediaTrackSettings | undefined;

  if (isLocal) {
    for (const p of currentRoom.localParticipant.videoTrackPublications.values()) {
      if (p.source === wantedSource && p.track) {
        sender = (p.track as any).sender as RTCRtpSender | undefined;
        webrtcEndpoint = sender;
        const mst = (p.track as any).mediaStreamTrack as MediaStreamTrack | undefined;
        if (mst) {
          try { captureSettings = mst.getSettings(); } catch {}
        }
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
    const now = performance.now();
    const layers: LayerStats[] = [];
    let totalBytes = 0;
    let aggWidth = 0, aggHeight = 0, aggFps = 0;

    // Encoder / health stats (outbound only)
    let limitationReason: LimitationReason | undefined;
    let encoderImpl: string | undefined;
    let totalEncodeTime = 0;
    let framesEncoded = 0;
    let pliCount = 0, nackCount = 0, firCount = 0;
    let targetBitrate = 0;

    const targetType = inbound ? "inbound-rtp" : "outbound-rtp";

    report.forEach((r: any) => {
      if (r.type === targetType && r.kind === "video") {
        const w = r.frameWidth ?? 0;
        const h = r.frameHeight ?? 0;
        const f = Math.round(r.framesPerSecond ?? 0);
        const bytes = inbound ? (r.bytesReceived ?? 0) : (r.bytesSent ?? 0);

        const layerKey = `${identity}:${source}:${r.rid ?? r.ssrc ?? "single"}`;
        const layerBr = deltaLayerBitrate(layerKey, bytes, now);

        layers.push({
          rid: r.rid,
          width: w,
          height: h,
          fps: f,
          bitrateKbps: layerBr,
          active: r.active !== false,
        });

        totalBytes += bytes;
        if (w >= aggWidth) {
          aggWidth = w;
          aggHeight = h;
          aggFps = f;
        }

        if (!inbound) {
          limitationReason ??= (r.qualityLimitationReason as LimitationReason | undefined);
          encoderImpl ??= (r.encoderImplementation as string | undefined);
          totalEncodeTime += r.totalEncodeTime ?? 0;
          framesEncoded += r.framesEncoded ?? 0;
          pliCount += r.pliCount ?? 0;
          nackCount += r.nackCount ?? 0;
          firCount += r.firCount ?? 0;
          targetBitrate += r.targetBitrate ?? 0;
        } else {
          // Inbound NACK/PLI counters are still useful on the viewer side for diagnostics
          pliCount += r.pliCount ?? 0;
          nackCount += r.nackCount ?? 0;
          firCount += r.firCount ?? 0;
        }
      }
    });

    const aggKey = `${identity}:${source}`;
    const bitrateKbps = deltaBitrate(aggKey, totalBytes, now);

    // Per-frame encode time delta (publisher only).
    let encodeMsPerFrame: number | undefined;
    if (!inbound && sender) {
      const encKey = `${identity}:${source}:enc`;
      const prev = lastEncode.get(encKey);
      lastEncode.set(encKey, { time: totalEncodeTime, frames: framesEncoded });
      if (prev) {
        const df = framesEncoded - prev.frames;
        const dt = totalEncodeTime - prev.time;
        if (df > 0) encodeMsPerFrame = Math.round((dt / df) * 1000);
      }
    }

    const stats: TrackStats = {
      width: aggWidth,
      height: aggHeight,
      fps: aggFps,
      bitrateKbps,
      layers: layers.sort((a, b) => b.width - a.width),
    };

    if (!inbound) {
      stats.limitationReason = limitationReason ?? "none";
      stats.encoderImplementation = encoderImpl;
      stats.encodeMsPerFrame = encodeMsPerFrame;
      stats.pliCount = pliCount;
      stats.nackCount = nackCount;
      stats.firCount = firCount;
      if (targetBitrate > 0) stats.targetBitrateKbps = Math.round(targetBitrate / 1000);
      if (captureSettings) {
        stats.captureWidth = captureSettings.width;
        stats.captureHeight = captureSettings.height;
        stats.captureFps = captureSettings.frameRate ? Math.round(captureSettings.frameRate) : undefined;
      }
    } else {
      stats.pliCount = pliCount;
      stats.nackCount = nackCount;
      stats.firCount = firCount;
    }
    return stats;
  } catch {
    return null;
  }
}

function deltaLayerBitrate(key: string, bytes: number, now: number): number {
  const prev = lastLayerBytes.get(key);
  lastLayerBytes.set(key, { bytes, ts: now });
  if (!prev || now <= prev.ts) return 0;
  const deltaBytes = bytes - prev.bytes;
  const deltaSecs = (now - prev.ts) / 1000;
  return deltaSecs > 0 ? Math.round((deltaBytes * 8) / deltaSecs / 1000) : 0;
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
