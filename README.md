# piko-site

A static site canister for the Internet Computer with no update methods.

- The built site (`dist/`) is embedded in the module at compile time, with
  gzip copies. Changing the site means installing a new module.
- `canister_inspect_message` refuses every ingress update call, which costs
  the canister 0 cycles. Pages are served by the `http_request` query.
- Responses are certified (response verification v2) with DFINITY's
  `ic-asset-certification` crate.
- HTML pages carry the same `ic_env` cookie as the stock asset canister
  (root key + `PUBLIC_*` canister environment variables), read by
  `getCanisterEnv()` in `@icp-sdk/core`.
- Paths without a file get the site's `404.html`, or `src/404.html`.

## Build and verify

    ./build-site.sh <path to dist> out/site.wasm

Same `dist/`, `Cargo.lock` and toolchain (`rust-toolchain.toml`) give the
same module hash on any machine. Compare it with the `module_hash` shown by
`icp canister status <id> -e ic`.

## Install

Switching from the stock asset canister: use `--mode reinstall` so the old
module's stable memory (about 100 MB) is dropped. The canister id, its
environment variables and its URL stay the same.

## PIKO sites using it

| Site | Canister | Module (sha256 prefix) |
|---|---|---|
| Pixel | cglm2-byaaa-aaaac-bf4aq-cai | c1266033 |
| Legacy | vnwxz-wiaaa-aaaak-qzk6a-cai | 7385262a |
| Mining | 5xdl7-taaaa-aaaaj-qseeq-cai | b1b09d5b |
| Dice | 77zu2-baaaa-aaaaj-qseiq-cai | 2ab82695 |
| Blackjack | bcd2h-5iaaa-aaaai-ax4hq-cai | ef0304dc |
| Landing | 7w27g-xiaaa-aaaaj-qseja-cai | ac13946d |
| Poker | 2uuxi-qyaaa-aaaac-qhbyq-cai | b2d8d61e |
| Monitor | k6tpq-iqaaa-aaaaa-qhpfq-cai | 6089fe15 |

Each module embeds that site's built files at the commit of its own
repository that is live on mainnet. Rebuild with `build-site.sh` from that
`dist/` to check the hash.
