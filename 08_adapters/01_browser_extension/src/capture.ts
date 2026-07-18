import type {
  BookmarkCapturePayload,
  CandidateEnvelope,
  DomCapturePayload,
} from "./types.js";

export async function toPageCandidate(
  payload: DomCapturePayload,
  captureKind: "page" | "selection",
): Promise<CandidateEnvelope> {
  const text =
    captureKind === "selection"
      ? payload.selectedText?.trim()
      : payload.text.trim();
  if (!text) {
    throw new Error(`The ${captureKind} contains no readable text.`);
  }
  return {
    protocolVersion: "1",
    routeId: "source.browser_pages",
    sourceReference: payload.url,
    contentType: "web_page",
    payloadSha256: await sha256Hex(text),
    metadata: {
      title: payload.title,
      captureKind,
      selectedLength: captureKind === "selection" ? text.length : undefined,
    },
    payload: { kind: "text", text },
    context: captureKind === "selection" ? "visible selection" : "visible page text",
  };
}

export async function toBookmarkCandidate(
  payload: BookmarkCapturePayload,
): Promise<CandidateEnvelope> {
  const text = `${payload.title}\n${payload.url}`;
  return {
    protocolVersion: "1",
    routeId: "source.browser_bookmarks",
    sourceReference: payload.url,
    contentType: "document",
    payloadSha256: await sha256Hex(text),
    metadata: {
      title: payload.title,
      captureKind: "bookmark",
      bookmarkFolder: payload.folderPath,
      locatorOnly: true,
    },
    payload: { kind: "text", text },
    context: payload.folderPath,
    nativeId: payload.id,
  };
}

export async function sha256Hex(value: string): Promise<string> {
  const bytes = new TextEncoder().encode(value);
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  return Array.from(new Uint8Array(digest), (byte) =>
    byte.toString(16).padStart(2, "0"),
  ).join("");
}
