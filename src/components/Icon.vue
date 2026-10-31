<script setup lang="ts">
import { icons, type IconName } from '../icons';

defineProps<{ name: IconName }>();
</script>

<template>
  <span class="icon" aria-hidden="true" v-html="icons[name]" />
</template>
