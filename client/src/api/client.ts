export async function hashPassword(password: string): Promise<string> {
  const encoded = new TextEncoder().encode(password);
  const hash = await crypto.subtle.digest("SHA-256", encoded);
  return Array.from(new Uint8Array(hash)).map(b => b.toString(16).padStart(2, "0")).join("");
}

export async function resolveBaseUrl(input: string): Promise<string> {
  const stripped = input.replace(/^https?:\/\//, "").replace(/\/+$/, "");
  for (const scheme of ["https", "http"]) {
    try {
      const url = `${scheme}://${stripped}`;
      const res = await fetch(`${url}/info`, { mode: "cors" });
      if (res.ok) return url;
    } catch {}
  }
  throw new Error("Server unreachable");
}

export async function request<T>(
  baseUrl: string,
  path: string,
  token?: string,
  options: RequestInit = {}
): Promise<T> {
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
  };

  const res = await fetch(`${baseUrl}/api${path}`, { ...options, headers });
  if (!res.ok) throw new Error(`${res.status}`);
  if (res.status === 204) return undefined as T;
  return res.json();
}
