import { type ClassValue, clsx } from 'clsx'
import { twMerge } from 'tailwind-merge'

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

/** Immutably set a value at a dotted path such as `airship.aero.cd_axial`. */
export function setIn<T>(obj: T, path: string, value: unknown): T {
  const keys = path.split('.')
  const clone = (o: unknown): Record<string, unknown> => (Array.isArray(o) ? ([...o] as unknown as Record<string, unknown>) : { ...(o as Record<string, unknown>) })
  const root = clone(obj)
  let cur = root
  for (let i = 0; i < keys.length - 1; i++) {
    const k = keys[i] as string
    cur[k] = clone(cur[k])
    cur = cur[k] as Record<string, unknown>
  }
  cur[keys[keys.length - 1] as string] = value
  return root as T
}

/** Read a value at a dotted path. */
export function getIn(obj: unknown, path: string): unknown {
  return path.split('.').reduce<unknown>((o, k) => (o == null ? undefined : (o as Record<string, unknown>)[k]), obj)
}

export function downloadBlob(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  document.body.appendChild(a)
  a.click()
  a.remove()
  setTimeout(() => URL.revokeObjectURL(url), 1000)
}

export function slug(s: string): string {
  return s.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || 'experiment'
}
