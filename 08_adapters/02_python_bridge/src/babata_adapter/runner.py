from .envelope import CandidateEnvelope


def validate(envelope: CandidateEnvelope) -> None:
    if envelope.protocol_version != "1":
        raise ValueError("unsupported candidate protocol version")


def run(_envelope: CandidateEnvelope) -> None:
    raise RuntimeError("capability_unavailable: Python bridge has no active tool")
