# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Header-only HTTP GET/HEAD observation. No redirects, body, credentials or raw errors in output."""
from __future__ import annotations

import argparse
import http.client
import json
import math
import re
import socket
import ssl
import time
from urllib.parse import urlsplit


def safe_header(name: str, value: str) -> str:
    if name.lower() == "content-length" and value.isascii() and value.isdigit():
        return value
    if name.lower() == "content-type":
        media = value.split(";", 1)[0].strip()
        if re.fullmatch(r"[A-Za-z0-9!#$&^_.+-]+/[A-Za-z0-9!#$&^_.+-]+", media):
            return media
    return "[redacted]"


class SafeParser(argparse.ArgumentParser):
    def error(self, message: str) -> None:
        print(json.dumps({"schema": "babata.http-probe/v1", "error": "invalid-arguments"}))
        raise SystemExit(2)


def probe(url: str, method: str = "GET", timeout: float = 5.0) -> tuple[dict, int]:
    result = {"schema": "babata.http-probe/v1", "tool_version": "1.0.0", "method": method,
              "body_read": False, "redirects_followed": False, "fault_origin": "unknown"}
    connection = None
    try:
        parsed = urlsplit(url)
        if (parsed.scheme not in ("http", "https") or not parsed.hostname
                or parsed.username is not None or parsed.password is not None
                or parsed.fragment or method not in ("GET", "HEAD")
                or any(ord(char) < 33 or ord(char) == 127 for char in url)
                or not math.isfinite(timeout) or not 0 < timeout <= 120):
            raise ValueError("invalid-input")
        port = parsed.port
        cls = http.client.HTTPSConnection if parsed.scheme == "https" else http.client.HTTPConnection
        connection = cls(parsed.hostname, port=port, timeout=timeout)
        started = time.monotonic()
        target = parsed.path or "/"
        if parsed.query:
            target += "?" + parsed.query
        connection.request(method, target, headers={"User-Agent": "babata-ops/1.0"})
        response = connection.getresponse()
        pairs = [[name, safe_header(name, value)]
                 for name, value in response.getheaders()]
        counts: dict[str, int] = {}
        for name, _ in pairs:
            key = name.lower()
            counts[key] = counts.get(key, 0) + 1
        healthy = 200 <= response.status < 300
        result.update(transport="responded", http_healthy=healthy, status=response.status,
                      http_version={10: "1.0", 11: "1.1"}.get(response.version, "unknown"),
                      elapsed_ms=round((time.monotonic() - started) * 1000, 1),
                      headers=pairs, duplicate_headers={k: v for k, v in counts.items() if v > 1})
        response.close()
        return result, 0 if healthy else 1
    except (TimeoutError, socket.timeout):
        result.update(transport="timeout", http_healthy=None, error="socket-timeout")
        return result, 5
    except ssl.SSLError:
        result.update(transport="failed", http_healthy=None, error="tls-failed")
        return result, 4
    except (ValueError, UnicodeError, http.client.InvalidURL):
        result.update(transport="not-attempted-or-invalid", http_healthy=None, error="invalid-input")
        return result, 2
    except (OSError, http.client.HTTPException):
        result.update(transport="failed", http_healthy=None, error="connection-or-protocol-failed")
        return result, 4
    finally:
        if connection is not None:
            connection.close()


def main(argv: list[str] | None = None) -> int:
    parser = SafeParser(description=__doc__)
    parser.add_argument("--url", required=True)
    parser.add_argument("--method", default="GET", choices=("GET", "HEAD"))
    parser.add_argument("--timeout", type=float, default=5.0)
    args = parser.parse_args(argv)
    report, code = probe(args.url, args.method, args.timeout)
    print(json.dumps(report, ensure_ascii=True, indent=2))
    return code


if __name__ == "__main__":
    raise SystemExit(main())
