---
lang: python
sdk_version: 0.3.1
source_sha: 8bafa90e7d9319b78514a1e95b19c0fb3b73d558
route_surface_sha: 8bafa90e7d9319b78514a1e95b19c0fb3b73d558
conformance_baseline: ax-hub-backend@5a7b57d
generated_by: scripts/gen-sdk-distill.py
note: generated knowledge pack — do not hand-edit; regenerate from the SDK source
---

# AxHub python SDK — migrate knowledge pack

This pack is the load-bearing context for `axhub-sdk-python-expert`. The **Client
init** block below is the canonical, byte-exact wrapper the conversion must emit —
codegen against it, never paraphrase. Everything else is reference.

## 1. Client init (canonical wrapper — emit this exactly)

```python
import os
from axhub_sdk import AxHubClient, TokenType


def build_axhub_client() -> AxHubClient:
    return AxHubClient(
        base_url="https://api.axhub.ai",
        token=os.environ["AXHUB_TOKEN"],
        token_type=TokenType.PAT,
        default_tenant_id=os.environ["AXHUB_TENANT_ID"],
        default_tenant_slug=os.environ.get("AXHUB_TENANT_SLUG", "test"),
    )
```

## 2. Auth

PAT → `X-Api-Key`, JWT → `Authorization: Bearer`. `token_type=TokenType.PAT | TokenType.JWT`.

Required env: `AXHUB_TOKEN` (or `AX_HUB_PAT` for node), `AXHUB_TENANT_ID`,
optional `AXHUB_TENANT_SLUG`.

## 3. Idioms (distilled from the SDK README)

### Install

```bash
pip install axhub-sdk==0.2.0
```

Local development:

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install -e .
```

### Required environment for agent work

```bash
export AXHUB_TOKEN="<short-lived PAT>"
export AXHUB_TENANT_ID="cc1e58f1-8e46-4ac7-96c1-190c4cdd5b70"   # test tenant
export AXHUB_TENANT_SLUG="test"
```

PAT mode is explicit: `TokenType.PAT` sends `X-Api-Key`. JWT mode is `TokenType.JWT` and sends `Authorization: Bearer`.

### Agent quickstart: create a disposable app and table

```python
import os, time
from axhub_sdk import AxHubClient, TokenType

client = AxHubClient(
    base_url="https://api.axhub.ai",
    token=os.environ["AXHUB_TOKEN"],
    token_type=TokenType.PAT,
    default_tenant_id=os.environ["AXHUB_TENANT_ID"],
    default_tenant_slug=os.environ.get("AXHUB_TENANT_SLUG", "test"),
)

me = client.request("authGetApiV1Me")
user_id = me.get("userId") or (me.get("user") or {}).get("id")
if not user_id:
    raise RuntimeError("authGetApiV1Me did not return a user id")

suffix = str(int(time.time() * 1000))[-8:]
slug = f"agent-py-{suffix}"
table = f"items{suffix[-6:]}"

app = client.apps.create({
    "slug": slug,
    "name": "Agent Python README QA",
    "visibility": "private",
    "auth_mode": "anonymous",
    "resource_tier": "S",
    "deploy_method": "docker",
    "subdomain": slug,
})
app_id = app["id"]

client.request(
    "schemaPostApiV1AppsByAppIDTables",
    path_params={"appID": app_id},
    body={
        "table_name": table,
        "owner_column": "owner_id",
        "columns": [
            {"name": "owner_id", "type": "uuid", "nullable": False},
            {"name": "title", "type": "text", "nullable": False},
            {"name": "status", "type": "text", "nullable": False},
        ],
    },
)

