import type { RunParams } from '../types'

interface Props {
  run: RunParams
  onChange: (r: RunParams) => void
}

function Field({ label, hint, children }: { label: string; hint?: string; children: React.ReactNode }) {
  return (
    <div>
      <label className="block text-xs font-semibold text-slate-400 uppercase tracking-wider mb-1">
        {label}
        {hint && <span className="ml-2 font-normal normal-case text-slate-500">{hint}</span>}
      </label>
      {children}
    </div>
  )
}

const inp = "w-full bg-slate-700 border border-slate-600 rounded-lg px-3 py-2 text-sm text-white placeholder-slate-500 focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500"
const sel = inp + " cursor-pointer"

export default function GlobalConfig({ run, onChange }: Props) {
  const set = (k: keyof RunParams, v: string | number | undefined) =>
    onChange({ ...run, [k]: v === '' ? undefined : v })

  return (
    <div className="space-y-5">
      <div className="grid grid-cols-2 gap-4">
        <Field label="Concurrency" hint="parallel workers">
          <input type="number" min={1} className={inp} value={run.concurrency ?? ''}
            placeholder="10"
            onChange={e => set('concurrency', e.target.value ? +e.target.value : undefined)} />
        </Field>
        <Field label="Timeout" hint="ms per request">
          <input type="number" min={100} className={inp} value={run.timeout_ms ?? ''}
            placeholder="5000"
            onChange={e => set('timeout_ms', e.target.value ? +e.target.value : undefined)} />
        </Field>
      </div>

      <div className="grid grid-cols-2 gap-4">
        <Field label="Total Runs" hint="mutually exclusive with duration">
          <input type="number" min={1} className={inp} value={run.requests ?? ''}
            placeholder="e.g. 100"
            onChange={e => set('requests', e.target.value ? +e.target.value : undefined)} />
        </Field>
        <Field label="Duration" hint="seconds (mutually exclusive with runs)">
          <input type="number" min={1} className={inp} value={run.duration_secs ?? ''}
            placeholder="e.g. 30"
            onChange={e => set('duration_secs', e.target.value ? +e.target.value : undefined)} />
        </Field>
      </div>

      <div className="grid grid-cols-2 gap-4">
        <Field label="Output Format">
          <select className={sel} value={run.output_format ?? 'json'}
            onChange={e => set('output_format', e.target.value)}>
            <option value="json">JSON</option>
            <option value="html">HTML</option>
            <option value="pdf">PDF</option>
          </select>
        </Field>
        <Field label="Output File">
          <input type="text" className={inp} value={run.output ?? ''}
            placeholder="report.html"
            onChange={e => set('output', e.target.value)} />
        </Field>
      </div>
    </div>
  )
}
