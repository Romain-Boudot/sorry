<template>
  <Teleport to="body">
    <div class="lightbox-overlay" @click.self="$emit('close')" @wheel="onWheel">
      <div class="lightbox-toolbar">
        <button class="lightbox-btn" title="Telecharger" @click="downloadImage">
          <Download :size="18" />
        </button>
        <button class="lightbox-btn" title="Copier l'image" @click="copyImage">
          <Copy :size="18" />
        </button>
        <button class="lightbox-btn" title="Fermer" @click="$emit('close')">
          <X :size="18" />
        </button>
      </div>
      <img
        ref="imgEl"
        :src="src"
        :alt="filename"
        class="lightbox-img"
        :style="{ transform: `scale(${zoom}) translate(${panX / zoom}px, ${panY / zoom}px)` }"
        @mousedown="startDrag"
        draggable="false"
      />
      <div class="lightbox-bottom">
        <span class="lightbox-filename">{{ filename }}</span>
        <span class="lightbox-hint"><ZoomIn :size="12" /> Molette pour zoomer</span>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { X, Download, Copy, ZoomIn } from "lucide-vue-next";
import { showToast } from "../../composables/useToast";

const props = defineProps<{
  src: string;
  filename: string;
}>();

const emit = defineEmits<{ close: [] }>();

const zoom = ref(1);
const panX = ref(0);
const panY = ref(0);
const dragging = ref(false);
const lastX = ref(0);
const lastY = ref(0);

function onWheel(e: WheelEvent) {
  e.preventDefault();
  const delta = e.deltaY > 0 ? -0.1 : 0.1;
  zoom.value = Math.max(0.5, Math.min(5, zoom.value + delta));
}

function startDrag(e: MouseEvent) {
  if (zoom.value <= 1) return;
  e.preventDefault();
  dragging.value = true;
  lastX.value = e.clientX;
  lastY.value = e.clientY;
}

function onMouseMove(e: MouseEvent) {
  if (!dragging.value) return;
  panX.value += e.clientX - lastX.value;
  panY.value += e.clientY - lastY.value;
  lastX.value = e.clientX;
  lastY.value = e.clientY;
}

function onMouseUp() {
  dragging.value = false;
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === "Escape") {
    emit("close");
  }
}

async function downloadImage() {
  try {
    const res = await fetch(props.src);
    const blob = await res.blob();
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = props.filename;
    a.click();
    URL.revokeObjectURL(url);
    showToast(`${props.filename} telecharge`);
  } catch {
    showToast("Erreur lors du telechargement", "error");
  }
}

async function copyImage() {
  try {
    const res = await fetch(props.src);
    const blob = await res.blob();
    await navigator.clipboard.write([
      new ClipboardItem({ [blob.type]: blob }),
    ]);
    showToast("Image copiee");
  } catch {
    showToast("Impossible de copier l'image", "error");
  }
}

onMounted(() => {
  document.addEventListener("mousemove", onMouseMove);
  document.addEventListener("mouseup", onMouseUp);
  document.addEventListener("keydown", onKeydown);
});

onUnmounted(() => {
  document.removeEventListener("mousemove", onMouseMove);
  document.removeEventListener("mouseup", onMouseUp);
  document.removeEventListener("keydown", onKeydown);
});
</script>

<style scoped>
.lightbox-overlay {
  position: fixed;
  inset: 0;
  z-index: 200;
  background: rgba(0, 0, 0, 0.85);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: default;
}

.lightbox-toolbar {
  position: absolute;
  top: 12px;
  right: 12px;
  display: flex;
  gap: 4px;
  z-index: 1;
}

.lightbox-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  margin: 0;
  padding: 0;
  border-radius: 8px;
  background: rgba(0, 0, 0, 0.5);
  border: none;
  color: rgba(255, 255, 255, 0.8);
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
  text-decoration: none;
}

.lightbox-btn:hover {
  background: rgba(255, 255, 255, 0.15);
  color: #fff;
}

.lightbox-img {
  max-width: 90vw;
  max-height: 85vh;
  object-fit: contain;
  border-radius: 4px;
  user-select: none;
  transition: transform 0.1s ease-out;
}

.lightbox-bottom {
  position: absolute;
  bottom: 16px;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}

.lightbox-filename {
  font-size: 0.8125rem;
  color: rgba(255, 255, 255, 0.6);
  background: rgba(0, 0, 0, 0.5);
  padding: 4px 12px;
  border-radius: 6px;
  max-width: 80vw;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.lightbox-hint {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 0.6875rem;
  color: rgba(255, 255, 255, 0.35);
}
</style>
