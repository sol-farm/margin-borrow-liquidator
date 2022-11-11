# margin-borrow-liquidator
liquidator for tulip lending borrow


# Configuration

```yaml
---
analytics:
  obligation_refresh_concurrency: 0
  price_feeds: []
  reserves:
    - name: USDC
      account: FTkSmGsJ3ZqDSHdcnY7ejN1pWV3Ej7i88MYpZyyaqgGt

    - name: USDT
      account: Csn3exasdhDzxYApmnci3d8Khb629VmgK4NQqdeyZBNt

    - name: RAY
      account: 9Bm8d2izGsf9eT6Wr79DTnXBkW2LHYVQa57QzeoTbsAF

    - name: SOL
      account: FzbfXR7sopQL29Ubu312tkqWMxSre4dYSrFyYAjUYiC4

    - name: SRM
      account: 9AiGVt7Qtap2ijvim4JSudDYgTrSWhwaZmKv8BWGFms9

    - name: MER
      account: 9wFUsWXt9vc69mU1jcjgPziLSYy6dLu7Dy9idNjo33vy

    - name: MEDIA
      account: 4YUqefxqtfa8PwFynQKJjDC74cJzPtR69uP1UqZKQLZQ

    - name: TULIP
      account: DdFHZu9n41MuH2dNJMgXpnnmuefxDMdUbCp4iizPfz1o

    - name: SLRS
      account: 3YzfgFtos1cq1PGWABSYXj3txhwAnWjVZxacPafMPwZH

    - name: ALEPH
      account: 5nestDtwfXxCHbJ7BcWgucBmSp7ApxUtW9cDwDa3apED

    - name: ROPE
      account: BAkQnFTVBHE9XGo7rEidRMEhrFyXXxKPchW2KXtkPKzG

    - name: COPE
      account: DrYZA2Q6eBwFD7d2x8mmsLWNcQJGVEX6ntF9nMmNpPBe

    - name: SNY
      account: DiC9QF1MLQgzVNRDUdLaDgmAQ5JA8ksd8AaaYrJbEHnD

    - name: STEP
      account: HpYGGceBPSWhemfsUtdAXjDJpTiWa6MppMr8LaCfkwyX

    - name: ETH
      account: FxH3A2Bw9q3MDivXuWtr8zxiN3S7dGTEkK4Mm95NR2yB

    - name: LIKE
      account: BQk6St4EmdBUo6jx4XCM9bobwV7wwcc4L7QwZHgN3GwL

    - name: ORCA
      account: 6sJg8f3zcAjrd38QhSA3C34n8MzLq1XVTiQr4msozAuv

    - name: ATLAS
      account: 41Pgp5bSZtNgLiWuwi16Nhf6o75cKzbkKULUd53EFdcU

    - name: POLIS
      account: 7hxTjiLvBuZcUQnztSRhtvthcsVdu7Na5WWXocwBWA8y

    - name: whETH
      account: 7n9bDFUxehuw7yAHnK9eWKSfMx5u6NYWPVLKYnz93fzj

    - name: mSOL
      account: 5LKgrsUF72MityTntAHWLcXivBGxnxapikFArtKUULwX

    - name: BTC
      account: 5CXZ2xKG1i2w3fBXsCEC6zhpK5j164gxJ3bzhyoRB3ZP

    - name: SAMO
      account: 6nVuFQdDEPsh7yhPuHR3WbsYYEHHDjetEhvy3hrzbUBb

    - name: STARS
      account: HJDm6bso3CXHjUZRLRnV3VLupgeNbeYD4SGXEiaqrDEh

    - name: weUNI
      account: F3y6c19hcn91RRkqZc6BN6d2B5F9etkNks9BzUxvqc2M

    - name: weSUSHI
      account: FEDEBKAtZzod5oXv1UkSzEeDZGsFe3DK9Wq23o6B4QVN

    - name: weDYDX
      account: 2vzY9tJNqutsGnUwPmka3LmAEjDXJ2qKeV9fAztD7Sbo

    - name: GENE
      account: 3gmxqqfQhMtqAnQWuFNiTqgLkAok54SGuvdncPYEAq2i

    - name: DFL
      account: 9z3qY3jfoPVGAfCqr1w95q5RW29tSNvyitLL67o4E6Lk

    - name: CAVE
      account: 4eEZmrgcBnQ6XrtzVkZB3Ae9zvWF3AaDio8Xem4qZ5cb

    - name: wbWBNB
      account: Fbe9bgforFZfK1Zf14R4S2EasUimk64JRUi1hgJZXf26

    - name: REAL
      account: 3PBWn8kyNhvjzbjBPtgsukuw6jLv6YJ6gicxi7fo26Mj

    - name: PRSM
      account: 5ZETtVZiqomusvAKLtCJvfKdhotc1HornaL5VF1Z2L71

    - name: MBS
      account: H6GW9vVsGoibZMQkt5MaUrEmmqzPrTFgvVdAQssEytpv

    - name: SHDW
      account: E22L7J4KmTLFoARUmfKtdG59jP8sUderp8CJoNvM4gk5

    - name: BASIS
      account: 7wAiwRyM66qfDrDBZD9xLii95tX47xzRPAfQiomrqrsN

    - name: stSOL
      account: BsCdKC2ncgS3VnuibTiA5Etx6MZGRnUL2w88iDee3A6Z

    - name: GST
      account: 9CwVAjhpKqkPA27GsikXHxQQgG3oJiiF1ybkcC1pZtGf

    - name: wUST
      account: 8s5Gc63C8zUYRHXxjqyyNwXzK4fqQidcjx5a43Rmf54v

    - name: ZBC
      account: F9pwMLPQy1MJv14EE3XWdncUaJbPZdaqgfuHmfwxcWzc

    - name: wALEPH
      account: 7MicHAbktN1AmuuBxUGpdmb4iqeiD9GqduejMAX9g415

    - name: SLCL
      account: 3PP7T3RGf6UKG6BvAsQudyZg7qzPLcpmo5eeqoxENvKB

    - name: SLC
      account: 5BZgs8KZ79e12GPse8qDarUvN5bS1R4krRqAGqpbdcFd

    - name: GMT
      account: EPm5gyRafEZMHijXuyoA5imPFoEir8KsJ9fd2cyEFrPU

    - name: sRLY
      account: 6jNicvm4rToeRe3MbkFXNmNfg8iVtZuySGJqqijsZ6j2

    - name: sRLY
      account: 6jNicvm4rToeRe3MbkFXNmNfg8iVtZuySGJqqijsZ6j2

    - name: wETH
      account: EffQjqa2vWm5JMPyCrRJSDGYGEHTuQWmEz8VJSYGRCBL

  scrape_interval: 0
debug_log: false
key_path: ""
liquidator:
  frequency: 100
  max_concurrency: 32
  min_ltv: 85.0
log_file: ""
programs:
  lending:
    id: ""
    idl_path: ""
  pyth:
    id: ""
    idl_path: ""
refresher:
  frequency: 100
  max_concurrency: 32
rpc_endpoints:
  failover_endpoints:
    - http_url: "https://solana-api.projectserum.com"
      ws_url: "ws://solana-api.projectserum.com"
  primary_endpoint:
    http_url: "https://solfarm.rpcpool.com/d9ccba03e485d981c3f467525402"
    ws_url: "ws://api.mainnet-beta.solana.com"
sled_db:
  db:
    compression_factor: ~
    debug: false
    mode: ~
    path: liquidator.db
    system_page_cache: ~
  rpc:
    auth_token: ""
    connection:
      HTTP:
        - 127.0.0.1
        - "6969"
    tls_cert: ""
    tls_key: ""
telemetry:
  agent_endpoint: "http://localhost:8126"
  enabled: true
