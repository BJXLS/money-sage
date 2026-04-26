<script setup lang="ts">
import { onMounted } from 'vue'
import { useAppStore, type RoleScope } from '../stores'

const props = defineProps<{ scope: RoleScope }>()
const store = useAppStore()

const onApply = async (presetId: string) => {
  await store.applyRolePreset(presetId, props.scope)
}

onMounted(async () => {
  await store.fetchRolePresets()
})
</script>

<template>
  <el-dropdown>
    <span class="trigger">🎭 Persona</span>
    <template #dropdown>
      <el-dropdown-menu>
        <el-dropdown-item
          v-for="preset in store.rolePresets"
          :key="preset.preset_id"
          @click="onApply(preset.preset_id)"
        >
          {{ preset.display_name }}
        </el-dropdown-item>
      </el-dropdown-menu>
    </template>
  </el-dropdown>
</template>

<style scoped>
.trigger { cursor: pointer; color: #a5b4fc; }
</style>
