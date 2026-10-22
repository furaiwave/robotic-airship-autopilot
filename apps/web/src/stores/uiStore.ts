import { create } from 'zustand'

export type PageId = 'dashboard' | 'airship' | 'environment' | 'flight-path' | 'autopilot' | 'simulation' | 'results' | 'compare'
export type Theme = 'dark' | 'light'

export const PAGES: PageId[] = ['dashboard', 'airship', 'environment', 'flight-path', 'autopilot', 'simulation', 'results', 'compare']

function readHash(): PageId {
  if (typeof window === 'undefined') return 'dashboard'
  const h = window.location.hash.replace(/^#\/?/, '') as PageId
  return PAGES.includes(h) ? h : 'dashboard'
}

function readTheme(): Theme {
  try {
    return localStorage.getItem('airship.theme') === 'light' ? 'light' : 'dark'
  } catch {
    return 'dark'
  }
}

export function applyTheme(theme: Theme) {
  if (typeof document !== 'undefined') document.documentElement.dataset.theme = theme
}

interface UiState {
  page: PageId
  theme: Theme
  setPage: (p: PageId) => void
  toggleTheme: () => void
}

export const useUiStore = create<UiState>((set, get) => ({
  page: readHash(),
  theme: readTheme(),
  setPage: (page) => {
    if (typeof window !== 'undefined' && window.location.hash !== `#/${page}`) window.location.hash = `/${page}`
    set({ page })
  },
  toggleTheme: () => {
    const theme: Theme = get().theme === 'dark' ? 'light' : 'dark'
    try {
      localStorage.setItem('airship.theme', theme)
    } catch {
      /* storage unavailable: theme still applies for this session */
    }
    applyTheme(theme)
    set({ theme })
  },
}))

if (typeof window !== 'undefined') {
  window.addEventListener('hashchange', () => useUiStore.setState({ page: readHash() }))
}
