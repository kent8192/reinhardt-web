CREATE TABLE oidc_op_subject_reservations (
    sub TEXT PRIMARY KEY
);
CREATE TABLE oidc_op_subjects (
    user_id TEXT PRIMARY KEY,
    sub TEXT NOT NULL UNIQUE REFERENCES oidc_op_subject_reservations(sub)
);
CREATE TABLE oidc_op_pending (
    id TEXT PRIMARY KEY,
    payload JSONB NOT NULL,
    expires_at BIGINT NOT NULL
);
CREATE INDEX oidc_op_pending_expiry ON oidc_op_pending (expires_at);
CREATE TABLE oidc_op_codes (
    digest TEXT PRIMARY KEY,
    payload JSONB NOT NULL,
    expires_at BIGINT NOT NULL
);
CREATE INDEX oidc_op_codes_expiry ON oidc_op_codes (expires_at);
CREATE TABLE oidc_op_keys (
    kid TEXT PRIMARY KEY,
    public JSONB NOT NULL,
    activated_at BIGINT NOT NULL,
    active BOOLEAN NOT NULL,
    publish_until BIGINT,
    compromised BOOLEAN NOT NULL DEFAULT FALSE
);
CREATE UNIQUE INDEX oidc_op_one_active_key ON oidc_op_keys (active) WHERE active = TRUE;
