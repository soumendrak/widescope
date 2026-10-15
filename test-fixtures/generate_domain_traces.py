"""Generate domain-specific and edge-case trace fixtures for WideScope.

This script writes ten JSON trace files into ``test-fixtures/domains/``,
covering domains not already represented in ``test-fixtures/{otlp,jaeger,
openinference}/upload-samples/`` (ecommerce checkout, service mesh, LLM
agent pipelines): Kubernetes control plane, banking wire transfer, IoT
telemetry, CI/CD pipelines, healthcare FHIR, video streaming CDN, mobile
RUM, legal-document RAG, and two synthetic stress/edge-case traces.

Each generator function returns a plain ``dict`` matching the exact shape
the corresponding WideScope parser expects (see
``crates/widescope-core/src/parsers/{otlp_json,jaeger,openinference}.rs``).
Timestamps are generated bottom-up (children first) so that, except where
explicitly noted with an ``# intentional:`` comment, every child span's
time range is fully contained within its parent's.

Run directly to (re)generate all fixtures::

    python3 test-fixtures/generate_domain_traces.py
"""

import json
import random
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

random.seed(42)

OUTPUT_DIR = Path(__file__).resolve().parent / "domains"

HEX_DIGITS = "0123456789abcdef"


def new_hex_id(n_chars: int) -> str:
    """Generate a random lowercase hex string of a fixed length.

    Args:
        n_chars: Number of hex characters to produce.

    Returns:
        A string of exactly ``n_chars`` lowercase hex digits.
    """
    return "".join(random.choice(HEX_DIGITS) for _ in range(n_chars))


def ms(n: float) -> int:
    """Convert a millisecond duration to nanoseconds.

    Args:
        n: Duration in milliseconds.

    Returns:
        The equivalent duration in nanoseconds, rounded to the nearest int.
    """
    return int(round(n * 1_000_000))


def sec(n: float) -> int:
    """Convert a second duration to nanoseconds.

    Args:
        n: Duration in seconds.

    Returns:
        The equivalent duration in nanoseconds, rounded to the nearest int.
    """
    return int(round(n * 1_000_000_000))


def span_range(
    children: List[Tuple[int, int]], pre_buffer_ns: int, post_buffer_ns: int
) -> Tuple[int, int]:
    """Compute a start/end range (in ns) that encloses a list of child ranges.

    Used to size a parent span so that every direct child's time range is
    fully contained within it, which is required for self-consistency
    everywhere except the deliberately-broken edge cases.

    Args:
        children: List of (start_ns, end_ns) tuples, one per direct child.
        pre_buffer_ns: Nanoseconds of slack before the earliest child start.
        post_buffer_ns: Nanoseconds of slack after the latest child end.

    Returns:
        A (start_ns, end_ns) tuple guaranteed to contain every child range.
    """
    starts = [s for s, _ in children]
    ends = [e for _, e in children]
    return min(starts) - pre_buffer_ns, max(ends) + post_buffer_ns


# ---------------------------------------------------------------------------
# OTLP JSON helpers (see crates/widescope-core/src/parsers/otlp_json.rs)
# ---------------------------------------------------------------------------


def otlp_value(value: Any) -> Dict[str, Any]:
    """Encode a Python value as an OTLP ``AnyValue`` object.

    Args:
        value: A bool, int, float, str, or list of homogeneous scalars.

    Returns:
        The OTLP ``value`` object, e.g. ``{"stringValue": "..."}``.
    """
    if isinstance(value, bool):
        return {"boolValue": value}
    if isinstance(value, int):
        return {"intValue": str(value)}
    if isinstance(value, float):
        return {"doubleValue": value}
    if isinstance(value, list):
        return {"arrayValue": {"values": [otlp_value(v) for v in value]}}
    return {"stringValue": str(value)}


def otlp_attrs(attributes: Dict[str, Any]) -> List[Dict[str, Any]]:
    """Encode a plain dict of attributes as an OTLP attribute-list.

    Args:
        attributes: Mapping of attribute key to Python scalar/list value.

    Returns:
        A list of ``{"key": ..., "value": {...}}`` OTLP attribute entries.
    """
    return [{"key": k, "value": otlp_value(v)} for k, v in attributes.items()]


def otlp_event(
    name: str, timestamp_ns: int, attributes: Optional[Dict[str, Any]] = None
) -> Dict[str, Any]:
    """Build a single OTLP span event.

    Args:
        name: Event name (e.g. ``"exception"``).
        timestamp_ns: Event timestamp in unix nanoseconds.
        attributes: Plain dict of event attributes.

    Returns:
        The OTLP event object.
    """
    return {
        "name": name,
        "timeUnixNano": str(timestamp_ns),
        "attributes": otlp_attrs(attributes or {}),
    }


def otlp_span(
    trace_id: str,
    span_id: str,
    name: str,
    kind: int,
    start_ns: int,
    end_ns: int,
    attributes: Optional[Dict[str, Any]] = None,
    parent_span_id: Optional[str] = None,
    status_code: int = 1,
    status_message: Optional[str] = None,
    events: Optional[List[Dict[str, Any]]] = None,
) -> Dict[str, Any]:
    """Build a single OTLP span object.

    Args:
        trace_id: 32-hex-character trace id.
        span_id: 16-hex-character span id.
        name: Span/operation name.
        kind: OTLP numeric span kind (1=INTERNAL, 2=SERVER, 3=CLIENT,
            4=PRODUCER, 5=CONSUMER).
        start_ns: Start time in unix nanoseconds.
        end_ns: End time in unix nanoseconds.
        attributes: Plain dict of span attributes.
        parent_span_id: Parent span id, or ``None`` for a root span.
        status_code: OTLP status code (0=UNSET, 1=OK, 2=ERROR).
        status_message: Error message, used only when ``status_code == 2``.
        events: Pre-built list of OTLP span events.

    Returns:
        The OTLP span object.
    """
    span: Dict[str, Any] = {
        "traceId": trace_id,
        "spanId": span_id,
        "name": name,
        "kind": kind,
        "startTimeUnixNano": str(start_ns),
        "endTimeUnixNano": str(end_ns),
        "status": {"code": status_code},
        "attributes": otlp_attrs(attributes or {}),
        "events": events or [],
    }
    if status_code == 2:
        span["status"]["message"] = status_message or "error"
    if parent_span_id is not None:
        span["parentSpanId"] = parent_span_id
    return span


def otlp_resource_span(
    service_name: str,
    spans: List[Dict[str, Any]],
    scope_name: str = "widescope-fixture-tracer",
) -> Dict[str, Any]:
    """Wrap a list of spans in a single ``resourceSpans`` entry for one service.

    Args:
        service_name: Value for the ``service.name`` resource attribute.
        spans: List of already-built OTLP span objects.
        scope_name: Instrumentation scope name.

    Returns:
        A single OTLP ``resourceSpans`` entry.
    """
    return {
        "resource": {
            "attributes": otlp_attrs(
                {"service.name": service_name, "service.version": "1.0.0"}
            ),
        },
        "scopeSpans": [
            {
                "scope": {"name": scope_name, "version": "1.0.0"},
                "spans": spans,
            }
        ],
    }


def otlp_doc(resource_spans: List[Dict[str, Any]]) -> Dict[str, Any]:
    """Wrap ``resourceSpans`` entries in the top-level OTLP document.

    Args:
        resource_spans: List of ``resourceSpans`` entries.

    Returns:
        The full OTLP JSON document.
    """
    return {"resourceSpans": resource_spans}


# ---------------------------------------------------------------------------
# Jaeger JSON helpers (see crates/widescope-core/src/parsers/jaeger.rs)
# ---------------------------------------------------------------------------


def jaeger_tag(key: str, value: Any) -> Dict[str, Any]:
    """Encode a single Jaeger tag, inferring its ``type`` from the Python type.

    Args:
        key: Tag key.
        value: A bool, int, float, or str value.

    Returns:
        A Jaeger tag object with ``key``, ``type``, and ``value``.
    """
    if isinstance(value, bool):
        return {"key": key, "type": "bool", "value": value}
    if isinstance(value, int):
        return {"key": key, "type": "int64", "value": value}
    if isinstance(value, float):
        return {"key": key, "type": "double", "value": value}
    return {"key": key, "type": "string", "value": str(value)}


def jaeger_tags(attributes: Dict[str, Any]) -> List[Dict[str, Any]]:
    """Encode a plain dict of attributes as a Jaeger tags list.

    Args:
        attributes: Mapping of tag key to Python scalar value.

    Returns:
        A list of Jaeger tag objects.
    """
    return [jaeger_tag(k, v) for k, v in attributes.items()]


def jaeger_log(timestamp_us: int, fields: Dict[str, Any]) -> Dict[str, Any]:
    """Build a single Jaeger log entry (parsed by WideScope as a span event).

    Args:
        timestamp_us: Log timestamp in unix microseconds.
        fields: Plain dict of log fields; an ``"event"`` field becomes the
            resulting span event's name.

    Returns:
        A Jaeger log object.
    """
    return {"timestamp": timestamp_us, "fields": jaeger_tags(fields)}


