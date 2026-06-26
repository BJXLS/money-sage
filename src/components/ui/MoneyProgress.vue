<script setup lang="ts">
withDefaults(
  defineProps<{
    percentage: number
    color?: 'primary' | 'warning' | 'danger' | 'success'
    size?: 'sm' | 'md'
    showText?: boolean
  }>(),
  {
    color: 'primary',
    size: 'md',
    showText: false,
  }
)

const colorMap: Record<string, string> = {
  primary: 'var(--ms-gradient-primary)',
  warning: 'var(--ms-warning)',
  danger: 'var(--ms-gradient-expense)',
  success: 'var(--ms-gradient-income)',
}
</script>

<template>
  <div class="ms-progress">
    <div v-if="showText" class="ms-progress-text">
      <slot name="label" />
      <span class="ms-progress-percentage">{{ Math.min(Math.round(percentage), 100) }}%</span>
    </div>
    <div class="ms-progress-bar" :class="size">
      <div
        class="ms-progress-fill"
        :style="{
          width: `${Math.min(percentage, 100)}%`,
          background: colorMap[color],
        }"
      ></div>
    </div>
  </div>
</template>

<style scoped>
.ms-progress {
  width: 100%;
}

.ms-progress-text {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
  font-size: var(--ms-text-sm);
  color: var(--ms-text-secondary);
}

.ms-progress-percentage {
  font-weight: var(--ms-font-semibold);
  color: var(--ms-text-primary);
}

.ms-progress-bar {
  border-radius: var(--ms-radius-full);
  background-color: var(--ms-bg-tertiary);
  overflow: hidden;
}

.ms-progress-bar.sm {
  height: 6px;
}

.ms-progress-bar.md {
  height: 8px;
}

.ms-progress-fill {
  height: 100%;
  border-radius: var(--ms-radius-full);
  transition: width 0.6s ease;
}
</style>
