# DeGov - Decentralized Governance

DeGov is a decentralized, version-controlled, composable framework for defining and running government services with cryptographic identity and verifiable audit trails.

## Getting started

To setup foundation db run this command:

```sh
container run --rm -it --name fdb -m 12G -e FDB_LISTEN_IP_VERSION=ipv4 -p 4500:4500 -e FDB_CLUSTER_FILE_CONTENTS="docker:docker@127.0.0.1:4500" --arch x86-64 fdb
```

This only needs to be ran a single time.

```sh
container exec fdb fdbcli --exec "configure new single memory"
```

```sh
container run --rm -it --name fdb -m 12G -p 4500:4500 -e FDB_CLUSTER_FILE_CONTENTS="docker:docker@127.0.0.1:4500" --arch x86-64 fdb
```



