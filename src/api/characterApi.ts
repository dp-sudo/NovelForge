import { invokeCommand } from "./tauriClient.js";
import type { CharacterInput } from "../domain/types.js";

export interface CharacterRow {
  id: string;
  project_id: string;
  name: string;
  aliases: string[];
  role_type: string;
  age: string | null;
  gender: string | null;
  identity_text: string | null;
  appearance: string | null;
  motivation: string | null;
  desire: string | null;
  fear: string | null;
  flaw: string | null;
  arc_stage: string | null;
  locked_fields: string[];
  notes: string | null;
  is_deleted: number;
  created_at: string;
  updated_at: string;
}

export async function listCharacters(projectRoot: string): Promise<CharacterRow[]> {
  return invokeCommand<CharacterRow[]>("list_characters", { projectRoot });
}

export async function createCharacter(input: CharacterInput, projectRoot: string): Promise<string> {
  return invokeCommand<string>("create_character", { projectRoot, input });
}

export async function updateCharacter(id: string, input: Partial<CharacterInput>, projectRoot: string): Promise<void> {
  await invokeCommand<void>("update_character", { projectRoot, input: { id, ...input } });
}

export async function deleteCharacter(id: string, projectRoot: string): Promise<void> {
  await invokeCommand<void>("delete_character", { projectRoot, id });
}

// ── AI Character Creation ──

export async function aiGenerateCharacter(projectRoot: string, userDescription: string): Promise<string> {
  return invokeCommand<string>("ai_generate_character", { input: { projectRoot, userDescription } });
}

// ── Character Relationships ──

export interface CharacterRelationship {
  id: string;
  sourceCharacterId: string;
  targetCharacterId: string;
  relationshipType: string;
  description: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface CreateRelationshipInput {
  sourceCharacterId: string;
  targetCharacterId: string;
  relationshipType: string;
  description?: string;
}

export async function listCharacterRelationships(projectRoot: string, characterId?: string): Promise<CharacterRelationship[]> {
  return invokeCommand<CharacterRelationship[]>("list_character_relationships", { projectRoot, characterId });
}

export async function createCharacterRelationship(projectRoot: string, input: CreateRelationshipInput): Promise<string> {
  return invokeCommand<string>("create_character_relationship", { projectRoot, input });
}

export async function deleteCharacterRelationship(projectRoot: string, id: string): Promise<void> {
  await invokeCommand<void>("delete_character_relationship", { projectRoot, id });
}

