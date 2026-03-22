import { useEffect, useState } from 'react'
import { Loader2 } from 'lucide-react'
import ReportViewer from './report/ReportViewer'
import EditorApp from './EditorApp'

type AppMode = 'editor' | 'report' | null

export default function App() {
  const [mode, setMode] = useState<AppMode>(null)

  useEffect(() => {
    fetch('/api/mode')
      .then(r => r.json())
      .then((data: { mode: AppMode }) => setMode(data.mode))
      .catch(() => setMode('editor')) // fall back to editor
  }, [])

  if (mode === null) {
    return (
      <div className="min-h-screen bg-slate-900 flex items-center justify-center">
        <Loader2 size={32} className="animate-spin text-indigo-400" />
      </div>
    )
  }

  return mode === 'report' ? <ReportViewer /> : <EditorApp />
}
