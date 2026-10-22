// Tabs, select, tooltip, dialog and slider wrappers (shadcn/ui style, Radix primitives).
import { Check, ChevronDown, X } from 'lucide-react'
import { Dialog as DialogPrimitive, Select as SelectPrimitive, Slider as SliderPrimitive, Tabs as TabsPrimitive, Tooltip as TooltipPrimitive } from 'radix-ui'
import * as React from 'react'

import { cn } from '@/lib/utils'

// ---------------------------------------------------------------- tabs
export const Tabs = TabsPrimitive.Root

export function TabsList({ className, ...props }: React.ComponentProps<typeof TabsPrimitive.List>) {
  return <TabsPrimitive.List className={cn('inline-flex h-8 items-center gap-0.5 rounded-md border border-border bg-surface p-0.5', className)} {...props} />
}

export function TabsTrigger({ className, ...props }: React.ComponentProps<typeof TabsPrimitive.Trigger>) {
  return (
    <TabsPrimitive.Trigger
      className={cn(
        'inline-flex h-6 items-center gap-1.5 rounded px-2.5 text-xs font-medium text-muted-foreground transition-colors',
        'hover:text-foreground data-[state=active]:bg-surface-2 data-[state=active]:text-foreground data-[state=active]:shadow-sm [&_svg]:size-3.5',
        className,
      )}
      {...props}
    />
  )
}

export function TabsContent({ className, ...props }: React.ComponentProps<typeof TabsPrimitive.Content>) {
  return <TabsPrimitive.Content className={cn('mt-3 outline-none', className)} {...props} />
}

// ---------------------------------------------------------------- select
export interface SelectOption {
  value: string
  label: string
}

export function Select({
  value,
  onValueChange,
  options,
  className,
  ariaLabel,
  disabled,
}: {
  value: string
  onValueChange: (v: string) => void
  options: SelectOption[]
  className?: string
  ariaLabel?: string
  disabled?: boolean
}) {
  return (
    <SelectPrimitive.Root value={value} onValueChange={onValueChange} disabled={disabled}>
      <SelectPrimitive.Trigger
        aria-label={ariaLabel}
        className={cn(
          'flex h-7 w-full items-center justify-between gap-2 rounded-md border border-input bg-background px-2 text-[13px] outline-none',
          'focus-visible:border-ring focus-visible:ring-1 focus-visible:ring-ring disabled:opacity-50',
          className,
        )}
      >
        <SelectPrimitive.Value />
        <SelectPrimitive.Icon>
          <ChevronDown className="size-3.5 opacity-60" />
        </SelectPrimitive.Icon>
      </SelectPrimitive.Trigger>
      <SelectPrimitive.Portal>
        <SelectPrimitive.Content position="popper" sideOffset={4} className="z-50 max-h-80 min-w-[var(--radix-select-trigger-width)] overflow-hidden rounded-md border border-border bg-surface shadow-lg">
          <SelectPrimitive.Viewport className="p-1">
            {options.map((o) => (
              <SelectPrimitive.Item
                key={o.value}
                value={o.value}
                className="relative flex cursor-default items-center rounded px-2 py-1.5 pr-7 text-[13px] outline-none select-none data-[highlighted]:bg-accent"
              >
                <SelectPrimitive.ItemText>{o.label}</SelectPrimitive.ItemText>
                <SelectPrimitive.ItemIndicator className="absolute right-2">
                  <Check className="size-3.5" />
                </SelectPrimitive.ItemIndicator>
              </SelectPrimitive.Item>
            ))}
          </SelectPrimitive.Viewport>
        </SelectPrimitive.Content>
      </SelectPrimitive.Portal>
    </SelectPrimitive.Root>
  )
}

// ---------------------------------------------------------------- tooltip
export const TooltipProvider = TooltipPrimitive.Provider

export function Tooltip({ content, children, side = 'top' }: { content: React.ReactNode; children: React.ReactNode; side?: 'top' | 'bottom' | 'left' | 'right' }) {
  return (
    <TooltipPrimitive.Root delayDuration={250}>
      <TooltipPrimitive.Trigger asChild>{children}</TooltipPrimitive.Trigger>
      <TooltipPrimitive.Portal>
        <TooltipPrimitive.Content
          side={side}
          sideOffset={5}
          className="z-50 max-w-xs rounded-md border border-border bg-surface-2 px-2.5 py-1.5 text-xs leading-snug text-foreground shadow-lg"
        >
          {content}
        </TooltipPrimitive.Content>
      </TooltipPrimitive.Portal>
    </TooltipPrimitive.Root>
  )
}

// ---------------------------------------------------------------- dialog
export function Dialog({
  open,
  onOpenChange,
  title,
  description,
  children,
  className,
}: {
  open: boolean
  onOpenChange: (open: boolean) => void
  title: string
  description?: string
  children: React.ReactNode
  className?: string
}) {
  return (
    <DialogPrimitive.Root open={open} onOpenChange={onOpenChange}>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay className="fixed inset-0 z-40 bg-black/50" />
        <DialogPrimitive.Content
          className={cn(
            'fixed top-1/2 left-1/2 z-50 max-h-[85vh] w-[min(640px,calc(100vw-32px))] -translate-x-1/2 -translate-y-1/2 overflow-auto rounded-lg border border-border bg-surface p-5 shadow-2xl',
            className,
          )}
        >
          <DialogPrimitive.Title className="text-sm font-semibold">{title}</DialogPrimitive.Title>
          {description ? <DialogPrimitive.Description className="mt-1 text-xs text-muted-foreground">{description}</DialogPrimitive.Description> : null}
          <div className="mt-4">{children}</div>
          <DialogPrimitive.Close className="absolute top-3 right-3 rounded p-1 text-muted-foreground hover:bg-accent" aria-label="Close">
            <X className="size-4" />
          </DialogPrimitive.Close>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  )
}

// ---------------------------------------------------------------- slider
export function Slider({ className, ...props }: React.ComponentProps<typeof SliderPrimitive.Root>) {
  return (
    <SliderPrimitive.Root className={cn('relative flex h-4 w-full touch-none items-center select-none', className)} {...props}>
      <SliderPrimitive.Track className="relative h-1 grow overflow-hidden rounded-full bg-border-strong">
        <SliderPrimitive.Range className="absolute h-full bg-primary" />
      </SliderPrimitive.Track>
      <SliderPrimitive.Thumb aria-label={props['aria-label']} className="block size-3.5 rounded-full border-2 border-primary bg-surface focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none" />
    </SliderPrimitive.Root>
  )
}
