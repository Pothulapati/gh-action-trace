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
$ gh-action-trace --owner linkerd --repo linkerd2 --token <GITHUB_TOKEN>
[1/11]   Completed workflow CI
[2/11]   Completed workflow Coverage
[3/11]   Completed workflow CodeQL
[4/11]   Completed workflow Integration tests
[5/11]   Completed workflow KinD integration
[6/11]   Completed workflow Lock Threads
[7/11]   Completed workflow Policy Controller
[8/11]   Completed workflow Release
[9/11]   Completed workflow Static checks
[10/11]   Completed workflow Unit tests
[11/11]   Completed workflow CI
```

You should be able to see traces in the Jaeger UI. :)
