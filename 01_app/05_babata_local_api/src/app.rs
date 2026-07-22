use std::{io::Read, net::SocketAddr};

use babata_application::{
    CancelCollectionCommand, CollectorSessionService, ExploreService, RetryCollectionItemCommand,
    SearchQuery, StartCollectionCommand,
};
use babata_domain::{CollectionSelection, CollectionSessionId, ItemId, SourceRouteId};
use babata_infrastructure::{
    AppConfig, FileAssetStore, SqliteRawRepository, SqliteReadProjection, SystemClock,
    open_collection_database, sources::providers::browser::BrowserCandidateAdapter,
};
use serde::de::DeserializeOwned;
use serde_json::json;
use tiny_http::{Header, Request, Response, Server, StatusCode};

use crate::{
    ApiError,
    auth::verify_token,
    requests::{
        BrowserSessionRequest, CancelCollectionRequest, RecollectRequest, RetryCollectionRequest,
        SelectCollectionRequest,
    },
    responses::{ApiResponse, ErrorResponse},
    routes,
    state::ApiState,
};

const DEFAULT_MAX_BODY_BYTES: usize = 1024 * 1024;
const MAX_BROWSER_CANDIDATES: usize = 200;
const BROWSER_PAGE_ROUTE: &str = "source.browser_pages";
const BROWSER_BOOKMARK_ROUTE: &str = "source.browser_bookmarks";

#[derive(Debug, Clone)]
pub struct ApiDescriptor {
    pub state: ApiState,
    pub endpoints: Vec<routes::Endpoint>,
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    app: AppConfig,
    bind_address: SocketAddr,
    token: String,
    max_body_bytes: usize,
}

impl ServerConfig {
    pub fn new(app: AppConfig, bind_address: SocketAddr, token: String) -> Result<Self, ApiError> {
        ApiState::enabled(bind_address.ip())?;
        if token.len() < 32 {
            return Err(ApiError::InvalidRequest(
                "BABATA_BROWSER_TOKEN must contain at least 32 characters".to_owned(),
            ));
        }
        Ok(Self {
            app,
            bind_address,
            token,
            max_body_bytes: DEFAULT_MAX_BODY_BYTES,
        })
    }
}

pub fn build() -> ApiDescriptor {
    ApiDescriptor {
        state: ApiState::disabled(),
        endpoints: routes::all(),
    }
}

pub fn serve(config: ServerConfig) -> Result<(), ApiError> {
    let server =
        Server::http(config.bind_address).map_err(|error| ApiError::Io(error.to_string()))?;
    println!("Babata browser API listening on {}", config.bind_address);
    for request in server.incoming_requests() {
        let config = config.clone();
        std::thread::spawn(move || handle_request(request, &config));
    }
    Ok(())
}

fn handle_request(mut request: Request, config: &ServerConfig) {
    let method = request.method().to_string();
    let url = request.url().to_owned();
    let origin = header_value(&request, "Origin").map(str::to_owned);
    let result = if method == "OPTIONS" {
        verify_origin(origin.as_deref()).map(|()| DispatchResponse::empty(204))
    } else {
        let body = read_body(&mut request, config.max_body_bytes);
        body.and_then(|body| {
            verify_origin(origin.as_deref())?;
            let token = bearer_token(header_value(&request, "Authorization"));
            dispatch(config, &method, &url, token, &body)
        })
    };
    let response = match result {
        Ok(response) => response,
        Err(error) => DispatchResponse::error(&error),
    };
    let _ = request.respond(to_http_response(response, origin.as_deref()));
}