def jaeger_span(
    trace_id: str,
    span_id: str,
    operation_name: str,
    start_us: int,
    duration_us: int,
    process_id: str,
    tags: Optional[Dict[str, Any]] = None,
    parent_span_id: Optional[str] = None,
    logs: Optional[List[Dict[str, Any]]] = None,
) -> Dict[str, Any]:
    """Build a single Jaeger span object.

    Args:
        trace_id: Trace id shared by all spans in the trace.
        span_id: Unique id for this span.
        operation_name: Span/operation name.
        start_us: Start time in unix microseconds.
        duration_us: Duration in microseconds.
        process_id: Key into the trace's ``processes`` map.
        tags: Plain dict of span tags/attributes.
        parent_span_id: Id of the ``CHILD_OF`` parent span, or ``None`` for
            a root span.
        logs: Pre-built list of Jaeger log entries.

    Returns:
        The Jaeger span object.
    """
    references = []
    if parent_span_id is not None:
        references.append(
            {"refType": "CHILD_OF", "traceID": trace_id, "spanID": parent_span_id}
        )
    return {
        "traceID": trace_id,
        "spanID": span_id,
        "operationName": operation_name,
        "references": references,
        "startTime": start_us,
        "duration": duration_us,
        "processID": process_id,
        "tags": jaeger_tags(tags or {}),
        "logs": logs or [],
    }


def jaeger_doc(
    trace_id: str, processes: Dict[str, str], spans: List[Dict[str, Any]]
) -> Dict[str, Any]:
    """Wrap spans/processes into the top-level Jaeger document.

    Args:
        trace_id: Trace id shared by all spans.
        processes: Mapping of ``processID`` to service name.
        spans: List of already-built Jaeger span objects.

    Returns:
        The full Jaeger JSON document (``{"data": [...]}``).
    """
    return {
        "data": [
            {
                "traceID": trace_id,
                "processes": {pid: {"serviceName": name} for pid, name in processes.items()},
                "spans": spans,
            }
        ]
    }


# ---------------------------------------------------------------------------
# OpenInference JSON helpers
# (see crates/widescope-core/src/parsers/openinference.rs)
# ---------------------------------------------------------------------------


def oi_event(
    name: str, timestamp_ns: int, attributes: Optional[Dict[str, Any]] = None
) -> Dict[str, Any]:
    """Build a single OpenInference span event with a nanosecond timestamp.

    Args:
        name: Event name.
        timestamp_ns: Event timestamp in unix nanoseconds.
        attributes: Plain JSON-serializable dict of event attributes.

    Returns:
        The OpenInference event object.
    """
    return {
        "name": name,
        "timestamp_unix_nano": str(timestamp_ns),
        "attributes": attributes or {},
    }


def oi_span(
    trace_id: str,
    span_id: str,
    name: str,
    span_kind: str,
    start_ns: int,
    end_ns: int,
    attributes: Optional[Dict[str, Any]] = None,
    parent_id: Optional[str] = None,
    status_code: str = "OK",
    status_message: Optional[str] = None,
    events: Optional[List[Dict[str, Any]]] = None,
) -> Dict[str, Any]:
    """Build a single OpenInference span object.

    Args:
        trace_id: 32-hex-character trace id shared across the trace.
        span_id: Unique span id (any non-empty string).
        name: Span/operation name.
        span_kind: OpenInference span kind, e.g. ``"AGENT"``, ``"LLM"``,
            ``"RETRIEVER"``, ``"TOOL"``, ``"CHAIN"``.
        start_ns: Start time in unix nanoseconds.
        end_ns: End time in unix nanoseconds.
        attributes: Plain JSON-serializable dict of span attributes.
        parent_id: Parent span id, or ``None`` for a root span.
        status_code: ``"OK"``, ``"ERROR"``, or ``"UNSET"``.
        status_message: Error message, used only when ``status_code ==
            "ERROR"``.
        events: Pre-built list of OpenInference span events.

    Returns:
        The OpenInference span object.
    """
    attrs = dict(attributes or {})
    attrs.setdefault("openinference.span.kind", span_kind)
    span: Dict[str, Any] = {
        "name": name,
        "context": {"trace_id": trace_id, "span_id": span_id},
        "parent_id": parent_id,
        "span_kind": span_kind,
        "start_time_unix_nano": str(start_ns),
        "end_time_unix_nano": str(end_ns),
        "status_code": status_code,
        "attributes": attrs,
        "events": events or [],
    }
    if status_message is not None:
        span["status_message"] = status_message
    return span


def oi_doc(spans: List[Dict[str, Any]], fixture_name: str) -> Dict[str, Any]:
    """Wrap spans in the top-level OpenInference document.

    Args:
        spans: List of already-built OpenInference span objects.
        fixture_name: Short identifier stored in ``metadata.fixture``.

    Returns:
        The full OpenInference JSON document.
    """
    return {
        "metadata": {"source": "widescope-fixture-generator", "fixture": fixture_name},
        "spans": spans,
    }


def count_spans(document: Dict[str, Any], fmt: str) -> int:
    """Count the spans in a generated trace document.

    Args:
        document: The generated document (OTLP, Jaeger, or OpenInference).
        fmt: One of ``"otlp"``, ``"jaeger"``, ``"openinference"``.

    Returns:
        The total number of spans in the document.

    Raises:
        ValueError: If ``fmt`` is not a recognized format name.
    """
    if fmt == "otlp":
        return sum(
            len(scope["spans"])
            for rs in document["resourceSpans"]
            for scope in rs["scopeSpans"]
        )
    if fmt == "jaeger":
        return sum(len(trace["spans"]) for trace in document["data"])
    if fmt == "openinference":
        return len(document["spans"])
    raise ValueError(f"unknown format: {fmt}")


# ---------------------------------------------------------------------------
# 1. Kubernetes control plane (OTLP)
# ---------------------------------------------------------------------------


