use serde_json::json;

const DEFAULT_BASE_URL: &str = "https://axhub-api.jocodingax.ai";
const DEFAULT_ROW_LIMIT: u64 = 100;
const _: &str = include_str!("../templates/mode-a-ts.tmpl");
const _: &str = include_str!("../templates/mode-b-python.tmpl");
const _: &str = include_str!("../templates/mode-b-ts.tmpl");
const _: &str = include_str!("../templates/mode-b-go.tmpl");
const _: &str = include_str!("../templates/mode-b-shell.tmpl");

#[derive(Debug, Clone)]
struct SnippetArgs {
    mode: String,
    language: String,
    target: String,
    tenant: Option<String>,
    connector: String,
    path: String,
    sql: String,
    allowed_columns: String,
    masked: String,
    row_limit: u64,
    base_url: String,
}

pub fn run_snippet(args: &[String]) -> anyhow::Result<i32> {
    let parsed = match SnippetArgs::parse(args) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("axhub-helpers snippet: {message}\n\n{USAGE}");
            return Ok(64);
        }
    };

    match render(&parsed) {
        Ok(snippet) => {
            print!("{snippet}");
            Ok(0)
        }
        Err(message) => {
            eprintln!("axhub-helpers snippet: {message}\n\n{USAGE}");
            Ok(64)
        }
    }
}

const USAGE: &str = "Usage: axhub-helpers snippet --mode A|B --language typescript|python|go|shell --target <target> --connector <name> --path <resource/path> --sql <read-sql> --allowed-columns <csv> [--masked <csv>] [--tenant <tenant>] [--row-limit <n>] [--base-url <url>]";

impl SnippetArgs {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut mode: Option<String> = None;
        let mut language: Option<String> = None;
        let mut target: Option<String> = None;
        let mut tenant: Option<String> = None;
        let mut connector: Option<String> = None;
        let mut path: Option<String> = None;
        let mut sql: Option<String> = None;
        let mut allowed_columns: Option<String> = None;
        let mut masked = String::new();
        let mut row_limit = DEFAULT_ROW_LIMIT;
        let mut base_url = DEFAULT_BASE_URL.to_string();

        let mut i = 0;
        while i < args.len() {
            let flag = args[i].as_str();
            let value = |i: &mut usize| -> Result<String, String> {
                *i += 1;
                args.get(*i)
                    .cloned()
                    .ok_or_else(|| format!("missing value for {flag}"))
            };

            match flag {
                "--mode" => mode = Some(value(&mut i)?),
                "--language" | "--lang" => language = Some(value(&mut i)?),
                "--target" => target = Some(value(&mut i)?),
                "--tenant" => tenant = Some(value(&mut i)?),
                "--connector" => connector = Some(value(&mut i)?),
                "--path" => path = Some(value(&mut i)?),
                "--sql" => sql = Some(value(&mut i)?),
                "--allowed-columns" => allowed_columns = Some(value(&mut i)?),
                "--masked" => masked = value(&mut i)?,
                "--row-limit" => {
                    let raw = value(&mut i)?;
                    row_limit = raw.parse::<u64>().map_err(|_| {
                        format!("--row-limit must be a positive integer, got {raw:?}")
                    })?;
                    if row_limit == 0 {
                        return Err("--row-limit must be greater than 0".to_string());
                    }
                }
                "--base-url" => base_url = value(&mut i)?,
                "--help" | "-h" => return Err("help requested".to_string()),
                unknown => return Err(format!("unknown flag {unknown:?}")),
            }
            i += 1;
        }

        let mode = mode.ok_or_else(|| "missing --mode".to_string())?;
        if mode != "A" && mode != "B" {
            return Err("--mode must be A or B".to_string());
        }

        Ok(Self {
            mode,
            language: language.ok_or_else(|| "missing --language".to_string())?,
            target: target.ok_or_else(|| "missing --target".to_string())?,
            tenant,
            connector: connector.ok_or_else(|| "missing --connector".to_string())?,
            path: path.ok_or_else(|| "missing --path".to_string())?,
            sql: sql.ok_or_else(|| "missing --sql".to_string())?,
            allowed_columns: allowed_columns
                .ok_or_else(|| "missing --allowed-columns".to_string())?,
            masked,
            row_limit,
            base_url,
        })
    }
}

