import { useEffect, useState } from 'react'
import { AlertCircle, Loader2, CheckCircle, XCircle, Zap, Clock } from 'lucide-react'
import type { Report, ReportGroup, ReportStep } from './types'

// ── Helpers ───────────────────────────────────────────────────────────────────

function fmt(n: number, d = 1) { return n.toLocaleString(undefined, { maximumFractionDigits: d }) }
function pct(a: number, b: number) { return b === 0 ? '0' : fmt((a / b) * 100, 1) }

function StatusBadge({ ok, total }: { ok: number; total: number }) {
  const rate = total === 0 ? 100 : (ok / total) * 100
  const color = rate === 100 ? 'text-emerald-400' : rate >= 95 ? 'text-yellow-400' : 'text-red-400'
  return <span className={`font-semibold ${color}`}>{pct(ok, total)}%</span>
}

// ── Inline bar chart (CSS) ────────────────────────────────────────────────────

function Bar({ value, max, color = 'bg-indigo-500' }: { value: number; max: number; color?: string }) {
  const w = max === 0 ? 0 : Math.round((value / max) * 100)
  return (
    <div className="h-2 bg-slate-700 rounded-full overflow-hidden">
      <div className={`h-full ${color} rounded-full transition-all`} style={{ width: `${w}%` }} />
    </div>
  )
}

// ── Latency table for one step ────────────────────────────────────────────────

function LatencyRow({ label, value, max, color }: { label: string; value: number; max: number; color: string }) {
  return (
    <div className="grid grid-cols-[5rem_5rem_1fr] items-center gap-3">
      <span className="text-xs text-slate-400 font-mono">{label}</span>
      <span className="text-xs text-right font-mono text-white">{fmt(value, 2)} ms</span>
      <Bar value={value} max={max} color={color} />
    </div>
  )
}

