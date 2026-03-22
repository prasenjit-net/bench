import { Trash2, Plus } from 'lucide-react'
import type { RequestDef } from '../types'
import { METHODS } from '../types'

interface Props {
  name: string
  def: RequestDef
  onNameChange: (n: string) => void
  onChange: (d: RequestDef) => void
  onDelete: () => void
  nameError?: string
}

const inp = "w-full bg-slate-700 border border-slate-600 rounded-lg px-3 py-2 text-sm text-white placeholder-slate-500 focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500"
const sel = inp + " cursor-pointer"

export default function RequestEditor({ name, def, onNameChange, onChange, onDelete, nameError }: Props) {
  const setHeader = (i: number, key: string, val: string) => {
    const entries = Object.entries(def.headers)
    entries[i] = [key, val]
    onChange({ ...def, headers: Object.fromEntries(entries) })
  }
  const addHeader = () => onChange({ ...def, headers: { ...def.headers, '': '' } })
  const removeHeader = (i: number) => {
    const entries = Object.entries(def.headers)
    entries.splice(i, 1)
    onChange({ ...def, headers: Object.fromEntries(entries) })
  }

  return (
    <div className="bg-slate-800 border border-slate-700 rounded-xl p-5 space-y-4">
      {/* Name + Delete row */}
      <div className="flex items-center gap-3">
        <div className="flex-1">
          <label className="block text-xs font-semibold text-slate-400 uppercase tracking-wider mb-1">Request Name</label>
          <input type="text" className={`${inp}${nameError ? ' border-red-500' : ''}`}
            value={name} placeholder="e.g. hello"
            onChange={e => onNameChange(e.target.value)} />
          {nameError && <p className="text-red-400 text-xs mt-1">{nameError}</p>}
        </div>
        <button onClick={onDelete}
          className="mt-5 p-2 text-slate-500 hover:text-red-400 hover:bg-slate-700 rounded-lg transition-colors"
          title="Delete request">
          <Trash2 size={16} />
        </button>
      </div>

      {/* Method + URL */}
      <div className="flex gap-3">
        <div className="w-32">
          <label className="block text-xs font-semibold text-slate-400 uppercase tracking-wider mb-1">Method</label>
          <select className={sel} value={def.method}
            onChange={e => onChange({ ...def, method: e.target.value })}>
            {METHODS.map(m => <option key={m}>{m}</option>)}
          </select>
        </div>
        <div className="flex-1">
          <label className="block text-xs font-semibold text-slate-400 uppercase tracking-wider mb-1">URL</label>
          <input type="text" className={inp} value={def.url}
            placeholder="https://api.example.com/endpoint"
            onChange={e => onChange({ ...def, url: e.target.value })} />
        </div>
      </div>

      {/* Headers */}
      <div>
        <div className="flex items-center justify-between mb-2">
          <label className="text-xs font-semibold text-slate-400 uppercase tracking-wider">Headers</label>
          <button onClick={addHeader}
            className="flex items-center gap-1 text-xs text-indigo-400 hover:text-indigo-300 transition-colors">
            <Plus size={12} /> Add header
          </button>
        </div>
        <div className="space-y-2">
          {Object.entries(def.headers).map(([k, v], i) => (
            <div key={i} className="flex gap-2 items-center">
              <input type="text" className={`${inp} flex-1`} value={k} placeholder="Header name"
                onChange={e => setHeader(i, e.target.value, v)} />
              <input type="text" className={`${inp} flex-1`} value={v} placeholder="Value"
                onChange={e => setHeader(i, k, e.target.value)} />
              <button onClick={() => removeHeader(i)}
                className="text-slate-500 hover:text-red-400 transition-colors p-1">
                <Trash2 size={14} />
              </button>
            </div>
          ))}
          {Object.keys(def.headers).length === 0 &&
            <p className="text-slate-600 text-xs italic">No headers — click "Add header" to add one</p>}
        </div>
      </div>

      {/* Body */}
      <div>
        <label className="block text-xs font-semibold text-slate-400 uppercase tracking-wider mb-1">Body <span className="font-normal normal-case text-slate-500">(optional)</span></label>
        <textarea rows={3} className={`${inp} font-mono resize-y`}
          value={def.body ?? ''}
          placeholder='{"key": "value"}'
          onChange={e => onChange({ ...def, body: e.target.value || undefined })} />
      </div>
    </div>
  )
}
