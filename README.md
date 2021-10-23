# gh-action-trace

`gh-action-trace` is a binary that can be used to generate traces
for GitHub Action runs by retrieving the metadata from GitHub
API.

![Jaeger](https://imgur.com/IXIdXTx.png)

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
Processing 30 runs out of 640 for workflow CI
Processing 13 runs out of 13 for workflow Coverage
Processing 30 runs out of 1394 for workflow CodeQL
Processing 30 runs out of 4107 for workflow Integration tests
Processing 30 runs out of 3294 for workflow KinD integration
Processing 30 runs out of 580 for workflow Lock Threads
Processing 30 runs out of 175 for workflow Policy Controller
Processing 30 runs out of 151 for workflow Release
Processing 30 runs out of 7456 for workflow Static checks
Processing 30 runs out of 7402 for workflow Unit tests
Processing 30 runs out of 1262 for workflow CI
```
