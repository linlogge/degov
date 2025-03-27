# degov

```sh
docker run --rm -it -m 8G --name fdb -p 4500:4500 foundationdb/foundationdb:7.3.62
docker exec fdb fdbcli --exec "configure new single memory"