def gen_k8s_control_plane() -> Dict[str, Any]:
    """Build an OTLP trace modeling a Kubernetes control-plane request flow.

    Twelve independent pod-lifecycle cycles flow through kube-apiserver,
    etcd, kube-scheduler, kubelet, and CNI/CSI plugins. A few etcd writes
    deliberately fail with a lease-expiry error carrying an exception
    event, matching ``SpanStatus::Error`` in the OTLP parser.

    Returns:
        The OTLP JSON document.
    """
    trace_id = new_hex_id(32)
    t = 1_712_500_000_000_000_000
    buckets: Dict[str, List[Dict[str, Any]]] = {
        "kube-apiserver": [],
        "etcd": [],
        "kube-scheduler": [],
        "kubelet": [],
        "cni-plugin": [],
        "csi-driver": [],
    }

    for cycle in range(12):
        pod_name = f"web-{cycle:02d}-{new_hex_id(5)}"
        ids = {
            k: new_hex_id(16)
            for k in ["root", "auth", "authz", "etcd", "wal", "sched", "filter", "score", "kubelet", "cni", "csi"]
        }

        auth_start = t + ms(1)
        auth_end = auth_start + ms(random.uniform(1, 4))
        authz_start = auth_end + ms(0.2)
        authz_end = authz_start + ms(random.uniform(1, 3))

        wal_start = authz_end + ms(1)
        wal_end = wal_start + ms(random.uniform(1, 4))
        etcd_start, etcd_end = span_range([(wal_start, wal_end)], ms(0.5), ms(1))

        filter_start = etcd_end + ms(2)
        filter_end = filter_start + ms(random.uniform(2, 10))
        score_start = filter_end + ms(0.5)
        score_end = score_start + ms(random.uniform(2, 15))
        sched_start, sched_end = span_range(
            [(filter_start, filter_end), (score_start, score_end)], ms(1), ms(1)
        )

        cni_start = sched_end + ms(5)
        cni_end = cni_start + ms(random.uniform(20, 90))
        csi_start = cni_end + ms(2)
        csi_end = csi_start + ms(random.uniform(15, 70))
        kubelet_start, kubelet_end = span_range(
            [(cni_start, cni_end), (csi_start, csi_end)], ms(3), ms(1)
        )

        _, root_end = span_range(
            [
                (auth_start, auth_end),
                (authz_start, authz_end),
                (etcd_start, etcd_end),
                (sched_start, sched_end),
                (kubelet_start, kubelet_end),
            ],
            0,
            ms(2),
        )
        root_start = t  # the cycle's nominal start time

        is_lease_error = cycle in (2, 5, 9)  # a few deliberate etcd lease-expiry failures
        etcd_events = []
        if is_lease_error:
            etcd_events.append(
                otlp_event(
                    "exception",
                    etcd_end - ms(0.1),
                    {
                        "exception.type": "etcdserver.ErrLeaseExpired",
                        "exception.message": f"lease {new_hex_id(16)} expired before commit",
                        "exception.stacktrace": "etcdserver/lease.go:214 -> mvcc/kvstore_txn.go:88",
                    },
                )
            )

        buckets["kube-apiserver"].append(
            otlp_span(
                trace_id, ids["root"], "apiserver.handle_request", 2, root_start, root_end,
                attributes={
                    "http.method": "POST",
                    "http.route": "/api/v1/namespaces/default/pods",
                    "k8s.pod.name": pod_name,
                    "k8s.namespace": "default",
                    "apiserver.verb": "create",
                },
            )
        )
        buckets["kube-apiserver"].append(
            otlp_span(
                trace_id, ids["auth"], "apiserver.authenticate", 1, auth_start, auth_end,
                attributes={"auth.method": "x509"}, parent_span_id=ids["root"],
            )
        )
        buckets["kube-apiserver"].append(
            otlp_span(
                trace_id, ids["authz"], "apiserver.authorize", 1, authz_start, authz_end,
                attributes={"authz.decision": "allow", "rbac.role": "pod-creator"},
                parent_span_id=ids["root"],
            )
        )
        buckets["etcd"].append(
            otlp_span(
                trace_id, ids["etcd"], "etcd.txn.put", 3, etcd_start, etcd_end,
                attributes={
                    "etcd.key": f"/registry/pods/default/{pod_name}",
                    "etcd.revision": 100000 + cycle,
                },
                parent_span_id=ids["root"],
                status_code=2 if is_lease_error else 1,
                status_message="etcdserver: requested lease not found" if is_lease_error else None,
                events=etcd_events,
            )
        )
        buckets["etcd"].append(
            otlp_span(
                trace_id, ids["wal"], "etcd.wal.fsync", 1, wal_start, wal_end,
                attributes={"wal.bytes": random.randint(200, 4000)}, parent_span_id=ids["etcd"],
            )
        )
        buckets["kube-scheduler"].append(
            otlp_span(
                trace_id, ids["sched"], "scheduler.schedule_pod", 3, sched_start, sched_end,
                attributes={"k8s.pod.name": pod_name, "scheduler.nodes_considered": random.randint(3, 20)},
                parent_span_id=ids["root"],
            )
        )
        buckets["kube-scheduler"].append(
            otlp_span(
                trace_id, ids["filter"], "scheduler.filter_nodes", 1, filter_start, filter_end,
                attributes={"scheduler.predicates": "PodFitsResources,PodFitsHostPorts"},
                parent_span_id=ids["sched"],
            )
        )
        buckets["kube-scheduler"].append(
            otlp_span(
                trace_id, ids["score"], "scheduler.score_nodes", 1, score_start, score_end,
                attributes={"scheduler.selected_node": f"node-{random.randint(1, 8)}"},
                parent_span_id=ids["sched"],
            )
        )
        buckets["kubelet"].append(
            otlp_span(
                trace_id, ids["kubelet"], "kubelet.sync_pod", 3, kubelet_start, kubelet_end,
                attributes={"k8s.pod.name": pod_name, "k8s.node.name": f"node-{random.randint(1, 8)}"},
                parent_span_id=ids["root"],
            )
        )
        buckets["cni-plugin"].append(
            otlp_span(
                trace_id, ids["cni"], "cni.add", 1, cni_start, cni_end,
                attributes={
                    "cni.plugin": "calico",
                    "cni.pod_ip": f"10.244.{random.randint(0, 255)}.{random.randint(1, 254)}",
                },
                parent_span_id=ids["kubelet"],
            )
        )
        buckets["csi-driver"].append(
            otlp_span(
                trace_id, ids["csi"], "csi.node_publish_volume", 1, csi_start, csi_end,
                attributes={"csi.driver": "ebs.csi.aws.com", "csi.volume_id": f"vol-{new_hex_id(12)}"},
                parent_span_id=ids["kubelet"],
            )
        )

        t = root_end + ms(random.uniform(50, 200))

    resource_spans = [otlp_resource_span(svc, s) for svc, s in buckets.items()]
    return otlp_doc(resource_spans)


# ---------------------------------------------------------------------------
# 2. Banking wire transfer (OTLP)
# ---------------------------------------------------------------------------


def gen_banking_wire_transfer() -> Dict[str, Any]:
    """Build an OTLP trace modeling core-banking wire transfer processing.

    Eight independent transfers pass through fraud scoring, an AML/
    compliance check, ledger debit/credit, and a SWIFT MT103 gateway call
    that retries three times (the first two attempts fail with a timeout
    before the third succeeds). One compliance check deliberately fails
    with an AML hold (``SpanStatus::Error``). Account numbers and IBANs
    are masked/fake, never real.

    Returns:
        The OTLP JSON document.
    """
    trace_id = new_hex_id(32)
    t = 1_712_600_000_000_000_000
    buckets: Dict[str, List[Dict[str, Any]]] = {
        "channel-gateway": [],
        "fraud-scoring": [],
        "compliance-aml": [],
        "ledger-service": [],
        "swift-gateway": [],
    }

    for i in range(8):
        amount = round(random.uniform(500.0, 250_000.0), 2)
        currency = random.choice(["USD", "EUR", "GBP"])
        masked_from = f"****{random.randint(1000, 9999)}"
        masked_to_iban = f"DE89 3704 **** **** **{random.randint(10, 99)}"
        txn_id = f"txn_{new_hex_id(10)}"
        ids = {k: new_hex_id(16) for k in ["root", "fraud", "aml", "debit", "credit", "audit", "notify"]}

        fraud_start = t + ms(2)
        fraud_end = fraud_start + ms(random.uniform(30, 120))
        aml_start = fraud_end + ms(1)
        aml_end = aml_start + ms(random.uniform(50, 200))
        debit_start = aml_end + ms(1)
        debit_end = debit_start + ms(random.uniform(20, 80))
        credit_start = debit_end + ms(1)
        credit_end = credit_start + ms(random.uniform(20, 80))

        swift_cursor = credit_end + ms(2)
        swift_children: List[Dict[str, Any]] = []
        swift_ranges: List[Tuple[int, int]] = []
        n_attempts = 3
        for attempt in range(1, n_attempts + 1):
            a_start = swift_cursor
            a_end = a_start + ms(random.uniform(200, 900))
            is_final = attempt == n_attempts
            swift_children.append(
                otlp_span(
                    trace_id, new_hex_id(16), "swift.send_mt103", 3, a_start, a_end,
                    attributes={
                        "transaction.id": txn_id,
                        "swift.message_type": "MT103",
                        "retry.attempt": attempt,
                        "retry.max_attempts": n_attempts,
                    },
                    parent_span_id=ids["root"],
                    status_code=1 if is_final else 2,
                    status_message=None if is_final else "SWIFT network gateway timeout",
                )
            )
            swift_ranges.append((a_start, a_end))
            swift_cursor = a_end + ms(random.uniform(100, 400))

        audit_start = swift_ranges[-1][1] + ms(1)
        audit_end = audit_start + ms(random.uniform(5, 20))
        notify_start = audit_end + ms(1)
        notify_end = notify_start + ms(random.uniform(10, 40))

        _, root_end = span_range(
            [
                (fraud_start, fraud_end),
                (aml_start, aml_end),
                (debit_start, debit_end),
                (credit_start, credit_end),
                *swift_ranges,
                (audit_start, audit_end),
                (notify_start, notify_end),
            ],
            0,
            ms(2),
        )
        root_start = t

        is_aml_hold = i == 5  # deliberate single ERROR span: AML hold

        buckets["channel-gateway"].append(
            otlp_span(
                trace_id, ids["root"], "wire.transfer.execute", 2, root_start, root_end,
                attributes={
                    "transaction.id": txn_id,
                    "transaction.amount": amount,
                    "transaction.currency": currency,
                    "account.from": masked_from,
                    "account.to.iban_masked": masked_to_iban,
                },
            )
        )
        buckets["fraud-scoring"].append(
            otlp_span(
                trace_id, ids["fraud"], "fraud.score_transaction", 3, fraud_start, fraud_end,
                attributes={
                    "transaction.id": txn_id,
                    "fraud.score": round(random.uniform(0.01, 0.35), 3),
                    "fraud.model_version": "fraud-v3.2",
                },
                parent_span_id=ids["root"],
            )
        )
        buckets["compliance-aml"].append(
            otlp_span(
                trace_id, ids["aml"], "compliance.aml_check", 3, aml_start, aml_end,
                attributes={
                    "transaction.id": txn_id,
                    "aml.rule_set": "ofac-2024",
                    "aml.sanctions_list_checked": True,
                },
                parent_span_id=ids["root"],
                status_code=2 if is_aml_hold else 1,
                status_message="AML watchlist match requires manual review" if is_aml_hold else None,
            )
        )
        buckets["ledger-service"].append(
            otlp_span(
                trace_id, ids["debit"], "ledger.debit", 3, debit_start, debit_end,
                attributes={
                    "transaction.id": txn_id,
                    "transaction.amount": amount,
                    "transaction.currency": currency,
                    "account.number": masked_from,
                    "ledger.entry_type": "debit",
                },
                parent_span_id=ids["root"],
            )
        )
        buckets["ledger-service"].append(
            otlp_span(
                trace_id, ids["credit"], "ledger.credit", 3, credit_start, credit_end,
                attributes={
                    "transaction.id": txn_id,
                    "transaction.amount": amount,
                    "transaction.currency": currency,
                    "account.number.iban_masked": masked_to_iban,
                    "ledger.entry_type": "credit",
                },
                parent_span_id=ids["root"],
            )
        )
        buckets["swift-gateway"].extend(swift_children)
        buckets["compliance-aml"].append(
            otlp_span(
                trace_id, ids["audit"], "audit.record_transaction", 1, audit_start, audit_end,
                attributes={"transaction.id": txn_id, "audit.retention_years": 7},
                parent_span_id=ids["root"],
            )
        )
        buckets["channel-gateway"].append(
            otlp_span(
                trace_id, ids["notify"], "notify.customer", 1, notify_start, notify_end,
                attributes={"transaction.id": txn_id, "notify.channel": random.choice(["email", "push", "sms"])},
                parent_span_id=ids["root"],
            )
        )

        t = root_end + ms(random.uniform(200, 900))

    resource_spans = [otlp_resource_span(svc, s) for svc, s in buckets.items()]
    return otlp_doc(resource_spans)


