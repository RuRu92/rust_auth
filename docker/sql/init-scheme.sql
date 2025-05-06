USE auth;

-- 1) realm table (holds settings and configs)
CREATE TABLE IF NOT EXISTS realm (
    realm_name                              VARCHAR(255)  NOT NULL,
    is_confirmation_required                TINYINT(1)    NOT NULL DEFAULT 0,
    is_guest_allowed                        TINYINT(1)    NOT NULL DEFAULT 0,
    realm_salt_itr                          INT           NOT NULL DEFAULT 10000,
    authentication_token_duration_seconds   INT           NOT NULL DEFAULT 900,    -- Example: 15min
    refresh_token_duration_seconds          INT           NOT NULL DEFAULT 604800, -- Example: 7d
    password_reset_token_duration_seconds   INT           NOT NULL DEFAULT 1800,   -- Example: 30min

    CONSTRAINT PK_realm PRIMARY KEY (realm_name)
);

-- 2) realm_user table (stores user info for each realm)
CREATE TABLE IF NOT EXISTS realm_user (
    id                      BIGINT        NOT NULL AUTO_INCREMENT,
    user_id                 VARCHAR(36)   NOT NULL UNIQUE,
    realm_name              VARCHAR(255)  NOT NULL,
    username                VARCHAR(255)  NOT NULL,
    auth_token              TEXT,
    reset_token             TEXT,
    expires_at              DATETIME      NOT NULL DEFAULT (CURRENT_TIMESTAMP + INTERVAL 60 DAY),
    is_god                  BOOLEAN       NOT NULL DEFAULT FALSE,
    role                    VARCHAR(20)   NOT NULL DEFAULT 'CUSTOMER',
    password                TEXT          NOT NULL,
    email                   VARCHAR(100),
    name                    VARCHAR(100),
    password_reset_required BOOLEAN       NOT NULL DEFAULT 0,

    CONSTRAINT PK_realm_user PRIMARY KEY (id),
    CONSTRAINT UQ_realm_username UNIQUE (realm_name, username),
    CONSTRAINT FK_realm_user_realm
        FOREIGN KEY (realm_name)
        REFERENCES realm (realm_name)
        ON UPDATE CASCADE
        ON DELETE RESTRICT
);

-- 3) address table (optional, referencing realm_user)
CREATE TABLE IF NOT EXISTS address (
    address_id   VARCHAR(36) NOT NULL,
    user_id      VARCHAR(36) NOT NULL,
    street       TEXT        NOT NULL,
    city         TEXT,
    post_code    VARCHAR(50),
    country      VARCHAR(100),
    country_code VARCHAR(10),

    CONSTRAINT PK_address PRIMARY KEY (address_id),
    CONSTRAINT FK_address_user
        FOREIGN KEY (user_id)
        REFERENCES realm_user (user_id)
        ON UPDATE CASCADE
        ON DELETE CASCADE
);

-- Example realm entries (seed data):
INSERT INTO realm (
    realm_name,
    is_confirmation_required,
    is_guest_allowed,
    realm_salt_itr,
    authentication_token_duration_seconds,
    refresh_token_duration_seconds,
    password_reset_token_duration_seconds
) VALUES
('rj.wire', 1, 0, 15000, 900, 604800, 1800),
('rj.haven', 0, 1, 12000, 900, 604800, 1800);

-- Example realm_user entries
INSERT INTO realm_user (
    user_id,
    realm_name,
    username,
    auth_token,
    reset_token,
    expires_at,
    is_god,
    role,
    password,
    email,
    name,
    password_reset_required
) VALUES
-- Example user in rj.wire
('111e4567-e89b-12d3-a456-426614174000', 'rj.wire', 'wire_admin', 'token_wire', 'reset_wire',
 DATE_ADD(NOW(), INTERVAL 1 YEAR), 1, 'ADMIN', 'some_hashed_pass', 'wire@example.com', 'Wire Admin', TRUE),
-- Example user in rj.haven
('222e4567-e89b-12d3-a456-426614174000', 'rj.haven', 'haven_user', 'token_haven', 'reset_haven',
 DATE_ADD(NOW(), INTERVAL 6 MONTH), 0, 'CUSTOMER', 'some_hashed_pass', 'haven@example.com', 'Haven User', TRUE);