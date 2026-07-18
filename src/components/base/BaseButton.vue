<script setup>
import { computed, useSlots, Comment, Text } from 'vue'
import BaseIcon from './BaseIcon.vue'

// Themeable button, the single home for the button styling that was copy-pasted
// across ~20 scoped <style> blocks (`.btn`, `.btn.primary`, `.btn.danger`,
// `.icon-btn`). Consolidating it here lets those duplicated blocks be deleted as
// call sites migrate. The four variants below cover the dominant classes; genuinely
// bespoke buttons (toolbar run buttons, tab buttons, …) intentionally stay as-is.
//
// Single root element, so native attrs (`title`, `type`, `@click`, `aria-*`) and any
// extra `class` fall through to the <button> automatically — no v-bind needed. The
// declared props below are consumed here and therefore never leak onto the element.
const props = defineProps({
  // Visual role. 'default' is the neutral filled button; 'primary' is the accent
  // confirm action; 'danger' is destructive; 'ghost' is transparent-until-hover for
  // toolbars (icon+label, no fill at rest).
  variant: { type: String, default: 'default' },
  // Density: 'md' (28px form control) or 'sm' (compact inline / pills).
  size: { type: String, default: 'md' },
  // Outlined chrome for the neutral button: 1px border + inset background, as used
  // by the admin/tool modals. Orthogonal to `variant` — a bordered modal's confirm
  // button is still `variant="primary"` (the accent fill already reads correctly).
  bordered: { type: Boolean, default: false },
  // Toggled/pressed state (e.g. a panel-open toggle): highlights the neutral button
  // with the hover fill + an accent edge. Meant for the neutral/bordered look.
  active: { type: Boolean, default: false },
  // BaseIcon name rendered before the label. Rendered only when a name is given;
  // with no label content it becomes a square icon-only button (the old `.icon-btn`).
  icon: { type: String, default: '' },
  // Override the glyph size (px). Defaults to a size that fits the button height;
  // set it when a call site needs a specific icon size (e.g. a denser toolbar).
  iconSize: { type: Number, default: 0 },
  disabled: { type: Boolean, default: false },
})

const slots = useSlots()

// True when the default slot carries real content (text or an element) — comments
// and whitespace-only text don't count. An icon with no label = icon-only chrome.
const hasLabel = computed(() => {
  const nodes = slots.default ? slots.default() : []
  return nodes.some((node) => {
    if (node.type === Comment) {
      return false
    }
    if (node.type === Text) {
      return String(node.children).trim() !== ''
    }
    return true
  })
})
const isIconOnly = computed(() => Boolean(props.icon) && !hasLabel.value)

// Icons read a touch smaller than BaseIcon's 18px default so they sit inside the
// 28/24px button heights without crowding the label. A call site can override.
const resolvedIconSize = computed(() => props.iconSize || (props.size === 'sm' ? 14 : 16))
</script>

<template>
  <button
    class="base-btn"
    :class="[`v-${variant}`, `s-${size}`, { 'icon-only': isIconOnly, bordered: bordered, active: active }]"
    :disabled="disabled"
  >
    <BaseIcon v-if="icon" :name="icon" :size="resolvedIconSize" />
    <slot />
  </button>
</template>

<style scoped>
.base-btn {
  height: 28px;
  padding: 0 14px;
  border-radius: 5px;
  border: none;
  font-size: 13px;
  cursor: pointer;
  background: var(--bg-toolbar);
  color: var(--text);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
}
.base-btn:hover:not(:disabled) { background: var(--bg-hover); }
.base-btn:disabled { opacity: .5; cursor: default; }

.base-btn.v-primary { background: var(--accent); color: #fff; }
.base-btn.v-primary:hover:not(:disabled) { opacity: .88; }
.base-btn.v-danger { background: var(--danger); color: #fff; }
.base-btn.v-danger:hover:not(:disabled) { background: var(--danger-hover); }

/* Ghost — transparent at rest, fills on hover. For toolbar rows (icon + label). */
.base-btn.v-ghost { background: transparent; }
.base-btn.v-ghost:hover:not(:disabled) { background: var(--bg-hover); }
.base-btn.v-ghost:disabled { color: var(--text-faint); }
.base-btn.v-ghost.active { background: var(--bg-hover); }

.base-btn.s-sm { height: 24px; padding: 0 11px; font-size: 12px; }

/* Toolbar (ghost) buttons keep the original toolbars' compact, padding-driven
   metrics — one density regardless of `size`. Declared after .s-sm so it wins. */
.base-btn.v-ghost { height: auto; padding: 4px 9px; font-size: 12.5px; border-radius: 6px; }

/* Outlined neutral button used by the admin/tool modals (bg-input + 1px border).
   Only alters the neutral look; primary/danger keep their fill and just take a
   matching-colour border so a bordered+coloured combo never shows a stray edge. */
.base-btn.bordered {
  background: var(--bg-input);
  border: 1px solid var(--border-soft);
  border-radius: 6px;
}
.base-btn.bordered:hover:not(:disabled) { background: var(--bg-hover); }
.base-btn.bordered.v-primary { border-color: var(--accent); }
.base-btn.bordered.v-danger { border-color: var(--danger); }

/* Toggled state — declared after .bordered so its fill wins the specificity tie. */
.base-btn.active { background: var(--bg-hover); }
.base-btn.active.bordered { border-color: var(--accent); }

/* Icon-only: transparent chrome that lights up on hover, matching the old .icon-btn. */
.base-btn.icon-only {
  height: auto;
  padding: 5px;
  gap: 0;
  background: none;
  border: 1px solid transparent;
  border-radius: 6px;
  color: var(--text-dim);
  display: grid;
  place-items: center;
}
.base-btn.icon-only:hover:not(:disabled) { background: var(--bg-hover); color: var(--text); }
.base-btn.icon-only.s-sm { padding: 4px; }
/* Icon-only colours the glyph rather than filling: danger reddens on hover, active
   accents the glyph + edge (e.g. a toggled read-only lock). */
.base-btn.icon-only.v-danger:hover:not(:disabled) { color: var(--danger-text); }
.base-btn.icon-only.active { background: var(--bg-hover); border-color: var(--accent); color: var(--accent); }
</style>
