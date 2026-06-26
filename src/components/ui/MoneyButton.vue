<script setup lang="ts">
withDefaults(
  defineProps<{
    variant?: 'primary' | 'secondary' | 'ghost' | 'danger'
    size?: 'sm' | 'md' | 'lg'
    loading?: boolean
    disabled?: boolean
    type?: 'button' | 'submit' | 'reset'
  }>(),
  {
    variant: 'primary',
    size: 'md',
    type: 'button',
  }
)

const emit = defineEmits<{
  click: [event: MouseEvent]
}>()
</script>

<template>
  <button
    :type="type"
    class="ms-button"
    :class="[variant, size]"
    :disabled="disabled || loading"
    @click="emit('click', $event)"
  >
    <span v-if="loading" class="ms-button-loading"></span>
    <slot />
  </button>
</template>

<style scoped>
.ms-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  border-radius: var(--ms-radius-md);
  font-weight: var(--ms-font-semibold);
  cursor: pointer;
  transition: all 0.2s ease;
  border: none;
  white-space: nowrap;
}

.ms-button:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.ms-button.sm {
  padding: 6px 12px;
  font-size: var(--ms-text-xs);
}

.ms-button.md {
  padding: 8px 16px;
  font-size: var(--ms-text-sm);
}

.ms-button.lg {
  padding: 10px 20px;
  font-size: var(--ms-text-base);
}

.ms-button.primary {
  background: var(--ms-gradient-primary);
  color: white;
  box-shadow: 0 4px 12px rgba(99, 102, 241, 0.25);
}

.ms-button.primary:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 6px 16px rgba(99, 102, 241, 0.4);
}

.ms-button.secondary {
  background-color: var(--ms-surface-secondary);
  color: var(--ms-text-primary);
  border: 1px solid var(--ms-border-subtle);
}

.ms-button.secondary:hover:not(:disabled) {
  background-color: var(--ms-surface-hover);
  border-color: var(--ms-border-default);
}

.ms-button.ghost {
  background-color: transparent;
  color: var(--ms-text-secondary);
  border: 1px solid transparent;
}

.ms-button.ghost:hover:not(:disabled) {
  background-color: var(--ms-surface-hover);
  color: var(--ms-text-primary);
}

.ms-button.danger {
  background-color: rgba(244, 63, 94, 0.08);
  color: var(--ms-expense);
  border: 1px solid rgba(244, 63, 94, 0.15);
}

.ms-button.danger:hover:not(:disabled) {
  background-color: rgba(244, 63, 94, 0.12);
}

.ms-button-loading {
  width: 14px;
  height: 14px;
  border: 2px solid currentColor;
  border-right-color: transparent;
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>