fn dispatch(
    config: &ServerConfig,
    method: &str,
    url: &str,
    supplied_token: Option<&str>,
    body: &[u8],
) -> Result<DispatchResponse, ApiError> {
    verify_token(&config.token, supplied_token)?;
    let (path, query) = split_url(url);
    let (status, value) = match (method, path) {
        ("GET", "/v1/health") => (
            200,
            json!({
                "enabled": true,
                "protocolVersion": "1",
                "routes": [BROWSER_PAGE_ROUTE, BROWSER_BOOKMARK_ROUTE],
                "maxBodyBytes": config.max_body_bytes,
                "maxCandidates": MAX_BROWSER_CANDIDATES,
            }),
        ),
        ("POST", "/v1/explore/search") => (200, search_projection(&config.app, body)?),
        ("POST", "/v1/collector/sessions") => {
            let request: BrowserSessionRequest = decode(body)?;
            validate_browser_start(&request)?;
            let route_id = SourceRouteId(request.route_id.clone());
            let service = browser_service(&config.app, Some((&route_id, request.candidates)))?;
            let session = service.start(StartCollectionCommand {
                route_id,
                source_reference: request.source_reference,
                scope_description: request.scope_description,
                authorisation_id: format!("browser-extension:{}", request.installation_id),
            })?;
            let candidates = service.candidates(&session.session_id)?;
            (201, json!({"session": session, "candidates": candidates}))
        }
        ("GET", "/v1/collector/candidates") => {
            let session_id = session_from_query(query)?;
            let service = browser_service(&config.app, None)?;
            (
                200,
                serde_json::to_value(service.candidates(&session_id)?).map_err(json_error)?,
            )
        }
        ("POST", "/v1/collector/select") => {
            let request: SelectCollectionRequest = decode(body)?;
            let session_id = parse_session_id(request.session_id)?;
            let service = browser_service(&config.app, None)?;
            let session = service.session(&session_id)?;
            let items = service.select(CollectionSelection {
                session_id,
                candidate_ids: request.candidate_ids,
                scope_description: request.scope_description,
                confirmed: request.confirmed,
                authorised_context: session.authorisation_id,
                requested_attachments: false,
            })?;
            (200, serde_json::to_value(items).map_err(json_error)?)
        }
        ("GET", "/v1/collector/status") => {
            let session_id = session_from_query(query)?;
            let service = browser_service(&config.app, None)?;
            (
                200,
                serde_json::to_value(service.status(&session_id)?).map_err(json_error)?,
            )
        }
        ("POST", "/v1/collector/retry") => {
            let request: RetryCollectionRequest = decode(body)?;
            let service = browser_service(&config.app, None)?;
            let item = service.retry(RetryCollectionItemCommand {
                session_id: parse_session_id(request.session_id)?,
                candidate_id: request.candidate_id,
            })?;
            (200, serde_json::to_value(item).map_err(json_error)?)
        }
        ("POST", "/v1/collector/cancel") => {
            let request: CancelCollectionRequest = decode(body)?;
            let service = browser_service(&config.app, None)?;
            let items = service.cancel(CancelCollectionCommand {
                session_id: parse_session_id(request.session_id)?,
                reason: request.reason,
            })?;
            (200, serde_json::to_value(items).map_err(json_error)?)
        }
        ("POST", "/v1/collector/recollect") => {
            let request: RecollectRequest = decode(body)?;
            let service = browser_service(&config.app, None)?;
            let item_id = ItemId::parse(request.item_id)
                .map_err(|error| ApiError::Application(error.into()))?;
            let outcome = service.recollect(&item_id)?;
            (200, serde_json::to_value(outcome).map_err(json_error)?)
        }
        _ => {
            return Err(ApiError::InvalidRequest(format!(
                "unsupported local API route: {method} {path}"
            )));
        }
    };
    DispatchResponse::json(status, &ApiResponse { data: value })
}

fn search_projection(app: &AppConfig, body: &[u8]) -> Result<serde_json::Value, ApiError> {
    let request: SearchQuery = decode(body)?;
    let service = ExploreService::new(SqliteReadProjection::new(
        app.paths(),
        app.sqlite.busy_timeout_ms,
    ));
    serde_json::to_value(service.search(request)?).map_err(json_error)
}

type BrowserService = CollectorSessionService<SqliteRawRepository, FileAssetStore, SystemClock>;

