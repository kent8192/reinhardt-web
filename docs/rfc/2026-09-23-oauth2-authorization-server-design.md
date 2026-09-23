# OAuth 2.0 Authorization Server Design

## Status

- Issue: [#6355](https://github.com/kent8192/reinhardt-web/issues/6355)
- Target branch: `develop/0.4.0`
- Design interview: decisions confirmed; implementation on `feature/issue-6355-oauth-authorization-server`.

## Problem

`reinhardt-auth` currently exposes in-process authorization-code helpers and an
in-memory token store, but no routable authorization or token endpoints. The
advertised grant variants exceed the implemented behavior. Code issuance does not
check the client registry or redirect allowlist. Access-token lookup does not
check expiry, and code exchange returns a refresh token that cannot be refreshed.
The separate social-auth module is a client of external providers, not this
server-side feature.

## Glossary

- **Authorization server**: A Reinhardt application that issues OAuth
  authorization codes and access tokens to registered clients. This is distinct
  from a social provider used by Reinhardt social authentication.
- **Host application**: The Reinhardt application operating the authorization
  server and deciding how its users authenticate and approve access.
- **Registered client**: An application registered with the authorization server
  to request access on behalf of a user or itself. The current Rust API calls
  this an `OAuth2Application`; avoid the unqualified word "application" when the
  host and client could be confused.
- **Authorization decision**: The host application's decision about which user
  and requested scopes, if any, a registered client may receive access for.
- **Token principal**: The identity represented by an access token. It is either
  a user for delegated authorization or a registered client acting on its own.
- **Resource server**: A service that accepts an access token to protect an API.
  It may be the host application or a separate service.

## Confirmed Decisions

1. Implement Authorization Code with PKCE and Client Credentials grants. Do not
   offer the Implicit grant. Registered clients may use only the grants they are
   allowed to use.
2. Do not issue refresh tokens until the refresh flow includes validation,
   rotation, expiry, and revocation. The token response omits `refresh_token`
   for this delivery.
3. The host application owns user authentication and its consent UI. It supplies
   an authorization decision, including the approved scopes, to the OAuth
   server. The library owns protocol validation and issuance and must never
   assume consent merely because a user is authenticated.
4. Provide a built-in durable store for client registration, authorization
   codes, and access tokens that works across server instances. Keep the
   in-memory store for development and tests.
5. Preserve existing public helper call shapes where practical and strengthen
   their validation. Add a new storage contract where the current
   `OAuth2TokenStore` cannot represent expiry and other required metadata.
   Identify and document any unavoidable incompatibility individually.

6. Support public and confidential clients for the authorization-code grant.
   Require PKCE `S256` for both. Authenticate confidential clients at the token
   endpoint with HTTP Basic; public clients do not have a client secret.
7. Represent user and client token principals distinctly. Client Credentials
   tokens represent a registered client, not a fabricated user. Add principal
   lookup separately from the existing user-oriented `AuthBackend` contract.
8. Issue opaque access tokens. Provide authenticated token introspection for
   resource servers outside the authorization-server deployment, with current
   expiry and revocation state.
9. Constrain issued scopes to the client's registered allowlist and the host
   authorization decision. Return scopes in token inspection and provide a
   scope check for protected APIs.
10. The authorization endpoint passes a typed pending request to the host
    application for login and consent. The eventual approval or denial is
    bound to the original request; a caller-supplied user ID is insufficient.
11. Use PostgreSQL for the built-in durable store, with migrations and a
    server-side administrative client-registration API. Do not add public
    dynamic client registration in this issue.
12. Provide an HTTP token-revocation endpoint following RFC 7009.
13. Publish RFC 8414 authorization-server metadata showing only supported
    endpoints, grants, client authentication, and PKCE methods.

14. Register resource servers and bind each access token to an audience.
    Only an authenticated resource server authorized for that audience may
    receive an active introspection result.
15. Match registered redirect URIs exactly, except that a registered native
    loopback URI may use a varying port. Its scheme, host, path, and other
    components still match literally.
16. Persist pending authorization requests in PostgreSQL with a short lifetime
    and single-use completion. Bind each request to a host-supplied browser
    session identifier so login and consent can resume on another node.
17. Store client secrets using a password-hashing scheme and authorization
    codes and opaque access tokens as lookup digests. Show raw values only at
    issuance.
18. Redeem a code at most once using a database transaction. If a successfully
    redeemed code is later reused, revoke tokens issued from that code. Do not
    reveal the replay distinction in the public error response.
19. Configure one HTTPS issuer per server instance/configuration. Multiple
    issuers require independent configurations and stores.
20. The new HTTP server requires the new expiry- and principal-aware storage
    contract. Existing `OAuth2TokenStore` implementations are not accepted as
    a fallback that could silently skip security checks.
21. The revocation endpoint allows a client to revoke only its own issued
    tokens. Provide separate host-side operations for user or administrator
    bulk revocation.
22. If `scope` is omitted, start from an explicitly registered per-client
    default scope, not every registered scope. User-delegated grants still
    require the host application's authorization decision.

23. Support the three RFC 8252 native redirect forms: private-use schemes,
    app-claimed HTTPS, and loopback IP redirects with a variable port. Validate
    complete registered URIs according to each form's rules.
24. Support cross-origin browser public clients with client-registered origins
    and corresponding CORS responses on token, revocation, and metadata
    endpoints. Never infer a trusted origin from an unvalidated request.
25. Use the RFC 8707 `resource` parameter to select one registered audience
    per access token. A code exchange may not expand beyond the resource
    approved during authorization.
26. Add a typed client-registration API for public/confidential client kind,
    allowed grants, scopes, audiences, redirect URIs, and browser origins.
    Retain the existing `OAuth2Application` struct for the legacy helper path
    rather than adding fields that break struct literals.
27. Keep `generate_authorization_code` and related in-process helpers as a
    documented deprecated compatibility path. Do not use them to back the new
    HTTP server because the old signature cannot carry the required PKCE
    challenge.
28. Do not import legacy access tokens or authorization codes into the new
    server. They are invalid there. Provide an explicit client-registration
    migration procedure.
29. Default authorization-code lifetime to 5 minutes, pending authorization
    lifetime to 10 minutes, and access-token lifetime to 1 hour; allow server
    configuration to shorten them. After expiry, users reauthorize and
    Client Credentials clients request a new token.
30. Include RFC 9207 `iss` in successful and error authorization responses
    and advertise support in authorization-server metadata.

31. If `resource` is omitted, use the client's sole explicit default audience.
    Reject requests with no default or more than one eligible default.
32. Authenticate introspection callers with resource-server-specific HTTP
    Basic credentials, separate from OAuth client secrets, and check the
    caller's registered audience permissions.
33. Let a public client identify itself with `client_id` and present a token
    issued to it when revoking. Require HTTP Basic for confidential clients.
34. Reject user tokens when the host identifies the user account as inactive.
    Web-session logout does not automatically revoke OAuth tokens. The host
    owns remembered-consent policy and invokes bulk revocation on relevant
    account security events.
35. Expose a rate-limit integration contract. Require a shared limiter when
    building a production server; an in-memory limiter is permitted only for
    development and tests.
36. Issue audience-restricted Bearer tokens in this delivery. Do not claim
    DPoP or mTLS sender-constrained token support.
37. Retain legacy `GrantType::RefreshToken` and `Implicit` variants for source
    compatibility, mark them deprecated, reject them in the new registration
    API, and omit them from metadata.
38. Rotate client and resource-server credentials through administrative
    APIs, invalidating old credentials immediately. Previously issued access
    tokens remain valid until expiry unless a separate revocation operation
    is requested.

## Protocol and Integration Contract

### Endpoints

The library provides `reinhardt_http::Handler` implementations that a host
application mounts explicitly through Reinhardt routing. It does not require
`reinhardt-auth` to own the application's URL table. The configured issuer and
mounted paths determine metadata URLs; metadata must match the actual routes.

| Endpoint | Method | Purpose |
| --- | --- | --- |
| Authorization | `GET` | Validate `response_type=code`, client, registered redirect URI, PKCE `S256`, requested scopes, and `resource`; create a pending request for host login and consent. |
| Authorization completion | Host integration | Approve or deny the same single-use pending request with a browser-session binding and host-authenticated user. |
| Token | `POST` | Exchange a code with client authentication and PKCE, or issue a Client Credentials token. Accept form-encoded requests and return OAuth JSON responses. |
| Revocation | `POST` | Revoke an access token for its owning client following RFC 7009. |
| Introspection | `POST` | Return RFC 7662 metadata only to an authenticated resource server permitted for the audience. |
| Authorization-server metadata | `GET` | Publish issuer, supported endpoints, grants, client authentication, PKCE `S256`, and RFC 9207 `iss` support. |

Invalid or unregistered client IDs and redirect URIs must never cause a redirect
to the submitted URI. Once the client and redirect URI are valid, authorization
errors and user denial use standard OAuth error parameters, preserve `state`
when supplied, and include `iss`. Token and revocation errors use their
respective standard response formats. Sensitive responses use no-store cache
headers, and codes and tokens never enter logs or URL query parameters other
than the authorization code in its standard redirect.

The host chooses the login and consent presentation, then returns an explicit
user identity and approved scope subset. The library binds that decision to the
pending request and browser session, validates it once, and creates the code.
An absent approval is a denial. A Client Credentials request has no user or
consent step: the authenticated confidential client receives only its own
registered scopes for the selected audience.

### Registration and Credential Model

A new client-registration type records client kind, allowed grants, redirect
URIs, allowed/default scopes, allowed/default audiences, browser origins, and
status. Public clients have no secret and cannot use Client Credentials. A
resource-server registration has its own audience identifier, introspection
credential, and permission to introspect that audience. Dynamic client
registration and end-user self-service registration are out of scope.

Scope and audience selection is grant-specific. For a user-delegated grant,
issued scope is a subset of the client's allowed scopes and the host's approved
scopes. A Client Credentials token can carry only the authenticated client's
allowed scopes. Each token carries one audience; the client cannot change an
authorization code's audience during exchange. A `resource` omission selects
only a unique registered default audience.

All three native redirect classes are validated at registration, with exact
comparison at authorization except the registered loopback IP port exception.
Browser origins are separately registered; redirect URIs do not implicitly
authorize CORS origins. Issuer and protocol endpoints use HTTPS in production.

### State and Storage

The PostgreSQL implementation stores registered clients and resource servers,
pending authorization requests, authorization codes, access-token metadata,
and credential rotation state. Use Reinhardt migrations for schema changes; do
not create or alter production tables implicitly on request handling. The
persistent contract supports atomic pending-request completion, single-use
code redemption, replay detection and linked-token revocation, token lookup
with expiry and principal metadata, client-owned revocation, and audience-aware
introspection. Expired and revoked tokens are inactive regardless of whether
physical cleanup has run.

Persist client and resource-server secrets with a password hash; persist opaque
codes and tokens as lookup digests. No endpoint returns a stored secret after
issuance. `User` and `Client` principals are distinct in lookup results, so a
Client Credentials token cannot be interpreted as an application user.
Production construction requires the durable store and a shared rate limiter.
The in-memory implementations are explicit development and test options.

### Compatibility and Migration

Keep the existing `OAuth2Application` struct, `OAuth2TokenStore` trait, and
in-process helper signatures for source compatibility. Document them as legacy
helpers, and do not route the new server through their incomplete contract.
Strengthen validation in the legacy path where its signatures allow it:
registered client/grant and redirect checks, one-time codes, and in-memory
token expiry. Stop returning refresh tokens there too. Any custom legacy store
that cannot report expiry or other required metadata is not an acceptable
production server store. New registrations use the typed API; old codes and
tokens are invalid in the new server. Provide a migration guide and update the
crate README, crate rustdoc, and integration examples to avoid claiming more
support than exists.

### Verification Plan

Protocol-level tests must mount the actual handlers through Reinhardt routing.
Cover both successful grants and negative cases: unregistered or disabled
client, wrong authentication, unsupported grant, invalid or mismatched redirect,
loopback boundary, native redirect variants, origin/CORS mismatch, missing or
wrong PKCE verifier, PKCE downgrade, scope and audience escalation, expired
pending request/code/token, code redemption racing across two instances, code
replay with linked-token revocation, inactive user, public and confidential
revocation ownership, introspection authentication and audience isolation,
metadata accuracy, and secret rotation. PostgreSQL integration tests prove
atomicity and multi-instance behavior; in-memory tests cover development mode.
Documentation examples must match the public API and compile.

## Non-Goals

- Refresh tokens, Implicit, Resource Owner Password Credentials, and other
  grant types are not available from the new server.
- OpenID Connect identity tokens, userinfo, and social-login client behavior
  remain separate work.
- Dynamic client registration, built-in login/consent UI, remembered-consent
  storage, DPoP, and mTLS sender-constrained tokens are outside this issue.
- Legacy helper stores are not a durability or protocol-compliance guarantee.

## Sources

- [OAuth 2.0 framework (RFC 6749)](https://www.rfc-editor.org/rfc/rfc6749.html)
- [Proof Key for Code Exchange (RFC 7636)](https://www.rfc-editor.org/rfc/rfc7636.html)
- [OAuth 2.0 Security Best Current Practice (RFC 9700)](https://www.rfc-editor.org/rfc/rfc9700.html)
- [OAuth 2.0 token revocation (RFC 7009)](https://www.rfc-editor.org/rfc/rfc7009.html)
- [OAuth 2.0 token introspection (RFC 7662)](https://www.rfc-editor.org/rfc/rfc7662.html)
- [OAuth 2.0 authorization-server metadata (RFC 8414)](https://www.rfc-editor.org/rfc/rfc8414.html)
- [OAuth 2.0 for native apps (RFC 8252)](https://www.rfc-editor.org/rfc/rfc8252.html)
- [Resource indicators for OAuth 2.0 (RFC 8707)](https://www.rfc-editor.org/rfc/rfc8707.html)
- [OAuth authorization response issuer identification (RFC 9207)](https://www.rfc-editor.org/rfc/rfc9207.html)
- [OAuth 2.0 for browser-based applications (RFC 10017)](https://www.rfc-editor.org/rfc/rfc10017.html)
