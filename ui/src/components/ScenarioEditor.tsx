import { Trash2, GripVertical, Plus, ChevronDown, ChevronUp } from 'lucide-react'
import { useState } from 'react'
import { useSortable, SortableContext, verticalListSortingStrategy, arrayMove } from '@dnd-kit/sortable'
import { DndContext, closestCenter } from '@dnd-kit/core'
import type { DragEndEvent } from '@dnd-kit/core'
import { CSS } from '@dnd-kit/utilities'
import type { ScenarioRef, RunParams } from '../types'

const inp = "w-full bg-slate-700 border border-slate-600 rounded-lg px-3 py-2 text-sm text-white placeholder-slate-500 focus:outline-none focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500"

interface StepItemProps { id: string; onRemove: () => void }
function StepItem({ id, onRemove }: StepItemProps) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } =
    useSortable({ id })
  return (
    <div ref={setNodeRef}
      style={{ transform: CSS.Transform.toString(transform), transition, opacity: isDragging ? 0.5 : 1 }}
      className="flex items-center gap-2 bg-slate-700 border border-slate-600 rounded-lg px-3 py-2">
      <button {...attributes} {...listeners} className="text-slate-500 hover:text-slate-300 cursor-grab active:cursor-grabbing">
        <GripVertical size={14} />
      </button>
      <span className="flex-1 text-sm font-mono text-indigo-300">{id}</span>
      <button onClick={onRemove} className="text-slate-500 hover:text-red-400 transition-colors">
        <Trash2 size={14} />
      </button>
    </div>
  )
}

interface RunOverrideProps { run?: RunParams; onChange: (r: RunParams | undefined) => void }
function RunOverride({ run, onChange }: RunOverrideProps) {
  const [open, setOpen] = useState(!!run)
  const set = (k: keyof RunParams, v: string | number | undefined) =>
    onChange({ ...(run ?? {}), [k]: v === '' ? undefined : v })

  return (
    <div>
      <button onClick={() => { setOpen(o => !o); if (open) onChange(undefined) }}
        className="flex items-center gap-2 text-xs font-semibold text-slate-400 hover:text-slate-200 uppercase tracking-wider transition-colors">
        {open ? <ChevronUp size={13} /> : <ChevronDown size={13} />}
        Run Override <span className="font-normal normal-case text-slate-500">(overrides global for this scenario)</span>
      </button>
      {open && (
        <div className="mt-3 grid grid-cols-2 gap-3 pl-4 border-l-2 border-indigo-800">
          {([
            ['concurrency', 'Concurrency', 1],
            ['requests', 'Total Runs', 1],
            ['duration_secs', 'Duration (s)', 1],
            ['timeout_ms', 'Timeout (ms)', 100],
          ] as [keyof RunParams, string, number][]).map(([k, label, min]) => (
            <div key={k}>
              <label className="block text-xs text-slate-400 mb-1">{label}</label>
              <input type="number" min={min} className={inp} value={(run?.[k] as number) ?? ''}
                placeholder="inherit"
                onChange={e => set(k, e.target.value ? +e.target.value : undefined)} />
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

interface Props {
  scenario: ScenarioRef
  availableRequests: string[]
  onChange: (s: ScenarioRef) => void
  onDelete: () => void
}

export default function ScenarioEditor({ scenario, availableRequests, onChange, onDelete }: Props) {
  const unusedRequests = availableRequests.filter(r => !scenario.steps.includes(r))

  const handleDragEnd = (e: DragEndEvent) => {
    const { active, over } = e
    if (over && active.id !== over.id) {
      const from = scenario.steps.indexOf(active.id as string)
      const to = scenario.steps.indexOf(over.id as string)
      onChange({ ...scenario, steps: arrayMove(scenario.steps, from, to) })
    }
  }

  return (
    <div className="bg-slate-800 border border-slate-700 rounded-xl p-5 space-y-5">
      {/* Name + delete */}
      <div className="flex items-center gap-3">
        <div className="flex-1">
          <label className="block text-xs font-semibold text-slate-400 uppercase tracking-wider mb-1">Scenario Name</label>
          <input type="text" className={inp} value={scenario.name}
            placeholder="e.g. Steady State"
            onChange={e => onChange({ ...scenario, name: e.target.value })} />
        </div>
        <button onClick={onDelete}
          className="mt-5 p-2 text-slate-500 hover:text-red-400 hover:bg-slate-700 rounded-lg transition-colors"
          title="Delete scenario">
          <Trash2 size={16} />
        </button>
      </div>

      {/* Run override */}
      <RunOverride run={scenario.run} onChange={r => onChange({ ...scenario, run: r })} />

      {/* Steps */}
      <div>
        <label className="block text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2">
          Steps <span className="font-normal normal-case text-slate-500">— ordered list of request references</span>
        </label>

        {scenario.steps.length === 0
          ? <p className="text-slate-600 text-xs italic mb-2">No steps yet — add requests below</p>
          : (
            <DndContext collisionDetection={closestCenter} onDragEnd={handleDragEnd}>
              <SortableContext items={scenario.steps} strategy={verticalListSortingStrategy}>
                <div className="space-y-2 mb-3">
                  {scenario.steps.map(s => (
                    <StepItem key={s} id={s}
                      onRemove={() => onChange({ ...scenario, steps: scenario.steps.filter(x => x !== s) })} />
                  ))}
                </div>
              </SortableContext>
            </DndContext>
          )
        }

        {unusedRequests.length > 0 && (
          <div>
            <p className="text-xs text-slate-500 mb-2">Add request to steps:</p>
            <div className="flex flex-wrap gap-2">
              {unusedRequests.map(r => (
                <button key={r}
                  onClick={() => onChange({ ...scenario, steps: [...scenario.steps, r] })}
                  className="flex items-center gap-1 text-xs bg-slate-700 hover:bg-indigo-700 border border-slate-600 hover:border-indigo-500 text-slate-300 hover:text-white rounded-lg px-3 py-1.5 transition-colors">
                  <Plus size={11} /> {r}
                </button>
              ))}
            </div>
          </div>
        )}

        {unusedRequests.length === 0 && scenario.steps.length > 0 && availableRequests.length > 0 && (
          <p className="text-xs text-slate-600 italic">All requests are in steps. You can reorder by dragging.</p>
        )}

        {availableRequests.length === 0 && (
          <p className="text-xs text-slate-600 italic">Define requests in the Requests tab first.</p>
        )}
      </div>
    </div>
  )
}