# ---------------------------------------------------------------------------
# 3. IoT telemetry ingest (Jaeger)
# ---------------------------------------------------------------------------


def gen_iot_telemetry_ingest() -> Dict[str, Any]:
    """Build a Jaeger trace modeling an IoT telemetry ingestion pipeline.

    Two publish batches flow from an MQTT broker through an ingest gateway
    (which fans out into 40+ sub-millisecond per-message validation
    spans), a Kafka topic, a stream processor, and a time-series database.

    Returns:
        The Jaeger JSON document.
    """
    trace_id = new_hex_id(32)
    processes = {
        "mqtt": "mqtt-broker",
        "ingest": "ingest-gateway",
        "kafka": "kafka",
        "stream": "stream-processor",
        "tsdb": "timeseries-db",
    }
    spans: List[Dict[str, Any]] = []
    t = 1_712_700_000_000_000  # microseconds

    for batch in range(2):
        device_prefix = f"sensor-{batch:02d}"
        root_id = new_hex_id(16)
        ingest_id = new_hex_id(16)
        kafka_id = new_hex_id(16)
        stream_id = new_hex_id(16)
        tsdb_id = new_hex_id(16)

        ingest_start = t + 2_000
        fanout_cursor = ingest_start + 1_000
        fanout_spans = []
        fanout_ranges: List[Tuple[int, int]] = []
        n_children = 42
        for i in range(n_children):
            c_start = fanout_cursor + random.randint(0, 300)
            c_dur = random.randint(120, 900)  # sub-millisecond
            c_end = c_start + c_dur
            fanout_spans.append(
                jaeger_span(
                    trace_id, new_hex_id(16), "ingest.validate_message", c_start, c_dur, "ingest",
                    tags={
                        "span.kind": "internal",
                        "device.id": f"{device_prefix}-{i:03d}",
                        "mqtt.topic": f"telemetry/{device_prefix}/temp",
                        "payload.bytes": random.randint(16, 128),
                        "otel.status_code": "OK",
                    },
                    parent_span_id=ingest_id,
                )
            )
            fanout_ranges.append((c_start, c_end))
            fanout_cursor = max(fanout_cursor, c_end)

        kafka_start = max(e for _, e in fanout_ranges) + 500
        kafka_dur = random.randint(5_000, 20_000)
        kafka_end = kafka_start + kafka_dur

        ingest_end = kafka_end + 1_000

        stream_cursor = kafka_end + 500
        stream_children = []
        stream_ranges: List[Tuple[int, int]] = []
        for i in range(15):
            c_start = stream_cursor
            c_dur = random.randint(50, 400)
            c_end = c_start + c_dur
            stream_children.append(
                jaeger_span(
                    trace_id, new_hex_id(16), "stream.transform_window", c_start, c_dur, "stream",
                    tags={"span.kind": "internal", "window.index": i, "otel.status_code": "OK"},
                    parent_span_id=stream_id,
                )
            )
            stream_ranges.append((c_start, c_end))
            stream_cursor = c_end + random.randint(0, 50)
        stream_start, stream_end = span_range(stream_ranges, 500, 500)

        tsdb_cursor = stream_end + 500
        tsdb_children = []
        tsdb_ranges: List[Tuple[int, int]] = []
        for i in range(15):
            c_start = tsdb_cursor
            c_dur = random.randint(50, 500)
            c_end = c_start + c_dur
            tsdb_children.append(
                jaeger_span(
                    trace_id, new_hex_id(16), "tsdb.write_point", c_start, c_dur, "tsdb",
                    tags={"span.kind": "internal", "point.index": i, "otel.status_code": "OK"},
                    parent_span_id=tsdb_id,
                )
            )
            tsdb_ranges.append((c_start, c_end))
            tsdb_cursor = c_end + random.randint(0, 50)
        tsdb_start, tsdb_end = span_range(tsdb_ranges, 500, 500)

        root_start = t
        root_end = tsdb_end + 1_000

        spans.append(
            jaeger_span(
                trace_id, root_id, "mqtt.publish_batch", root_start, root_end - root_start, "mqtt",
                tags={"span.kind": "producer", "mqtt.qos": 1, "mqtt.batch_size": n_children, "otel.status_code": "OK"},
            )
        )
        spans.append(
            jaeger_span(
                trace_id, ingest_id, "ingest.batch_receive", ingest_start, ingest_end - ingest_start, "ingest",
                tags={"span.kind": "server", "ingest.batch_id": f"batch-{batch}", "otel.status_code": "OK"},
                parent_span_id=root_id,
            )
        )
        spans.extend(fanout_spans)
        spans.append(
            jaeger_span(
                trace_id, kafka_id, "kafka.produce", kafka_start, kafka_dur, "kafka",
                tags={
                    "span.kind": "producer",
                    "messaging.system": "kafka",
                    "messaging.destination": "telemetry.raw",
                    "otel.status_code": "OK",
                },
                parent_span_id=root_id,
            )
        )
        spans.append(
            jaeger_span(
                trace_id, stream_id, "stream.process_batch", stream_start, stream_end - stream_start, "stream",
                tags={"span.kind": "consumer", "messaging.system": "kafka", "otel.status_code": "OK"},
                parent_span_id=root_id,
            )
        )
        spans.extend(stream_children)
        spans.append(
            jaeger_span(
                trace_id, tsdb_id, "tsdb.write_batch", tsdb_start, tsdb_end - tsdb_start, "tsdb",
                tags={"span.kind": "client", "db.system": "timeseries", "otel.status_code": "OK"},
                parent_span_id=root_id,
            )
        )
        spans.extend(tsdb_children)

        t = root_end + 100_000

    return jaeger_doc(trace_id, processes, spans)


# ---------------------------------------------------------------------------
# 4. CI/CD pipeline (OTLP) - deep nesting
# ---------------------------------------------------------------------------

CHECKOUT_STEPS = [
    "resolve_ref", "negotiate_pack", "fetch_objects", "index_pack", "verify_connectivity",
    "unpack_objects", "update_ref", "write_tree", "checkout_index", "update_submodule",
    "run_hooks", "gc_prune",
]


def build_nested_chain(
    trace_id: str, names: List[str], start_ns: int, leaf_dur_ns: int, buffer_ns: int
) -> Tuple[List[Dict[str, Any]], int, int]:
    """Build a linear parent/child chain of OTLP spans, one level per name.

    Each span is the sole child of the previous one, so the returned list
    represents ``len(names)`` levels of nesting rooted at ``names[0]``.

    Args:
        trace_id: Shared trace id.
        names: Span names from outermost (index 0) to innermost (last).
        start_ns: Start time of the outermost span.
        leaf_dur_ns: Duration of the innermost span.
        buffer_ns: Extra nanoseconds each ancestor adds on both sides of
            its child, guaranteeing containment.

    Returns:
        A tuple of ``(spans, chain_start_ns, chain_end_ns)`` where
        ``spans`` is already parented into a single chain and ordered
        outermost to innermost.
    """
    depth = len(names)
    ids = [new_hex_id(16) for _ in range(depth)]
    starts = [0] * depth
    ends = [0] * depth

    inner_start = start_ns + buffer_ns * (depth - 1)
    starts[-1] = inner_start
    ends[-1] = inner_start + leaf_dur_ns
    for level in range(depth - 2, -1, -1):
        starts[level] = starts[level + 1] - buffer_ns
        ends[level] = ends[level + 1] + buffer_ns

    spans = []
    for level in range(depth):
        parent_id = ids[level - 1] if level > 0 else None
        spans.append(
            otlp_span(
                trace_id, ids[level], names[level], 1, starts[level], ends[level],
                attributes={"pipeline.chain_depth": level}, parent_span_id=parent_id,
            )
        )
    return spans, starts[0], ends[0]


