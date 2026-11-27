use super::*;

// Publication content belongs here; CLI coverage follows the published links.
#[test]
fn checked_bundle_preserves_standard_library_documentation_content() {
    let bundle = checked_bundle().unwrap();
    let cases: &[(&str, &str, &[&str])] = &[
        ("", "", &["- Package identity: std"]),
        (
            "prelude",
            "",
            &[
                "- Source path: prelude.veln",
                "[alias byte_chunk_len](",
                "[alias byte_view_len](",
            ],
        ),
        (
            "prelude",
            "alias Byte",
            &["- Kind: alias", "- Target: bytes::Byte"],
        ),
        (
            "net",
            "",
            &[
                "- Source path: net.veln",
                "[alias Listener](",
                "[alias Stream](",
                "[alias ByteCount](",
                "[type ReadOutcome](",
                "[type WriteOutcome](",
                "fn split_host_port(address: String)",
                "fn write_all(stream: Stream, bytes: ByteChunk, deadline: Option<Deadline>, token: Option<CancelToken>)",
                "type Address",
                "type Endpoint",
                "type NetErrorKind",
                "type NetError",
                "fn connect_with(address: Address, deadline: Option<Deadline>, token: Option<CancelToken>) -> Result<Stream, NetError> effects [IO]",
            ],
        ),
        (
            "net",
            "type Network",
            &[
                "# Type Network",
                "### Tcp6",
                "- Signature: type Network",
                "### Tcp4",
            ],
        ),
        (
            "net",
            "function join_host_port",
            &[
                "# Function join_host_port",
                "fn join_host_port(host: String, port: Int) -> Result<String, NetError>",
            ],
        ),
        (
            "net",
            "type Address",
            &["Address { network: Network, host: String, port: Int }"],
        ),
        (
            "net",
            "type Endpoint",
            &["Endpoint { network: Network, address: String }"],
        ),
        (
            "net",
            "type NetErrorKind",
            &["InvalidAddress", "InvalidResource"],
        ),
        (
            "net",
            "type NetError",
            &[
                "NetError { operation: String, network: Option<Network>, address: Option<String>, kind: NetErrorKind, message: String }",
            ],
        ),
        (
            "net",
            "function split_host_port",
            &[
                "fn split_host_port(address: String) -> Result<{ host : String, port : Int }, NetError>",
            ],
        ),
        (
            "net",
            "effect IO",
            &[
                "# Effect IO",
                "resolve(address: Address) -> Result<List<Endpoint>, NetError>",
                "close_stream(stream: Stream) -> Result<(), NetError>",
            ],
        ),
        (
            "net",
            "handler system",
            &[
                "# Handler system",
                "- Kind: handler",
                "- Signature: handler system() for IO effects [net, time]",
                "fn resolve_with_system() -> Result<List<Endpoint>, NetError> effects [net, time]",
                "fn listen_with_system() -> Result<(), NetError> effects [net, time]",
            ],
        ),
        (
            "net",
            "function write_all",
            &[
                "# Function write_all",
                "Write the complete byte chunk through the active IO handler.",
            ],
        ),
        (
            "observe",
            "",
            &[
                "# Module observe",
                "handler noop() for Observe",
                "handler buffered_json_lines(exporter: BufferedJsonLinesExporter) for Observe effects [time, concurrency]",
                "fn buffered_start_export_span(exporter: BufferedJsonLinesExporter, request: SpanRequest) -> BufferedSpanStart effects [time, concurrency]",
                "fn shutdown_buffered(exporter: BufferedJsonLinesExporter) -> BufferedShutdownReport effects [concurrency]",
            ],
        ),
        (
            "observe",
            "handler noop",
            &["# Handler noop", "- Signature: handler noop() for Observe"],
        ),
        (
            "observe",
            "handler recording",
            &[
                "# Handler recording",
                "handler recording(recording: Recording) for Observe effects [time, concurrency]",
            ],
        ),
        (
            "trace",
            "",
            &[
                "# Module trace",
                "fn in_span<effect E>(request: SpanRequest, body: fn(Span) -> Result<(), String> effects [...E])",
            ],
        ),
        (
            "trace",
            "function in_span",
            &[
                "# Function in_span",
                "fn in_span<effect E>(request: SpanRequest, body: fn(Span) -> Result<(), String> effects [...E]) -> Result<(), String> effects [observe::Observe, ...E]",
            ],
        ),
        ("observe", "type Production", &["# Type Production"]),
        (
            "observe",
            "type ProductionHealth",
            &[
                "ProductionHealth { metric_series: Int, admitted_metric_observations: Int, dropped_metric_observations: Int }",
            ],
        ),
        (
            "observe",
            "function new_production",
            &["fn new_production(metric_series_limit: Int) -> Production effects [concurrency]"],
        ),
        (
            "observe",
            "function production_health",
            &[
                "fn production_health(production: Production) -> ProductionHealth effects [concurrency]",
            ],
        ),
        (
            "observe",
            "handler production",
            &["handler production(production: Production) for Observe effects [concurrency]"],
        ),
    ];
    for (module, label, expected) in cases {
        let resource = publication_resource(&bundle, module, label);
        for text in *expected {
            assert!(
                resource.text.contains(text),
                "{module}::{label}: missing {text:?}"
            );
        }
    }
    for unpublished in [
        "prelude_builtin::net_system_",
        "net_system_resolve",
        "NoopSpanRequest",
        "NoopSpanFinish",
        "NoopSpan",
        "byte_decode_schema_width_sample",
        "byte_decode_schema_validation_sample",
    ] {
        assert!(
            !checked_artifact_bytes().contains(unpublished),
            "published excluded detail {unpublished}"
        );
    }
}

// Scope declaration lookup to its owning module: different modules may export
// the same name, such as net::write_all and io::write_all.
fn publication_resource<'a>(
    bundle: &'a CheckedBundle,
    module: &str,
    label: &str,
) -> &'a CheckedResource {
    let title = if module.is_empty() {
        "Veln package documentation: std".to_owned()
    } else {
        format!("Veln package module: {module}")
    };
    let modules: Vec<_> = bundle
        .resources
        .iter()
        .filter(|r| r.title == title)
        .collect();
    assert_eq!(modules.len(), 1, "expected one resource for {title}");
    if label.is_empty() {
        return modules[0];
    }
    let prefix = format!("- [{label}](");
    let links: Vec<_> = modules[0]
        .text
        .lines()
        .filter_map(|line| line.strip_prefix(&prefix))
        .collect();
    assert_eq!(links.len(), 1, "expected one link for {module}::{label}");
    let uri = links[0].split_once(')').unwrap().0;
    bundle
        .resources
        .iter()
        .find(|r| r.uri == uri)
        .unwrap_or_else(|| panic!("missing linked declaration {module}::{label}"))
}
