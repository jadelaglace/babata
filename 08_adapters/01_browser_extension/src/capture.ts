import type { CandidateEnvelope, DomCapturePayload } from "./types.js";

export async function toCandidate(
  payload: DomCapturePayload,
): Promise<CandidateEnvelope> {
  const text = payload.selectedText ?? payload.title;
  return {
    protocolVersion: "1",
    routeId: "source.browser",
    sourceReference: payload.url,
    contentType: "web_page",
    payloadSha256: await sha256Hex(text),
    metadata: { title: payload.title, selectedText: payload.selectedText },
    payload: { kind: "text", text },
  };
}

async function sha256Hex(value: string): Promise<string> {
  const bytes = new TextEncoder().encode(value);
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  return Array.from(new Uint8Array(digest), (byte) =>
    byte.toString(16).padStart(2, "0"),
  ).join("");
}
