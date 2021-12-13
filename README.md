# gh-action-trace

`gh-action-trace` is a binary that can be used to generate traces
for GitHub Action runs by retrieving the metadata from GitHub
API.

![Jaeger](https://imgur.com/6fJ3iui.png)

## Installation

```bash
cargo install gh-action-trace
```

## Usage

First, Run jaeger locally to collect the traces.

```bash
docker run -d  -p6831:6831/udp -p6832:6832/udp -p16686:16686 jaegertracing/all-in-one:latest --log-level debug
```

Now, Run the binary to generate and send the traces to Jaeger. Though, The
binary should work without a GitHub token, It is **recommended** to pass
a GitHub token through the `--token` flag for the binary to not be rate-limited.

```bash
$ gh-action-trace --owner linkerd --repo linkerd2 --runs 100 --token <GITHUB_TOKEN>
Completed workflow CI                                                         10/10 [========================================] (0s)
Completed workflow Coverage                                                   10/10 [========================================] (0s)
Completed workflow CodeQL                                                     10/10 [========================================] (0s)
Completed workflow Integration tests                                          10/10 [========================================] (0s)
Completed workflow KinD integration                                           10/10 [========================================] (0s)
Completed workflow Lock Threads                                               10/10 [========================================] (0s)
Completed workflow Policy Controller                                          10/10 [========================================] (0s)
Completed workflow Release                                                    10/10 [========================================] (0s)
Completed workflow Static checks                                              10/10 [========================================] (0s)
Completed workflow Unit tests                                                 10/10 [========================================] (0s)
Completed workflow CI                                                         10/10 [========================================] (0s)
```

You should be able to see traces in the Jaeger UI. :)
