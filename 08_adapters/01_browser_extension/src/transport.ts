import type { CandidateEnvelope } from "./types.js";

export async function submitCandidate(
  _candidate: CandidateEnvelope,
): Promise<never> {
  throw new Error(
    "capability_unavailable: save the candidate envelope and submit it with babata capture candidate",
  );
}