fn render(args: &SnippetArgs) -> Result<String, String> {
    let lang = args.language.to_ascii_lowercase();
    let is_shell = matches!(lang.as_str(), "shell" | "bash" | "sh");
    if args.target == "local-bash" {
        if is_shell {
            return Ok(render_local_bash(args));
        }
        return Err(
            "target local-bash only supports shell snippets; use a local-* HTTP target for other languages"
                .to_string(),
        );
    }

    match (args.mode.as_str(), lang.as_str()) {
        ("A", "typescript") | ("A", "ts") => Ok(render_mode_a_typescript(args)),
        ("B", "python") | ("B", "py") => Ok(render_mode_b_python(args)),
        ("B", "typescript") | ("B", "ts") | ("B", "javascript") | ("B", "js") => {
            Ok(render_mode_b_typescript(args))
        }
        ("B", "go") | ("B", "golang") => Ok(render_mode_b_go(args)),
        ("B", "shell") | ("B", "bash") | ("B", "sh") => Ok(render_mode_b_shell(args)),
        _ => Err(format!(
            "unsupported mode/language combination: mode={} language={}",
            args.mode, args.language
        )),
    }
}

fn metadata(args: &SnippetArgs) -> String {
    format!(
        "mode={} target={} connector={} path={} allowed_columns={} masked={}",
        args.mode, args.target, args.connector, args.path, args.allowed_columns, args.masked
    )
}

fn render_mode_a_typescript(args: &SnippetArgs) -> String {
    let connector = js_string(&args.connector);
    let path = url_encode(&args.path);
    let path_literal = js_string(&path);
    let sql = js_string(&args.sql);
    let tenant = js_string(args.tenant.as_deref().unwrap_or("YOUR_TENANT_SLUG"));
    format!(
        r#"/**
 * axhub data snippet
 * {metadata}
 * Auth: browser cookie session via credentials include. Do not add manual auth headers.
 */
export async function readAxhubData() {{
  const tenant = {tenant};
  const connector = {connector};
  const encodedPath = {path_literal};
  const response = await fetch(`/api/v1/tenants/${{encodeURIComponent(tenant)}}/catalog/resources/${{encodeURIComponent(connector)}}/${{encodedPath}}:read`, {{
    method: 'POST',
    credentials: 'include',
    headers: {{ 'Content-Type': 'application/json' }},
    body: JSON.stringify({{ sql: {sql}, row_limit: {row_limit} }}),
  }});
  if (!response.ok) {{
    throw new Error(`axhub read failed: ${{response.status}} ${{await response.text()}}`);
  }}
  return response.json();
}}
"#,
        metadata = metadata(args),
        tenant = tenant,
        connector = connector,
        path_literal = path_literal,
        sql = sql,
        row_limit = args.row_limit
    )
}

fn render_mode_b_python(args: &SnippetArgs) -> String {
    let connector = py_string(&url_encode(&args.connector));
    let path = py_string(&url_encode(&args.path));
    let sql = py_string(&args.sql);
    let tenant_comment = args
        .tenant
        .as_deref()
        .map(|tenant| format!("# tenant={}\n", tenant))
        .unwrap_or_default();
    format!(
        r#"# axhub data snippet
# {metadata}
{tenant_comment}# Auth: export AXHUB_PAT; send it as X-Api-Key. Do not hardcode PATs.
import os
import requests

AXHUB_BASE_URL = os.environ.get("AXHUB_BASE_URL", {base_url})
AXHUB_PAT = os.environ["AXHUB_PAT"]
AXHUB_TENANT = os.environ["AXHUB_TENANT"]

url = f"{{AXHUB_BASE_URL}}/api/v1/tenants/{{AXHUB_TENANT}}/catalog/resources/" + {connector} + "/" + {path} + ":read"
response = requests.post(
    url,
    headers={{"Content-Type": "application/json", "X-Api-Key": AXHUB_PAT}},
    json={{"sql": {sql}, "row_limit": {row_limit}}},
    timeout=30,
)
response.raise_for_status()
print(response.json())
"#,
        metadata = metadata(args),
        tenant_comment = tenant_comment,
        base_url = py_string(&args.base_url),
        connector = connector,
        path = path,
        sql = sql,
        row_limit = args.row_limit
    )
}

