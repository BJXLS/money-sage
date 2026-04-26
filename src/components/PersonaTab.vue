<script setup lang="ts">
import { onMounted, reactive, ref, watch } from 'vue'
import { ElMessage } from 'element-plus'
import { useAppStore, type RoleScope, type RoleValue } from '../stores'
import PersonaPresetCard from './PersonaPresetCard.vue'

const store = useAppStore()
const loading = ref(false)
const scope = ref<RoleScope>('analysis')
const applyingPresetId = ref('')
const activePresetId = ref('')
const saving = ref(false)

const roleForm = reactive({
  scope: 'analysis' as RoleScope,
  display_name: '',
  self_reference: '',
  user_address: '',
  tone: {
    style: '',
    emoji: false,
    verbosity: 'normal',
    language_flavor: 'zh-casual',
  },
  traits: [],
  do: [],
  dont: [],
  preset_id: '',
  notes: '',
})
const traitsText = ref('')
const doText = ref('')
const dontText = ref('')

const load = async () => {
  loading.value = true
  try {
    await store.fetchRolePresets()
    await loadCurrentRole()
  } finally {
    loading.value = false
  }
}

const loadCurrentRole = async () => {
  const role = await store.getAgentRole(scope.value)
  const raw = (role?.value_json || {}) as any
  activePresetId.value = raw.preset_id || ''

  roleForm.scope = scope.value
  roleForm.display_name = raw.display_name || ''
  roleForm.self_reference = raw.self_reference || ''
  roleForm.user_address = raw.user_address || ''
  roleForm.tone = {
    style: raw.tone?.style || '',
    emoji: !!raw.tone?.emoji,
    verbosity: raw.tone?.verbosity || 'normal',
    language_flavor: raw.tone?.language_flavor || 'zh-casual',
  }
  roleForm.traits = Array.isArray(raw.traits) ? raw.traits : []
  roleForm.do = Array.isArray(raw.do) ? raw.do : []
  roleForm.dont = Array.isArray(raw.dont) ? raw.dont : []
  roleForm.preset_id = raw.preset_id || ''
  roleForm.notes = raw.notes || ''
  traitsText.value = (roleForm.traits || []).join('\n')
  doText.value = (roleForm.do || []).join('\n')
  dontText.value = (roleForm.dont || []).join('\n')
}

const save = async () => {
  saving.value = true
  try {
    const payload: RoleValue = {
      scope: scope.value,
      display_name: roleForm.display_name || undefined,
      self_reference: roleForm.self_reference || undefined,
      user_address: roleForm.user_address || undefined,
      tone: {
        style: roleForm.tone.style || undefined,
        emoji: !!roleForm.tone.emoji,
        verbosity: roleForm.tone.verbosity || undefined,
        language_flavor: roleForm.tone.language_flavor || undefined,
      },
      traits: traitsText.value.split('\n').map(s => s.trim()).filter(Boolean),
      do: doText.value.split('\n').map(s => s.trim()).filter(Boolean),
      dont: dontText.value.split('\n').map(s => s.trim()).filter(Boolean),
      preset_id: roleForm.preset_id || undefined,
      notes: roleForm.notes || undefined,
    }
    await store.setAgentRole(scope.value, payload)
    await loadCurrentRole()
    ElMessage.success('人格已保存')
  } catch (error: any) {
    console.error('保存人格失败:', error)
    ElMessage.error(error?.toString?.() || '保存失败，请重试')
  } finally {
    saving.value = false
  }
}

const apply = async (presetId: string) => {
  applyingPresetId.value = presetId
  try {
    await store.applyRolePreset(presetId, scope.value)
    await loadCurrentRole()
    ElMessage.success('人格预设已应用')
  } catch (error: any) {
    console.error('应用人格预设失败:', error)
    ElMessage.error(error?.toString?.() || '应用失败，请重试')
  } finally {
    applyingPresetId.value = ''
  }
}

onMounted(load)
watch(scope, () => {
  void loadCurrentRole()
})
</script>

<template>
  <div>
    <el-space>
      <span>作用域：</span>
      <el-select v-model="scope" style="width: 180px">
        <el-option label="全局" value="global" />
        <el-option label="快速记账" value="quick_note" />
        <el-option label="智能分析" value="analysis" />
      </el-select>
    </el-space>
    <el-row :gutter="12" style="margin-top: 12px">
      <el-col :span="8" v-for="preset in store.rolePresets" :key="preset.preset_id">
        <PersonaPresetCard
          :title="preset.display_name"
          :summary="preset.summary"
          :active="activePresetId === preset.preset_id"
          @use-preset="apply(preset.preset_id)"
        />
      </el-col>
    </el-row>
    <el-alert
      v-if="applyingPresetId"
      type="info"
      :closable="false"
      style="margin-top: 12px"
      :title="`正在应用预设：${applyingPresetId}`"
    />

    <el-card shadow="never" style="margin-top: 16px">
      <template #header>
        <div style="display:flex;justify-content:space-between;align-items:center;">
          <span>人格编辑</span>
          <el-button type="primary" :loading="saving" @click="save">保存人格</el-button>
        </div>
      </template>

      <el-form label-width="110px">
        <el-row :gutter="12">
          <el-col :span="12">
            <el-form-item label="显示名称">
              <el-input v-model="roleForm.display_name" placeholder="例如：Money 小管家" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="称呼用户">
              <el-input v-model="roleForm.user_address" placeholder="例如：你 / 老板" />
            </el-form-item>
          </el-col>
        </el-row>

        <el-row :gutter="12">
          <el-col :span="12">
            <el-form-item label="自称">
              <el-input v-model="roleForm.self_reference" placeholder="例如：我" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="语言风格">
              <el-input v-model="roleForm.tone.language_flavor" placeholder="例如：zh-casual" />
            </el-form-item>
          </el-col>
        </el-row>

        <el-row :gutter="12">
          <el-col :span="8">
            <el-form-item label="语气风格">
              <el-select v-model="roleForm.tone.style" style="width:100%">
                <el-option label="正式 formal" value="formal" />
                <el-option label="温柔 gentle" value="gentle" />
                <el-option label="简洁 concise" value="concise" />
                <el-option label="活泼 playful" value="playful" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="详细程度">
              <el-select v-model="roleForm.tone.verbosity" style="width:100%">
                <el-option label="简短 short" value="short" />
                <el-option label="普通 normal" value="normal" />
                <el-option label="详细 detailed" value="detailed" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="允许 emoji">
              <el-switch v-model="roleForm.tone.emoji" />
            </el-form-item>
          </el-col>
        </el-row>

        <el-form-item label="人格特质">
          <el-input v-model="traitsText" type="textarea" :rows="3" placeholder="每行一个，例如：耐心" />
        </el-form-item>
        <el-form-item label="应该做">
          <el-input v-model="doText" type="textarea" :rows="3" placeholder="每行一个，例如：先给结论再给依据" />
        </el-form-item>
        <el-form-item label="不要做">
          <el-input v-model="dontText" type="textarea" :rows="3" placeholder="每行一个，例如：不要评判用户消费" />
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="roleForm.notes" type="textarea" :rows="2" placeholder="额外说明（可选）" />
        </el-form-item>
      </el-form>
    </el-card>

    <el-empty v-if="!loading && !store.rolePresets.length" description="暂无预设" />
  </div>
</template>
