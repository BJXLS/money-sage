<script setup lang="ts">
import { ref, watch } from 'vue'
import McpConfigPanel from './McpConfigPanel.vue'

const props = defineProps<{ modelValue: boolean }>()
const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  'change': []
}>()

const visible = ref(false)

watch(() => props.modelValue, (v) => { visible.value = v })
watch(visible, (v) => { emit('update:modelValue', v) })

function handleChange() {
  emit('change')
}
</script>

<template>
  <el-dialog
    v-model="visible"
    title="MCP 工具服务器"
    width="780px"
    :close-on-click-modal="false"
    class="mcp-dialog"
  >
    <McpConfigPanel class="dialog-panel" @change="handleChange" />
  </el-dialog>
</template>

<style scoped>
.dialog-panel {
  height: 520px;
}
</style>
