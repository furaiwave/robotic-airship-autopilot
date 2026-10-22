// Small form and display primitives in the shadcn/ui style.
import { cva, type VariantProps } from 'class-variance-authority'
import { Label as LabelPrimitive, Separator as SeparatorPrimitive, Switch as SwitchPrimitive } from 'radix-ui'
import * as React from 'react'

import { cn } from '@/lib/utils'

export function Input({ className, ...props }: React.ComponentProps<'input'>) {
  return (
    <input
      data-slot="input"
      className={cn(
        'h-7 w-full min-w-0 rounded-md border border-input bg-background px-2 text-[13px] tabular outline-none transition-colors',
        'placeholder:text-subtle-foreground focus-visible:border-ring focus-visible:ring-1 focus-visible:ring-ring',
        'aria-invalid:border-critical aria-invalid:ring-critical/40 disabled:opacity-50',
        className,
      )}
      {...props}
    />
  )
}

export function Label({ className, ...props }: React.ComponentProps<typeof LabelPrimitive.Root>) {
  return <LabelPrimitive.Root className={cn('text-xs font-medium text-muted-foreground select-none', className)} {...props} />
}

export function Separator({ className, orientation = 'horizontal', ...props }: React.ComponentProps<typeof SeparatorPrimitive.Root>) {
  return (
    <SeparatorPrimitive.Root
      orientation={orientation}
      className={cn('shrink-0 bg-border', orientation === 'horizontal' ? 'h-px w-full' : 'h-full w-px', className)}
      {...props}
    />
  )
}

export function Switch({ className, ...props }: React.ComponentProps<typeof SwitchPrimitive.Root>) {
  return (
    <SwitchPrimitive.Root
      className={cn(
        'peer inline-flex h-4 w-7 shrink-0 cursor-pointer items-center rounded-full border border-transparent transition-colors',
        'data-[state=checked]:bg-primary data-[state=unchecked]:bg-border-strong focus-visible:ring-2 focus-visible:ring-ring',
        className,
      )}
      {...props}
    >
      <SwitchPrimitive.Thumb className="pointer-events-none block size-3 rounded-full bg-white shadow transition-transform data-[state=checked]:translate-x-3.5 data-[state=unchecked]:translate-x-0.5" />
    </SwitchPrimitive.Root>
  )
}

const badgeVariants = cva('inline-flex items-center gap-1 rounded px-1.5 py-0.5 text-[11px] font-medium whitespace-nowrap [&_svg]:size-3', {
  variants: {
    variant: {
      default: 'bg-surface-2 text-foreground border border-border',
      outline: 'border border-border-strong text-muted-foreground',
      good: 'bg-good/15 text-success-text border border-good/30',
      warning: 'bg-warning/15 text-foreground border border-warning/40',
      serious: 'bg-serious/15 text-foreground border border-serious/40',
      critical: 'bg-critical/15 text-foreground border border-critical/50',
      info: 'bg-primary/15 text-foreground border border-primary/40',
    },
  },
  defaultVariants: { variant: 'default' },
})

export function Badge({ className, variant, ...props }: React.ComponentProps<'span'> & VariantProps<typeof badgeVariants>) {
  return <span className={cn(badgeVariants({ variant }), className)} {...props} />
}