function StepCard({ step }: { step: ReportStep }) {
  const maxLat = step.latency_p999_ms
  const statusEntries = Object.entries(step.status_distribution)
    .sort(([a], [b]) => Number(a) - Number(b))
  const errorEntries = Object.entries(step.error_distribution)
    .sort(([, a], [, b]) => b - a)
  const maxStatus = Math.max(...statusEntries.map(([, v]) => v), step.error_requests, 1)

  const methodColor: Record<string, string> = {
    GET: 'bg-emerald-700 text-emerald-200',
    POST: 'bg-blue-700 text-blue-200',
    PUT: 'bg-yellow-700 text-yellow-200',
    PATCH: 'bg-orange-700 text-orange-200',
    DELETE: 'bg-red-700 text-red-200',
  }
  const mc = methodColor[step.method] ?? 'bg-slate-600 text-slate-200'

  return (
    <div className="bg-slate-800 border border-slate-700 rounded-xl p-5 space-y-5">
      {/* Header */}
      <div className="flex items-start gap-3">
        <span className={`text-xs font-bold px-2 py-1 rounded font-mono mt-0.5 ${mc}`}>{step.method}</span>
        <div className="flex-1 min-w-0">
          <p className="font-semibold text-white">{step.name}</p>
          <p className="text-xs text-slate-400 font-mono truncate">{step.url}</p>
        </div>
      </div>

      {/* Key metrics */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
        {[
          { label: 'Throughput', value: `${fmt(step.throughput_rps)} req/s`, icon: <Zap size={13} /> },
          { label: 'Total', value: fmt(step.total_requests, 0), icon: null },
          { label: 'Success rate', value: <StatusBadge ok={step.successful_requests} total={step.total_requests} />, icon: null },
          { label: 'Duration', value: `${fmt(step.duration_secs)} s`, icon: <Clock size={13} /> },
        ].map(({ label, value, icon }) => (
          <div key={label} className="bg-slate-900 rounded-lg p-3">
            <div className="flex items-center gap-1 text-xs text-slate-400 mb-1">{icon}{label}</div>
            <div className="text-sm font-semibold text-white">{value}</div>
          </div>
        ))}
      </div>

      {/* Latency percentiles */}
      <div>
        <p className="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-3">Latency Percentiles</p>
        <div className="space-y-2">
          <LatencyRow label="min"   value={step.latency_min_ms}  max={maxLat} color="bg-emerald-500" />
          <LatencyRow label="p50"   value={step.latency_p50_ms}  max={maxLat} color="bg-indigo-500" />
          <LatencyRow label="p75"   value={step.latency_p75_ms}  max={maxLat} color="bg-indigo-400" />
          <LatencyRow label="p90"   value={step.latency_p90_ms}  max={maxLat} color="bg-yellow-500" />
          <LatencyRow label="p95"   value={step.latency_p95_ms}  max={maxLat} color="bg-orange-500" />
          <LatencyRow label="p99"   value={step.latency_p99_ms}  max={maxLat} color="bg-red-500" />
          <LatencyRow label="p99.9" value={step.latency_p999_ms} max={maxLat} color="bg-red-700" />
          <LatencyRow label="mean"  value={step.latency_mean_ms} max={maxLat} color="bg-slate-400" />
          <LatencyRow label="max"   value={step.latency_max_ms}  max={maxLat} color="bg-slate-600" />
        </div>
      </div>

      {/* Status distribution */}
      <div className="grid grid-cols-2 gap-4">
        <div>
          <p className="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-3">Status Codes</p>
          {statusEntries.length === 0 && step.error_requests === 0 &&
            <p className="text-xs text-slate-600 italic">No responses recorded</p>}
          <div className="space-y-2">
            {statusEntries.map(([code, count]) => {
              const c = Number(code)
              const color = c < 300 ? 'bg-emerald-500' : c < 400 ? 'bg-yellow-500' : 'bg-red-500'
              return (
                <div key={code} className="grid grid-cols-[3.5rem_3.5rem_1fr] items-center gap-2">
                  <span className="text-xs font-mono text-slate-300">{code}</span>
                  <span className="text-xs text-right font-mono text-slate-400">{fmt(count, 0)}</span>
                  <Bar value={count} max={maxStatus} color={color} />
                </div>
              )
            })}
            {step.error_requests > 0 && (
              <div className="grid grid-cols-[3.5rem_3.5rem_1fr] items-center gap-2">
                <span className="text-xs font-mono text-red-400">errors</span>
                <span className="text-xs text-right font-mono text-red-400">{fmt(step.error_requests, 0)}</span>
                <Bar value={step.error_requests} max={maxStatus} color="bg-red-600" />
              </div>
            )}
          </div>
        </div>

        {/* Error breakdown */}
        {errorEntries.length > 0 && (
          <div>
            <p className="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-3">Errors</p>
            <div className="space-y-1">
              {errorEntries.map(([msg, count]) => (
                <div key={msg} className="flex justify-between items-center">
                  <span className="text-xs text-red-400 truncate flex-1">{msg}</span>
                  <span className="text-xs font-mono text-slate-400 ml-2">{fmt(count, 0)}</span>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>

      {/* Timeline sparkline */}
      {step.timeline.length > 0 && (
        <div>
          <p className="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2">Timeline (req/s)</p>
          <TimelineSpark data={step.timeline} />
        </div>
      )}
    </div>
  )
}

function TimelineSpark({ data }: { data: [number, number][] }) {
  const maxVal = Math.max(...data.map(([, v]) => v), 1)
  const w = 500
  const h = 48
  const barW = Math.max(2, Math.floor(w / data.length) - 1)
  return (
    <svg viewBox={`0 0 ${w} ${h}`} className="w-full h-12 rounded overflow-hidden">
      {data.map(([, v], i) => {
        const bh = Math.max(1, (v / maxVal) * h)
        return (
          <rect key={i} x={i * (barW + 1)} y={h - bh} width={barW} height={bh}
            fill="#6366f1" fillOpacity={0.8} />
        )
      })}
    </svg>
  )
}

// ── Group card ────────────────────────────────────────────────────────────────

function GroupCard({ group }: { group: ReportGroup }) {
  const [expanded, setExpanded] = useState(true)
  const successRate = group.total_requests === 0 ? 100 :
    (group.successful_requests / group.total_requests) * 100
  const color = successRate === 100 ? 'border-emerald-700' : successRate >= 95 ? 'border-yellow-700' : 'border-red-700'

  return (
    <div className={`border-l-4 ${color} pl-4`}>
      <button onClick={() => setExpanded(e => !e)}
        className="w-full text-left mb-4">
        <div className="flex items-center gap-4">
          <h2 className="text-lg font-bold text-white">{group.name}</h2>
          <span className="text-xs text-slate-500">{group.run_desc} · concurrency {group.concurrency}</span>
          <div className="flex-1" />
          <div className="flex items-center gap-4 text-sm">
            <span className="text-slate-400">{fmt(group.total_requests, 0)} requests</span>
            <StatusBadge ok={group.successful_requests} total={group.total_requests} />
          </div>
          <span className="text-slate-500 text-xs ml-2">{expanded ? '▲ collapse' : '▼ expand'}</span>
        </div>
      </button>

      {expanded && (
        <div className="space-y-4">
          {group.steps.map(step => (
            <StepCard key={step.name} step={step} />
          ))}
        </div>
      )}
    </div>
  )
}

// ── Top-level summary bar ─────────────────────────────────────────────────────

function SummaryBanner({ report }: { report: Report }) {
  const successRate = report.total_requests === 0 ? 100 :
    (report.successful_requests / report.total_requests) * 100
  const bgColor = successRate === 100 ? 'from-emerald-900/40' : successRate >= 95 ? 'from-yellow-900/40' : 'from-red-900/40'

  return (
    <div className={`bg-gradient-to-r ${bgColor} to-slate-800/50 border border-slate-700 rounded-xl p-5 mb-8`}>
      <div className="grid grid-cols-2 sm:grid-cols-5 gap-4">
        {[
          { label: 'Scenarios', value: report.group_count, icon: null },
          { label: 'Total Requests', value: fmt(report.total_requests, 0), icon: null },
          { label: 'Successful', value: fmt(report.successful_requests, 0), icon: <CheckCircle size={14} className="text-emerald-400" /> },
          { label: 'Failed', value: fmt(report.failed_requests, 0), icon: <XCircle size={14} className="text-red-400" /> },
          { label: 'Success Rate', value: <StatusBadge ok={report.successful_requests} total={report.total_requests} />, icon: null },
        ].map(({ label, value, icon }) => (
          <div key={label}>
            <div className="flex items-center gap-1.5 text-xs text-slate-400 mb-1">{icon}{label}</div>
            <div className="text-xl font-bold text-white">{value}</div>
          </div>
        ))}
      </div>
    </div>
  )
}

// ── Main viewer ───────────────────────────────────────────────────────────────

export default function ReportViewer() {
  const [report, setReport] = useState<Report | null>(null)
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    fetch('/api/report')
      .then(r => { if (!r.ok) throw new Error(r.statusText); return r.json() })
      .then(setReport)
      .catch(e => setError(e.message))
      .finally(() => setLoading(false))
  }, [])

  return (
    <div className="min-h-screen bg-slate-900 text-white">
      {/* Header */}
      <header className="bg-slate-800 border-b border-slate-700 px-6 py-4 sticky top-0 z-10 flex items-center gap-4">
        <div className="w-8 h-8 bg-indigo-600 rounded-lg flex items-center justify-center font-bold text-sm">B</div>
        <div>
          <h1 className="font-bold text-white leading-none">bench</h1>
          <p className="text-xs text-slate-400 leading-none mt-0.5">Report Viewer</p>
        </div>
        {report && (
          <span className="ml-4 text-xs text-slate-500">Generated {report.generated_at}</span>
        )}
      </header>

      <main className="max-w-5xl mx-auto px-6 py-8">
        {loading && (
          <div className="flex items-center justify-center py-24">
            <Loader2 size={32} className="animate-spin text-indigo-400" />
            <span className="ml-3 text-slate-400">Loading report…</span>
          </div>
        )}

        {!loading && error && (
          <div className="flex flex-col items-center py-24 gap-4">
            <AlertCircle size={48} className="text-red-400" />
            <p className="text-red-400 font-semibold text-lg">{error}</p>
            <p className="text-slate-500 text-sm">Make sure the report file exists and is valid JSON.</p>
          </div>
        )}

        {report && (
          <>
            <SummaryBanner report={report} />
            <div className="space-y-10">
              {report.groups.map(group => (
                <GroupCard key={group.name} group={group} />
              ))}
            </div>
          </>
        )}
      </main>
    </div>
  )
}
