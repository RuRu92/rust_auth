--------------------------------------------------------------------------------
-- 1) user table
--------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS user (
    user_id   VARCHAR(36)   NOT NULL,
    username  VARCHAR(100)  NOT NULL,
    realm     VARCHAR(255)  NOT NULL,
    role      VARCHAR(20)   NOT NULL,
    name      VARCHAR(100),
    password  TEXT          NOT NULL,
    email     VARCHAR(100),

    CONSTRAINT PK_user PRIMARY KEY (user_id),
    CONSTRAINT UQ_username_realm UNIQUE (username, realm),
    CONSTRAINT FK_user_realm 
        FOREIGN KEY (realm) 
        REFERENCES realm_settings (realm) 
        ON UPDATE CASCADE 
        ON DELETE RESTRICT
);

--------------------------------------------------------------------------------
-- 2) address table
--------------------------------------------------------------------------------
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
        REFERENCES user (user_id)
        ON UPDATE CASCADE
        ON DELETE CASCADE
);

--------------------------------------------------------------------------------
-- 3) realm_settings table
--------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS realm_settings (
    realm                                  VARCHAR(255)  NOT NULL,
    is_confirmation_required               TINYINT(1)    NOT NULL DEFAULT 0,
    is_guest_allowed                       TINYINT(1)    NOT NULL DEFAULT 0,
    realm_salt_itr                         INT           NOT NULL DEFAULT 10000,
    authentication_token_duration_seconds  INT           NOT NULL DEFAULT 120,
    refresh_token_duration_seconds         INT           NOT NULL DEFAULT 60,
    password_reset_token_duration_seconds  INT           NOT NULL DEFAULT 30,

    CONSTRAINT PK_realm_settings PRIMARY KEY (realm)
);

--------------------------------------------------------------------------------
-- realm table (optional, if you want a separate entity from realm_settings)
--------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS realm (
    realm_id    VARCHAR(36)   NOT NULL,
    realm_name  VARCHAR(255)  NOT NULL, UNIQUE
    username    VARCHAR(255)  NOT NULL, UNIQUE
    
    -- You can reference realm_settings if needed:
    FOREIGN KEY (realm_name) REFERENCES realm_settings (realm)
    CONSTRAINT PK_realm PRIMARY KEY (realm_id)
);