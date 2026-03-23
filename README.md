# bench

A fast, ergonomic HTTP/REST API benchmarking tool with an embedded web UI.

- Run benchmarks from the command line with a single flag, or define rich multi-scenario test plans in a JSON file
- Generates polished reports in **JSON**, **HTML** (self-contained, single file), or **PDF** formats
- Built-in **Scenario Editor** — edit your test plan visually in the browser
- Built-in **Report Viewer** — explore benchmark results interactively in the browser
- Parallel request execution with configurable concurrency
- Detailed latency percentiles (p50 → p99.9), throughput, timeline, and status/error distributions

---

## Installation

### Pre-built binaries

Download the latest binary for your platform from the [Releases](https://github.com/prasenjit-net/bench/releases) page.

| Platform | File |
|---|---|
| Linux x86_64 | `bench-vX.Y.Z-x86_64-unknown-linux-musl.tar.gz` |
| Linux ARM64 | `bench-vX.Y.Z-aarch64-unknown-linux-musl.tar.gz` |
| macOS Intel | `bench-vX.Y.Z-x86_64-apple-darwin.tar.gz` |
| macOS Apple Silicon | `bench-vX.Y.Z-aarch64-apple-darwin.tar.gz` |
| Windows x86_64 | `bench-vX.Y.Z-x86_64-pc-windows-msvc.zip` |
| Windows ARM64 | `bench-vX.Y.Z-aarch64-pc-windows-msvc.zip` |

Extract and place the `bench` binary somewhere on your `PATH`.

### Build from source

Requires: Rust (stable), Node.js ≥ 20, npm

```sh
git clone https://github.com/prasenjit-net/bench.git
cd bench
cargo build --release
# binary at: target/release/bench
```

---

## Quick Start

### Benchmark a single URL

```sh
# Run 10,000 requests with 20 concurrent workers
bench run --url https://api.example.com/health --requests 10000 --concurrency 20

# Run for 30 seconds
bench run --url https://api.example.com/health --duration 30 --concurrency 50

# POST with a JSON body, open the report in the browser when done
bench run --url https://api.example.com/users \
  --method POST \
  --header "Authorization:Bearer token123" \
  --content-type application/json \
  --body '{"name":"test"}' \
  --requests 5000 \
  --open
```

### Run a scenario file

```sh
bench run --file scenarios.json
```

### Open the visual editor to create/edit a scenario file

```sh
bench edit                          # opens scenarios.json (creates if missing)
bench edit --file my-scenarios.json
```

### View a report in the browser

```sh
bench report                        # opens report.json
bench report --file results.json
```

---

## CLI Reference

### `bench run` — Run a benchmark

```
bench run [OPTIONS]
```

#### File mode (multi-scenario)

| Flag | Short | Description |
|---|---|---|
| `--file <FILE>` | `-f` | Path to a JSON scenario file |

#### Single-step mode

| Flag | Short | Description |
|---|---|---|
| `--url <URL>` | | Target URL (**required** when not using `--file`) |
| `--method <METHOD>` | `-X` | HTTP method (default: `GET`) |
| `--header <KEY:VALUE>` | `-H` | Request header, repeatable (e.g. `-H "Accept:application/json"`) |
| `--body <BODY>` | `-d` | Raw request body |
| `--content-type <TYPE>` | | Shorthand for `Content-Type` header |
| `--name <NAME>` | | Scenario name shown in the report (default: `Benchmark`) |

#### Run parameters

| Flag | Short | Description |
|---|---|---|
| `--requests <N>` | `-n` | Total number of scenario executions (mutually exclusive with `--duration`) |
| `--duration <SECS>` | | Run for this many seconds (mutually exclusive with `--requests`) |
| `--concurrency <N>` | `-c` | Number of concurrent workers (default: `10`) |
| `--timeout <MS>` | | Per-request timeout in milliseconds (default: `5000`) |

#### Output

| Flag | Short | Description |
|---|---|---|
| `--output-format <FMT>` | | Report format: `json`, `html`, or `pdf` (default: `json`) |
| `--output <FILE>` | `-o` | Output file path (default: `report.json`, `report.html`, or `report.pdf`) |
| `--open` | | Open the JSON report in the browser after the benchmark completes |

> **Note:** When using `--file`, CLI flags override the global `run` block in the JSON file.

---

### `bench edit` — Visual Scenario Editor

```
bench edit [OPTIONS]
```

| Flag | Short | Description |
|---|---|---|
| `--file <FILE>` | `-f` | Scenario file to edit (default: `scenarios.json`, created if missing) |

Starts a local web server, opens the editor UI in your default browser. Press **Ctrl+C** to stop.

---

### `bench report` — Interactive Report Viewer

```
bench report [OPTIONS]
```

| Flag | Short | Description |
|---|---|---|
| `--file <FILE>` | `-f` | Path to a JSON report file (default: `report.json`) |

Starts a local web server, opens the report viewer in your default browser. Press **Ctrl+C** to stop.

---

## Scenario File Format

Scenario files describe your full test plan: a library of HTTP requests and one or more named scenarios that reference those requests as ordered steps.

### Top-level structure

```json
{
  "run": { ... },        // global run parameters (inherited by all scenarios)
  "requests": { ... },   // named HTTP request definitions
  "scenarios": [ ... ]   // list of test scenarios
}
```

### `run` — Global run parameters

```json
"run": {
  "concurrency": 10,        // concurrent workers
  "requests": 100000,       // total scenario executions (or use duration_secs)
  "duration_secs": 60,      // run for N seconds (mutually exclusive with requests)
  "timeout_ms": 5000,       // per-request timeout in ms
  "output_format": "json",  // "json" | "html" | "pdf"
  "output": "report.json"   // output file path
}
```

All fields are optional. Per-scenario `run` blocks override these globally.

### `requests` — HTTP request library

Named request definitions. The key becomes the step name in reports.

```json
"requests": {
  "health-check": {
    "url": "https://api.example.com/health",
    "method": "GET",
    "headers": {
      "Accept": "application/json",
      "Authorization": "Bearer my-token"
    }
  },
  "create-user": {
    "url": "https://api.example.com/users",
    "method": "POST",
    "headers": { "Content-Type": "application/json" },
    "body": "{\"name\": \"test-user\"}"
  }
}
```

| Field | Required | Description |
|---|---|---|
| `url` | ✅ | Full URL including scheme |
| `method` | | HTTP method (default: `GET`) |
| `headers` | | Map of header name → value |
| `body` | | Raw request body string |

### `scenarios` — Test scenarios

Each scenario runs all its steps sequentially, in order, for every iteration.

```json
"scenarios": [
  {
    "name": "Warm Up",
    "run": {
      "concurrency": 2,
      "requests": 5000
    },
    "steps": ["health-check"]
  },
  {
    "name": "Steady State",
    "run": {
      "requests": 100000
    },
    "steps": ["health-check", "create-user"]
  },
  {
    "name": "Over Stress",
    "run": {
      "concurrency": 50,
      "requests": 50000
    },
    "steps": ["health-check", "create-user"]
  }
]
```

| Field | Required | Description |
|---|---|---|
| `name` | ✅ | Scenario name shown in reports |
| `steps` | ✅ | Ordered list of request names (must exist in `requests`) |
| `run` | | Per-scenario run parameters (override the global `run` block) |

Each scenario is executed independently. Within one scenario execution, all steps run sequentially. The `concurrency` setting controls how many concurrent scenario executions run in parallel.

---

## Full example scenario file

```json
{
  "run": {
    "concurrency": 10,
    "timeout_ms": 5000,
    "output_format": "html",
    "output": "report.html"
  },
  "requests": {
    "health": {
      "url": "https://api.example.com/health",
      "method": "GET",
      "headers": { "Accept": "application/json" }
    },
    "list-users": {
      "url": "https://api.example.com/users",
      "method": "GET",
      "headers": { "Authorization": "Bearer token123" }
    },
    "create-user": {
      "url": "https://api.example.com/users",
      "method": "POST",
      "headers": {
        "Authorization": "Bearer token123",
        "Content-Type": "application/json"
      },
      "body": "{\"name\": \"loadtest-user\"}"
    }
  },
  "scenarios": [
    {
      "name": "Warm Up",
      "run": { "concurrency": 2, "requests": 1000 },
      "steps": ["health"]
    },
    {
      "name": "Steady State",
      "run": { "requests": 50000 },
      "steps": ["health", "list-users", "create-user"]
    },
    {
      "name": "Peak Load",
      "run": { "concurrency": 100, "duration_secs": 60 },
      "steps": ["health", "list-users"]
    }
  ]
}
```

---

## Report Formats

### JSON (default)
Machine-readable report with full latency histogram, timeline, status/error distributions, and all percentiles.

```sh
bench run --file scenarios.json                       # report.json (default)
bench run --file scenarios.json --output-format json
```

### HTML
Self-contained, single `.html` file with the React report viewer embedded — works offline, no server required. Open it by double-clicking.

```sh
bench run --file scenarios.json --output-format html
```

### PDF
Professional PDF report with branded header, metric cards, latency tables, and color-coded bar charts.

```sh
bench run --file scenarios.json --output-format pdf
```

### Interactive viewer (JSON reports)
After generating a JSON report, view it in the browser:

```sh
# Automatically after a run:
bench run --file scenarios.json --open

# From an existing report file:
bench report --file report.json
```

---

## Report metrics

Each step in a report includes:

| Metric | Description |
|---|---|
| **Throughput** | Requests per second (req/s) |
| **Total requests** | Total HTTP requests sent |
| **Successful** | Responses with 2xx status codes |
| **Failed** | Responses with 4xx/5xx status codes |
| **Errors** | Network errors (connection refused, timeout, etc.) |
| **Success rate** | `successful / total × 100%` |
| **Latency min/max** | Fastest and slowest observed response times |
| **Latency mean/stddev** | Average and standard deviation |
| **p50 / p75 / p90 / p95 / p99 / p99.9** | Response time percentiles |
| **Latency histogram** | Response time distribution by bucket |
| **Status distribution** | Count per HTTP status code |
| **Error distribution** | Count per error type |
| **Timeline** | Throughput (req/s) sampled per second |

---

## License

MIT
