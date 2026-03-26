<template>
  <Teleport to="body">
    <div class="context-menu-overlay" @click="close" @contextmenu.prevent="close">
      <div
        class="context-menu"
        :style="{ top: `${y}px`, left: `${x}px` }"
        @click.stop
      >
        <div
          v-for="item in items"
          :key="item.label"
          class="context-menu-item"
          :class="{ danger: item.danger }"
          @click="item.action(); close()"
        >
          {{ item.label }}
        </div>
      </div>
    </div>
  </Teleport>
</template>

<script setup lang="ts">
export interface MenuItem {
  label: string;
  action: () => void;
  danger?: boolean;
}

defineProps<{
  x: number;
  y: number;
  items: MenuItem[];
}>();

const emit = defineEmits<{ close: [] }>();

function close() {
  emit("close");
}
</script>
