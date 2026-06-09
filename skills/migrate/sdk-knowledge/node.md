---
lang: node
sdk_version: 2.0.0
source_sha: e75032f79aed03bf138c67e8b12b8f0cffac29bb
route_surface_sha: 44c6dcc5e413595098d2ea037c26b2cf238d2216
conformance_baseline: ax-hub-backend@5a7b57d
generated_by: scripts/gen-sdk-distill.py
note: generated knowledge pack — do not hand-edit; regenerate from the SDK source
---

# AxHub node SDK — migrate knowledge pack

This pack is the load-bearing context for `axhub-sdk-node-expert`. The **Client
init** block below is the canonical, byte-exact wrapper the conversion must emit —
codegen against it, never paraphrase. Everything else is reference.

## 1. Client init (canonical wrapper — emit this exactly)

```ts
import { AxHubClient } from '@ax-hub/sdk';

export const axhub = new AxHubClient({
  token: process.env.AX_HUB_PAT!,
  tokenType: 'pat',
  defaultTenantId: process.env.AX_HUB_TENANT_ID,
  defaultTenantSlug: process.env.AX_HUB_TENANT_SLUG,
});
```

## 2. Auth

PAT → `X-Api-Key`, JWT → `Authorization: Bearer`. `tokenType: 'pat' | 'jwt'`.

Required env: `AXHUB_TOKEN` (or `AX_HUB_PAT` for node), `AXHUB_TENANT_ID`,
optional `AXHUB_TENANT_SLUG`.

## 3. Idioms (distilled from the SDK README)

### I want to...

