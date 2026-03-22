import { useEffect, useState, useCallback } from 'react'
import { Save, Settings, Radio, Play, AlertCircle, CheckCircle, Loader2, Plus } from 'lucide-react'
import type { RequestDef } from './types'
import { emptyRequest, emptyScenario, emptyFile } from './types'
import type { ScenarioFile } from './types'
import { fetchScenario, saveScenario } from './api'
import GlobalConfig from './components/GlobalConfig'
import RequestEditor from './components/RequestEditor'
import ScenarioEditor from './components/ScenarioEditor'

type Tab = 'global' | 'requests' | 'scenarios'
type SaveState = 'idle' | 'saving' | 'saved' | 'error'

export default function App() {
  const [file, setFile] = useState<ScenarioFile>(emptyFile())
  const [tab, setTab] = useState<Tab>('global')
  const [saveState, setSaveState] = useState<SaveState>('idle')
  const [saveError, setSaveError] = useState('')
  const [loading, setLoading] = useState(true)
  const [loadError, setLoadError] = useState('')
  const [reqNameErrors, setReqNameErrors] = useState<Record<number, string>>({})

  useEffect(() => {
    fetchScenario()
      .then(setFile)
      .catch(e => setLoadError(e.message))
      .finally(() => setLoading(false))
  }, [])

  const requestEntries = Object.entries(file.requests)

  const validateRequestNames = useCallback((entries: [string, RequestDef][]) => {
    const errors: Record<number, string> = {}
    const seen = new Set<string>()
    entries.forEach(([name], i) => {
      if (!name.trim()) errors[i] = 'Name required'
      else if (seen.has(name)) errors[i] = 'Duplicate name'
      else seen.add(name)
    })
    setReqNameErrors(errors)
    return Object.keys(errors).length === 0
  }, [])

  const updateRequestEntry = (i: number, newName: string, def: RequestDef) => {
    const entries = [...requestEntries]
    entries[i] = [newName, def]
    validateRequestNames(entries)
    setFile(f => ({ ...f, requests: Object.fromEntries(entries) }))
  }

  const addRequest = () => {
    const baseName = 'request'
    let name = baseName
    let n = 1
    while (file.requests[name]) name = `${baseName}${++n}`
    setFile(f => ({ ...f, requests: { ...f.requests, [name]: emptyRequest() } }))
    setTab('requests')
  }

  const deleteRequest = (name: string) => {
    const { [name]: _, ...rest } = file.requests
    // Also remove from scenario steps
    const scenarios = file.scenarios.map(s => ({ ...s, steps: s.steps.filter(x => x !== name) }))
    setFile(f => ({ ...f, requests: rest, scenarios }))
  }

  const addScenario = () => {
    setFile(f => ({ ...f, scenarios: [...f.scenarios, emptyScenario()] }))
    setTab('scenarios')
  }

  const handleSave = async () => {
    if (Object.keys(reqNameErrors).length > 0) {
      setSaveError('Fix request name errors before saving')
      setSaveState('error')
      return
    }
    setSaveState('saving')
    setSaveError('')
    try {
      await saveScenario(file)
      setSaveState('saved')
      setTimeout(() => setSaveState('idle'), 2500)
    } catch (e: any) {
      setSaveError(e.message)
      setSaveState('error')
    }
  }

  const tabs: { id: Tab; label: string; icon: React.ReactNode; count?: number }[] = [
    { id: 'global', label: 'Global Config', icon: <Settings size={16} /> },
    { id: 'requests', label: 'Requests', icon: <Radio size={16} />, count: requestEntries.length },
    { id: 'scenarios', label: 'Scenarios', icon: <Play size={16} />, count: file.scenarios.length },
  ]

  return (
    <div className="min-h-screen bg-slate-900 text-white flex flex-col">
      {/* Top bar */}
      <header className="bg-slate-800 border-b border-slate-700 px-6 py-4 flex items-center gap-4 sticky top-0 z-10">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 bg-indigo-600 rounded-lg flex items-center justify-center text-white font-bold text-sm">B</div>
          <div>
            <h1 className="font-bold text-white leading-none">bench</h1>
            <p className="text-xs text-slate-400 leading-none mt-0.5">Scenario Editor</p>
          </div>
        </div>

        <div className="flex-1" />

        {saveState === 'error' && (
          <div className="flex items-center gap-2 text-red-400 text-sm bg-red-900/30 px-3 py-1.5 rounded-lg border border-red-800">
            <AlertCircle size={14} /> {saveError}
          </div>
        )}
        {saveState === 'saved' && (
          <div className="flex items-center gap-2 text-emerald-400 text-sm bg-emerald-900/30 px-3 py-1.5 rounded-lg border border-emerald-800">
            <CheckCircle size={14} /> Saved successfully
          </div>
        )}

        <button onClick={handleSave} disabled={saveState === 'saving'}
          className="flex items-center gap-2 bg-indigo-600 hover:bg-indigo-500 disabled:bg-slate-700 disabled:text-slate-500 text-white font-semibold px-4 py-2 rounded-lg text-sm transition-colors">
          {saveState === 'saving' ? <Loader2 size={15} className="animate-spin" /> : <Save size={15} />}
          {saveState === 'saving' ? 'Saving…' : 'Save'}
        </button>
      </header>

      {/* Loading / error states */}
      {loading && (
        <div className="flex-1 flex items-center justify-center">
          <Loader2 size={32} className="animate-spin text-indigo-400" />
          <span className="ml-3 text-slate-400">Loading scenario…</span>
        </div>
      )}
      {!loading && loadError && (
        <div className="flex-1 flex flex-col items-center justify-center gap-4">
          <AlertCircle size={48} className="text-red-400" />
          <p className="text-red-400 font-semibold">{loadError}</p>
          <p className="text-slate-500 text-sm">Starting with a blank scenario file.</p>
        </div>
      )}

      {!loading && (
        <div className="flex flex-1">
          {/* Sidebar */}
          <nav className="w-56 bg-slate-800 border-r border-slate-700 p-4 flex flex-col gap-1 sticky top-[65px] self-start h-[calc(100vh-65px)]">
            {tabs.map(t => (
              <button key={t.id} onClick={() => setTab(t.id)}
                className={`flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium text-left transition-colors w-full
                  ${tab === t.id ? 'bg-indigo-600 text-white' : 'text-slate-400 hover:text-white hover:bg-slate-700'}`}>
                {t.icon}
                <span className="flex-1">{t.label}</span>
                {t.count !== undefined && (
                  <span className={`text-xs px-1.5 py-0.5 rounded-full font-mono
                    ${tab === t.id ? 'bg-indigo-500 text-white' : 'bg-slate-700 text-slate-400'}`}>
                    {t.count}
                  </span>
                )}
              </button>
            ))}

            <div className="flex-1" />

            {/* Quick-add buttons */}
            <div className="space-y-1 pt-4 border-t border-slate-700">
              <button onClick={addRequest}
                className="flex items-center gap-2 w-full px-3 py-2 text-xs text-slate-400 hover:text-white hover:bg-slate-700 rounded-lg transition-colors">
                <Plus size={13} /> New Request
              </button>
              <button onClick={addScenario}
                className="flex items-center gap-2 w-full px-3 py-2 text-xs text-slate-400 hover:text-white hover:bg-slate-700 rounded-lg transition-colors">
                <Plus size={13} /> New Scenario
              </button>
            </div>
          </nav>

          {/* Main content */}
          <main className="flex-1 p-8 max-w-3xl">
            {tab === 'global' && (
              <section>
                <h2 className="text-xl font-bold text-white mb-1">Global Run Configuration</h2>
                <p className="text-slate-400 text-sm mb-6">
                  Default settings applied to all scenarios. Each scenario can override any field.
                </p>
                <GlobalConfig run={file.run ?? {}} onChange={run => setFile(f => ({ ...f, run }))} />
              </section>
            )}

            {tab === 'requests' && (
              <section>
                <div className="flex items-center justify-between mb-1">
                  <h2 className="text-xl font-bold text-white">Request Library</h2>
                  <button onClick={addRequest}
                    className="flex items-center gap-2 text-sm bg-indigo-600 hover:bg-indigo-500 text-white font-semibold px-3 py-1.5 rounded-lg transition-colors">
                    <Plus size={14} /> New Request
                  </button>
                </div>
                <p className="text-slate-400 text-sm mb-6">
                  Define HTTP requests once here. Reference them by name in scenario steps.
                </p>
                {requestEntries.length === 0
                  ? (
                    <div className="text-center py-16 border-2 border-dashed border-slate-700 rounded-xl">
                      <Radio size={36} className="mx-auto text-slate-600 mb-3" />
                      <p className="text-slate-500 font-medium">No requests yet</p>
                      <p className="text-slate-600 text-sm mt-1">Click "New Request" to define your first HTTP request</p>
                    </div>
                  ) : (
                    <div className="space-y-4">
                      {requestEntries.map(([name, def], i) => (
                        <RequestEditor key={i} name={name} def={def}
                          nameError={reqNameErrors[i]}
                          onNameChange={n => updateRequestEntry(i, n, def)}
                          onChange={d => updateRequestEntry(i, name, d)}
                          onDelete={() => deleteRequest(name)} />
                      ))}
                    </div>
                  )
                }
              </section>
            )}

            {tab === 'scenarios' && (
              <section>
                <div className="flex items-center justify-between mb-1">
                  <h2 className="text-xl font-bold text-white">Scenarios</h2>
                  <button onClick={addScenario}
                    className="flex items-center gap-2 text-sm bg-indigo-600 hover:bg-indigo-500 text-white font-semibold px-3 py-1.5 rounded-lg transition-colors">
                    <Plus size={14} /> New Scenario
                  </button>
                </div>
                <p className="text-slate-400 text-sm mb-6">
                  Scenarios run sequentially. Each references requests from the library as ordered steps.
                </p>
                {file.scenarios.length === 0
                  ? (
                    <div className="text-center py-16 border-2 border-dashed border-slate-700 rounded-xl">
                      <Play size={36} className="mx-auto text-slate-600 mb-3" />
                      <p className="text-slate-500 font-medium">No scenarios yet</p>
                      <p className="text-slate-600 text-sm mt-1">Click "New Scenario" to create a benchmark scenario</p>
                    </div>
                  ) : (
                    <div className="space-y-4">
                      {file.scenarios.map((s, i) => (
                        <ScenarioEditor key={i} scenario={s}
                          availableRequests={Object.keys(file.requests)}
                          onChange={updated => {
                            const scenarios = [...file.scenarios]
                            scenarios[i] = updated
                            setFile(f => ({ ...f, scenarios }))
                          }}
                          onDelete={() => setFile(f => ({
                            ...f,
                            scenarios: f.scenarios.filter((_, j) => j !== i)
                          }))} />
                      ))}
                    </div>
                  )
                }
              </section>
            )}
          </main>
        </div>
      )}
    </div>
  )
}
