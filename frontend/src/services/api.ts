export const API = import.meta.env.VITE_API_URL || "http://localhost:5050";
let token = "";
export function setOperatorToken(value: string) {
  token = value;
}
export async function request<T>(
  path: string,
  options: RequestInit = {},
): Promise<T> {
  const headers = new Headers(options.headers);
  if (options.body && !(options.body instanceof FormData))
    headers.set("Content-Type", "application/json");
  if (options.method && options.method !== "GET" && token)
    headers.set("Authorization", `Bearer ${token}`);
  const response = await fetch(`${API}${path}`, { ...options, headers });
  if (!response.ok) {
    let message = `Request failed (${response.status})`;
    try {
      const body = await response.json();
      message = body.error?.message || message;
    } catch {
      /* non-JSON framework error */
    }
    throw new Error(message);
  }
  return response.json();
}
export function post<T>(path: string, body?: unknown) {
  return request<T>(path, {
    method: "POST",
    body: body === undefined ? undefined : JSON.stringify(body),
  });
}
export const imageUrl = (id: string) =>
  `${API}/api/inspections/${encodeURIComponent(id)}/image`;
