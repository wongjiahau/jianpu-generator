import * as Tooltip from '@radix-ui/react-tooltip'
import type { ReactNode } from 'react'
import './EditorToolbarButton.css'

interface EditorToolbarButtonProps {
  /** Also used as the tooltip text, and the `aria-label`. */
  label: string
  icon: ReactNode
  onClick: () => void
  disabled?: boolean
}

/** A single icon button in the editor's `toolbar` slot (see `Editor`'s
 * `toolbar` prop), with a Radix tooltip showing `label` on hover/focus. */
export function EditorToolbarButton({
  label,
  icon,
  onClick,
  disabled = false,
}: EditorToolbarButtonProps) {
  return (
    <Tooltip.Root>
      <Tooltip.Trigger asChild>
        <button
          type="button"
          className="editor-toolbar-button"
          onClick={onClick}
          disabled={disabled}
          aria-label={label}
        >
          {icon}
        </button>
      </Tooltip.Trigger>
      <Tooltip.Portal>
        <Tooltip.Content
          className="editor-toolbar-tooltip-content"
          sideOffset={4}
        >
          {label}
        </Tooltip.Content>
      </Tooltip.Portal>
    </Tooltip.Root>
  )
}
