export type CandidateEnvelope = {
  protocolVersion: "1";
  routeId: "source.browser";
  sourceReference: string;
  contentType: "web_page";
  payloadSha256: string;
  metadata: Record<string, unknown>;
  payload: { kind: "text"; text: string };
  context?: string;
  nativeId?: string;
};

export type DomCapturePayload = {
  url: string;
  title: string;
  selectedText?: string;
};