fn render_mode_b_typescript(args: &SnippetArgs) -> String {
    let connector = js_string(&args.connector);
    let path = js_string(&url_encode(&args.path));
    let sql = js_string(&args.sql);
    format!(
        r#"/**
 * axhub data snippet
 * {metadata}
 * Auth: read AXHUB_PAT from env and send it as X-Api-Key. Do not hardcode PATs.
 */
const baseUrl = process.env.AXHUB_BASE_URL ?? {base_url};
const pat = process.env.AXHUB_PAT;
if (!pat) throw new Error('Missing AXHUB_PAT');
const tenant = process.env.AXHUB_TENANT;
if (!tenant) throw new Error('Missing AXHUB_TENANT');

const connector = {connector};
const encodedPath = {path};
const response = await fetch(`${{baseUrl}}/api/v1/tenants/${{encodeURIComponent(tenant)}}/catalog/resources/${{encodeURIComponent(connector)}}/${{encodedPath}}:read`, {{
  method: 'POST',
  headers: {{ 'Content-Type': 'application/json', 'X-Api-Key': pat }},
  body: JSON.stringify({{ sql: {sql}, row_limit: {row_limit} }}),
}});
if (!response.ok) throw new Error(`axhub read failed: ${{response.status}} ${{await response.text()}}`);
console.log(await response.json());
"#,
        metadata = metadata(args),
        base_url = js_string(&args.base_url),
        connector = connector,
        path = path,
        sql = sql,
        row_limit = args.row_limit
    )
}

fn render_mode_b_go(args: &SnippetArgs) -> String {
    let body = json!({ "sql": args.sql, "row_limit": args.row_limit }).to_string();
    format!(
        r#"// axhub data snippet
// {metadata}
// Auth: read AXHUB_PAT from env and send it as X-Api-Key. Do not hardcode PATs.
package main

import (
  "bytes"
  "fmt"
  "net/http"
  "os"
)

func main() {{
  baseURL := os.Getenv("AXHUB_BASE_URL")
  if baseURL == "" {{ baseURL = {base_url} }}
  pat := os.Getenv("AXHUB_PAT")
  if pat == "" {{ panic("missing AXHUB_PAT") }}
  tenant := os.Getenv("AXHUB_TENANT")
  if tenant == "" {{ panic("missing AXHUB_TENANT") }}
  url := baseURL + "/api/v1/tenants/" + tenant + "/catalog/resources/{connector}/{encoded_path}:read"
  req, err := http.NewRequest("POST", url, bytes.NewBufferString({body}))
  if err != nil {{ panic(err) }}
  req.Header.Set("Content-Type", "application/json")
  req.Header.Set("X-Api-Key", pat)
  resp, err := http.DefaultClient.Do(req)
  if err != nil {{ panic(err) }}
  defer resp.Body.Close()
  fmt.Println(resp.Status)
}}
"#,
        metadata = metadata(args),
        base_url = go_string(&args.base_url),
        connector = url_encode(&args.connector),
        encoded_path = url_encode(&args.path),
        body = go_string(&body)
    )
}

fn render_mode_b_shell(args: &SnippetArgs) -> String {
    let body = json!({ "sql": args.sql, "row_limit": args.row_limit }).to_string();
    format!(
        r#"# axhub data snippet
# {metadata}
# Auth: export AXHUB_PAT; send it as X-Api-Key. Do not hardcode PATs.
AXHUB_BASE_URL="${{AXHUB_BASE_URL:-}}"
if [ -z "$AXHUB_BASE_URL" ]; then
  AXHUB_BASE_URL={base_url}
fi
: "${{AXHUB_PAT:?Missing AXHUB_PAT}}"
: "${{AXHUB_TENANT:?Missing AXHUB_TENANT}}"

curl -sS -X POST \
  -H "Content-Type: application/json" \
  -H "X-Api-Key: $AXHUB_PAT" \
  "$AXHUB_BASE_URL/api/v1/tenants/$AXHUB_TENANT/catalog/resources/{connector}/{encoded_path}:read" \
  --data {body}
"#,
        metadata = metadata(args),
        base_url = shell_single(&args.base_url),
        connector = url_encode(&args.connector),
        encoded_path = url_encode(&args.path),
        body = shell_single(&body)
    )
}

fn render_local_bash(args: &SnippetArgs) -> String {
    format!(
        "# axhub data snippet\n# {}\n# Auth: uses local axhub CLI/keychain. No PAT is printed.\naxhub catalog invoke --connector {} --path {} --action read --sql {} --row-limit {} --execute --json\n",
        metadata(args),
        shell_single(&args.connector),
        shell_single(&args.path),
        shell_single(&args.sql),
        args.row_limit
    )
}

fn js_string(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string())
}

fn py_string(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string())
}

fn go_string(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string())
}

fn shell_single(s: &str) -> String {
    format!("'{}'", s.replace('\'', r#"'\''"#))
}

fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
