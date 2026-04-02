import {
  Room,
  RoomEvent,
  Track,
  Participant,
  RemoteParticipant,
} from "livekit-client";

let currentRoom: Room | null = null;

export interface VoiceCallbacks {
  onConnected: (room: Room) => void;
  onDisconnected: () => void;
  onParticipantJoined: (identity: string, name: string) => void;
  onParticipantLeft: (identity: string) => void;
  onActiveSpeakersChanged: (identities: string[]) => void;
  onError: (error: string) => void;
}

export async function joinVoice(
  url: string,
  token: string,
  callbacks: VoiceCallbacks
): Promise<Room> {
  // Quitter la room actuelle si déjà connecté
  await leaveVoice();

  const room = new Room({
    adaptiveStream: true,
    dynacast: true,
  });

  room.on(RoomEvent.Disconnected, () => {
    currentRoom = null;
    callbacks.onDisconnected();
  });

  room.on(
    RoomEvent.ParticipantConnected,
    (participant: RemoteParticipant) => {
      callbacks.onParticipantJoined(
        participant.identity,
        participant.name || participant.identity
      );
    }
  );

  room.on(
    RoomEvent.ParticipantDisconnected,
    (participant: RemoteParticipant) => {
      callbacks.onParticipantLeft(participant.identity);
    }
  );

  room.on(
    RoomEvent.ActiveSpeakersChanged,
    (speakers: Participant[]) => {
      callbacks.onActiveSpeakersChanged(speakers.map((s) => s.identity));
    }
  );

  // Auto-subscribe aux tracks audio des autres
  room.on(
    RoomEvent.TrackSubscribed,
    (track, _publication, _participant) => {
      if (track.kind === Track.Kind.Audio) {
        const el = track.attach();
        document.body.appendChild(el);
      }
    }
  );

  room.on(
    RoomEvent.TrackUnsubscribed,
    (track, _publication, _participant) => {
      track.detach().forEach((el) => el.remove());
    }
  );

  try {
    await room.connect(url, token);

    // Apply saved audio devices
    const savedMic = localStorage.getItem("audioInputDevice");
    const savedSpeaker = localStorage.getItem("audioOutputDevice");
    if (savedMic) await room.switchActiveDevice("audioinput", savedMic);
    if (savedSpeaker) await room.switchActiveDevice("audiooutput", savedSpeaker);

    // Publier le micro
    await room.localParticipant.setMicrophoneEnabled(true);

    currentRoom = room;
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
}

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

export function toggleDeafen(): boolean {
  if (!currentRoom) return false;
  const tracks = currentRoom.remoteParticipants;
  let deafened = false;
  tracks.forEach((p) => {
    p.audioTrackPublications.forEach((pub_) => {
      if (pub_.track) {
        deafened = !pub_.isSubscribed;
        pub_.setSubscribed(!pub_.isSubscribed);
      }
    });
  });
  return deafened;
}

export function setDeafened(deafened: boolean) {
  if (!currentRoom) return;
  currentRoom.remoteParticipants.forEach((p) => {
    p.audioTrackPublications.forEach((pub_) => {
      if (pub_.track) {
        pub_.setSubscribed(!deafened);
      }
    });
  });
}

export function isConnected(): boolean {
  return currentRoom !== null && currentRoom.state === "connected";
}

export function getCurrentRoom(): Room | null {
  return currentRoom;
}

/// Enumerate available audio devices
export async function getAudioDevices(): Promise<{ inputs: MediaDeviceInfo[]; outputs: MediaDeviceInfo[] }> {
  try {
    const devices = await Room.getLocalDevices("audioinput");
    const outputs = await Room.getLocalDevices("audiooutput");
    return { inputs: devices, outputs };
  } catch {
    return { inputs: [], outputs: [] };
  }
}

/// Switch microphone device
export async function switchMicrophone(deviceId: string) {
  if (!currentRoom) return;
  await currentRoom.switchActiveDevice("audioinput", deviceId);
}

/// Switch speaker device
export async function switchSpeaker(deviceId: string) {
  if (!currentRoom) return;
  await currentRoom.switchActiveDevice("audiooutput", deviceId);
}

/// Set mic enabled/disabled directly (for applying state on join)
export function setMicEnabled(enabled: boolean) {
  if (!currentRoom) return;
  currentRoom.localParticipant.setMicrophoneEnabled(enabled);
}

/// Get WebRTC connection stats
export interface ConnectionStats {
  transport: "udp" | "tcp" | "turn-udp" | "turn-tcp" | "unknown";
  localAddress: string;
  remoteAddress: string;
  rtt: number; // ms
}

export async function getConnectionStats(): Promise<ConnectionStats | null> {
  if (!currentRoom) return null;

  try {
    // Find the peer connection via Room internals
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
