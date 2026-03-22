export interface ReportStep {
  name: string
  url: string
  method: string
  concurrency: number
  total_requests: number
  successful_requests: number
  failed_requests: number
  error_requests: number
  duration_secs: number
  throughput_rps: number
  latency_min_ms: number
  latency_max_ms: number
  latency_mean_ms: number
  latency_stddev_ms: number
  latency_p50_ms: number
  latency_p75_ms: number
  latency_p90_ms: number
  latency_p95_ms: number
  latency_p99_ms: number
  latency_p999_ms: number
  status_distribution: Record<string, number>
  error_distribution: Record<string, number>
  timeline: [number, number][]
  latency_histogram: [string, number][]
}

export interface ReportGroup {
  name: string
  concurrency: number
  run_desc: string
  step_count: number
  total_requests: number
  successful_requests: number
  failed_requests: number
  error_requests: number
  steps: ReportStep[]
}

export interface Report {
  generated_at: string
  group_count: number
  total_requests: number
  successful_requests: number
  failed_requests: number
  error_requests: number
  groups: ReportGroup[]
}