fn browser_service(
    config: &AppConfig,
    active: Option<(&SourceRouteId, Vec<babata_domain::CandidateEnvelope>)>,
) -> Result<BrowserService, ApiError> {
    let repository = open_collection_database(&config.paths(), config.sqlite.busy_timeout_ms)?;
    let adapters = [BROWSER_PAGE_ROUTE, BROWSER_BOOKMARK_ROUTE]
        .into_iter()
        .map(|route| {
            let route_id = SourceRouteId(route.to_owned());
            let candidates = active
                .as_ref()
                .filter(|(active_route, _)| **active_route == route_id)
                .map(|(_, candidates)| candidates.clone())
                .unwrap_or_default();
            Box::new(BrowserCandidateAdapter::for_route(route_id, candidates))
                as Box<dyn babata_application::ports::SourceAdapterPort>
        })
        .collect();
    Ok(CollectorSessionService::new(
        repository,
        FileAssetStore::new(config.paths()),
        SystemClock,
        adapters,
    ))
}

fn validate_browser_start(request: &BrowserSessionRequest) -> Result<(), ApiError> {
    if !matches!(
        request.route_id.as_str(),
        BROWSER_PAGE_ROUTE | BROWSER_BOOKMARK_ROUTE
    ) {
        return Err(ApiError::InvalidRequest(
            "only browser page and bookmark routes may use this endpoint".to_owned(),
        ));
    }
    if !request.source_reference.starts_with("submitted:") {
        return Err(ApiError::InvalidRequest(
            "browser sourceReference must identify a submitted batch".to_owned(),
        ));
    }
    if request.scope_description.trim().is_empty() {
        return Err(ApiError::InvalidRequest(
            "scopeDescription is required".to_owned(),
        ));
    }
    if request.installation_id.is_empty()
        || request.installation_id.len() > 80
        || !request
            .installation_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(ApiError::InvalidRequest(
            "installationId is invalid".to_owned(),
        ));
    }
    if request.candidates.is_empty() || request.candidates.len() > MAX_BROWSER_CANDIDATES {
        return Err(ApiError::InvalidRequest(format!(
            "candidate count must be between 1 and {MAX_BROWSER_CANDIDATES}"
        )));
    }
    Ok(())
}

fn decode<T: DeserializeOwned>(body: &[u8]) -> Result<T, ApiError> {
    serde_json::from_slice(body)
        .map_err(|_| ApiError::InvalidRequest("request body is invalid JSON".to_owned()))
}

fn json_error(error: serde_json::Error) -> ApiError {
    ApiError::Io(error.to_string())
}

fn split_url(url: &str) -> (&str, Option<&str>) {
    url.split_once('?')
        .map_or((url, None), |(path, query)| (path, Some(query)))
}

fn session_from_query(query: Option<&str>) -> Result<CollectionSessionId, ApiError> {
    let value = query
        .and_then(|query| {
            url::form_urlencoded::parse(query.as_bytes())
                .find(|(key, _)| key == "session")
                .map(|(_, value)| value.into_owned())
        })
        .ok_or_else(|| {
            ApiError::InvalidRequest("session query parameter is required".to_owned())
        })?;
    parse_session_id(value)
}

fn parse_session_id(value: impl AsRef<str>) -> Result<CollectionSessionId, ApiError> {
    CollectionSessionId::parse(value).map_err(|error| ApiError::Application(error.into()))
}

fn read_body(request: &mut Request, max_bytes: usize) -> Result<Vec<u8>, ApiError> {
    if request
        .body_length()
        .is_some_and(|length| length > max_bytes)
    {
        return Err(ApiError::PayloadTooLarge);
    }
    let mut body = Vec::new();
    request
        .as_reader()
        .take((max_bytes + 1) as u64)
        .read_to_end(&mut body)
        .map_err(|error| ApiError::Io(error.to_string()))?;
    if body.len() > max_bytes {
        return Err(ApiError::PayloadTooLarge);
    }
    Ok(body)
}

fn header_value<'a>(request: &'a Request, name: &str) -> Option<&'a str> {
    request
        .headers()
        .iter()
        .find(|header| header.field.as_str().as_str().eq_ignore_ascii_case(name))
        .map(|header| header.value.as_str())
}

fn bearer_token(value: Option<&str>) -> Option<&str> {
    value.and_then(|value| value.strip_prefix("Bearer "))
}

