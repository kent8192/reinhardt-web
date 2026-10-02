CREATE TABLE oauth_server_clients (
    client_id TEXT PRIMARY KEY,
    payload JSONB NOT NULL
);
CREATE TABLE oauth_server_resources (
    resource_id TEXT PRIMARY KEY,
    payload JSONB NOT NULL
);
CREATE UNIQUE INDEX oauth_server_resource_audience ON oauth_server_resources ((payload->>'audience'));
CREATE TABLE oauth_server_pending (
    id TEXT PRIMARY KEY,
    payload JSONB NOT NULL,
    expires_at BIGINT NOT NULL
);
CREATE INDEX oauth_server_pending_expiry ON oauth_server_pending (expires_at);
CREATE TABLE oauth_server_codes (
    digest TEXT PRIMARY KEY,
    payload JSONB NOT NULL,
    client_id TEXT NOT NULL,
    expires_at BIGINT NOT NULL,
    redeemed BOOLEAN NOT NULL DEFAULT FALSE,
    replayed BOOLEAN NOT NULL DEFAULT FALSE
);
CREATE INDEX oauth_server_codes_expiry ON oauth_server_codes (expires_at);
CREATE TABLE oauth_server_tokens (
    digest TEXT PRIMARY KEY,
    payload JSONB NOT NULL,
    client_id TEXT NOT NULL,
    user_id TEXT,
    code_digest TEXT REFERENCES oauth_server_codes(digest),
    expires_at BIGINT NOT NULL,
    revoked BOOLEAN NOT NULL DEFAULT FALSE
);
CREATE INDEX oauth_server_tokens_client ON oauth_server_tokens (client_id);
CREATE INDEX oauth_server_tokens_user ON oauth_server_tokens (user_id) WHERE user_id IS NOT NULL;
CREATE INDEX oauth_server_tokens_expiry ON oauth_server_tokens (expires_at);
CREATE INDEX oauth_server_tokens_code ON oauth_server_tokens (code_digest) WHERE code_digest IS NOT NULL;
