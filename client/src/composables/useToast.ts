import { reactive } from "vue";

export interface Toast {
  id: number;
  message: string;
  type: "success" | "error" | "info";
}

let nextId = 0;

export const toasts = reactive<Toast[]>([]);

export function showToast(message: string, type: Toast["type"] = "success", duration = 3000) {
  const id = nextId++;
  toasts.push({ id, message, type });
  setTimeout(() => {
    const idx = toasts.findIndex((t) => t.id === id);
    if (idx !== -1) toasts.splice(idx, 1);
  }, duration);
}
