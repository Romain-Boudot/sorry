import { hashPassword, request } from "./client";
import type { User } from "./types";

export async function login(baseUrl: string, username: string, password: string, inviteCode?: string, totpCode?: string) {
  const hashed = await hashPassword(password);
  return request<{ token?: string; user?: User; totp_required?: boolean }>(baseUrl, "/auth/login", undefined, {
    method: "POST",
    body: JSON.stringify({ username, password: hashed, invite_code: inviteCode, totp_code: totpCode }),
  });
}

export function totpSetup(baseUrl: string, token: string) {
  return request<{ secret: string; otpauth_url: string }>(baseUrl, "/auth/totp/setup", token, {
    method: "POST",
  });
}

export function totpVerify(baseUrl: string, token: string, code: string) {
  return request<void>(baseUrl, "/auth/totp/verify", token, {
    method: "POST",
    body: JSON.stringify({ code }),
  });
}

export function totpDisable(baseUrl: string, token: string) {
  return request<void>(baseUrl, "/auth/totp/disable", token, {
    method: "POST",
  });
}

export function totpStatus(baseUrl: string, token: string) {
  return request<{ enabled: boolean }>(baseUrl, "/auth/totp/status", token);
}

export function quickLogin(baseUrl: string, inviteCode: string, displayName: string) {
  return request<{ token?: string; user?: User }>(baseUrl, "/auth/quick", undefined, {
    method: "POST",
    body: JSON.stringify({ invite_code: inviteCode, display_name: displayName }),
  });
}

export function refreshToken(baseUrl: string, token: string) {
  return request<{ token: string }>(baseUrl, "/auth/refresh", token, {
    method: "POST",
  });
}

export async function changePassword(baseUrl: string, token: string, currentPassword: string, newPassword: string) {
  const currentHashed = await hashPassword(currentPassword);
  const newHashed = await hashPassword(newPassword);
  return request<void>(baseUrl, "/users/me/password", token, {
    method: "POST",
    body: JSON.stringify({ current_password: currentHashed, new_password: newHashed }),
  });
}
