import type { Report } from './report/types'

declare global {
  interface Window {
    __BENCH_MODE__?: 'editor' | 'report'
    __BENCH_REPORT__?: Report
  }
}

export {}
