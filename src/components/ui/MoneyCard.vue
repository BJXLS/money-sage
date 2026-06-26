<script setup lang="ts">
defineProps<{
  title?: string
  headerAction?: string
  noPadding?: boolean
}>()

const emit = defineEmits<{
  action: []
}>()
</script>

<template>
  <div class="ms-card">
    <div v-if="title || $slots.header" class="ms-card-header">
      <div class="ms-card-title">
        <slot name="header">
          <h3>{{ title }}</h3>
        </slot>
      </div>
      <button
        v-if="headerAction"
        class="ms-card-action"
        @click="emit('action')"
      >
        {{ headerAction }}
      </button>
    </div>
    <div :class="['ms-card-body', { 'no-padding': noPadding }]">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.ms-card {
  background-color: var(--ms-surface-primary);
  border: 1px solid var(--ms-border-subtle);
  border-radius: var(--ms-radius-xl);
  box-shadow: var(--ms-shadow-sm);
  transition: box-shadow 0.2s ease;
}

.ms-card:hover {
  box-shadow: var(--ms-shadow-md);
}

.ms-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 16px 20px;
  border-bottom: 1px solid var(--ms-border-subtle);
}

.ms-card-title h3 {
  margin: 0;
  font-size: var(--ms-text-base);
  font-weight: var(--ms-font-semibold);
  color: var(--ms-text-primary);
}

.ms-card-action {
  font-size: var(--ms-text-sm);
  font-weight: var(--ms-font-medium);
  color: var(--ms-primary-500);
  background: none;
  border: none;
  cursor: pointer;
  padding: 0;
}

.ms-card-action:hover {
  color: var(--ms-primary-600);
}

.ms-card-body {
  padding: 20px;
}

.ms-card-body.no-padding {
  padding: 0;
}
</style>
