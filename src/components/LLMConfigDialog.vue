<script setup lang="ts">
import { ref, watch } from 'vue'
import LLMConfigPanel from './LLMConfigPanel.vue'

const props = defineProps<{ modelValue: boolean }>()
const emit = defineEmits<{
  'update:modelValue': [value: boolean]
  'success': []
}>()

const visible = ref(false)

watch(() => props.modelValue, (v) => { visible.value = v })
watch(visible, (v) => { emit('update:modelValue', v) })

function handleSuccess() {
  emit('success')
}
</script>

<template>
  <el-dialog
    v-model="visible"
    title="大模型接口配置"
    width="760px"
    :close-on-click-modal="false"
    class="llm-dialog"
  >
    <LLMConfigPanel class="dialog-panel" @success="handleSuccess" />
  </el-dialog>
</template>

<style scoped>
.dialog-panel {
  height: 520px;
}
</style>
