<script setup lang="ts">
import { onMounted, ref, watch } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { useAppStore, type RolePreset, type RoleScope } from '../stores'
import PersonaPresetCard from './PersonaPresetCard.vue'
import PersonaPresetEditor from './PersonaPresetEditor.vue'

const store = useAppStore()
const loading = ref(false)
const scope = ref<RoleScope>('analysis')
const applyingPresetId = ref('')
const activePresetId = ref('')

const editorVisible = ref(false)
const editingPreset = ref<RolePreset | null>(null)

const load = async () => {
  loading.value = true
  try {
    await store.fetchRolePresets()
    await loadActivePreset()
  } finally {
    loading.value = false
  }
}

const loadActivePreset = async () => {
  const role = await store.getAgentRole(scope.value)
  const raw = (role?.value_json || {}) as any
  activePresetId.value = raw.preset_id || ''
}

const apply = async (presetId: string) => {
  applyingPresetId.value = presetId
  try {
    await store.applyRolePreset(presetId, scope.value)
    await loadActivePreset()
    ElMessage.success('人格预设已应用')
  } catch (error: any) {
    console.error('应用人格预设失败:', error)
    ElMessage.error(error?.toString?.() || '应用失败，请重试')
  } finally {
    applyingPresetId.value = ''
  }
}

const openCreate = () => {
  editingPreset.value = null
  editorVisible.value = true
}

const openEdit = (preset: RolePreset) => {
  editingPreset.value = preset
  editorVisible.value = true
}

const confirmDelete = async (preset: RolePreset) => {
  try {
    await ElMessageBox.confirm(
      `确定删除预设「${preset.display_name}」吗？此操作不可恢复。`,
      '删除预设',
      { type: 'warning', confirmButtonText: '删除', cancelButtonText: '取消' },
    )
  } catch {
    return
  }
  try {
    await store.deleteRolePreset(preset.preset_id)
    ElMessage.success('已删除')
  } catch (error: any) {
    console.error('删除预设失败:', error)
    ElMessage.error(error?.toString?.() || '删除失败')
  }
}

const confirmReset = async (preset: RolePreset) => {
  try {
    await ElMessageBox.confirm(
      `将预设「${preset.display_name}」重置为出厂内容？`,
      '重置默认',
      { type: 'warning', confirmButtonText: '重置', cancelButtonText: '取消' },
    )
  } catch {
    return
  }
  try {
    await store.resetRolePreset(preset.preset_id)
    ElMessage.success('已重置为默认')
  } catch (error: any) {
    console.error('重置预设失败:', error)
    ElMessage.error(error?.toString?.() || '重置失败')
  }
}

const onSaved = async () => {
  await store.fetchRolePresets()
}

onMounted(load)
watch(scope, () => {
  void loadActivePreset()
})
</script>

<template>
  <div>
    <div class="header-bar">
      <el-space>
        <span>作用域：</span>
        <el-select v-model="scope" style="width: 180px">
          <el-option label="全局" value="global" />
          <el-option label="快速记账" value="quick_note" />
          <el-option label="智能分析" value="analysis" />
        </el-select>
      </el-space>
      <el-button type="primary" @click="openCreate">+ 新增预设</el-button>
    </div>

    <el-row :gutter="12" style="margin-top: 12px">
      <el-col :span="8" v-for="preset in store.rolePresets" :key="preset.preset_id">
        <PersonaPresetCard
          :title="preset.display_name"
          :summary="preset.summary"
          :active="activePresetId === preset.preset_id"
          :is-builtin="preset.is_builtin"
          @apply="apply(preset.preset_id)"
          @edit="openEdit(preset)"
          @delete="confirmDelete(preset)"
          @reset="confirmReset(preset)"
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

    <el-empty v-if="!loading && !store.rolePresets.length" description="暂无预设" />

    <PersonaPresetEditor
      v-model="editorVisible"
      :preset="editingPreset"
      @saved="onSaved"
    />
  </div>
</template>

<style scoped>
.header-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
}
</style>
