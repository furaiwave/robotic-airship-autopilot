import { zodResolver } from '@hookform/resolvers/zod'
import { Info } from 'lucide-react'
import { useEffect, useRef } from 'react'
import { type FieldValues, useForm } from 'react-hook-form'
import type { z } from 'zod'

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Tooltip } from '@/components/ui/overlays'
import { Badge, Input, Label } from '@/components/ui/primitives'
import type { FieldGroup, FieldSpec } from '@/lib/schemas'
import { getIn } from '@/lib/utils'
import type { ValidationIssue } from '@/types/api'

interface SectionFormProps<T extends object> {
  value: T
  groups: FieldGroup[]
  schema: z.ZodType
  /** Called with the full section object whenever the form is valid after a change. */
  onChange: (value: T) => void
  /** Prefix of server-side validation field paths, e.g. `airship`. */
  issuePrefix: string
  serverIssues?: ValidationIssue[]
  columns?: 1 | 2 | 3
}

/** Number-only configuration form: React Hook Form state, Zod validation, store sync. */
export function SectionForm<T extends object>({ value, groups, schema, onChange, issuePrefix, serverIssues = [], columns = 2 }: SectionFormProps<T>) {
  const form = useForm<FieldValues>({
    resolver: zodResolver(schema as never),
    mode: 'onChange',
    defaultValues: value as FieldValues,
  })
  const lastEmitted = useRef<string>(JSON.stringify(value))

  // External changes (preset loaded, reset) replace the form state.
  useEffect(() => {
    const json = JSON.stringify(value)
    if (json !== lastEmitted.current) {
      lastEmitted.current = json
      form.reset(value as FieldValues)
    }
  }, [value, form])

  useEffect(() => {
    const sub = form.watch((values) => {
      const parsed = schema.safeParse(values)
      if (parsed.success) {
        const next = parsed.data as T
        lastEmitted.current = JSON.stringify(next)
        onChange(next)
      }
    })
    return () => sub.unsubscribe()
  }, [form, schema, onChange])

  const grid = columns === 1 ? 'grid-cols-1' : columns === 3 ? 'sm:grid-cols-2 xl:grid-cols-3' : 'sm:grid-cols-2'

  return (
    <div className="space-y-3">
      {groups.map((g) => (
        <Card key={g.title}>
          <CardHeader>
            <div>
              <CardTitle>{g.title}</CardTitle>
              {g.description ? <CardDescription>{g.description}</CardDescription> : null}
            </div>
          </CardHeader>
          <CardContent className={`grid grid-cols-1 gap-x-5 gap-y-2.5 ${grid}`}>
            {g.fields.map((f) => (
              <NumberField
                key={f.name}
                spec={f}
                form={form}
                serverIssue={serverIssues.find((i) => i.field === `${issuePrefix}.${f.name}`)}
              />
            ))}
          </CardContent>
        </Card>
      ))}
    </div>
  )
}

function NumberField({ spec, form, serverIssue }: { spec: FieldSpec; form: ReturnType<typeof useForm<FieldValues>>; serverIssue?: ValidationIssue }) {
  const id = `f-${spec.name.replace(/\./g, '-')}`
  const error = getIn(form.formState.errors, spec.name) as { message?: string } | undefined
  const message = error?.message ?? serverIssue?.message
  const isWarning = !error && serverIssue?.severity === 'warning'
  return (
    <div className="grid grid-cols-[minmax(0,1fr)_7.5rem] items-center gap-x-2 gap-y-0.5">
      <div className="flex min-w-0 items-center gap-1.5">
        <Label htmlFor={id} className="truncate">
          {spec.label}
          {spec.unit ? <span className="ml-1 text-subtle-foreground">[{spec.unit}]</span> : null}
        </Label>
        {spec.kind === 'estimate' ? (
          <Badge variant="outline" className="px-1 py-0 text-[10px]">
            оц.
          </Badge>
        ) : null}
        {spec.help ? (
          <Tooltip content={spec.help}>
            <button type="button" className="text-subtle-foreground hover:text-foreground" aria-label={`Пояснення: ${spec.label}`}>
              <Info className="size-3" />
            </button>
          </Tooltip>
        ) : null}
      </div>
      <Input
        id={id}
        type="number"
        step={spec.step ?? 'any'}
        placeholder={spec.nullable ? 'авто' : undefined}
        aria-invalid={Boolean(error) || (serverIssue?.severity === 'error')}
        {...form.register(spec.name, {
          setValueAs: (v: unknown) => {
            if (v === '' || v === null || v === undefined) return spec.nullable ? null : Number.NaN
            const n = typeof v === 'number' ? v : Number(v)
            return n
          },
        })}
      />
      {message ? <p className={`col-span-2 text-[11px] ${isWarning ? 'text-muted-foreground' : 'text-critical'}`}>{isWarning ? `⚠ ${message}` : message}</p> : null}
    </div>
  )
}
