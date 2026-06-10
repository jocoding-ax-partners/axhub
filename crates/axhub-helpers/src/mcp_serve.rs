//! stdio MCP 서버 (Track H frontend 3, plan §D.1). validator/site-scan 엔진을
//! `validate` / `scan_sites` MCP tool 로 노출해요. CLI subcommand·PostToolUse hook
//! 과 **같은 엔진 한 벌**을 쓰고(`ast_validate`/`site_scan`), transport 는 stdio
//! (`transport-io`)만 — http 스택은 안 써요.
//!
//! 사용자 코드는 로컬에서만 분석하고 서버로 전송하지 않아요(위치 원칙).

use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    schemars::{self, JsonSchema},
    tool, tool_handler, tool_router,
    transport::stdio,
    ErrorData, ServerHandler, ServiceExt,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValidateRequest {
    /// 검사할 파일 또는 디렉터리 경로 (1개 이상).
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ScanSitesRequest {
    /// 스캔할 파일 또는 디렉터리 경로 (1개 이상).
    pub paths: Vec<String>,
}

#[derive(Clone)]
pub struct AxhubHelpersMcp {
    tool_router: ToolRouter<Self>,
}

impl Default for AxhubHelpersMcp {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router(router = tool_router)]
impl AxhubHelpersMcp {
    #[must_use]
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "정적 AST 패턴 validator — 편집된 코드의 SDK 데이터/HTTP 계약 위반(or()/not() 비-pushable 필터, after/before 커서, /api/v1 prefix 누락, raw-http 데이터 엔드포인트 직타, use-client 컴포넌트의 server-only import)을 검출해요. block/advisory 로 분류한 JSON 을 내요."
    )]
    pub async fn validate(
        &self,
        Parameters(req): Parameters<ValidateRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let output = crate::ast_validate::validate_paths(&req.paths)
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        success_json(output)
    }

    #[tool(
        description = "변환 사이트 스캐너 — migrate 변환 후보(raw HTTP client 직타, 직접 DB driver, 하드코딩 API URL)를 6언어 AST 로 찾아 {file,line,kind,snippet} JSON 으로 내요."
    )]
    pub async fn scan_sites(
        &self,
        Parameters(req): Parameters<ScanSitesRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        let output = crate::site_scan::scan_paths(&req.paths);
        success_json(output)
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for AxhubHelpersMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "axhub-helpers stdio MCP — 로컬 코드 정적 검증 2 tool. `validate`(편집 코드의 SDK 데이터/HTTP 계약 위반 검출, block/advisory), `scan_sites`(migrate 변환 후보 탐지). 사용자 코드는 로컬에서만 분석하고 서버로 전송하지 않아요. 원격 SDK 지식 검색은 별도 ax-mcp `sdk_search` 를 써요.",
        )
    }
}

fn success_json(value: impl Serialize) -> Result<CallToolResult, ErrorData> {
    Ok(CallToolResult::success(vec![Content::json(value)?]))
}

/// `mcp-serve` 진입점. 자체 tokio 런타임에서 stdio MCP 서버를 띄워요(`main()` 은
/// sync 라 ambient 런타임이 없어요). 클라이언트 연결이 닫힐 때까지 대기해요.
pub fn run_mcp_serve() -> anyhow::Result<i32> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let service = AxhubHelpersMcp::new().serve(stdio()).await?;
        service.waiting().await?;
        Ok::<(), anyhow::Error>(())
    })?;
    Ok(0)
}
