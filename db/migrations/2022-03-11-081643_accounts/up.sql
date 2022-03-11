-- table that tracks all obligation accounts, their LTVs, as well as their
-- account data for quick deserialization
CREATE TABLE obligations (
  id BIGSERIAL PRIMARY KEY NOT NULL,
  -- the ltv/health of the obligation
  ltv FLOAT8 NOT NULL DEFAULT 0,
  -- the address of the obligation account
  account VARCHAR NOT NULL UNIQUE,
  -- the actual data of the account base64 encoded
  account_data BYTEA,
  -- the time at which this data was last updated at
  scraped_at TIMESTAMPTZ NOT NULL
);

-- table that tracks prices for all pyth price feeds used by the lending program
CREATE TABLE price_feeds (
    id BIGSERIAL PRIMARY KEY NOT NULL,
    token_mint VARCHAR NOT NULL DEFAULT '',
    price_account VARCHAR NOT NULL UNIQUE,
    decimals INT2 NOT NULL DEFAULT 0,
    price FLOAT8 NOT NULL DEFAULT 0,
    scraped_at TIMESTAMPTZ NOT NULL
);