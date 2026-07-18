import type {
  ApiEnvelope,
  ApiFailure,
  BrowserRouteId,
  CandidateEnvelope,
  CandidateSummary,
  CollectionItemStatus,
  CollectionSession,
  LocalApiConfig,
} from "./types.js";

export function normalizeBaseUrl(value: string): string {
  const url = new URL(value);
  if (url.protocol !== "http:" || url.hostname !== "127.0.0.1") {
    throw new Error("Babata must use an http://127.0.0.1 address.");
  }
  if (!url.port) {
    throw new Error("Babata local API address must include a port.");
  }
  return url.origin;
}

export async function checkPairing(
  config: LocalApiConfig,
): Promise<{ enabled: boolean; protocolVersion: string }> {
  return request(config, "/v1/health");
}

export async function startSession(
  config: LocalApiConfig,
  input: {
    routeId: BrowserRouteId;
    sourceReference: string;
    scopeDescription: string;
    installationId: string;
    candidates: CandidateEnvelope[];
  },
): Promise<{ session: CollectionSession; candidates: CandidateSummary[] }> {
  return request(config, "/v1/collector/sessions", {
    method: "POST",
    body: JSON.stringify(input),
  });
}

export async function selectCandidates(
  config: LocalApiConfig,
  input: {
    sessionId: string;
    candidateIds: string[];
    scopeDescription: string;
    confirmed: boolean;
  },
): Promise<CollectionItemStatus[]> {
  return request(config, "/v1/collector/select", {
    method: "POST",
    body: JSON.stringify(input),
  });
}

async function request<T>(
  config: LocalApiConfig,
  path: string,
  init: RequestInit = {},
): Promise<T> {
  const baseUrl = normalizeBaseUrl(config.baseUrl);
  if (config.token.length < 32) {
    throw new Error("Babata pairing token is missing or incomplete.");
  }
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 10_000);
  try {
    const response = await fetch(`${baseUrl}${path}`, {
      ...init,
      signal: controller.signal,
      headers: {
        Authorization: `Bearer ${config.token}`,
        "Content-Type": "application/json",
        ...init.headers,
      },
    });
    const body = (await response.json()) as ApiEnvelope<T> | ApiFailure;
    if (!response.ok || !("data" in body)) {
      const failure = body as ApiFailure;
      throw new Error(failure.message || `Babata returned HTTP ${response.status}.`);
    }
    return body.data;
  } catch (error) {
    if (error instanceof DOMException && error.name === "AbortError") {
      throw new Error("Babata local API did not respond in time.");
    }
    throw error;
  } finally {
    clearTimeout(timeout);
  }
}
