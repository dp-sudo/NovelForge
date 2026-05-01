import { invokeCommand } from "./tauriClient.js";
import type { ModelPool, ModelPoolEntry } from "../types/modelPool.js";

export interface CreateModelPoolInput {
  name: string;
  poolType: string;
  models: ModelPoolEntry[];
}

export async function listModelPools(): Promise<ModelPool[]> {
  return invokeCommand<ModelPool[]>("list_model_pools");
}

export async function createModelPool(input: CreateModelPoolInput): Promise<ModelPool> {
  return invokeCommand<ModelPool>("create_model_pool", { input });
}

export async function updateModelPool(poolId: string, config: ModelPool): Promise<ModelPool> {
  return invokeCommand<ModelPool>("update_model_pool", { poolId, config });
}

export async function deleteModelPool(poolId: string): Promise<void> {
  await invokeCommand<void>("delete_model_pool", { poolId });
}
