import * as React from 'react'

import { cn } from '@/lib/utils'

export function Table({ className, ...props }: React.ComponentProps<'table'>) {
  return (
    <div className="w-full overflow-x-auto">
      <table className={cn('w-full caption-bottom border-collapse text-[12px] tabular', className)} {...props} />
    </div>
  )
}

export function THead(props: React.ComponentProps<'thead'>) {
  return <thead {...props} />
}

export function TBody(props: React.ComponentProps<'tbody'>) {
  return <tbody {...props} />
}

export function Tr({ className, ...props }: React.ComponentProps<'tr'>) {
  return <tr className={cn('border-b border-border last:border-0 hover:bg-accent/40', className)} {...props} />
}

export function Th({ className, ...props }: React.ComponentProps<'th'>) {
  return <th className={cn('h-7 px-2 text-left text-[11px] font-medium tracking-wide text-muted-foreground uppercase', className)} {...props} />
}

export function Td({ className, ...props }: React.ComponentProps<'td'>) {
  return <td className={cn('px-2 py-1.5 align-top', className)} {...props} />
}
