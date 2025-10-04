# DeGov - Decentralized Governance

## Project Structure

degov/
├── crates/
│   ├── degov-core/             # Core engine: DGL, workflow, permissions
│   ├── degov-crypto/           # Cryptographic primitives, KMS/HSM integration
│   ├── degov-identity/         # DID, VC, citizen identity management
│   ├── degov-storage/          # FoundationDB layer + Merkle Search Trees (MST)
│   ├── degov-network/          # P2P libp2p networking, inter-authority comms
│   ├── degov-consent/          # Consent management & audit ledger
│   ├── degov-api/              # REST API & gRPC gateway
│   ├── degov-ui/               # Bindings for React UI framework (not full frontend)
│   ├── degov-admin/            # Low-code builder backend
│   ├── degov-governance/       # Federated governance & trust model
│   └── degov-cli/              # CLI tools for deployment, admin, dev tasks
├── examples/
│   ├── citizen-portal/
│   ├── inter-authority-demo/
│   └── plugin-demo/
├── tests/
│   └── integration/          # Cross-crate integration tests
└── Cargo.toml                # Workspace manifest

## Getting started

To setup foundation db run this command:

```sh
container run -d --name fdb -m 12G --entrypoint bash -p 4500:4500 --arch x86_64 -v fdb-config:/tmp/fdb:ro -v fdb-data:/var/lib/foundationdb:rw foundationdb/foundationdb:7.3.69 -c "cp /tmp/fdb/fdb.cluster /var/fdb/fdb.cluster && fdbserver --public-address=0.0.0.0:4500 --datadir=/var/lib/foundationdb"
```

This only needs to be ran a single time.

```sh
container exec fdb fdbcli --exec "configure new single memory"
```



