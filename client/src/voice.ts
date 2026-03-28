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

export function toggleDeafen(): boolean {
  if (!currentRoom) return false;
  // Mute tous les tracks audio distants
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
