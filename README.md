# DeGov - Decentralized Governance

DeGov is a decentralized, version-controlled, composable framework for defining and running government services with cryptographic identity and verifiable audit trails.

## Getting started

To setup foundation db run this command:

```sh
container run -d --name fdb -m 12G --entrypoint bash -p 4500:4500 --arch x86_64 -v fdb-config:/tmp/fdb:ro -v fdb-data:/var/lib/foundationdb:rw foundationdb/foundationdb:7.3.69 -c "cp /tmp/fdb/fdb.cluster /var/fdb/fdb.cluster && fdbserver --public-address=0.0.0.0:4500 --datadir=/var/lib/foundationdb"
```

```sh
container run -it -p 4500:4500 --arch x86_64 -e FDB_PUBLIC_IP=0.0.0.0 --entrypoint="bash" foundationdb/foundationdb:7.3.69 '
    FDB_CLUSTER_FILE="/etc/foundationdb/fdb.cluster";
    FDB_CLUSTER_FILE_CONTENTS="fdb:fdb@0.0.0.0:4500";
    echo "$FDB_CLUSTER_FILE_CONTENTS" > "$FDB_CLUSTER_FILE";

    echo "Starting FDB server on $FDB_PUBLIC_IP:$FDB_PORT";

    exec fdbserver \
      --listen-address 0.0.0.0:4500 \
      --public-address 0.0.0.0:4500 \
      --datadir /var/fdb/data \
  '
```


This only needs to be ran a single time.

```sh
container exec fdb fdbcli --exec "configure new single memory"
```