def gen_cicd_pipeline() -> Dict[str, Any]:
    """Build an OTLP trace modeling a CI/CD build pipeline.

    A single ~20-minute pipeline run: checkout (with a 25-level-deep
    nested chain of git internals), dependency resolution, compilation,
    five parallel test shards, a Docker build, and deploy. Several spans
    run for multiple minutes each. Spans are split across six services
    (ci-orchestrator, git-checkout-agent, build-runner, test-runner,
    artifact-registry, deploy-controller) while keeping the original
    parent/child relationships across those service boundaries.

    Returns:
        The OTLP JSON document.
    """
    trace_id = new_hex_id(32)
    t = 1_712_800_000_000_000_000
    buckets: Dict[str, List[Dict[str, Any]]] = {
        "ci-orchestrator": [],
        "git-checkout-agent": [],
        "build-runner": [],
        "test-runner": [],
        "artifact-registry": [],
        "deploy-controller": [],
    }

    checkout_names = ["checkout"] + [
        f"checkout.{CHECKOUT_STEPS[i % len(CHECKOUT_STEPS)]}.depth{i}" for i in range(1, 25)
    ]
    checkout_spans, checkout_start, checkout_end = build_nested_chain(
        trace_id, checkout_names, t + sec(1), ms(300), ms(80)
    )

    dep_start = checkout_end + sec(2)
    dep_end = dep_start + sec(120)
    dep_id = new_hex_id(16)

    compile_names = [
        "compile", "compile.parse", "compile.typecheck", "compile.borrowck",
        "compile.codegen", "compile.optimize", "compile.lint", "compile.link",
    ]
    compile_spans, compile_start_chain, compile_end_chain = build_nested_chain(
        trace_id, compile_names, dep_end + sec(2), sec(20), sec(30)
    )
    # Stretch the outer "compile" span to the full 300s stage duration.
    compile_id = compile_spans[0]["spanId"]
    compile_start = compile_start_chain
    compile_end = compile_start + sec(300)
    compile_spans[0]["endTimeUnixNano"] = str(compile_end)

    shard_start = compile_end + sec(3)
    shard_dur = sec(240)
    shard_spans: List[Dict[str, Any]] = []
    for shard in range(5):
        shard_id = new_hex_id(16)
        case_a_start = shard_start + sec(2)
        case_a_end = case_a_start + sec(90)
        case_b_start = case_a_end + sec(1)
        case_b_end = case_b_start + sec(100)
        shard_spans.append(
            otlp_span(
                trace_id, shard_id, f"test.shard.{shard}", 1, shard_start, shard_start + shard_dur,
                attributes={"test.shard_index": shard, "test.shard_count": 5},
            )
        )
        shard_spans.append(
            otlp_span(
                trace_id, new_hex_id(16), "test.case.unit_suite", 1, case_a_start, case_a_end,
                attributes={"test.suite": "unit"}, parent_span_id=shard_id,
            )
        )
        shard_spans.append(
            otlp_span(
                trace_id, new_hex_id(16), "test.case.integration_suite", 1, case_b_start, case_b_end,
                attributes={"test.suite": "integration"}, parent_span_id=shard_id,
            )
        )

    docker_names = [
        "docker.build", "docker.build.base_layer", "docker.build.deps_layer",
        "docker.build.app_layer", "docker.build.security_scan", "docker.build.push",
    ]
    docker_spans, docker_start_chain, _ = build_nested_chain(
        trace_id, docker_names, shard_start + shard_dur + sec(3), sec(30), sec(60)
    )
    docker_id = docker_spans[0]["spanId"]
    docker_start = docker_start_chain
    docker_end = docker_start + sec(360)
    docker_spans[0]["endTimeUnixNano"] = str(docker_end)

    deploy_start = docker_end + sec(2)
    deploy_end = deploy_start + sec(90)
    deploy_id = new_hex_id(16)

    root_start, root_end = span_range(
        [
            (checkout_start, checkout_end),
            (dep_start, dep_end),
            (compile_start, compile_end),
            (shard_start, shard_start + shard_dur),
            (docker_start, docker_end),
            (deploy_start, deploy_end),
        ],
        sec(1),
        sec(5),
    )
    root_id = new_hex_id(16)

    buckets["ci-orchestrator"].append(
        otlp_span(
            trace_id, root_id, "ci.pipeline.run", 2, root_start, root_end,
            attributes={"ci.pipeline": "widescope-release", "ci.trigger": "push", "vcs.ref": "refs/heads/main"},
        )
    )
    checkout_spans[0]["parentSpanId"] = root_id
    buckets["git-checkout-agent"].extend(checkout_spans)
    buckets["build-runner"].append(
        otlp_span(
            trace_id, dep_id, "dependency.resolve", 1, dep_start, dep_end,
            attributes={"deps.package_manager": "cargo", "deps.count": 214}, parent_span_id=root_id,
        )
    )
    compile_spans[0]["parentSpanId"] = root_id
    buckets["build-runner"].extend(compile_spans)
    for s in shard_spans:
        if "parentSpanId" not in s:
            s["parentSpanId"] = root_id
    buckets["test-runner"].extend(shard_spans)
    docker_spans[0]["parentSpanId"] = root_id
    buckets["artifact-registry"].extend(docker_spans)
    buckets["deploy-controller"].append(
        otlp_span(
            trace_id, deploy_id, "deploy", 1, deploy_start, deploy_end,
            attributes={"deploy.target": "production", "deploy.strategy": "rolling"}, parent_span_id=root_id,
        )
    )

    resource_spans = [otlp_resource_span(svc, s) for svc, s in buckets.items()]
    return otlp_doc(resource_spans)


# ---------------------------------------------------------------------------
# 5. Healthcare FHIR (Jaeger) - non-ASCII attributes
# ---------------------------------------------------------------------------

# All patient data below is synthetic/fake; used only to exercise Unicode
# handling (accented Latin, CJK, RTL Arabic, and emoji) end to end.
SYNTHETIC_PATIENTS = [
    ("María José Fernández", "🩺 Routine check-up completed ✅"),
    ("田中 太郎", "健康状態は良好です 😊"),
    ("محمد الأحمد", "مريض يعاني من ارتفاع ضغط الدم"),
    ("François Müller", "⚠️ Allergic to penicillin!"),
    ("Søren Åkesson", "Follow-up scheduled in 2 weeks 📅"),
    ("最上 陽菜", "術後経過は順調 👍"),
    ("فاطمة الزهراء", "تحتاج إلى مراجعة دورية"),
    ("Renée Côté", "Vaccination up to date ✔️"),
    ("Björk Guðmundsdóttir", "Referred to cardiology 🫀"),
    ("이 서연", "정기 검진 예약됨"),
]


def gen_healthcare_fhir() -> Dict[str, Any]:
    """Build a Jaeger trace modeling a FHIR patient-record API request flow.

    Ten independent request cycles pass through auth, patient lookup
    (EHR adapter), consent check, HL7 transform, audit logging, and the
    FHIR bundle response. Patient names and clinical notes are synthetic
    and deliberately span multiple scripts (accented Latin, CJK, RTL
    Arabic) and emoji to exercise Unicode rendering.

    Returns:
        The Jaeger JSON document.
    """
    trace_id = new_hex_id(32)
    processes = {
        "gw": "fhir-gateway",
        "auth": "auth-service",
        "ehr": "ehr-adapter",
        "hl7": "hl7-transformer",
        "audit": "audit-log",
    }
    spans: List[Dict[str, Any]] = []
    t = 1_712_900_000_000_000  # microseconds

    for i in range(10):
        patient_name, clinical_note = SYNTHETIC_PATIENTS[i % len(SYNTHETIC_PATIENTS)]
        patient_id = f"synthetic-patient-{i:04d}"
        ids = {k: new_hex_id(16) for k in ["root", "auth", "lookup", "consent", "hl7", "audit", "bundle"]}

        auth_start = t + 1_000
        auth_end = auth_start + random.randint(3_000, 9_000)
        lookup_start = auth_end + 1_000
        lookup_end = lookup_start + random.randint(15_000, 60_000)
        consent_start = lookup_end + 500
        consent_end = consent_start + random.randint(2_000, 8_000)
        hl7_start = consent_end + 1_000
        hl7_end = hl7_start + random.randint(8_000, 30_000)
        audit_start = hl7_end + 500
        audit_end = audit_start + random.randint(2_000, 6_000)
        bundle_start = audit_end + 500
        bundle_end = bundle_start + random.randint(3_000, 10_000)

        root_start = t
        root_end = bundle_end + 2_000

        spans.append(
            jaeger_span(
                trace_id, ids["root"], "GET /fhir/Patient/{id}", root_start, root_end - root_start, "gw",
                tags={
                    "span.kind": "server", "http.method": "GET", "http.route": "/fhir/Patient/{id}",
                    "patient.id": patient_id, "otel.status_code": "OK",
                },
            )
        )
        spans.append(
            jaeger_span(
                trace_id, ids["auth"], "auth.validate_token", auth_start, auth_end - auth_start, "auth",
                tags={"span.kind": "client", "auth.scope": "patient/*.read", "otel.status_code": "OK"},
                parent_span_id=ids["root"],
            )
        )
        spans.append(
            jaeger_span(
                trace_id, ids["lookup"], "patient.lookup", lookup_start, lookup_end - lookup_start, "ehr",
                tags={
                    "span.kind": "client", "patient.id": patient_id,
                    "patient.name": patient_name, "patient.clinical_note": clinical_note,
                    "otel.status_code": "OK",
                },
                parent_span_id=ids["root"],
            )
        )
        spans.append(
            jaeger_span(
                trace_id, ids["consent"], "patient.consent_check", consent_start, consent_end - consent_start, "ehr",
                tags={"span.kind": "internal", "consent.status": "granted", "otel.status_code": "OK"},
                parent_span_id=ids["root"],
            )
        )
        spans.append(
            jaeger_span(
                trace_id, ids["hl7"], "hl7.transform", hl7_start, hl7_end - hl7_start, "hl7",
                tags={"span.kind": "internal", "hl7.version": "2.5.1", "hl7.message_type": "ADT^A01", "otel.status_code": "OK"},
                parent_span_id=ids["root"],
            )
        )
        spans.append(
            jaeger_span(
                trace_id, ids["audit"], "audit.log_access", audit_start, audit_end - audit_start, "audit",
                tags={"span.kind": "internal", "patient.id": patient_id, "audit.reason": "treatment", "otel.status_code": "OK"},
                parent_span_id=ids["root"],
            )
        )
        spans.append(
            jaeger_span(
                trace_id, ids["bundle"], "fhir.bundle_response", bundle_start, bundle_end - bundle_start, "gw",
                tags={"span.kind": "internal", "fhir.resource_type": "Bundle", "otel.status_code": "OK"},
                parent_span_id=ids["root"],
            )
        )

        t = root_end + random.randint(20_000, 80_000)

    return jaeger_doc(trace_id, processes, spans)