fn verify_origin(origin: Option<&str>) -> Result<(), ApiError> {
    let Some(origin) = origin else {
        return Ok(());
    };
    let parsed = url::Url::parse(origin).map_err(|_| ApiError::OriginForbidden)?;
    if parsed.scheme() == "chrome-extension"
        && parsed.host_str().is_some_and(|host| !host.is_empty())
    {
        Ok(())
    } else {
        Err(ApiError::OriginForbidden)
    }
}

#[derive(Debug)]
struct DispatchResponse {
    status: u16,
    body: Vec<u8>,
}

impl DispatchResponse {
    fn empty(status: u16) -> Self {
        Self {
            status,
            body: Vec::new(),
        }
    }

    fn json<T: serde::Serialize>(status: u16, value: &T) -> Result<Self, ApiError> {
        Ok(Self {
            status,
            body: serde_json::to_vec(value).map_err(json_error)?,
        })
    }

    fn error(error: &ApiError) -> Self {
        let body = serde_json::to_vec(&ErrorResponse {
            code: error.code().to_owned(),
            message: error.to_string(),
        })
        .unwrap_or_else(|_| {
            b"{\"code\":\"internal_error\",\"message\":\"serialization failed\"}".to_vec()
        });
        Self {
            status: error.status_code(),
            body,
        }
    }
}

fn to_http_response(
    response: DispatchResponse,
    origin: Option<&str>,
) -> Response<std::io::Cursor<Vec<u8>>> {
    let mut response = Response::from_data(response.body)
        .with_status_code(StatusCode(response.status))
        .with_header(header("Content-Type", "application/json; charset=utf-8"))
        .with_header(header("Cache-Control", "no-store"))
        .with_header(header(
            "Access-Control-Allow-Headers",
            "Authorization, Content-Type",
        ))
        .with_header(header("Access-Control-Allow-Methods", "GET, POST, OPTIONS"))
        .with_header(header("Access-Control-Allow-Private-Network", "true"));
    if let Some(origin) = origin.filter(|origin| verify_origin(Some(origin)).is_ok()) {
        response.add_header(header("Access-Control-Allow-Origin", origin));
        response.add_header(header("Vary", "Origin"));
    }
    response
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name.as_bytes(), value.as_bytes()).expect("static HTTP header is valid")
}

#[cfg(test)]
mod tests {
    use babata_domain::{
        CandidateEnvelope, CandidatePayload, ContentType, Metadata, Sha256, SourceRouteId,
    };
    use babata_infrastructure::{DataRoot, SqliteOptions};
    use serde_json::Value;
    use tempfile::tempdir;

    use super::*;

    const TOKEN: &str = "0123456789abcdef0123456789abcdef";

    #[test]
    fn server_config_rejects_public_bindings_and_short_tokens() {
        let temporary = tempdir().unwrap();
        let app = test_config(temporary.path());
        assert!(
            ServerConfig::new(
                app.clone(),
                "0.0.0.0:43873".parse().unwrap(),
                TOKEN.to_owned()
            )
            .is_err()
        );
        assert!(
            ServerConfig::new(app, "127.0.0.1:43873".parse().unwrap(), "short".to_owned()).is_err()
        );
    }

    #[test]
    fn browser_origin_is_narrowly_limited() {
        assert!(verify_origin(None).is_ok());
        assert!(verify_origin(Some("chrome-extension://abcdefghijklmnop")).is_ok());
        assert!(verify_origin(Some("https://example.test")).is_err());
    }

