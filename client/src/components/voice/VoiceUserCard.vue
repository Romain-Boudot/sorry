<template>
  <div
    class="vuser"
    :class="{ compact, speaking: isSpeaking, self: isSelf }"
    @click="$emit('click', $event)"
    @contextmenu.prevent="$emit('contextMenu', $event)"
  >
    <div class="vuser-avatar">
      <img v-if="avatarUrl" :src="avatarUrl" />
      <span v-else>{{ name[0]?.toUpperCase() }}</span>
      <span
        v-if="quality && quality !== 'excellent' && quality !== 'unknown'"
        class="vuser-quality"
        :class="quality"
        :title="quality === 'good' ? 'Connexion correcte' : 'Connexion faible'"
      ></span>
    </div>
    <div class="vuser-name">{{ name }}</div>
    <div v-if="voiceState" class="vuser-icons">
      <MicOff v-if="voiceState.muted || voiceState.force_muted" :size="11" :class="{ forced: voiceState.force_muted }" />
      <HeadphoneOff v-if="voiceState.deafened || voiceState.force_deafened" :size="11" :class="{ forced: voiceState.force_deafened }" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { MicOff, HeadphoneOff } from "lucide-vue-next";
import type { VoiceUserState } from "../../api";
import type { QualityLevel } from "../../voice";

defineProps<{
  name: string;
  avatarUrl: string | null;
  voiceState?: VoiceUserState;
  isSpeaking: boolean;
  isSelf: boolean;
  quality: QualityLevel | null;
  /** `true` → small chip form (bottom strip); `false` → bigger centered card (no-stream mode). */
  compact: boolean;
}>();

defineEmits<{ contextMenu: [event: MouseEvent]; click: [event: MouseEvent] }>();
</script>

<style scoped>
.vuser {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 12px 8px;
  position: relative;
  cursor: pointer;
  border-radius: 8px;
  transition: background 0.1s;
}
.vuser:hover { background: var(--bg-modifier-hover); }

.vuser.compact {
  padding: 6px 8px;
  gap: 4px;
  min-width: 68px;
}

.vuser-avatar {
  position: relative;
  width: 80px;
  height: 80px;
  border-radius: 50%;
  background: var(--accent);
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 1.5rem;
  color: var(--text-bright);
  overflow: hidden;
  border: 3px solid transparent;
  transition: border-color 0.15s, transform 0.15s;
}
.vuser.compact .vuser-avatar {
  width: 36px;
  height: 36px;
  font-size: 0.9375rem;
  border-width: 2px;
}
.vuser.speaking .vuser-avatar { border-color: var(--green); }

.vuser-avatar img { width: 100%; height: 100%; object-fit: cover; }

.vuser-quality {
  position: absolute;
  bottom: 2px;
  right: 2px;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  border: 2px solid var(--bg-primary);
}
.vuser.compact .vuser-quality {
  width: 10px;
  height: 10px;
  bottom: 0;
  right: 0;
  border-width: 2px;
}
.vuser-quality.good { background: var(--orange); }
.vuser-quality.poor { background: var(--danger); }

.vuser-name {
  font-size: 0.8125rem;
  color: var(--text-normal);
  font-weight: 500;
  max-width: 120px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  text-align: center;
}
.vuser.compact .vuser-name {
  font-size: 0.6875rem;
  color: var(--text-muted);
  max-width: 76px;
}

.vuser-icons {
  display: flex;
  gap: 4px;
  align-items: center;
  color: var(--text-faint);
}
.vuser-icons .forced { color: var(--danger); }
</style>