# ---------------------------------------------------------------------------
# 6. Video streaming CDN (OTLP)
# ---------------------------------------------------------------------------


def gen_video_streaming_cdn() -> Dict[str, Any]:
    """Build an OTLP trace modeling a video playback/CDN session.

    A single playback session: manifest request, ~85 sequential ABR
    segment fetches (many siblings at the same depth, a few incurring an
    origin-shield fetch on cache miss and buffering span events), an ABR
    transcode ladder, and a DRM license request. Spans are split across
    five services (playback-api, cdn-edge, origin-shield,
    transcode-worker, drm-license-service) while keeping the original
    parent/child relationships across those service boundaries.

    Returns:
        The OTLP JSON document.
    """
    trace_id = new_hex_id(32)
    t = 1_713_000_000_000_000_000
    root_id = new_hex_id(16)

    manifest_start = t + ms(5)
    manifest_end = manifest_start + ms(random.uniform(30, 90))

    segment_cursor = manifest_end + ms(2)
    segment_spans: List[Dict[str, Any]] = []
    shield_spans: List[Dict[str, Any]] = []
    segment_ranges: List[Tuple[int, int]] = []
    n_segments = 85
    for i in range(n_segments):
        seg_start = segment_cursor
        seg_dur = ms(random.uniform(1500, 4000))  # segment fetch + playback interval
        seg_end = seg_start + seg_dur
        seg_id = new_hex_id(16)

        events = []
        if random.random() < 0.15:
            buf_start = seg_start + ms(seg_dur / 1_000_000 * 0.3)
            buf_end = buf_start + ms(random.uniform(100, 800))
            events.append(otlp_event("buffering.start", buf_start, {"buffer.level_ms": random.randint(0, 200)}))
            events.append(otlp_event("buffering.end", buf_end, {"buffer.refill_ms": random.randint(500, 3000)}))

        segment_spans.append(
            otlp_span(
                trace_id, seg_id, "cdn.edge.fetch_segment", 3, seg_start, seg_end,
                attributes={
                    "segment.index": i,
                    "segment.bitrate_kbps": random.choice([480, 800, 1200, 2500, 4500, 8000]),
                    "cdn.pop": random.choice(["iad1", "sjc2", "fra3", "sin1"]),
                    "cdn.cache_status": "hit" if random.random() > 0.06 else "miss",
                },
                events=events,
                parent_span_id=root_id,
            )
        )
        segment_ranges.append((seg_start, seg_end))

        # A handful of segments incur a nested origin-shield fetch on cache miss.
        if i % 17 == 0:
            shield_start = seg_start + ms(2)
            shield_end = shield_start + ms(random.uniform(50, 250))
            shield_spans.append(
                otlp_span(
                    trace_id, new_hex_id(16), "origin.shield.fetch", 3, shield_start, shield_end,
                    attributes={"origin.region": "us-east-1", "segment.index": i},
                    parent_span_id=seg_id,
                )
            )

        segment_cursor = seg_end

    ladder_start = segment_ranges[0][0]
    ladder_id = new_hex_id(16)
    rendition_spans = []
    rendition_ranges: List[Tuple[int, int]] = []
    for bitrate in (240, 480, 800, 1200, 2500, 4500):
        r_start = ladder_start + ms(random.uniform(0, 5))
        r_end = r_start + ms(random.uniform(10, 60))
        rendition_spans.append(
            otlp_span(
                trace_id, new_hex_id(16), "transcode.render_rendition", 1, r_start, r_end,
                attributes={"rendition.bitrate_kbps": bitrate, "rendition.codec": "h264"},
                parent_span_id=ladder_id,
            )
        )
        rendition_ranges.append((r_start, r_end))
    ladder_range_start, ladder_range_end = span_range(rendition_ranges, ms(1), ms(1))

    drm_start = manifest_end + ms(3)
    drm_end = drm_start + ms(random.uniform(20, 80))

    root_start, root_end = span_range(
        [
            (manifest_start, manifest_end),
            *segment_ranges,
            (ladder_range_start, ladder_range_end),
            (drm_start, drm_end),
        ],
        0,
        ms(5),
    )

    buckets: Dict[str, List[Dict[str, Any]]] = {
        "playback-api": [
            otlp_span(
                trace_id, root_id, "playback.session", 2, root_start, root_end,
                attributes={"session.id": f"sess_{new_hex_id(8)}", "player.name": "widescope-player", "player.version": "9.4.1"},
            ),
            otlp_span(
                trace_id, new_hex_id(16), "manifest.request", 3, manifest_start, manifest_end,
                attributes={"manifest.type": "hls", "manifest.url": "https://cdn.example.com/stream/master.m3u8"},
                parent_span_id=root_id,
            ),
        ],
        "cdn-edge": list(segment_spans),
        "origin-shield": list(shield_spans),
        "transcode-worker": [
            otlp_span(
                trace_id, ladder_id, "transcode.ladder", 1, ladder_range_start, ladder_range_end,
                attributes={"transcode.rendition_count": len(rendition_ranges)}, parent_span_id=root_id,
            ),
            *rendition_spans,
        ],
        "drm-license-service": [
            otlp_span(
                trace_id, new_hex_id(16), "drm.license_request", 3, drm_start, drm_end,
                attributes={"drm.system": "widevine", "drm.key_id": new_hex_id(32)}, parent_span_id=root_id,
            ),
        ],
    }

    resource_spans = [otlp_resource_span(svc, s) for svc, s in buckets.items()]
    return otlp_doc(resource_spans)


# ---------------------------------------------------------------------------
# 7. Mobile RUM session (OTLP) - clock skew and zero-duration spans
# ---------------------------------------------------------------------------


