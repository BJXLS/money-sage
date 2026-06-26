<script setup lang="ts">
withDefaults(
  defineProps<{
    label: string
    value: string | number
    trend?: number
    trendLabel?: string
    type?: 'neutral' | 'income' | 'expense' | 'primary'
    icon?: string
  }>(),
  {
    type: 'neutral',
    trendLabel: '较上月',
  }
)

const typeColors: Record<string, string> = {
  neutral: 'var(--ms-text-primary)',
  income: 'var(--ms-income)',
  expense: 'var(--ms-expense)',
  primary: 'var(--ms-primary-500)',
}

const iconBgColors: Record<string, string> = {
  neutral: 'rgba(99, 102, 241, 0.1)',
  income: 'rgba(16, 185, 129, 0.1)',
  expense: 'rgba(244, 63, 94, 0.1)',
  primary: 'rgba(99, 102, 241, 0.1)',
}
</script>

<template>
  <div class="ms-stat-card">
    <div class="ms-stat-header">
      <div class="ms-stat-icon" :style="{ backgroundColor: iconBgColors[type] }">
        <span v-if="icon">{{ icon }}</span>
        <svg
          v-else
          class="w-5 h-5"
          :style="{ color: typeColors[type] }"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
        >
          <path
            v-if="type === 'income'"
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M7 11l5-5m0 0l5 5m-5-5v12"
          />
          <path
            v-else-if="type === 'expense'"
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M17 13l-5 5m0 0l-5-5m5 5V6"
          />
          <path
            v-else
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
          />
        </svg>
      </div>
      <span class="ms-stat-label">{{ label }}</span>
    </div>
    <div class="ms-stat-value" :style="{ color: typeColors[type] }">
      {{ value }}
    </div>
    <div v-if="trend !== undefined" class="ms-stat-trend">
      <span :class="['ms-stat-trend-value', trend >= 0 ? 'up' : 'down']">
        {{ trend >= 0 ? '↑' : '↓' }} {{ Math.abs(trend) }}%
      </span>
      <span class="ms-stat-trend-label">{{ trendLabel }}</span>
    </div>
  </div>
</template>

<style scoped>
.ms-stat-card {
  background-color: var(--ms-surface-primary);
  border: 1px solid var(--ms-border-subtle);
  border-radius: var(--ms-radius-xl);
  box-shadow: var(--ms-shadow-sm);
  padding: 20px;
  transition: box-shadow 0.2s ease;
}

.ms-stat-card:hover {
  box-shadow: var(--ms-shadow-md);
}

.ms-stat-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
}

.ms-stat-icon {
  width: 36px;
  height: 36px;
  border-radius: var(--ms-radius-md);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 18px;
  flex-shrink: 0;
}

.ms-stat-label {
  font-size: var(--ms-text-sm);
  color: var(--ms-text-secondary);
}

.ms-stat-value {
  font-size: var(--ms-text-2xl);
  font-weight: var(--ms-font-bold);
  font-variant-numeric: tabular-nums;
  letter-spacing: -0.02em;
}

.ms-stat-trend {
  margin-top: 8px;
  font-size: var(--ms-text-xs);
}

.ms-stat-trend-value {
  font-weight: var(--ms-font-medium);
}

.ms-stat-trend-value.up {
  color: var(--ms-income);
}

.ms-stat-trend-value.down {
  color: var(--ms-expense);
}

.ms-stat-trend-label {
  color: var(--ms-text-tertiary);
  margin-left: 6px;
}
</style>
