export type CandidateEnvelope = {
  protocolVersion: "1";
  routeId: BrowserRouteId;
  sourceReference: string;
  contentType: "web_page" | "document";
  payloadSha256: string;
  metadata: Record<string, unknown>;
  payload: { kind: "text"; text: string };
  context?: string;
  nativeId?: string;
};

export type BrowserRouteId =
  | "source.browser_pages"
  | "source.browser_bookmarks";

export type DomCapturePayload = {
  url: string;
  title: string;
  text: string;
  selectedText?: string;
};

export type BookmarkCapturePayload = {
  id: string;
  title: string;
  url: string;
  folderPath: string;
};

export type CandidateSummary = {
  candidate_id: string;
  route_id: BrowserRouteId;
  title?: string;
  source_location?: string;
  hierarchy: string[];
  content_type: "web_page" | "document";
  limitations: string[];
};

export type CollectionSession = {
  session_id: string;
  route_id: BrowserRouteId;
  scope_description: string;
  state: string;
};

export type CollectionItemStatus = {
  candidate_id: string;
  state: "queued" | "running" | "saved" | "skipped" | "failed";
  reason?: string;
  item_id?: string;
  revision_id?: string;
};

export type ApiEnvelope<T> = { data: T };

export type ApiFailure = { code: string; message: string };

export type LocalApiConfig = {
  baseUrl: string;
  token: string;
};