row = client.request(
    "schemaPostDataByTenantSlugByAppSlugByTable",
    path_params={"tenantSlug": "test", "appSlug": slug, "table"
… (truncated; see SDK README)

### How to call the full API surface

- High-level app create: `client.apps.create(body)` uses `default_tenant_id`.
- Any route by operation id: `client.request(operation_id, path_params={...}, query={...}, body={...})`.
- Generated facade: `client.data.schema_post_data_by_tenant_slug_by_app_slug_by_table(path_params={...}, body={...})`.
- Async client: `AsyncAxHubClient` mirrors `request` and generated operation facades.
- Route inventory: `ROUTES`, `CONTEXT_ROUTES`, `ERROR_CODES`, and `OPERATION_METHODS`.
- Errors: catch `AxHubError` and branch on `code`, `category`, `status`, and `retryable`.

### Dynamic app, schema, and data operations

Use the high-level `apps.create` helper for the first app, then use generated operation IDs for every backend route. Request bodies use backend wire keys, usually `snake_case`. Responses are normalized to camelCase in this SDK family, so read `tableName`, `requestId`, `revokedAt`, and similar keys from responses.

| Task | Operation ID | Required path params | Success assertion |
|------|--------------|----------------------|-------------------|
| Create env var | `appsPostApiV1AppsByAppIDEnvVars` | `appID` | `env.list` includes `key` |
| Delete env var | `appsDeleteApiV1AppsByAppIDEnvVarsByKey` | `appID`, `key` | `env.list` no longer includes `key` |
| Create table | `schemaPostApiV1AppsByAppIDTables` | `appID` | response `tableName` equals requested name |
| Inspect table | `schemaGetApiV1AppsByAppIDTablesByTableName` | `appID`, `tableName` | response `id` and `tableName` match |
| Add column | `schemaPostApiV1AppsByAppIDTablesByTableNameColumns` | `appID`, `tableName` | inspect contains column name |
| Drop column | `schemaDeleteApiV1AppsByAppIDTablesByTableNameColumnsByColumnName` | `appID`, `tableName`, `columnName` | inspect no longer contains column name |
| Add table grant | `schemaPostApiV1AppsByAppIDTablesByTableNameGrants` | `appID`, `tableName` | response has grant `id` |
| List grants | `schemaGetApiV1AppsByAppIDTablesByTableNameGrants` | `appID`, `tableName` | lis
… (truncated; see SDK README)

### Troubleshooting for agents

- `tenant_id_required`: pass `defaultTenantId` / `AXHUB_TENANT_ID` before calling `apps.create`.
- `tokenType must be explicit`: set PAT mode when using a PAT. PATs are sent as `X-Api-Key`; JWTs are sent as `Authorization: Bearer`.
- `slug_taken` or `schema_name_taken`: append a timestamp suffix and retry. Never reuse fixture names in live destructive QA.
- `permission_denied` / `not_admin`: the SDK is working. The token lacks the role for that route.
- `precondition_failed` on deploy: connect git or use the app bootstrap flow first.
- 4xx responses are expected for negative assertions. SDK bugs are unexpected exceptions, response decode failures, or backend 5xx during a valid call.

## 4. Operation surface (identity reference)

_Total 189 operations across 12 tags. Identity only (operationId/method/path); request/response body schemas are NOT in scope for the wrapper conversion._

### Apps (52)
- `appsDeleteApiV1AppsByAppID` — DELETE /api/v1/apps/{appID}
- `appsDeleteApiV1AppsByAppIDAccess` — DELETE /api/v1/apps/{appID}/access
- `appsDeleteApiV1AppsByAppIDEnvVarsByKey` — DELETE /api/v1/apps/{appID}/env-vars/{key}
- `appsDeleteApiV1AppsByAppIDInvitationsByUserID` — DELETE /api/v1/apps/{appID}/invitations/{userID}
- `appsDeleteApiV1AppsByAppIDLikes` — DELETE /api/v1/apps/{appID}/likes
- `appsDeleteApiV1AppsByAppIDPermanent` — DELETE /api/v1/apps/{appID}/permanent
- `appsDeleteApiV1CommentsByCommentID` — DELETE /api/v1/comments/{commentID}
- `appsDeleteApiV1TenantsByTenantIDCategoriesByCategoryID` — DELETE /api/v1/tenants/{tenantID}/categories/{categoryID}
- `appsGetApiV1AdminTemplates` — GET /api/v1/admin/templates
- `appsGetApiV1AdminTemplatesByTemplateID` — GET /api/v1/admin/templates/{templateID}
- `appsGetApiV1AppsByAppID` — GET /api/v1/apps/{appID}
- `appsGetApiV1AppsByAppIDAccessMe` — GET /api/v1/apps/{appID}/access/me
- `appsGetApiV1AppsByAppIDComments` — GET /api/v1/apps/{appID}/comments
- `appsGetApiV1AppsByAppIDEnvVars` — GET /api/v1/apps/{appID}/env-vars
- `appsGetApiV1AppsByAppIDLikesMe` — GET /api/v1/apps/{appID}/likes/me
- `appsGetApiV1AppsByAppIDMembers` — GET /api/v1/apps/{appID}/members
- `appsGetApiV1AppsByAppIDReviewRequests` — GET /api/v1/apps/{appID}/review-requests
- `appsGetApiV1AppsDiscover` — GET /api/v1/apps/discover
- `appsGetApiV1AppsSearch` — GET /api/v1/apps/search
- `appsGetApiV1MeAppsOwned` — GET /api/v1/me/apps/owned
- `appsGetApiV1MeAppsReceived` — GET /api/v1/me/apps/received
- `appsGetApiV1MeAppsWorkspace` — GET /api/v1/me/apps/workspace
- `appsGetApiV1ReviewRequestsByRrID` — GET /api/v1/review-requests/{rrID}
- `appsGetApiV1ReviewRequestsHistory` — GET /api/v1/review-requests/history
- `appsGetApiV1ReviewRequestsPending` — GET /api/v1/review-requests/pending
- `appsGetApiV1Templates` — GET /api/v1/templates
- `appsGetApiV1TenantsByTenantIDApps` — GET /api/v1/tenants/{tenantID}/apps
- `appsGetApiV1TenantsByTenantIDAppsCheckAvailability` — GET /api/v1/tenants/{tenantID}/apps/check-availability
- `appsGetApiV1TenantsByTenantIDCategories` — GET /api/v1/tenants/{tenantID}/categories
- `appsGetApiV1TenantsByTenantIDCategoriesByCategoryID` — GET /api/v1/tenants/{tenantID}/categories/{categoryID}
- `appsGetApiV1TenantsByTenantIDDiscoverApps` — GET /api/v1/tenants/{tenantID}/discover/apps
- `appsGetApiV1UsersMeApps` — GET /api/v1/users/me/apps
- `appsGetInternalAppAccess` — GET /internal/app-access
- `appsPatchApiV1AdminTemplatesByTemplateID` — PATCH /api/v1/admin/templates/{templateID}
- `appsPatchApiV1AppsByAppID` — PATCH /api/v1/apps/{appID}
- `appsPatchApiV1TenantsByTenantIDCategoriesByCategoryID` — PATCH /api/v1/tenants/{tenantID}/categories/{categoryID}
- `appsPostApiV1AdminTemplates` — POST /api/v1/admin/templates
- `appsPostApiV1AppsByAppIDAccess` — POST /api/v1/apps/{appID}/access
- `appsPostApiV1AppsByAppIDComments` — POST /api/v1/apps/{appID}/comments
- `appsPostApiV1AppsByAppIDEnvVars` — POST /api/v1/apps/{appID}/env-vars
- `appsPostApiV1AppsByAppIDIconDarkUploadUrl` — POST /api/v1/apps/{appID}/icon-dark/upload-url
- `appsPostApiV1AppsByAppIDIconUploadUrl` — POST /api/v1/apps/{appID}/icon/upload-url
- `appsPostApiV1AppsByAppIDInvitations` — POST /api/v1/apps/{appID}/invitations
- `appsPostApiV1AppsByAppIDLikes` — POST /api/v1/apps/{appID}/likes
- `appsPostApiV1AppsByAppIDResume` — POST /api/v1/apps/{appID}/resume
- `appsPostApiV1AppsByAppIDReviewRequests` — POST /api/v1/apps/{appID}/review-requests
- `appsPostApiV1AppsByAppIDSuspend` — POST /api/v1/apps/{appID}/suspend
- `appsPostApiV1ReviewRequestsByRrIDApprove` — POST /api/v1/review-requests/{rrID}/approve
- `appsPostApiV1ReviewRequestsByRrIDReject` — POST /api/v1/review-requests/{rrID}/reject
- `appsPostApiV1TenantsByTenantIDApps` — POST /api/v1/tenants/{tenantID}/apps
- `appsPostApiV1TenantsByTenantIDAppsIconUploadUrl` — POST /api/v1/tenants/{tenantID}/apps/icon/upload-url
- `appsPostApiV1TenantsByTenantIDCategories` — POST /api/v1/tenants/{tenantID}/categories

### Audit (4)
- `auditGetApiV1TenantsByTenantIDAuditEvents` — GET /api/v1/tenants/{tenantID}/audit-events
- `auditGetApiV1TenantsByTenantIDAuditEventsByEventID` — GET /api/v1/tenants/{tenantID}/audit-events/{eventID}
- `auditGetApiV1TenantsByTenantIDAuditEventsIntegrityCheck` — GET /api/v1/tenants/{tenantID}/audit-events/integrity-check
- `auditPostApiV1TenantsByTenantIDAuditEventsAnonymize` — POST /api/v1/tenants/{tenantID}/audit-events/anonymize

### Auth (30)
- `authDeleteApiV1OauthClientsByClientIDGrantsMe` — DELETE /api/v1/oauth/clients/{clientID}/grants/me
- `authGetApiV1Me` — GET /api/v1/me
- `authGetApiV1OauthClientsByClientID` — GET /api/v1/oauth-clients/{clientID}
- `authGetApiV1TenantsByTenantIDIdentityProviders` — GET /api/v1/tenants/{tenantID}/identity-providers
- `authGetAuthByProviderIDStart` — GET /auth/{providerID}/start
- `authGetAuthGoogleOauth2Callback` — GET /auth/google_oauth2/callback
- `authGetAuthGoogleOauth2Start` — GET /auth/google_oauth2/start
- `authGetAuthOidcCallback` — GET /auth/oidc/callback
- `authGetAuthProviders` — GET /auth/providers
- `authGetAuthSilentCallback` — GET /auth/silent/callback
- `authGetAuthSilentStart` — GET /auth/silent/start
- `authGetOauthAuthorize` — GET /oauth/authorize
- `authGetOauthDeviceLookup` — GET /oauth/device/lookup
- `authGetOauthUserinfo` — GET /oauth/userinfo
- `authGetWellKnownJwksJson` — GET /.well-known/jwks.json
- `authGetWellKnownOpenidConfiguration` — GET /.well-known/openid-configuration
- `authPostApiV1AdminUsersByUidRevokeAll` — POST /api/v1/admin/users/{uid}/revoke-all
- `authPostApiV1AppsByAppIDOauthClients` — POST /api/v1/apps/{appID}/oauth-clients
- `authPostApiV1MeInvitationsByInvitationIDAccept` — POST /api/v1/me/invitations/{invitationID}/accept
- `authPostApiV1TenantsByTenantIDIdentityProviders` — POST /api/v1/tenants/{tenantID}/identity-providers
- `authPostApiV1TenantsByTenantIDIdentityProvidersByProviderIDDisable` — POST /api/v1/tenants/{tenantID}/identity-providers/{providerID}/disable
- `authPostApiV1TenantsByTenantIDIdentityProvidersByProviderIDEnable` — POST /api/v1/tenants/{tenantID}/identity-providers/{providerID}/enable
- `authPostAuthLogout` — POST /auth/logout
- `authPostAuthRefresh` — POST /auth/refresh
- `authPostOauthAuthorizeTenant` — POST /oauth/authorize/tenant
- `authPostOauthDeviceAuthorization` — POST /oauth/device_authorization
- `authPostOauthDeviceAuthorize` — POST /oauth/device/authorize
- `authPostOauthRegister` — POST /oauth/register
- `authPostOauthRevoke` — POST /oauth/revoke
- `authPostOauthToken` — POST /oauth/token

### Authorization (14)
- `authorizationDeleteApiV1TenantsByTenantIDSubjectsBySubjectID` — DELETE /api/v1/tenants/{tenantID}/subjects/{subjectID}
- `authorizationDeleteApiV1TenantsByTenantIDSubjectsBySubjectIDTagsByTagID` — DELETE /api/v1/tenants/{tenantID}/subjects/{subjectID}/tags/{tagID}
- `authorizationDeleteApiV1TenantsByTenantIDTagsByTagID` — DELETE /api/v1/tenants/{tenantID}/tags/{tagID}
- `authorizationGetApiV1TenantsByTenantIDGrants` — GET /api/v1/tenants/{tenantID}/grants
- `authorizationGetApiV1TenantsByTenantIDSubjects` — GET /api/v1/tenants/{tenantID}/subjects
- `authorizationGetApiV1TenantsByTenantIDTags` — GET /api/v1/tenants/{tenantID}/tags
- `authorizationPatchApiV1TenantsByTenantIDSubjectsBySubjectID` — PATCH /api/v1/tenants/{tenantID}/subjects/{subjectID}
- `authorizationPatchApiV1TenantsByTenantIDTagsByTagID` — PATCH /api/v1/tenants/{tenantID}/tags/{tagID}
- `authorizationPostApiV1TenantsByTenantIDGrantsByGrantIDGrant` — POST /api/v1/tenants/{tenantID}/grants/{grantID}/grant
- `authorizationPostApiV1TenantsByTenantIDGrantsByGrantIDRevoke` — POST /api/v1/tenants/{tenantID}/grants/{grantID}/revoke
- `authorizationPostApiV1TenantsByTenantIDSubjects` — POST /api/v1/tenants/{tenantID}/subjects
- `authorizationPostApiV1TenantsByTenantIDSubjectsBySubjectIDMove` — POST /api/v1/tenants/{tenantID}/subjects/{subjectID}/move
- `authorizationPostApiV1TenantsByTenantIDSubjectsBySubjectIDTags` — POST /api/v1/tenants/{tenantID}/subjects/{subjectID}/tags
- `authorizationPostApiV1TenantsByTenantIDTags` — POST /api/v1/tenants/{tenantID}/tags

### Config (1)
- `configGetConfigPublic` — GET /config/public

### Cost (5)
- `costGetApiV1TenantsByTenantIDCostByApp` — GET /api/v1/tenants/{tenantID}/cost/by-app
- `costGetApiV1TenantsByTenantIDCostByCostCenter` — GET /api/v1/tenants/{tenantID}/cost/by-cost-center
- `costGetApiV1TenantsByTenantIDCostExport` — GET /api/v1/tenants/{tenantID}/cost/export
- `costGetApiV1TenantsByTenantIDCostSummary` — GET /api/v1/tenants/{tenantID}/cost/summary
- `costGetApiV1TenantsByTenantIDCostTimeseries` — GET /api/v1/tenants/{tenantID}/cost/timeseries

### Deploy (14)
- `deployDeleteApiV1AppsByAppIDGitConnection` — DELETE /api/v1/apps/{appID}/git-connection
- `deployGetApiV1AppsByAppIDDeployments` — GET /api/v1/apps/{appID}/deployments
- `deployGetApiV1AppsByAppIDDeploymentsByDid` — GET /api/v1/apps/{appID}/deployments/{did}
- `deployGetApiV1AppsByAppIDGitConnection` — GET /api/v1/apps/{appID}/git-connection
- `deployGetApiV1AppsByAppIDGitGithubInstallStart` — GET /api/v1/apps/{appID}/git/github/install/start
- `deployGetApiV1AppsByAppIDLogs` — GET /api/v1/apps/{appID}/logs
- `deployGetApiV1TenantsByTenantIDAppBootstrapsByBootstrapID` — GET /api/v1/tenants/{tenantID}/app-bootstraps/{bootstrapID}
- `deployPatchApiV1AppsByAppIDGitConnection` — PATCH /api/v1/apps/{appID}/git-connection
- `deployPostApiV1AppsByAppIDDeployments` — POST /api/v1/apps/{appID}/deployments
- `deployPostApiV1AppsByAppIDDeploymentsByDidCancel` — POST /api/v1/apps/{appID}/deployments/{did}/cancel
- `deployPostApiV1AppsByAppIDDeploymentsByDidRollback` — POST /api/v1/apps/{appID}/deployments/{did}/rollback
- `deployPostApiV1AppsByAppIDGitConnection` — POST /api/v1/apps/{appID}/git-connection
- `deployPostApiV1TenantsByTenantIDAppBootstraps` — POST /api/v1/tenants/{tenantID}/app-bootstraps
- `deployPostWebhooksGithub` — POST /webhooks/github

### deploy (2)
- `deployGetApiV1GithubAccounts` — GET /api/v1/github/accounts
- `deployGetApiV1GithubInstallationsByInstallationIDRepositories` — GET /api/v1/github/installations/{installationID}/repositories

### Gateway (21)
- `gatewayDeleteApiV1TenantsByTenantIDConnectorsByConnectorID` — DELETE /api/v1/tenants/{tenantID}/connectors/{connectorID}
- `gatewayDeleteApiV1TenantsByTenantIDResourcesByResourceID` — DELETE /api/v1/tenants/{tenantID}/resources/{resourceID}
- `gatewayDeleteApiV1TenantsByTenantIDResourcesByResourceIDTagsByTagID` — DELETE /api/v1/tenants/{tenantID}/resources/{resourceID}/tags/{tagID}
- `gatewayGetApiV1CatalogKinds` — GET /api/v1/catalog/kinds
- `gatewayGetApiV1Engines` — GET /api/v1/engines
- `gatewayGetApiV1TenantsByTenantIDCatalogConnectors` — GET /api/v1/tenants/{tenantID}/catalog/connectors
- `gatewayGetApiV1TenantsByTenantIDCatalogResources` — GET /api/v1/tenants/{tenantID}/catalog/resources
- `gatewayGetApiV1TenantsByTenantIDCatalogResourcesByConnectorByPath` — GET /api/v1/tenants/{tenantID}/catalog/resources/{connector}/{path}
- `gatewayGetApiV1TenantsByTenantIDConnectors` — GET /api/v1/tenants/{tenantID}/connectors
- `gatewayGetApiV1TenantsByTenantIDConnectorsByConnectorIDDiscover` — GET /api/v1/tenants/{tenantID}/connectors/{connectorID}/discover
- `gatewayGetApiV1TenantsByTenantIDResources` — GET /api/v1/tenants/{tenantID}/resources
- `gatewayPatchApiV1TenantsByTenantIDConnectorsByConnectorID` — PATCH /api/v1/tenants/{tenantID}/connectors/{connectorID}
- `gatewayPatchApiV1TenantsByTenantIDResourcesByResourceID` — PATCH /api/v1/tenants/{tenantID}/resources/{resourceID}
- `gatewayPostApiV1TenantsByTenantIDCatalogResourcesByConnectorByPath` — POST /api/v1/tenants/{tenantID}/catalog/resources/{connector}/{path}
- `gatewayPostApiV1TenantsByTenantIDConnectors` — POST /api/v1/tenants/{tenantID}/connectors
- `gatewayPostApiV1TenantsByTenantIDConnectorsByConnectorIDCredentials` — POST /api/v1/tenants/{tenantID}/connectors/{connectorID}/credentials
- `gatewayPostApiV1TenantsByTenantIDGatewayQuery` — POST /api/v1/tenants/{tenantID}/gateway/query
- `gatewayPostApiV1TenantsByTenantIDResourcesBulk` — POST /api/v1/tenants/{tenantID}/resources/bulk
- `gatewayPostApiV1TenantsByTenantIDResourcesByResourceIDMove` — POST /api/v1/tenants/{tenantID}/resources/{resourceID}/move
- `gatewayPostApiV1TenantsByTenantIDResourcesByResourceIDTags` — POST /api/v1/tenants/{tenantID}/resources/{resourceID}/tags
- `gatewayPostApiV1TenantsByTenantIDResourcesNamespaces` — POST /api/v1/tenants/{tenantID}/resources/namespaces

### identity (2)
- `identityGetAuthGithub` — GET /auth/github
- `identityGetAuthGithubCallback` — GET /auth/github/callback

### Schema (21)
- `schemaDeleteApiV1AppsByAppIDTablesByTableName` — DELETE /api/v1/apps/{appID}/tables/{tableName}
- `schemaDeleteApiV1AppsByAppIDTablesByTableNameColumnsByColumnName` — DELETE /api/v1/apps/{appID}/tables/{tableName}/columns/{columnName}
- `schemaDeleteApiV1AppsByAppIDTablesByTableNameGrantsByGrantID` — DELETE /api/v1/apps/{appID}/tables/{tableName}/grants/{grantID}
- `schemaDeleteApiV1MePersonalAccessTokensByPatID` — DELETE /api/v1/me/personal-access-tokens/{patID}
- `schemaDeleteDataByTenantSlugByAppSlugByTableById` — DELETE /data/{tenantSlug}/{appSlug}/{table}/{id}
- `schemaGetApiV1AppsByAppIDTables` — GET /api/v1/apps/{appID}/tables
- `schemaGetApiV1AppsByAppIDTablesByTableName` — GET /api/v1/apps/{appID}/tables/{tableName}
- `schemaGetApiV1AppsByAppIDTablesByTableNameGrants` — GET /api/v1/apps/{appID}/tables/{tableName}/grants
- `schemaGetApiV1AppsByAppIDTablesByTableNameRows` — GET /api/v1/apps/{appID}/tables/{tableName}/rows
- `schemaGetApiV1AppsByAppIDTablesCheckAvailability` — GET /api/v1/apps/{appID}/tables/check-availability
- `schemaGetApiV1AppsByAppIDTablesColumnTypes` — GET /api/v1/apps/{appID}/tables/column-types
- `schemaGetApiV1MePersonalAccessTokens` — GET /api/v1/me/personal-access-tokens
- `schemaGetDataByTenantSlugByAppSlugByTable` — GET /data/{tenantSlug}/{appSlug}/{table}
- `schemaGetDataByTenantSlugByAppSlugByTableById` — GET /data/{tenantSlug}/{appSlug}/{table}/{id}
- `schemaGetDataByTenantSlugByAppSlugByTableCount` — GET /data/{tenantSlug}/{appSlug}/{table}/_count
- `schemaPatchDataByTenantSlugByAppSlugByTableById` — PATCH /data/{tenantSlug}/{appSlug}/{table}/{id}
- `schemaPostApiV1AppsByAppIDTables` — POST /api/v1/apps/{appID}/tables
- `schemaPostApiV1AppsByAppIDTablesByTableNameColumns` — POST /api/v1/apps/{appID}/tables/{tableName}/columns
- `schemaPostApiV1AppsByAppIDTablesByTableNameGrants` — POST /api/v1/apps/{appID}/tables/{tableName}/grants
- `schemaPostApiV1MePersonalAccessTokens` — POST /api/v1/me/personal-access-tokens
- `schemaPostDataByTenantSlugByAppSlugByTable` — POST /data/{tenantSlug}/{appSlug}/{table}

### Tenants (23)
- `tenantsDeleteApiV1TenantsByTenantID` — DELETE /api/v1/tenants/{tenantID}
- `tenantsDeleteApiV1TenantsByTenantIDEmailDomainsByDomain` — DELETE /api/v1/tenants/{tenantID}/email-domains/{domain}
- `tenantsDeleteApiV1TenantsByTenantIDIcon` — DELETE /api/v1/tenants/{tenantID}/icon
- `tenantsDeleteApiV1TenantsByTenantIDInvitationsByInvitationID` — DELETE /api/v1/tenants/{tenantID}/invitations/{invitationID}
- `tenantsDeleteApiV1TenantsByTenantIDInviteLinksByLinkID` — DELETE /api/v1/tenants/{tenantID}/invite-links/{linkID}
- `tenantsGetApiV1InviteLinksByToken` — GET /api/v1/invite-links/{token}
- `tenantsGetApiV1Tenants` — GET /api/v1/tenants
- `tenantsGetApiV1TenantsByTenantID` — GET /api/v1/tenants/{tenantID}
- `tenantsGetApiV1TenantsByTenantIDEmailDomains` — GET /api/v1/tenants/{tenantID}/email-domains
- `tenantsGetApiV1TenantsByTenantIDInvitations` — GET /api/v1/tenants/{tenantID}/invitations
- `tenantsGetApiV1TenantsByTenantIDInviteLinks` — GET /api/v1/tenants/{tenantID}/invite-links
- `tenantsGetApiV1TenantsByTenantIDMembers` — GET /api/v1/tenants/{tenantID}/members
- `tenantsPatchApiV1TenantsByTenantID` — PATCH /api/v1/tenants/{tenantID}
- `tenantsPatchApiV1TenantsByTenantIDMembersByMembershipID` — PATCH /api/v1/tenants/{tenantID}/members/{membershipID}
- `tenantsPostApiV1InviteLinksByTokenAccept` — POST /api/v1/invite-links/{token}/accept
- `tenantsPostApiV1Tenants` — POST /api/v1/tenants
- `tenantsPostApiV1TenantsByTenantIDEmailDomains` — POST /api/v1/tenants/{tenantID}/email-domains
- `tenantsPostApiV1TenantsByTenantIDIconUploadUrl` — POST /api/v1/tenants/{tenantID}/icon/upload-url
- `tenantsPostApiV1TenantsByTenantIDInvitations` — POST /api/v1/tenants/{tenantID}/invitations
- `tenantsPostApiV1TenantsByTenantIDInvitationsBulk` — POST /api/v1/tenants/{tenantID}/invitations/bulk
- `tenantsPostApiV1TenantsByTenantIDInviteLinks` — POST /api/v1/tenants/{tenantID}/invite-links
- `tenantsPostApiV1TenantsByTenantIDMembersByMembershipIDDeactivate` — POST /api/v1/tenants/{tenantID}/members/{membershipID}/deactivate
- `tenantsPostApiV1TenantsByTenantIDMembersByMembershipIDReactivate` — POST /api/v1/tenants/{tenantID}/members/{membershipID}/reactivate


## 5. Conformance contracts (never emit a call that violates these)

- **oauth-token-form-urlencoded** — `POST /oauth/token` (authPostOauthToken) — encoding MUST be `application/x-www-form-urlencoded` — MUST NOT use `application/json` — required keys: `grant_type`
- **oauth-revoke-form-urlencoded** — `POST /oauth/revoke` (authPostOauthRevoke) — encoding MUST be `application/x-www-form-urlencoded` — MUST NOT use `application/json` — required keys: `token`
- **oauth-device-authorization-form-urlencoded** — `POST /oauth/device_authorization` (authPostOauthDeviceAuthorization) — encoding MUST be `application/x-www-form-urlencoded` — MUST NOT use `application/json` — required keys: `client_id`
- **oauth-authorize-redirect-manual** — `GET /oauth/authorize` (authGetOauthAuthorize)
- **email-domains-api-v1-prefix** — `GET /api/v1/tenants/{tenantID}/email-domains` (tenantsGetApiV1TenantsByTenantIDEmailDomains)
- **public-invite-links-api-v1-prefix** — `GET /api/v1/invite-links/{token}` (tenantsGetApiV1InviteLinksByToken)
- **cost-first-class-context** — `GET /api/v1/tenants/{tenantID}/cost/summary` (costGetApiV1TenantsByTenantIDCostSummary)

## 6. Data operations (for `data_patch_plan`)

**python data access is the ergonomic FLUENT data layer — NOT a generic `method()` facade.** Convert the user's ORM / raw-SQL data access to this exact shape. `tenant(slug)` / `app(slug)` take **slugs**, not ids — reuse the §1 env contract (`AX_HUB_TENANT_SLUG` / `AX_HUB_APP_SLUG`; do NOT invent `AX_HUB_TENANT` / `AX_HUB_APP`).

### Scope + CRUD + DSL
```python
from axhub_sdk import AxHubClient
from axhub_sdk.data import and_, define_schema, where

data = sdk.tenant(AX_HUB_TENANT_SLUG).app(AX_HUB_APP_SLUG).data

# (a) typed:
Orders = define_schema("orders", {"id": "uuid", "total": "number", "status": "string"})
orders = data.table(Orders)
# (b) runtime-discovered (fresh=True re-introspects after live DDL):
orders = data.discover("orders")

page = orders.list(where=where("status").eq("paid"), order_by="-total", select=["id", "total"], page=1, page_size=50)
for entry in orders.list_all(where=where("total").gte(0), page_size=100):
    if entry.type == "item":
        use(entry.value)
n = orders.count(where=and_(where("total").gte(10), where("total").in_([10, 30])))
row = orders.get(row_id, select=["id", "total"])
orders.insert({"total": 10, "status": "paid"})
orders.insert_many([{"total": 20}, {"total": 30}])
orders.update(row_id, {"status": "shipped"})
orders.delete(row_id)
```
Filter builder: `where(col).eq/ne/gt/gte/lt/lte(v)`, `.in_([...])`, `.contains(s)` (LIKE, auto-escape), combined ONLY with top-level `and_(...)`. Errors: a single `AxHubError` carrying `.category/.code/.status` (404 → `code='not_found'`); data guards raise `ValidationError` / `LegacyCursorError` / `TableNotFoundError`.
### Live data contract (applies to EVERY language — verified live)
- **`list`/`count` need at least one `where` filter on NON-owner-scoped tables** (backend mass-scan guard → the SDK surfaces `ValidationError(code: where_required)` from the backend 400). **Owner-scoped tables (created with an `owner_column`) ACCEPT filterless list/count** — rows auto-scope to the caller, so "내 행 전부" reads need NO filter there. When converting an "everything" read on a table whose ownership you can't confirm, keep the call filterless and explain in the Korean preview that non-owner-scoped tables will reject it (an always-true range filter like `where(created_at).gte('1970-01-01T00:00:00Z')` is the fallback).
- **Pushable filters are a top-level AND of `eq/ne/gt/gte/lt/lte/in/like` ONLY.** `or`/`not` combinators exist in each DSL but are NOT pushable — the SDK rejects them with `ValidationError`. Express "A or B" on one column as `in([...])`; otherwise split into separate calls and merge in app code.
- **Pagination is OFFSET-ONLY**: 1-based `page` + `pageSize` (clamped 1..100; `limit` aliases `pageSize` where offered), or `cursor` = the numeric next-cursor a prior `list` returned. `after`/`before` keyset options throw `LegacyCursorError`. `list` does NOT return an exact total.
- **Tables/columns must already exist** — inserts do NOT auto-create them (DDL is owned by `axhub tables create` / `axhub tables columns add`). `discover` caches the schema per table; after live DDL re-introspect with the `fresh` option.
### Mapping guide (user code → fluent call; notation is per-language §6 above)
- `SELECT … WHERE x = v` → `list(where: where(x).eq(v))`
- `SELECT … WHERE a = v AND b > w` → `list(where: and(where(a).eq(v), where(b).gt(w)))`
- `SELECT … WHERE x IN (…)` / `WHERE a = v OR a = w` → `list(where: where(x).in([...]))`
- `SELECT … LIMIT n OFFSET m` → `list(where: <required filter>, pageSize: n, page: m/n + 1)` — the where stays REQUIRED
- `INSERT INTO t (…) VALUES (…)` → `insert({...})` · multi-VALUES → `insertMany([...])`
- `UPDATE t SET … WHERE id = ?` → `update(id, {...})`
- `DELETE FROM t WHERE id = ?` → `delete(id)`
- `SELECT COUNT(*) … WHERE …` → `count(where: ...)` — the where stays REQUIRED

### Wire paths (grounding only — call the methods above, do not hand-roll requests)
list/insert `GET|POST /data/{tenant}/{app}/{table}` · get/update/delete `…/{table}/{id}` · count `…/{table}/_count` · discover `GET /api/v1/tenants/{t}/apps/{a}/tables/{table}/inspect` (appId fallback inside the SDK).

### Reliability — discover()-verify (REQUIRED before apply)
Docs + LLM codegen cannot guarantee correct table/column names on their own. Before applying any `data_patch_plan` diff, run `discover(table)` against the real tenant/app and assert that **every** `.table(name)` / `.discover(name)` table AND every column referenced by where/select/insert/update keys exists in the discovered schema. A reference to a missing table or column is a HARD-STOP (it compiles but silently queries the wrong thing, and no vibe-coder will catch it in review) — surface it in the Korean preview, do not apply.