def gen_mobile_rum_session() -> Dict[str, Any]:
    """Build an OTLP trace modeling a mobile app Real User Monitoring session.

    A cold-start session with view renders, several network fetches (each
    with nested DNS/TLS sub-spans), a batch of zero-duration UI gesture
    spans, and one deliberately clock-skewed network span that starts
    ~200ms before its parent (simulating device clock drift). Spans are
    split across three services (mobile-app for client-side view/gesture
    spans, backend-api and auth-service for the network-call spans) while
    keeping the original parent/child relationships across those service
    boundaries.

    Returns:
        The OTLP JSON document.
    """
    trace_id = new_hex_id(32)
    t = 1_713_100_000_000_000_000
    root_id = new_hex_id(16)

    children_ranges: List[Tuple[int, int]] = []
    buckets: Dict[str, List[Dict[str, Any]]] = {
        "mobile-app": [],
        "backend-api": [],
        "auth-service": [],
    }

    splash_start = t + ms(1)
    splash_end = splash_start + ms(random.uniform(150, 350))
    paint_start = splash_start + ms(2)
    paint_end = paint_start + ms(random.uniform(5, 20))
    layout_start = paint_end + ms(1)
    layout_end = layout_start + ms(random.uniform(5, 15))
    splash_id = new_hex_id(16)
    buckets["mobile-app"].append(
        otlp_span(trace_id, splash_id, "view.render.splash", 1, splash_start, splash_end,
                   attributes={"view.name": "SplashScreen"}, parent_span_id=root_id)
    )
    buckets["mobile-app"].append(
        otlp_span(trace_id, new_hex_id(16), "view.paint", 1, paint_start, paint_end,
                   attributes={"view.name": "SplashScreen"}, parent_span_id=splash_id)
    )
    buckets["mobile-app"].append(
        otlp_span(trace_id, new_hex_id(16), "view.layout", 1, layout_start, layout_end,
                   attributes={"view.name": "SplashScreen"}, parent_span_id=splash_id)
    )
    children_ranges.append((splash_start, splash_end))

    home_start = splash_end + ms(2)
    home_end = home_start + ms(random.uniform(250, 550))
    home_paint_start = home_start + ms(3)
    home_paint_end = home_paint_start + ms(random.uniform(8, 25))
    home_id = new_hex_id(16)
    buckets["mobile-app"].append(
        otlp_span(trace_id, home_id, "view.render.home", 1, home_start, home_end,
                   attributes={"view.name": "HomeScreen"}, parent_span_id=root_id)
    )
    buckets["mobile-app"].append(
        otlp_span(trace_id, new_hex_id(16), "view.paint", 1, home_paint_start, home_paint_end,
                   attributes={"view.name": "HomeScreen"}, parent_span_id=home_id)
    )
    children_ranges.append((home_start, home_end))

    fetch_cursor = home_start + ms(5)
    for name in ("config", "user_profile", "feed", "recommendations", "notifications", "ads", "search_suggestions", "analytics_beacon"):
        f_start = fetch_cursor
        f_end = f_start + ms(random.uniform(60, 220))
        f_id = new_hex_id(16)
        dns_start = f_start + ms(1)
        dns_end = dns_start + ms(random.uniform(2, 15))
        tls_start = dns_end + ms(0.5)
        tls_end = tls_start + ms(random.uniform(5, 30))
        # The user_profile fetch requires an identity token, so it (and its
        # DNS/TLS sub-spans) is modeled as a call into auth-service; the
        # rest of the network fetches go through backend-api.
        fetch_bucket = "auth-service" if name == "user_profile" else "backend-api"
        buckets[fetch_bucket].append(
            otlp_span(trace_id, f_id, f"network.fetch.{name}", 3, f_start, f_end,
                       attributes={"http.url": f"https://api.example.com/v1/{name}", "http.status_code": 200},
                       parent_span_id=root_id)
        )
        buckets[fetch_bucket].append(
            otlp_span(trace_id, new_hex_id(16), "network.dns_lookup", 1, dns_start, dns_end,
                       attributes={"dns.host": "api.example.com"}, parent_span_id=f_id)
        )
        buckets[fetch_bucket].append(
            otlp_span(trace_id, new_hex_id(16), "network.tls_handshake", 1, tls_start, tls_end,
                       attributes={"tls.version": "1.3"}, parent_span_id=f_id)
        )
        children_ranges.append((f_start, f_end))
        fetch_cursor = f_end + ms(random.uniform(5, 40))

    # intentional: clock skew - this child span's clock reports a start time
    # ~200ms BEFORE the root span's start, simulating device clock drift
    # between the RUM SDK's monotonic timer and its wall-clock timestamp.
    # It is deliberately NOT contained within the root span's time range.
    skew_start = t - ms(200)
    skew_end = skew_start + ms(random.uniform(40, 90))
    buckets["backend-api"].append(
        otlp_span(
            trace_id, new_hex_id(16), "network.fetch.remote_config", 3, skew_start, skew_end,
            attributes={"http.url": "https://api.example.com/v1/remote_config", "clock.skew_suspected": True},
            parent_span_id=root_id,
        )
    )

    gesture_cursor = fetch_cursor + ms(10)
    for i in range(20):
        # intentional: zero-duration span - an instantaneous UI event where
        # start == end, as commonly emitted by mobile gesture instrumentation.
        g_ts = gesture_cursor + ms(random.uniform(5, 40))
        buckets["mobile-app"].append(
            otlp_span(
                trace_id, new_hex_id(16), "gesture.tap", 1, g_ts, g_ts,
                attributes={"gesture.target": random.choice(["button.like", "button.share", "card.item", "tab.home"])},
                parent_span_id=root_id,
            )
        )
        gesture_cursor = g_ts

    summary_start = gesture_cursor + ms(5)
    summary_end = summary_start + ms(random.uniform(1, 5))
    buckets["mobile-app"].append(
        otlp_span(
            trace_id, new_hex_id(16), "session.summary", 1, summary_start, summary_end,
            attributes={"session.crash_free": True, "session.duration_ms": 0}, parent_span_id=root_id,
        )
    )
    children_ranges.append((summary_start, summary_end))

    root_start, root_end = span_range(children_ranges, 0, ms(5))
    root_start = t
    buckets["mobile-app"].insert(
        0,
        otlp_span(
            trace_id, root_id, "app.cold_start", 3, root_start, root_end,
            attributes={"app.version": "4.2.0", "device.os": "iOS", "device.os.version": "17.4", "session.id": f"sess_{new_hex_id(8)}"},
        ),
    )

    resource_spans = [otlp_resource_span(svc, s) for svc, s in buckets.items()]
    return otlp_doc(resource_spans)


# ---------------------------------------------------------------------------
# 8. Legal document RAG (OpenInference)
# ---------------------------------------------------------------------------

CLAUSE_CATEGORIES = [
    "indemnification", "termination", "limitation_of_liability", "confidentiality",
    "governing_law", "assignment", "force_majeure", "dispute_resolution",
]


def gen_legal_doc_rag() -> Dict[str, Any]:
    """Build an OpenInference trace modeling a legal contract-review RAG agent.

    Eight review rounds (one per clause category), each retrieving
    candidate clauses, reranking them, and running an LLM chain with a
    citation-lookup tool call. Model names (``gpt-4o``,
    ``claude-3-5-sonnet``) match entries in ``conventions/pricing.json``
    so WideScope's cost resolution can price the tokens.

    Returns:
        The OpenInference JSON document.
    """
    trace_id = new_hex_id(32)
    t = 1_713_200_000_000_000_000
    spans: List[Dict[str, Any]] = []
    root_id = "legal-root-001"

    round_ranges: List[Tuple[int, int]] = []
    cursor = t + ms(200)

    for i, category in enumerate(CLAUSE_CATEGORIES):
        retriever_id = f"legal-retriever-{i:03d}"
        reranker_id = f"legal-reranker-{i:03d}"
        chain_id = f"legal-chain-{i:03d}"
        llm_id = f"legal-llm-{i:03d}"
        tool_id = f"legal-tool-{i:03d}"

        retriever_start = cursor
        doc_attrs: Dict[str, Any] = {}
        for d in range(4):
            doc_attrs[f"retrieval.documents.{d}.document.id"] = f"contract-clause-{category}-{d}"
            doc_attrs[f"retrieval.documents.{d}.document.score"] = round(random.uniform(0.55, 0.97), 3)
            doc_attrs[f"retrieval.documents.{d}.document.content"] = (
                f"Sample synthetic clause text for {category} scenario {d}, "
                "for fixture purposes only and not real legal language."
            )
        retriever_end = retriever_start + ms(random.uniform(80, 300))
        spans.append(
            oi_span(
                trace_id, retriever_id, "retriever.search_clauses", "RETRIEVER", retriever_start, retriever_end,
                attributes={"retrieval.query": f"{category} clause precedent", **doc_attrs},
                parent_id=root_id,
            )
        )

        reranker_start = retriever_end + ms(5)
        reranker_end = reranker_start + ms(random.uniform(30, 100))
        spans.append(
            oi_span(
                trace_id, reranker_id, "reranker.rerank_documents", "RERANKER", reranker_start, reranker_end,
                attributes={"rerank.model": "cross-encoder-v2", "rerank.top_n": 4}, parent_id=root_id,
            )
        )

        chain_start = reranker_end + ms(5)
        llm_provider, llm_model = random.choice([("openai", "gpt-4o"), ("anthropic", "claude-3-5-sonnet")])
        prompt_tokens = random.randint(900, 3200)
        completion_tokens = random.randint(150, 900)
        llm_start = chain_start + ms(2)
        llm_end = llm_start + ms(random.uniform(400, 2200))
        spans.append(
            oi_span(
                trace_id, llm_id, "llm.analyze_clause", "LLM", llm_start, llm_end,
                attributes={
                    "llm.model_name": llm_model,
                    "llm.provider": llm_provider,
                    "llm.token_count.prompt": prompt_tokens,
                    "llm.token_count.completion": completion_tokens,
                    "llm.invocation_parameters.temperature": 0.1,
                    "clause.category": category,
                },
                parent_id=chain_id,
            )
        )

        tool_start = llm_end + ms(5)
        tool_end = tool_start + ms(random.uniform(20, 90))
        spans.append(
            oi_span(
                trace_id, tool_id, "tool.citation_lookup", "TOOL", tool_start, tool_end,
                attributes={
                    "tool.name": "citation_lookup",
                    "tool.arguments": f'{{"category": "{category}"}}',
                    "tool.result": f"3 precedents found for {category}",
                },
                parent_id=chain_id,
            )
        )

        chain_end = tool_end + ms(2)
        spans.append(
            oi_span(
                trace_id, chain_id, "llm.chain", "CHAIN", chain_start, chain_end,
                attributes={"chain.type": "sequential", "clause.category": category}, parent_id=root_id,
            )
        )

        round_ranges.append((retriever_start, chain_end))
        cursor = chain_end + ms(30)

    root_start, root_end = span_range(round_ranges, ms(50), ms(50))
    spans.insert(
        0,
        oi_span(
            trace_id, root_id, "legal.contract_review_agent", "AGENT", root_start, root_end,
            attributes={"workflow.name": "contract-clause-review", "contract.id": f"synthetic-contract-{new_hex_id(6)}"},
        ),
    )

    return oi_doc(spans, "legal-doc-rag")


# ---------------------------------------------------------------------------
# 9. Edge cases (OTLP) - deliberately weird but still valid JSON
# ---------------------------------------------------------------------------


