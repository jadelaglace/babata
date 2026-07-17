from dataclasses import dataclass, field
from typing import Any


@dataclass(frozen=True)
class CandidateEnvelope:
    protocol_version: str
    route_id: str
    source_reference: str
    content_type: str
    payload_sha256: str
    metadata: dict[str, Any] = field(default_factory=dict)
