<template>
  <ModalSmall title="Modifier le groupe" @close="close">
    <form @submit.prevent="submit">
      <div class="field">
        <label>Nom</label>
        <input ref="inputRef" v-model="name" type="text" required />
      </div>
      <div class="modal-actions">
        <button type="button" class="btn-cancel" @click="close">Annuler</button>
        <button type="submit" :disabled="!name.trim() || name.trim() === props.group.name">Enregistrer</button>
      </div>
    </form>
  </ModalSmall>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { activeState, activeServer } from "../store";
import { api, type ChannelGroup } from "../api";
import ModalSmall from "./ModalSmall.vue";

const props = defineProps<{ group: ChannelGroup }>();
const emit = defineEmits<{ close: [] }>();

const name = ref(props.group.name);
const inputRef = ref<HTMLInputElement | null>(null);

onMounted(() => inputRef.value?.focus());

async function submit() {
  const s = activeServer();
  const st = activeState();
  if (!s || !st || !name.value.trim()) return;

  await api.updateGroup(s.url, s.token, props.group.id, name.value.trim());
  const idx = st.groups.findIndex((g) => g.id === props.group.id);
  if (idx >= 0) st.groups[idx] = { ...st.groups[idx], name: name.value.trim() };
  close();
}

function close() {
  emit("close");
}
</script>
