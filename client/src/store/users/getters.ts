import { activeState, activeServer } from "../core";

export function resolveUser(userId: number): string {
  const state = activeState();
  if (!state) return `User #${userId}`;
  return state.users.get(userId)?.display_name ?? `User #${userId}`;
}

export function resolveAvatarUrl(userId: number): string | null {
  const state = activeState();
  const server = activeServer();
  if (!state || !server) return null;
  const avatarPath = state.users.get(userId)?.avatar_url;
  if (!avatarPath) return null;
  return `${server.url}${avatarPath}`;
}

export function resolveUserColor(userId: number): string | null {
  const state = activeState();
  if (!state) return null;
  const roleIds = state.userRoles.get(userId);
  if (!roleIds) return null;
  const userRoles = state.roles
    .filter((r) => roleIds.includes(r.id) && r.color)
    .sort((a, b) => a.position - b.position);
  return userRoles[0]?.color ?? null;
}

export function isGuest(userId: number): boolean {
  const state = activeState();
  if (!state) return false;
  return state.users.get(userId)?.guest === true;
}
