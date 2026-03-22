import type { ScenarioFile } from './types'

export async function fetchScenario(): Promise<ScenarioFile> {
  const res = await fetch('/api/scenario')
  if (!res.ok) throw new Error(`Failed to load: ${res.statusText}`)
  return res.json()
}

export async function saveScenario(data: ScenarioFile): Promise<void> {
  const res = await fetch('/api/scenario', {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(data, null, 2),
  })
  if (!res.ok) {
    const msg = await res.text()
    throw new Error(msg || res.statusText)
  }
}
