# jwt-interceptor

Sample apps to test potential concurrency issue while managing a small cache for Google APIs KIDs retrieval in the context of an async `tonic` interceptor, following discussion on Discord.

This repository contains 2 different implementations :
- with std::sync::RwLock (blocking on request to Google APIs)
- with tokio::sync::RwLock (spawning a thread which blocks on request to Google APIs)

Use case is :
- Google APIs KIDs expire every ~5h or so
- these are concurrently used to authenticate JWT token from incoming requests, so it should be fast

Goal : ideally, a safe multi-threaded shared struct which allows for a maximum simultaneous readers at any time, but grants priority to single writer whenever KIDs expire.

## how to test ?

[ghz](https://ghz.sh/) can be used for testing.

Example :

1. Launch either of the servers :
   1. Launch `std-sample`

      ```sh
      RUST_BACKTRACE=1 RUST_LOG=debug cargo watch -x 'run -p std-sample'
      ```

   2. Launch `tokio-sample`

      ```sh
      RUST_BACKTRACE=1 RUST_LOG=debug cargo watch -x 'run -p tokio-sample'
      ```

3. Send requests

```sh
ghz --insecure \
  --async \
  -n 300 \
  -c 10 \
  --load-schedule=step \
  --load-start=50 \
  --load-step=10 \
  --load-step-duration=1s \
  --load-max-duration=10s \
  --proto ./common/protos/gateway/service.proto \
  --call gateway.service.Users.GetUser \
  -m '{"authorization":"SOME-RS256-JWT-TOKEN"}' \
  0.0.0.0:50051
```
> JWT token must contains a `kid` header