    #[test]
    fn explore_api_uses_the_same_rebuildable_projection_contract() {
        let temporary = tempdir().unwrap();
        let app = test_config(temporary.path());
        ExploreService::new(SqliteReadProjection::new(
            app.paths(),
            app.sqlite.busy_timeout_ms,
        ))
        .rebuild()
        .unwrap();
        let config =
            ServerConfig::new(app, "127.0.0.1:43873".parse().unwrap(), TOKEN.to_owned()).unwrap();
        let response = dispatch(
            &config,
            "POST",
            "/v1/explore/search",
            Some(TOKEN),
            br#"{"filter":{"text":"anything","limit":10}}"#,
        )
        .unwrap();
        assert_eq!(response.status, 200);
        let response: Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(response["data"]["records"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn browser_api_discovers_before_explicit_selection_and_writes_only_selected() {
        let temporary = tempdir().unwrap();
        babata_infrastructure::paths::ensure_layout(&babata_infrastructure::paths::DataPaths::new(
            temporary.path().to_path_buf(),
        ))
        .unwrap();
        let config = ServerConfig::new(
            test_config(temporary.path()),
            "127.0.0.1:43873".parse().unwrap(),
            TOKEN.to_owned(),
        )
        .unwrap();
        assert_eq!(
            dispatch(&config, "GET", "/v1/health", Some("wrong"), b"")
                .unwrap_err()
                .status_code(),
            401
        );

        let start = json!({
            "routeId": BROWSER_PAGE_ROUTE,
            "sourceReference": "submitted:test-batch",
            "scopeDescription": "one visible test page",
            "installationId": "test-installation",
            "candidates": [candidate("https://example.test/one", "One"), candidate("https://example.test/two", "Two")],
        });
        let started = dispatch(
            &config,
            "POST",
            "/v1/collector/sessions",
            Some(TOKEN),
            &serde_json::to_vec(&start).unwrap(),
        )
        .unwrap();
        assert_eq!(started.status, 201);
        let started: Value = serde_json::from_slice(&started.body).unwrap();
        let session = started["data"]["session"]["session_id"].as_str().unwrap();
        let candidates = started["data"]["candidates"].as_array().unwrap();
        assert_eq!(candidates.len(), 2);
        let selected_candidate = candidates[0]["candidate_id"].as_str().unwrap();

        let unconfirmed = json!({
            "sessionId": session,
            "candidateIds": [selected_candidate],
            "scopeDescription": "one visible test page",
            "confirmed": false,
        });
        assert_eq!(
            dispatch(
                &config,
                "POST",
                "/v1/collector/select",
                Some(TOKEN),
                &serde_json::to_vec(&unconfirmed).unwrap(),
            )
            .unwrap_err()
            .status_code(),
            409
        );
        let empty_status = dispatch(
            &config,
            "GET",
            &format!("/v1/collector/status?session={session}"),
            Some(TOKEN),
            b"",
        )
        .unwrap();
        let empty_status: Value = serde_json::from_slice(&empty_status.body).unwrap();
        assert_eq!(empty_status["data"].as_array().unwrap().len(), 0);

        let confirmed = json!({
            "sessionId": session,
            "candidateIds": [selected_candidate],
            "scopeDescription": "one visible test page",
            "confirmed": true,
        });
        let selected = dispatch(
            &config,
            "POST",
            "/v1/collector/select",
            Some(TOKEN),
            &serde_json::to_vec(&confirmed).unwrap(),
        )
        .unwrap();
        let selected: Value = serde_json::from_slice(&selected.body).unwrap();
        let items = selected["data"].as_array().unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0]["state"], "saved");
        assert!(items[0]["item_id"].as_str().is_some());
    }

    fn test_config(path: &std::path::Path) -> AppConfig {
        AppConfig {
            data_root: DataRoot(path.to_path_buf()),
            sqlite: SqliteOptions {
                busy_timeout_ms: 100,
            },
        }
    }

    fn candidate(url: &str, title: &str) -> CandidateEnvelope {
        let text = format!("{title}\n{url}");
        CandidateEnvelope {
            protocol_version: "1".to_owned(),
            route_id: SourceRouteId(BROWSER_PAGE_ROUTE.to_owned()),
            source_reference: url.to_owned(),
            content_type: ContentType::WebPage,
            payload_sha256: Sha256::of_bytes(text.as_bytes()),
            metadata: Metadata::parse(&json!({"title": title, "captureKind": "page"}).to_string())
                .unwrap(),
            payload: CandidatePayload::Text { text },
            context: Some("visible test page".to_owned()),
            native_id: None,
            common_metadata: babata_domain::CommonSourceMetadata::default(),
        }
    }
}
