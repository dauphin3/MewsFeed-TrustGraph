<template>
  <div>{{ data }}</div>
</template>

<script setup lang="ts">
import { AppAgentClient } from "@holochain/client";
import { useQuery } from "@tanstack/vue-query";
import { inject, ComputedRef, watch } from "vue";

const client = (inject("client") as ComputedRef<AppAgentClient>).value;

const fetchNetworkInfo = () => client.networkInfo();
const { data, error, refetch } = useQuery({
  queryKey: ["networkInfo"],
  queryFn: fetchNetworkInfo,
  refetchOnMount: true,
});
watch(error, console.error);
</script>