def gen_edge_cases() -> Dict[str, Any]:
    """Build an OTLP trace stress-testing parser/viewer edge cases.

    All spans are valid JSON and parse without error (none of these
    conditions trip ``NoValidSpans`` in ``otlp_json.rs``, since span/trace
    id presence is the only hard requirement); they are semantically
    unusual on purpose, each flagged with an ``# intentional:`` comment
    at its construction site below.

    Returns:
        The OTLP JSON document.
    """
    trace_id = new_hex_id(32)
    t = 1_713_300_000_000_000_000
    spans: List[Dict[str, Any]] = []

    root_id = new_hex_id(16)
    root_start, root_end = t, t + sec(5)
    spans.append(
        otlp_span(trace_id, root_id, "edge_cases.root", 2, root_start, root_end,
                   attributes={"fixture.purpose": "edge-case stress test"})
    )

    normal_start, normal_end = root_start + ms(10), root_start + ms(200)
    spans.append(
        otlp_span(trace_id, new_hex_id(16), "normal.child", 1, normal_start, normal_end,
                   attributes={"note": "an ordinary, well-formed span for contrast"}, parent_span_id=root_id)
    )

    # intentional: orphan span - parentSpanId references a span id that does
    # not exist anywhere else in this file.
    spans.append(
        otlp_span(
            trace_id, new_hex_id(16), "edge_cases.orphan_child", 1, root_start + ms(20), root_start + ms(50),
            attributes={"note": "parent span id below does not exist in this file"},
            parent_span_id="ffffffffffffffff",
        )
    )

    # intentional: duplicate span id - two distinct spans deliberately share
    # the same spanId.
    dup_span_id = new_hex_id(16)
    spans.append(
        otlp_span(trace_id, dup_span_id, "edge_cases.duplicate_id.first", 1, root_start + ms(30), root_start + ms(60),
                   attributes={"duplicate.slot": 1}, parent_span_id=root_id)
    )
    spans.append(
        otlp_span(trace_id, dup_span_id, "edge_cases.duplicate_id.second", 1, root_start + ms(70), root_start + ms(90),
                   attributes={"duplicate.slot": 2}, parent_span_id=root_id)
    )

    # intentional: end time before start time in the raw JSON. The OTLP
    # parser swaps start/end when start > end, but the fixture deliberately
    # encodes them reversed to exercise that path.
    reversed_end = root_start + ms(40)
    reversed_start = root_start + ms(140)
    spans.append(
        otlp_span(trace_id, new_hex_id(16), "edge_cases.reversed_timestamps", 1, reversed_start, reversed_end,
                   attributes={"note": "startTimeUnixNano > endTimeUnixNano as authored"}, parent_span_id=root_id)
    )

    # intentional: zero-duration span - start == end exactly.
    zero_ts = root_start + ms(150)
    spans.append(
        otlp_span(trace_id, new_hex_id(16), "edge_cases.zero_duration", 1, zero_ts, zero_ts,
                   attributes={"note": "start equals end"}, parent_span_id=root_id)
    )

    # intentional: ~200 KB attribute value.
    huge_value = "lorem-ipsum-payload-" * 10_000  # ~200,000 characters
    spans.append(
        otlp_span(
            trace_id, new_hex_id(16), "edge_cases.huge_attribute_value", 1, root_start + ms(160), root_start + ms(200),
            attributes={"payload.blob": huge_value, "payload.size_chars": len(huge_value)},
            parent_span_id=root_id,
        )
    )

    # intentional: span name of exactly 2000 characters.
    long_name = ("span_name_segment_" * 112)[:2000]
    spans.append(
        otlp_span(trace_id, new_hex_id(16), long_name, 1, root_start + ms(210), root_start + ms(230),
                   attributes={"note": "span name is exactly 2000 characters"}, parent_span_id=root_id)
    )

    # intentional: deeply escaped/quoted string attribute.
    escaped_value = 'He said "it\'s \\"nested\\" and \\\\escaped\\\\" then left.\n\ttab\tand\nnewline.'
    spans.append(
        otlp_span(trace_id, new_hex_id(16), "edge_cases.escaped_quotes", 1, root_start + ms(240), root_start + ms(250),
                   attributes={"note.escaped": escaped_value}, parent_span_id=root_id)
    )

    # intentional: empty attribute value.
    spans.append(
        otlp_span(trace_id, new_hex_id(16), "edge_cases.empty_attribute_value", 1, root_start + ms(260), root_start + ms(270),
                   attributes={"note.empty": ""}, parent_span_id=root_id)
    )

    # intentional: very large int64 attribute, near i64::MAX.
    spans.append(
        otlp_span(
            trace_id, new_hex_id(16), "edge_cases.huge_int64", 1, root_start + ms(280), root_start + ms(290),
            attributes={"metric.huge_count": 9223372036854775807}, parent_span_id=root_id,
        )
    )

    resource_spans = [otlp_resource_span("edge-case-fixture", spans)]
    return otlp_doc(resource_spans)


# ---------------------------------------------------------------------------
# 10. Wide fan-out, ~2000 spans (OTLP)
# ---------------------------------------------------------------------------


def gen_wide_fanout() -> Dict[str, Any]:
    """Build an OTLP trace with ~2000 mostly-flat spans under two roots.

    Stresses layout/rendering rather than modeling a realistic domain:
    two root spans each fan out into roughly 1000 short, flat internal
    children. The root spans live in a "fanout-root" service while the
    2000 children are spread evenly across seven "worker-pool-N"
    services (eight services total), keeping each child's parent/child
    relationship to its (cross-service) root intact.

    Returns:
        The OTLP JSON document.
    """
    trace_id = new_hex_id(32)
    t = 1_713_400_000_000_000_000
    n_worker_pools = 7
    buckets: Dict[str, List[Dict[str, Any]]] = {"fanout-root": []}
    for pool in range(n_worker_pools):
        buckets[f"worker-pool-{pool}"] = []

    for root_idx in range(2):
        root_id = new_hex_id(16)
        root_start = t
        cursor = root_start + ms(1)
        child_ranges: List[Tuple[int, int]] = []
        children_by_pool: Dict[str, List[Dict[str, Any]]] = {
            f"worker-pool-{pool}": [] for pool in range(n_worker_pools)
        }
        n_children = 1000
        for i in range(n_children):
            c_start = cursor + random.randint(0, 2_000_000)  # jitter within ~2ms
            c_dur = random.randint(50_000, 500_000)  # 0.05-0.5ms
            c_end = c_start + c_dur
            pool_name = f"worker-pool-{i % n_worker_pools}"
            children_by_pool[pool_name].append(
                otlp_span(
                    trace_id, new_hex_id(16), f"fanout.worker_task.{i % 25}", 1, c_start, c_end,
                    attributes={"worker.index": i, "worker.shard": i % 25},
                    parent_span_id=root_id,
                )
            )
            child_ranges.append((c_start, c_end))
            cursor = max(cursor, c_end)

        root_end = max(e for _, e in child_ranges) + ms(1)
        buckets["fanout-root"].append(
            otlp_span(
                trace_id, root_id, f"fanout.root.{root_idx}", 2, root_start, root_end,
                attributes={"fanout.root_index": root_idx, "fanout.child_count": n_children},
            )
        )
        for pool_name, pool_children in children_by_pool.items():
            buckets[pool_name].extend(pool_children)
        t = root_end + ms(5)

    resource_spans = [otlp_resource_span(svc, s) for svc, s in buckets.items()]
    return otlp_doc(resource_spans)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main() -> None:
    """Generate all ten fixture files into ``test-fixtures/domains/`` and
    print a one-line summary (path, byte size, span count) for each.
    """
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    jobs: List[Tuple[str, str, Any]] = [
        ("otlp-kubernetes-control-plane.json", "otlp", gen_k8s_control_plane),
        ("otlp-banking-wire-transfer.json", "otlp", gen_banking_wire_transfer),
        ("jaeger-iot-telemetry-ingest.json", "jaeger", gen_iot_telemetry_ingest),
        ("otlp-cicd-pipeline.json", "otlp", gen_cicd_pipeline),
        ("jaeger-healthcare-fhir.json", "jaeger", gen_healthcare_fhir),
        ("otlp-video-streaming-cdn.json", "otlp", gen_video_streaming_cdn),
        ("otlp-mobile-rum-session.json", "otlp", gen_mobile_rum_session),
        ("openinference-legal-doc-rag.json", "openinference", gen_legal_doc_rag),
        ("otlp-edge-cases.json", "otlp", gen_edge_cases),
        ("otlp-wide-fanout-2000-spans.json", "otlp", gen_wide_fanout),
    ]

    summary_rows: List[Tuple[str, int, int]] = []
    for filename, fmt, generator in jobs:
        document = generator()
        out_path = OUTPUT_DIR / filename
        indent = None if filename == "otlp-wide-fanout-2000-spans.json" else 2
        text = json.dumps(document, indent=indent, ensure_ascii=False)
        out_path.write_text(text, encoding="utf-8")
        n_spans = count_spans(document, fmt)
        size_bytes = out_path.stat().st_size
        summary_rows.append((str(out_path), size_bytes, n_spans))

    print(f"{'path':<70} {'bytes':>10} {'spans':>8}")
    for path, size_bytes, n_spans in summary_rows:
        print(f"{path:<70} {size_bytes:>10} {n_spans:>8}")


if __name__ == "__main__":
    main()
