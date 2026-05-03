<script setup lang="ts">
import { reactive, ref, watch } from 'vue'
import { ElMessage } from 'element-plus'
import { useAppStore, type RolePreset } from '../stores'

const props = defineProps<{
  modelValue: boolean
  preset?: RolePreset | null
}>()
const emit = defineEmits<{
  (e: 'update:modelValue', v: boolean): void
  (e: 'saved'): void
}>()

const store = useAppStore()
const saving = ref(false)

const form = reactive({
  display_name: '',
  summary: '',
  self_reference: '我',
  user_address: '你',
  tone: {
    style: 'formal',
    emoji: false,
    verbosity: 'normal',
    language_flavor: 'zh-casual',
  },
  notes: '',
})
const traitsText = ref('')
const doText = ref('')
const dontText = ref('')

const isEdit = () => !!props.preset
const isBuiltinEdit = () => !!props.preset?.is_builtin

const reset = () => {
  form.display_name = ''
  form.summary = ''
  form.self_reference = '我'
  form.user_address = '你'
  form.tone = { style: 'formal', emoji: false, verbosity: 'normal', language_flavor: 'zh-casual' }
  form.notes = ''
  traitsText.value = ''
  doText.value = ''
  dontText.value = ''
}

const fillFromPreset = (p: RolePreset) => {
  const v = (p.value || {}) as any
  form.display_name = p.display_name || v.display_name || ''
  form.summary = p.summary || ''
  form.self_reference = v.self_reference || '我'
  form.user_address = v.user_address || '你'
  form.tone = {
    style: v.tone?.style || 'formal',
    emoji: !!v.tone?.emoji,
    verbosity: v.tone?.verbosity || 'normal',
    language_flavor: v.tone?.language_flavor || 'zh-casual',
  }
  form.notes = v.notes || ''
  traitsText.value = Array.isArray(v.traits) ? v.traits.join('\n') : ''
  doText.value = Array.isArray(v.do) ? v.do.join('\n') : ''
  dontText.value = Array.isArray(v.dont) ? v.dont.join('\n') : ''
}

watch(
  () => [props.modelValue, props.preset],
  ([visible, preset]) => {
    if (!visible) return
    if (preset) fillFromPreset(preset as RolePreset)
    else reset()
  },
  { immediate: true },
)

const close = () => emit('update:modelValue', false)

const buildValue = () => ({
  display_name: form.display_name.trim(),
  self_reference: form.self_reference.trim() || undefined,
  user_address: form.user_address.trim() || undefined,
  tone: {
    style: form.tone.style || undefined,
    emoji: !!form.tone.emoji,
    verbosity: form.tone.verbosity || undefined,
    language_flavor: form.tone.language_flavor || undefined,
  },
  traits: traitsText.value.split('\n').map(s => s.trim()).filter(Boolean),
  do: doText.value.split('\n').map(s => s.trim()).filter(Boolean),
  dont: dontText.value.split('\n').map(s => s.trim()).filter(Boolean),
  notes: form.notes.trim() || undefined,
})

const save = async () => {
  const name = form.display_name.trim()
  if (!name) {
    ElMessage.warning('请填写显示名称')
    return
  }
  saving.value = true
  try {
    const value = buildValue()
    if (isEdit() && props.preset) {
      await store.updateRolePreset(props.preset.preset_id, {
        display_name: name,
        summary: form.summary,
        value,
      })
      ElMessage.success('预设已更新')
    } else {
      await store.createRolePreset({
        display_name: name,
        summary: form.summary,
        value,
      })
      ElMessage.success('预设已创建')
    }
    emit('saved')
    close()
  } catch (error: any) {
    console.error('保存预设失败:', error)
    ElMessage.error(error?.toString?.() || '保存失败，请重试')
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <el-dialog
    :model-value="modelValue"
    @update:model-value="emit('update:modelValue', $event)"
    :title="isEdit() ? (isBuiltinEdit() ? '编辑内置预设' : '编辑预设') : '新增预设'"
    width="640px"
    :close-on-click-modal="false"
    @closed="reset"
  >
    <el-alert
      v-if="isBuiltinEdit()"
      type="warning"
      :closable="false"
      title="内置预设：可修改但不可删除，可使用「重置默认」回到出厂内容"
      style="margin-bottom: 12px"
    />
    <el-form label-width="100px">
      <el-row :gutter="12">
        <el-col :span="12">
          <el-form-item label="显示名称" required>
            <el-input v-model="form.display_name" placeholder="例如：温柔教练" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="一句简介">
            <el-input v-model="form.summary" placeholder="将显示在预设卡片上" />
          </el-form-item>
        </el-col>
      </el-row>

      <el-row :gutter="12">
        <el-col :span="12">
          <el-form-item label="自称">
            <el-input v-model="form.self_reference" placeholder="例如：我 / 小Money" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="称呼用户">
            <el-input v-model="form.user_address" placeholder="例如：你 / 老板" />
          </el-form-item>
        </el-col>
      </el-row>

      <el-row :gutter="12">
        <el-col :span="8">
          <el-form-item label="语气风格">
            <el-select v-model="form.tone.style" style="width:100%">
              <el-option label="正式 formal" value="formal" />
              <el-option label="温柔 gentle" value="gentle" />
              <el-option label="简洁 concise" value="concise" />
              <el-option label="活泼 playful" value="playful" />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="8">
          <el-form-item label="详细程度">
            <el-select v-model="form.tone.verbosity" style="width:100%">
              <el-option label="简短 short" value="short" />
              <el-option label="普通 normal" value="normal" />
              <el-option label="详细 detailed" value="detailed" />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="8">
          <el-form-item label="允许 emoji">
            <el-switch v-model="form.tone.emoji" />
          </el-form-item>
        </el-col>
      </el-row>

      <el-form-item label="语言风味">
        <el-input v-model="form.tone.language_flavor" placeholder="例如：zh-casual / zh-formal" />
      </el-form-item>

      <el-form-item label="人格特质">
        <el-input v-model="traitsText" type="textarea" :rows="2" placeholder="每行一个，例如：耐心" />
      </el-form-item>
      <el-form-item label="应该做">
        <el-input v-model="doText" type="textarea" :rows="2" placeholder="每行一个，例如：先给结论再给依据" />
      </el-form-item>
      <el-form-item label="不要做">
        <el-input v-model="dontText" type="textarea" :rows="2" placeholder="每行一个，例如：不评判用户消费" />
      </el-form-item>
      <el-form-item label="备注">
        <el-input v-model="form.notes" type="textarea" :rows="2" placeholder="额外说明（可选）" />
      </el-form-item>
    </el-form>

    <template #footer>
      <el-button @click="close">取消</el-button>
      <el-button type="primary" :loading="saving" @click="save">保存</el-button>
    </template>
  </el-dialog>
</template>
