<script setup lang="ts">
import { reactive } from 'vue'
import { useAppStore } from '../stores'

const props = defineProps<{ type: string }>()
const emit = defineEmits<{ (e: 'saved'): void }>()
const store = useAppStore()
const form = reactive({
  key: '',
  value: '',
})

const save = async () => {
  await store.upsertMemoryFact({
    fact_type: props.type,
    key: form.key || null,
    value_json: { text: form.value, scope: props.type === 'agent_role' ? 'analysis' : undefined },
    source: 'user',
  })
  form.key = ''
  form.value = ''
  emit('saved')
}
</script>

<template>
  <el-form inline>
    <el-form-item label="Key">
      <el-input v-model="form.key" />
    </el-form-item>
    <el-form-item label="内容">
      <el-input v-model="form.value" />
    </el-form-item>
    <el-form-item>
      <el-button type="primary" @click="save">保存</el-button>
    </el-form-item>
  </el-form>
</template>