| Goal | Section |
|------|---------|
| make my first API call | [Magic Moment](#magic-moment) |
| scope work to a tenant/app | [Tenant Scoping](#tenant-scoping) |
| choose JWT vs PAT or OAuth | [Authentication](#authentication) |
| query dynamic tables | [Dynamic Data + Query DSL](#dynamic-data--query-dsl) |
| debug an error | [Errors & Debugging](#errors--debugging) |
| upgrade from 0.x | [Migration & Upgrade](#migration--upgrade) |

### Agent field guide from live QA (2026-06-08)

Use this section when an autonomous agent only has the README and must ship against AX Hub safely.

### Magic Moment

```ts
import { AxHubClient, defineSchema, where } from '@ax-hub/sdk'

const sdk = new AxHubClient({
  token: process.env.AX_HUB_PAT!,
  tokenType: 'pat',
})

const crm = sdk.tenant('acme').app('crm')
const Orders = defineSchema({
  table: 'orders',
  columns: {
    id: 'uuid',
    status: { type: 'enum', values: ['paid', 'pending'] as const },
    total: 'number',
  },
})

const app = await sdk.tenant('acme').apps.create({ slug: 'crm', name: 'CRM' })
const paid = await crm.data.table(Orders).list({
  where: where(Orders.cols.status).eq('paid'),
})

console.log(app.slug, paid.total)
```

5분 안에 첫 app ship 가능.

### Resource Catalog

| Namespace | Methods |
|-----------|---------|
| `sdk.apps` | `create`, `list`, `listAll`, `get`, `update`, `delete`, `permanent`, `listMine`, `signIconUploadURL`, `signIconDarkUploadURL`, `listEnvVars`, `setEnvVar`, `getEnvVar`, `deleteEnvVar` |
| `sdk.apps.publication` | `submit`, `list`, `unpublish` (owner-scoped lifecycle) |
| `sdk.apps.access` | `grant`, `revoke`, `me` (self-grant; `me()` returns `null` on 404) |
| `sdk.apps.likes` | `like`, `unlike`, `me` (idempotent — backend returns `inserted`/`deleted` booleans) |
| `sdk.apps.comments` | `add`, `list`, `listAll`, `delete` (1-500 char client-side validation) |
| `sdk.apps.oauthClients` | `create` (⚠ `clientSecret` surfaced ONCE), `delete` |
| `sdk.apps.git` | `connect`, `installStart` (GitHub App install flow) |
| `sdk.apps.categories` | tenant category read (`list`, `get`); CUD moved to `@ax-hub/admin-sdk` |
| `sdk.apps.discover` | catalog search facade |
| `sdk.apps.templates` | app template listing |
| `sdk.apps.tables` | `list`, `create`, `delete`, `addColumn`, `dropColumn`, `listGrants`, `addGrant`, `revokeGrant` (schema admin) |
| `sdk.publicationRequests` | `get`, `approve`, `reject`, `listPending` (reviewer/admin namespace — separate from owner-scoped `sdk.apps.publication`) |
| `sdk.deployments` | `create`, `list`, `listAll`, `get`, `cancel`, `rollback` |
| `sdk.identity` | `pat.*`, `oauth.*`, `oidc.*`, `devic
… (truncated; see SDK README)

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

**node data access is the ergonomic FLUENT builder — NOT a generic `method()` facade.** Convert the user's ORM / raw-SQL data access to this exact shape (source: `@ax-hub/sdk` `packages/sdk/src/resources/data`).

### Scope a table
```ts
import { AxHubClient, defineSchema, where, and } from '@ax-hub/sdk';

// (a) typed — preferred when the table/columns are known at conversion time:
const Orders = defineSchema({
  table: 'orders',
  columns: { id: 'uuid', total: 'number', status: { type: 'enum', values: ['paid', 'pending'] as const } },
});
const orders = axhub.tenant(process.env.AX_HUB_TENANT_SLUG!).app(process.env.AX_HUB_APP_SLUG!).data.table(Orders);

// (b) runtime-discovered — schema fetched from the backend (no compile-time schema):
const orders = await axhub.tenant(t).app(a).data.discover<{ id: string; total: number; status: 'paid' | 'pending' }>('orders');
```
`.tenant(slug)` / `.app(slug)` take **slugs**, not ids. Reuse the §1 env contract: tenant = `AX_HUB_TENANT_SLUG`, app = `AX_HUB_APP_SLUG` (add the latter to the app's env; do NOT invent `AX_HUB_TENANT` / `AX_HUB_APP` — the SDK examples are inconsistent, §1/§6 pin the `_SLUG` names).

### CRUD (DataTableClient)
- `await orders.list({ where, orderBy, select, page, pageSize })` → `{ items, nextCursor, firstCursor, hasNext, hasPrev }`
- `for await (const row of orders.listAll({ where })) { … }` — auto-paginate every page
- `await orders.count({ where })` → `number`
- `await orders.get(id, { select })` → row
- `await orders.insert({ …row })` → inserted row · `await orders.insertMany([{…}, {…}])` → `{ items, count }`
- `await orders.update(id, { …patch })` → updated row
- `await orders.delete(id)` → `void`

### Query DSL
- filter: `where(Orders.cols.status).eq('paid')`, combine with `and(…)`. (discovered handle: `where(orders.schema!.cols.status)`)
- projection: `select: ['id', 'total'] as const`
- sort: `orderBy`
- **pagination is OFFSET-ONLY**: `page` (1-based) + `pageSize` (clamped 1..100), or `cursor` = the numeric `nextCursor` a prior `list()` returned. NEVER `after` / `before` / keyset cursors — the SDK throws `LegacyCursorError`. `list()` does NOT return a total count (`totalIsExact: false`).

### Wire paths (grounding only — call the methods above, do not hand-roll requests)
list/insert `GET|POST /data/{tenant}/{app}/{table}` · get/update/delete `…/{table}/{id}` · count `…/{table}/_count` · discover `GET /api/v1/tenants/{t}/apps/{a}/tables/{table}/inspect`.

### Mapping guide (user code → fluent call)
- `SELECT … WHERE x = v` → `.list({ where: where(cols.x).eq(v) })`
- `SELECT … LIMIT n OFFSET m` → `.list({ pageSize: n, page: Math.floor(m / n) + 1 })`
- `INSERT INTO t (…) VALUES (…)` → `.insert({ … })`
- `UPDATE t SET … WHERE id = ?` → `.update(id, { … })`
- `DELETE FROM t WHERE id = ?` → `.delete(id)`
- `SELECT COUNT(*) …` → `.count({ where })`

### Reliability — discover()-verify (REQUIRED before apply)
Docs + LLM codegen cannot guarantee correct table/column names on their own. Before applying any `data_patch_plan` diff, run `discover(table)` against the real tenant/app and assert that **every** `.table(name)` / `.discover(name)` table AND every column referenced by `where(cols.X)` / `select: [X]` / `insert`/`update` keys exists in the discovered schema. A reference to a missing table or column is a HARD-STOP (it compiles but silently queries the wrong thing, and no vibe-coder will catch it in review) — surface it in the Korean preview, do not apply.
