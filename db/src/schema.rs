table! {
    use diesel::sql_types::*;

    obligations (id) {
        id -> Int8,
        ltv -> Float8,
        account -> Varchar,
        account_data -> Bytea,
        scraped_at -> Timestamptz,
    }
}

table! {
    use diesel::sql_types::*;

    price_feeds (id) {
        id -> Int8,
        token_mint -> Varchar,
        price_account -> Varchar,
        decimals -> Int2,
        price -> Float8,
        scraped_at -> Timestamptz,
    }
}

allow_tables_to_appear_in_same_query!(
    obligations,
    price_feeds,
);
