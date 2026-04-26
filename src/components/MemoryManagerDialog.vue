<script setup lang="ts">
const visible = defineModel<boolean>({ required: true })
</script>

<template>
  <el-dialog v-model="visible" title="记忆管理" width="900px">
    <slot />
  </el-dialog>
</template>
