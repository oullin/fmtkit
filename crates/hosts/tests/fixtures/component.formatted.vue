<script setup lang="ts">
import { computed, ref } from 'vue';

const props = defineProps<{ label: string }>();
const count = ref(0);

const doubled = computed(() => {
	return count.value * 2;
});
</script>

<template>
	<section class="counter" :class="{ active:count>0 }">
		<h2>{{ props.label }}</h2>
		<button type="button" @click="count++">Clicked {{ count }} times</button>
		<p v-if="doubled>10" style="color: red; font-weight: bold">Big</p>
		<ul><li v-for="item in [1,2,3]" :key="item">{{ item }}</li></ul>
	</section>
</template>

<style scoped>
.counter {
	display: flex;
	gap: 4px;
}
.counter .active {
	color: #fff;
}
</style>

<style lang="scss">
$gap: 8px;
.counter {
	button {
		margin: $gap;
	}
}
</style>

<i18n lang="yaml">
en:
  hello:   Hello
</i18n>
