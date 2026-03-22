export interface RequestDef {
  url: string
  method: string
  headers: Record<string, string>
  body?: string
}

export interface RunParams {
  concurrency?: number
  duration_secs?: number
  requests?: number
  timeout_ms?: number
  output_format?: string
  output?: string
}

export interface ScenarioRef {
  name: string
  run?: RunParams
  steps: string[]
}

export interface ScenarioFile {
  run?: RunParams
  requests: Record<string, RequestDef>
  scenarios: ScenarioRef[]
}

export const METHODS = ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'HEAD', 'OPTIONS']

export const emptyRequest = (): RequestDef => ({
  url: '',
  method: 'GET',
  headers: {},
})

export const emptyScenario = (): ScenarioRef => ({
  name: 'New Scenario',
  steps: [],
})

export const emptyFile = (): ScenarioFile => ({
  run: { concurrency: 10, timeout_ms: 5000, requests: 100, output_format: 'json', output: 'report.json' },
  requests: {},
  scenarios: [],
})
